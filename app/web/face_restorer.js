// Hybrid On-Device Face Detection (YuNet 228 KB MIT) + Face Restoration (GFPGANv1.4 Apache-2.0 WebGPU)
// Spec 145: Lazy-loaded ONLY when WebGPU is available AND a human face is detected on a photo/portrait.
// Runs 100% locally on the receiver's GPU (ZERO server GPU, ZERO plaintext upload, E2EE untouched).
(() => {
  const MODEL_CACHE_NAME = 'chatw-ai-models-v1';
  const YUNET_URL = '/models/face_detection_yunet_2023mar.onnx';
  const GFPGAN_URLS = [
    '/models/GFPGANv1.4.onnx',
    'https://huggingface.co/neurobytemind/GFPGANv1.4.onnx/resolve/main/GFPGANv1.4.onnx',
  ];
  const ORT_SCRIPT_URL = '/ort/ort.all.min.js';
  const ORT_WASM_PATHS = '/ort/';

  // Canonical FFHQ / GFPGAN 5-point landmarks for 512x512 aligned face
  const DST_LM = [
    [192.98138, 239.94708],
    [318.90277, 240.19360],
    [256.63416, 314.01935],
    [201.26117, 371.41043],
    [313.08905, 371.15118],
  ];

  let ortPromise = null;
  let yunetSession = null;
  let gfpganSession = null;
  let webgpuSupported = null;
  let gfpganDisabled = false;

  function withTimeout(promise, ms, label) {
    let timer = null;
    return Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`timeout:${label}:${ms}ms`)), ms);
      }),
    ]).finally(() => {
      if (timer !== null) clearTimeout(timer);
    });
  }

  async function isWebGpuAvailable() {
    if (gfpganDisabled) return false;
    if (webgpuSupported !== null) return webgpuSupported;
    try {
      if (!navigator.gpu) {
        webgpuSupported = false;
        return false;
      }
      const adapter = await withTimeout(navigator.gpu.requestAdapter(), 1500, 'gpu_adapter');
      if (!adapter || adapter.isFallbackAdapter) {
        webgpuSupported = false;
        return false;
      }
      const device = await withTimeout(adapter.requestDevice(), 1500, 'gpu_device');
      if (!device) {
        webgpuSupported = false;
        return false;
      }
      try { device.destroy(); } catch (_) {}
      webgpuSupported = true;
      return true;
    } catch (_) {
      webgpuSupported = false;
      return false;
    }
  }

  function loadOrt() {
    if (window.ort) return Promise.resolve(window.ort);
    if (ortPromise) return ortPromise;
    ortPromise = new Promise((resolve, reject) => {
      const s = document.createElement('script');
      s.src = ORT_SCRIPT_URL;
      s.async = true;
      s.onload = () => {
        if (window.ort) {
          window.ort.env.wasm.wasmPaths = ORT_WASM_PATHS;
          window.ort.env.wasm.numThreads = 1;
          resolve(window.ort);
        } else {
          reject(new Error('ORT loaded but window.ort missing'));
        }
      };
      s.onerror = () => reject(new Error('Failed to load ONNX Runtime Web'));
      document.head.appendChild(s);
    });
    return ortPromise;
  }

  async function fetchModelBytesCached(urls, minBytes = 100_000, allowCachePut = true) {
    let cache = null;
    if (allowCachePut) {
      try {
        if (typeof caches !== 'undefined') cache = await caches.open(MODEL_CACHE_NAME);
      } catch (_) {}
    }

    const cacheKey = urls[0];
    if (cache) {
      try {
        const hit = await withTimeout(cache.match(cacheKey), 1500, 'cache_match');
        if (hit && hit.ok) {
          const buf = await hit.arrayBuffer();
          if (buf.byteLength >= minBytes) return new Uint8Array(buf);
        }
      } catch (_) {}
    }

    for (const url of urls) {
      try {
        const resp = await withTimeout(
          fetch(url, { mode: url.startsWith('http') ? 'cors' : 'same-origin', cache: 'force-cache' }),
          6000,
          'model_fetch',
        );
        if (!resp.ok) continue;
        const buf = await withTimeout(resp.arrayBuffer(), 6000, 'model_read');
        if (buf.byteLength < minBytes) continue;
        // Only write small models (< 50 MB) to CacheStorage; large ONNX blobs use browser HTTP cache to avoid disk-quota hangs!
        if (cache && allowCachePut && buf.byteLength < 50_000_000) {
          try {
            await withTimeout(
              cache.put(cacheKey, new Response(buf.slice(0), { headers: { 'Content-Type': 'application/octet-stream' } })),
              2000,
              'cache_put',
            );
          } catch (_) {}
        }
        return new Uint8Array(buf);
      } catch (_) {}
    }
    return null;
  }

  async function getYunetSession(ort) {
    if (yunetSession) return yunetSession;
    const bytes = await fetchModelBytesCached([YUNET_URL], 100_000, true);
    if (!bytes) throw new Error('YuNet model unavailable');
    yunetSession = await withTimeout(
      ort.InferenceSession.create(bytes, { executionProviders: ['wasm'] }),
      4000,
      'yunet_init',
    );
    return yunetSession;
  }

  async function getGfpganSession(ort) {
    if (gfpganDisabled) throw new Error('GFPGAN disabled after previous timeout');
    if (gfpganSession) return gfpganSession;
    try {
      const bytes = await fetchModelBytesCached(GFPGAN_URLS, 100_000_000, false);
      if (!bytes) throw new Error('GFPGANv1.4 model unavailable');
      gfpganSession = await withTimeout(
        ort.InferenceSession.create(bytes, { executionProviders: ['webgpu'] }),
        8000,
        'gfpgan_init',
      );
      return gfpganSession;
    } catch (err) {
      gfpganDisabled = true;
      throw err;
    }
  }

  function iou(a, b) {
    const x1 = Math.max(a[0], b[0]);
    const y1 = Math.max(a[1], b[1]);
    const x2 = Math.min(a[0] + a[2], b[0] + b[2]);
    const y2 = Math.min(a[1] + a[3], b[1] + b[3]);
    const inter = Math.max(0, x2 - x1) * Math.max(0, y2 - y1);
    const union = a[2] * a[3] + b[2] * b[3] - inter;
    return union > 0 ? inter / union : 0;
  }

  function fitSimilarity(src, dst) {
    let sx = 0, sy = 0, dx = 0, dy = 0;
    const n = src.length;
    for (let i = 0; i < n; i++) {
      sx += src[i][0]; sy += src[i][1];
      dx += dst[i][0]; dy += dst[i][1];
    }
    sx /= n; sy /= n; dx /= n; dy /= n;
    let a = 0, b = 0, denom = 0;
    for (let i = 0; i < n; i++) {
      const xs = src[i][0] - sx, ys = src[i][1] - sy;
      const xd = dst[i][0] - dx, yd = dst[i][1] - dy;
      a += xs * xd + ys * yd;
      b += xs * yd - ys * xd;
      denom += xs * xs + ys * ys;
    }
    if (denom < 1e-6) return null;
    const m00 = a / denom;
    const m10 = b / denom;
    const tx = dx - (m00 * sx - m10 * sy);
    const ty = dy - (m10 * sx + m00 * sy);
    return [m00, m10, -m10, m00, tx, ty];
  }

  async function detectFaces(ort, canvas) {
    const sess = await getYunetSession(ort);
    const W = canvas.width;
    const H = canvas.height;
    const detSize = 640;
    const scaleDet = Math.min(detSize / W, detSize / H);
    const dw = Math.round(W * scaleDet);
    const dh = Math.round(H * scaleDet);

    const cDet = document.createElement('canvas');
    cDet.width = detSize;
    cDet.height = detSize;
    const ctxDet = cDet.getContext('2d', { willReadFrequently: true });
    ctxDet.fillStyle = '#000';
    ctxDet.fillRect(0, 0, detSize, detSize);
    ctxDet.drawImage(canvas, 0, 0, dw, dh);
    const detData = ctxDet.getImageData(0, 0, detSize, detSize).data;

    const dhw = detSize * detSize;
    const detIn = new Float32Array(3 * dhw);
    for (let i = 0; i < dhw; i++) {
      detIn[i] = detData[i * 4 + 2];
      detIn[dhw + i] = detData[i * 4 + 1];
      detIn[2 * dhw + i] = detData[i * 4 + 0];
    }

    const out = await sess.run({ [sess.inputNames[0]]: new ort.Tensor('float32', detIn, [1, 3, detSize, detSize]) });
    const raw = [];
    for (const stride of [8, 16, 32]) {
      const cls = out[`cls_${stride}`].data;
      const obj = out[`obj_${stride}`].data;
      const bbox = out[`bbox_${stride}`].data;
      const kps = out[`kps_${stride}`].data;
      const cols = detSize / stride;
      const rows = detSize / stride;
      for (let r = 0; r < rows; r++) {
        for (let c = 0; c < cols; c++) {
          const idx = r * cols + c;
          const score = Math.sqrt(Math.max(0, cls[idx]) * Math.max(0, obj[idx]));
          if (score < 0.55) continue;
          const cx = (c + bbox[idx * 4 + 0]) * stride / scaleDet;
          const cy = (r + bbox[idx * 4 + 1]) * stride / scaleDet;
          const bw = Math.exp(bbox[idx * 4 + 2]) * stride / scaleDet;
          const bh = Math.exp(bbox[idx * 4 + 3]) * stride / scaleDet;
          if (Math.min(bw, bh) < 36) continue;
          const lm = [];
          for (let k = 0; k < 5; k++) {
            lm.push([
              (c + kps[idx * 10 + k * 2 + 0]) * stride / scaleDet,
              (r + kps[idx * 10 + k * 2 + 1]) * stride / scaleDet,
            ]);
          }
          raw.push({ score, box: [cx - bw / 2, cy - bh / 2, bw, bh], lm });
        }
      }
    }

    // NMS deduplication (top 4 faces max)
    raw.sort((a, b) => b.score - a.score);
    const kept = [];
    for (const cand of raw) {
      if (kept.every((k) => iou(cand.box, k.box) < 0.35)) {
        kept.push(cand);
        if (kept.length >= 4) break;
      }
    }
    return kept;
  }

  /**
   * Hybrid Stage 2B: Detect human faces on `srcCanvas` (already upscaled by NomosUni SPAN)
   * and seamlessly restore eyes, eyelashes, lips & facial texture via WebGPU GFPGANv1.4.
   * Returns `{ applied, facesRestored, canvas, detMs, gfpMs }` (never throws).
   */
  async function restoreFacesOnCanvas(srcCanvas, opts = {}) {
    try {
      if (!srcCanvas || srcCanvas.width < 128 || srcCanvas.height < 128) {
        return { applied: false, reason: 'canvas_too_small', canvas: srcCanvas };
      }
      if (opts.signal && opts.signal.aborted) {
        return { applied: false, reason: 'aborted', canvas: srcCanvas };
      }
      if (!(await isWebGpuAvailable())) {
        return { applied: false, reason: 'webgpu_unavailable', canvas: srcCanvas };
      }

      const ort = await loadOrt();
      if (opts.signal && opts.signal.aborted) {
        return { applied: false, reason: 'aborted', canvas: srcCanvas };
      }

      const tDet0 = performance.now();
      const faces = await detectFaces(ort, srcCanvas);
      const detMs = Math.round(performance.now() - tDet0);
      if (!faces.length) {
        return { applied: false, reason: 'no_faces_detected', detMs, canvas: srcCanvas };
      }
      if (opts.signal && opts.signal.aborted) {
        return { applied: false, reason: 'aborted', detMs, canvas: srcCanvas };
      }

      const gfpSess = await getGfpganSession(ort);
      if (opts.signal && opts.signal.aborted) {
        return { applied: false, reason: 'aborted', detMs, canvas: srcCanvas };
      }

      // Copy source canvas into a mutable 2D canvas for seamless masked compositing
      const W = srcCanvas.width;
      const H = srcCanvas.height;
      const outCanvas = document.createElement('canvas');
      outCanvas.width = W;
      outCanvas.height = H;
      const outCtx = outCanvas.getContext('2d', { willReadFrequently: true });
      outCtx.drawImage(srcCanvas, 0, 0);

      // Adaptive identity-safe blend alpha: stronger (0.82) for crushed low-res photos (< 720px),
      // gentler (0.55) for already high-res photos (>= 720px) so original eyelashes & pores stay 100% authentic!
      const srcMaxDim = Number(opts.srcMaxDim || 0);
      const coreAlpha = srcMaxDim > 0 && srcMaxDim < 720 ? 0.82 : 0.55;

      const cAlign = document.createElement('canvas');
      cAlign.width = 512;
      cAlign.height = 512;
      const ctxAlign = cAlign.getContext('2d', { willReadFrequently: true });
      const cFace = document.createElement('canvas');
      cFace.width = 512;
      cFace.height = 512;
      const ctxFace = cFace.getContext('2d');

      const ghw = 512 * 512;
      const gfpIn = new Float32Array(3 * ghw);
      const tGfp0 = performance.now();
      let restored = 0;

      for (const face of faces) {
        if (opts.signal && opts.signal.aborted) break;
        const M = fitSimilarity(face.lm, DST_LM);
        if (!M) continue;

        ctxAlign.setTransform(1, 0, 0, 1, 0, 0);
        ctxAlign.clearRect(0, 0, 512, 512);
        ctxAlign.imageSmoothingEnabled = true;
        ctxAlign.imageSmoothingQuality = 'high';
        ctxAlign.setTransform(M[0], M[1], M[2], M[3], M[4], M[5]);
        ctxAlign.drawImage(outCanvas, 0, 0);
        const alignU8 = ctxAlign.getImageData(0, 0, 512, 512).data;

        for (let i = 0; i < ghw; i++) {
          gfpIn[i] = (alignU8[i * 4 + 0] / 255 - 0.5) * 2;
          gfpIn[ghw + i] = (alignU8[i * 4 + 1] / 255 - 0.5) * 2;
          gfpIn[2 * ghw + i] = (alignU8[i * 4 + 2] / 255 - 0.5) * 2;
        }

        const outMap = await withTimeout(
          gfpSess.run({ [gfpSess.inputNames[0]]: new ort.Tensor('float32', gfpIn, [1, 3, 512, 512]) }),
          6000,
          'gfpgan_run',
        );
        const gfpOut = outMap[gfpSess.outputNames[0]].data;

        // Local skin-tone color lock + smooth cosine-feathered elliptical mask (ZERO box seam, ZERO background touch!)
        let sr = 0, sg = 0, sb = 0, gr = 0, gg = 0, gb = 0, cnt = 0;
        const gfpU8 = new Uint8ClampedArray(ghw * 4);
        for (let y = 0; y < 512; y++) {
          for (let x = 0; x < 512; x++) {
            const i = y * 512 + x;
            const r = Math.max(0, Math.min(255, (gfpOut[i] * 0.5 + 0.5) * 255));
            const g = Math.max(0, Math.min(255, (gfpOut[ghw + i] * 0.5 + 0.5) * 255));
            const b = Math.max(0, Math.min(255, (gfpOut[2 * ghw + i] * 0.5 + 0.5) * 255));
            gfpU8[i * 4 + 0] = r;
            gfpU8[i * 4 + 1] = g;
            gfpU8[i * 4 + 2] = b;

            const nx = (x - 256) / 175;
            const ny = (y - 266) / 205;
            const d = Math.hypot(nx, ny);
            if (d < 0.72) {
              sr += alignU8[i * 4 + 0]; sg += alignU8[i * 4 + 1]; sb += alignU8[i * 4 + 2];
              gr += r; gg += g; gb += b;
              cnt++;
            }
            let alpha = 0;
            if (d < 0.62) alpha = coreAlpha;
            else if (d < 1.0) alpha = coreAlpha * 0.5 * (1 + Math.cos(Math.PI * (d - 0.62) / 0.38));
            gfpU8[i * 4 + 3] = Math.round(alpha * 255);
          }
        }
        const dr = (sr - gr) / Math.max(1, cnt);
        const dg = (sg - gg) / Math.max(1, cnt);
        const db = (sb - gb) / Math.max(1, cnt);
        for (let i = 0; i < ghw; i++) {
          gfpU8[i * 4 + 0] = Math.max(0, Math.min(255, gfpU8[i * 4 + 0] + dr * 0.85));
          gfpU8[i * 4 + 1] = Math.max(0, Math.min(255, gfpU8[i * 4 + 1] + dg * 0.85));
          gfpU8[i * 4 + 2] = Math.max(0, Math.min(255, gfpU8[i * 4 + 2] + db * 0.85));
        }

        ctxFace.putImageData(new ImageData(gfpU8, 512, 512), 0, 0);

        const det = M[0] * M[3] - M[1] * M[2];
        if (Math.abs(det) < 1e-8) continue;
        const inv = [
          M[3] / det, -M[1] / det, -M[2] / det, M[0] / det,
          (M[2] * M[5] - M[3] * M[4]) / det, (M[1] * M[4] - M[0] * M[5]) / det,
        ];
        outCtx.save();
        outCtx.imageSmoothingEnabled = true;
        outCtx.imageSmoothingQuality = 'high';
        outCtx.setTransform(inv[0], inv[1], inv[2], inv[3], inv[4], inv[5]);
        outCtx.drawImage(cFace, 0, 0);
        outCtx.restore();
        restored++;
      }

      const gfpMs = Math.round(performance.now() - tGfp0);
      return {
        applied: restored > 0,
        facesRestored: restored,
        detMs,
        gfpMs,
        coreAlpha,
        canvas: restored > 0 ? outCanvas : srcCanvas,
      };
    } catch (err) {
      return { applied: false, reason: String(err), canvas: srcCanvas };
    }
  }

  window.__chatwFaceRestorer = {
    isWebGpuAvailable,
    restoreFacesOnCanvas,
  };
})();
