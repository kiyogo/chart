// Client-side Adaptive Image/Avatar/Video/Voice Optimizer & WebRTC Peer Transfer (Spec 022)
(() => {
  const b64ToBytes = (b64) => {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  };
  const bytesToB64 = (buf) => {
    const u8 = buf instanceof Uint8Array ? buf : new Uint8Array(buf);
    let s = '';
    for (let i = 0; i < u8.length; i += 0x8000) {
      s += String.fromCharCode.apply(null, u8.subarray(i, i + 0x8000));
    }
    return btoa(s);
  };

  async function sniffBytesFormat(u8) {
    if (u8.length >= 8 && u8[0] === 0x89 && u8[1] === 0x50 && u8[2] === 0x4e && u8[3] === 0x47) return { ext: 'png', mime: 'image/png', animated: false };
    if (u8.length >= 3 && u8[0] === 0xff && u8[1] === 0xd8 && u8[2] === 0xff) return { ext: 'jpg', mime: 'image/jpeg', animated: false };
    if (u8.length >= 6 && String.fromCharCode(...u8.slice(0, 6)).startsWith('GIF8')) return { ext: 'gif', mime: 'image/gif', animated: true };
    if (u8.length >= 12 && String.fromCharCode(...u8.slice(0, 4)) === 'RIFF' && String.fromCharCode(...u8.slice(8, 12)) === 'WEBP') {
      const txt = String.fromCharCode(...u8.slice(0, Math.min(u8.length, 4096)));
      return { ext: 'webp', mime: 'image/webp', animated: txt.includes('ANIM') };
    }
    if (u8.length >= 12 && String.fromCharCode(...u8.slice(4, 8)) === 'ftyp') {
      const brand = String.fromCharCode(...u8.slice(8, 12));
      if (brand === 'avif' || brand === 'avis') return { ext: 'avif', mime: 'image/avif', animated: brand === 'avis' };
      if (['isom', 'mp42', 'qt  ', 'avc1'].includes(brand)) return { ext: 'mp4', mime: 'video/mp4', animated: false };
    }
    if (u8.length >= 4 && u8[0] === 0x1a && u8[1] === 0x45 && u8[2] === 0xdf && u8[3] === 0xa3) return { ext: 'webm', mime: 'video/webm', animated: false };
    return { ext: 'bin', mime: 'application/octet-stream', animated: false };
  }

  function detectCrispTextOrAlpha(canvas) {
    const w = Math.min(256, canvas.width);
    const h = Math.min(256, canvas.height);
    if (w < 4 || h < 4) return { isCrisp: false, hasAlpha: false };
    const probe = document.createElement('canvas');
    probe.width = w;
    probe.height = h;
    const ctx = probe.getContext('2d');
    ctx.drawImage(canvas, 0, 0, w, h);
    const data = ctx.getImageData(0, 0, w, h).data;
    let sharp = 0;
    let flat = 0;
    let hasAlpha = false;
    const total = (w - 1) * (h - 1);
    for (let y = 0; y < h - 1; y++) {
      for (let x = 0; x < w - 1; x++) {
        const i = (y * w + x) * 4;
        if (data[i + 3] < 250) hasAlpha = true;
        const ir = i + 4;
        const id = i + w * 4;
        const dh = Math.abs(data[i] - data[ir]) + Math.abs(data[i + 1] - data[ir + 1]) + Math.abs(data[i + 2] - data[ir + 2]);
        const dv = Math.abs(data[i] - data[id]) + Math.abs(data[i + 1] - data[id + 1]) + Math.abs(data[i + 2] - data[id + 2]);
        if (dh > 120 || dv > 120) sharp++;
        else if (dh <= 4 && dv <= 4) flat++;
      }
    }
    const isCrisp = hasAlpha || (flat / total > 0.45 && sharp / total > 0.005);
    return { isCrisp, hasAlpha };
  }

  function canvasToBlobPromise(canvas, type, quality) {
    return new Promise((resolve) => canvas.toBlob((b) => resolve(b), type, quality));
  }

  // Genuine AVIF ONLY via native canvas (strictly magic-byte verified `ftypavif` — ZERO silent JPEG/WebP/PNG fallback!)
  async function tryNativeCanvasAvif(canvas, quality) {
    try {
      const blob = await canvasToBlobPromise(canvas, 'image/avif', quality);
      if (blob && blob.size > 0 && blob.type === 'image/avif') {
        const u8 = new Uint8Array(await blob.arrayBuffer());
        const sniffed = await sniffBytesFormat(u8);
        if (sniffed.ext === 'avif') {
          return { u8, sniffed };
        }
      }
    } catch (_) {}
    return null;
  }

  // Adaptive resolution policy:
  // High-Fidelity & Pristine Detail Preservation (Spec 136):
  // - Images up to 1920px (e.g. 1024x768, 1280x720, 1920x1080) retain 100% full original resolution (tw = origW, th = origH).
  // - Gigantic images > 2048px (e.g. 48MP raw camera captures) are capped to 1920px max dimension.
  // - In data_saver mode, clamp to 1280px max dimension when > 1280px.
  function adaptiveMaxEdge(w, h, isCrisp, qualityMode) {
    if (qualityMode === 'original') return null;
    const maxEdge = Math.max(w, h);
    if (qualityMode === 'data_saver') {
      return maxEdge > 1280 ? 1280 : null;
    }
    if (maxEdge > 2048) {
      return 1920;
    }
    return null;
  }

  function getUserMediaQualityMode(preserveOriginal) {
    if (preserveOriginal) return 'original';
    try {
      const m = localStorage.getItem('chatw_media_quality_mode');
      if (m === 'original' || m === 'data_saver' || m === 'ultra_optimized') return m;
    } catch (_) {}
    return 'ultra_optimized';
  }

  // In-memory sender pending file registry for Instant Sender Original Preview (< 20ms)
  const pendingSenderFiles = new Map();
  const previewUrlToFile = new Map();
  const senderOriginalSession = new Map();
  let nextHandleSeq = 1;

  function stashPreviewFile(previewUrl, file, width = 0, height = 0, opts = {}) {
    if (!previewUrl || !file) return;
    const prev = previewUrlToFile.get(previewUrl) || {};
    previewUrlToFile.set(previewUrl, {
      file,
      width: width || prev.width || 0,
      height: height || prev.height || 0,
      preserveOriginal: opts.preserveOriginal !== undefined ? Boolean(opts.preserveOriginal) : Boolean(prev.preserveOriginal),
      allowSenderOriginal: opts.allowSenderOriginal !== undefined ? Boolean(opts.allowSenderOriginal) : (prev.allowSenderOriginal !== undefined ? prev.allowSenderOriginal : true),
    });
    while (previewUrlToFile.size > 32) {
      const oldestKey = previewUrlToFile.keys().next().value;
      previewUrlToFile.delete(oldestKey);
      let stillPending = false;
      for (const p of pendingSenderFiles.values()) {
        if (p && p.previewUrl === oldestKey) { stillPending = true; break; }
      }
      if (!stillPending && String(oldestKey).startsWith('blob:')) {
        try { URL.revokeObjectURL(oldestKey); } catch (_) {}
      }
    }
  }

  async function probeVideoCodecs() {
    const caps = {
      webCodecsAvailable: typeof VideoEncoder !== 'undefined',
      av1EncodeSupported: false,
      av1HardwareSupported: false,
      vp9EncodeSupported: false,
    };
    if (!caps.webCodecsAvailable || typeof VideoEncoder.isConfigSupported !== 'function') {
      return caps;
    }
    try {
      const hw = await VideoEncoder.isConfigSupported({
        codec: 'av01.0.04M.08',
        width: 1280,
        height: 720,
        bitrate: 1_200_000,
        framerate: 30,
        hardwareAcceleration: 'prefer-hardware',
      });
      caps.av1EncodeSupported = Boolean(hw && hw.supported);
      if (caps.av1EncodeSupported) {
        const sw = await VideoEncoder.isConfigSupported({
          codec: 'av01.0.04M.08',
          width: 1280,
          height: 720,
          bitrate: 1_200_000,
          framerate: 30,
          hardwareAcceleration: 'prefer-software',
        });
        caps.av1HardwareSupported = Boolean(hw.supported && (!sw.supported || hw.config?.hardwareAcceleration === 'prefer-hardware'));
      }
    } catch (_) {}
    try {
      const vp9 = await VideoEncoder.isConfigSupported({
        codec: 'vp09.00.10.08',
        width: 1280,
        height: 720,
        bitrate: 1_500_000,
        framerate: 30,
      });
      caps.vp9EncodeSupported = Boolean(vp9 && vp9.supported);
    } catch (_) {}
    return caps;
  }

  async function drawMultiStepResized(bitmap, targetW, targetH) {
    let cw = bitmap.width;
    let ch = bitmap.height;
    let srcCanvas = document.createElement('canvas');
    srcCanvas.width = cw;
    srcCanvas.height = ch;
    let sctx = srcCanvas.getContext('2d');
    sctx.imageSmoothingEnabled = true;
    sctx.imageSmoothingQuality = 'high';
    sctx.drawImage(bitmap, 0, 0);

    // Step-down halving prevents jagged text aliasing
    while (cw * 0.5 > targetW && ch * 0.5 > targetH) {
      const nw = Math.max(targetW, Math.round(cw * 0.5));
      const nh = Math.max(targetH, Math.round(ch * 0.5));
      const next = document.createElement('canvas');
      next.width = nw;
      next.height = nh;
      const nctx = next.getContext('2d');
      nctx.imageSmoothingEnabled = true;
      nctx.imageSmoothingQuality = 'high';
      nctx.drawImage(srcCanvas, 0, 0, nw, nh);
      srcCanvas = next;
      cw = nw;
      ch = nh;
    }

    if (cw !== targetW || ch !== targetH) {
      const out = document.createElement('canvas');
      out.width = targetW;
      out.height = targetH;
      const octx = out.getContext('2d');
      octx.imageSmoothingEnabled = true;
      octx.imageSmoothingQuality = 'high';
      octx.drawImage(srcCanvas, 0, 0, targetW, targetH);
      return out;
    }
    return srcCanvas;
  }

  async function makeMicroBlurDataUrl(bitmapOrVideo, srcW, srcH) {
    try {
      const bw = Math.max(4, Math.min(20, Math.round((srcW || 16) >= (srcH || 16) ? 16 : 16 * ((srcW || 16) / Math.max(1, srcH || 16)))));
      const bh = Math.max(4, Math.min(20, Math.round((srcH || 16) > (srcW || 16) ? 16 : 16 * ((srcH || 16) / Math.max(1, srcW || 16)))));
      const c = document.createElement('canvas');
      c.width = bw;
      c.height = bh;
      const ctx = c.getContext('2d');
      ctx.imageSmoothingEnabled = true;
      ctx.imageSmoothingQuality = 'medium';
      ctx.drawImage(bitmapOrVideo, 0, 0, bw, bh);
      const webp = c.toDataURL('image/webp', 0.35);
      if (webp && webp.startsWith('data:image/webp') && webp.length <= 1200) return webp;
      const jpg = c.toDataURL('image/jpeg', 0.38);
      if (jpg && jpg.length <= 1400) return jpg;
    } catch (_) {}
    return null;
  }

  async function extractVideoPosterMeta(file) {
    return await new Promise((resolve) => {
      let settled = false;
      const done = (val) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        try { URL.revokeObjectURL(objUrl); } catch (_) {}
        resolve(val);
      };
      const objUrl = URL.createObjectURL(file);
      const v = document.createElement('video');
      v.muted = true;
      v.playsInline = true;
      v.preload = 'metadata';
      const timer = setTimeout(() => done({ width: 0, height: 0, blurDataUrl: null }), 1500);
      v.onloadeddata = async () => {
        const w = v.videoWidth || 0;
        const h = v.videoHeight || 0;
        const blurDataUrl = w > 0 && h > 0 ? await makeMicroBlurDataUrl(v, w, h) : null;
        done({ width: w, height: h, blurDataUrl });
      };
      v.onerror = () => done({ width: 0, height: 0, blurDataUrl: null });
      v.src = objUrl;
    });
  }

  let mediaWorker = null;
  let mediaWorkerReqId = 1;
  const mediaWorkerPending = new Map();
  let nativeCanvasAvifSupport = null; // null = untested, true/false once probed

  function optimizeInMediaWorker(rawU8, mime, opts, task = 'optimize_image') {
    if (typeof Worker === 'undefined') {
      return Promise.resolve(null);
    }
    try {
      if (!mediaWorker) {
        mediaWorker = new Worker('/media_worker.js', { type: 'module' });
        mediaWorker.onmessage = (ev) => {
          const d = ev.data;
          if (!d || !d.id) return;
          const entry = mediaWorkerPending.get(d.id);
          if (entry) {
            clearTimeout(entry.timer);
            mediaWorkerPending.delete(d.id);
            if (d.avifUnsupportedInWorker) nativeCanvasAvifSupport = false;
            else if (d.ok && (d.codec === 'avif_native_canvas' || d.codec === 'avif_native_canvas_worker')) nativeCanvasAvifSupport = true;
            entry.resolve(d.ok ? d : null);
          }
        };
        mediaWorker.onerror = () => {
          for (const [id, entry] of mediaWorkerPending.entries()) {
            clearTimeout(entry.timer);
            entry.resolve(null);
            mediaWorkerPending.delete(id);
          }
          mediaWorker = null;
        };
      }
      const id = mediaWorkerReqId++;
      const copy = new Uint8Array(rawU8);
      return new Promise((resolve) => {
        const timer = setTimeout(() => {
          mediaWorkerPending.delete(id);
          resolve(null);
        }, 30000);
        mediaWorkerPending.set(id, { resolve, timer });
        mediaWorker.postMessage({ id, task, buffer: copy.buffer, mime, opts }, [copy.buffer]);
      });
    } catch (_) {
      return Promise.resolve(null);
    }
  }

  async function extractVoiceWaveform(audioBytes, bars = 48) {
    try {
      const AC = window.AudioContext || window.webkitAudioContext;
      if (!AC) return [];
      const ac = new AC();
      const copy = audioBytes.slice(0).buffer;
      const decoded = await ac.decodeAudioData(copy);
      const ch = decoded.getChannelData(0);
      const step = Math.max(1, Math.floor(ch.length / bars));
      const peaks = [];
      let maxPeak = 0.001;
      for (let i = 0; i < bars; i++) {
        let peak = 0;
        const start = i * step;
        const end = Math.min(ch.length, start + step);
        for (let j = start; j < end; j++) {
          const v = Math.abs(ch[j]);
          if (v > peak) peak = v;
        }
        peaks.push(peak);
        if (peak > maxPeak) maxPeak = peak;
      }
      try { await ac.close(); } catch (_) {}
      // Quantize to compact 5-bit integers (0..31) matching Telegram voice waveform format
      return peaks.map((p) => Math.max(1, Math.min(31, Math.round((p / maxPeak) * 31))));
    } catch (_) {
      return [];
    }
  }

  // Adaptive Ultra AVIF compression (NO 1920px clamp, ZERO silent JPEG/WebP/PNG fallback!).
  // Tries off-main-thread native AVIF worker -> pure-Rust WASM ravif encoder -> safe original passthrough.
  async function optimizeImageBlob(fileOrBlob, opts = {}) {
    const rawU8 = new Uint8Array(await fileOrBlob.arrayBuffer());
    const rawSniff = await sniffBytesFormat(rawU8);
    // If input is ALREADY genuine AVIF or animated AVIF (avis), NEVER re-compress!
    // Re-encoding AVIF only wastes CPU and cannot reduce file size further.
    if (rawSniff.ext === 'avif') {
      let w = 0;
      let h = 0;
      let blurDataUrl = null;
      try {
        const bmp = await createImageBitmap(new Blob([rawU8], { type: rawSniff.mime || 'image/avif' }));
        w = bmp.width;
        h = bmp.height;
        blurDataUrl = await makeMicroBlurDataUrl(bmp, w, h);
        bmp.close();
      } catch (_) {}
      return {
        bytesB64: bytesToB64(rawU8),
        mime: rawSniff.mime || 'image/avif',
        ext: 'avif',
        originalSize: rawU8.byteLength,
        optimizedSize: rawU8.byteLength,
        isCrispText: false,
        width: w,
        height: h,
        blurDataUrl,
        thumbB64: null,
        thumbMime: null,
        codec: 'already_avif_passthrough',
        estimatedSsim: 1.0,
        aiRecommended: false,
      };
    }

    const qualityMode = opts.qualityMode || getUserMediaQualityMode(Boolean(opts.preserveOriginal));
    // Spec 158: Keep active/fresh images untouched as-is on upload (`original_passthrough`) unless `forceCompress`
    // (15-day cold archive), `data_saver` mode, or > 24 MB single-upload cap applies.
    const keepAsIsFresh = !opts.forceCompress && qualityMode !== 'data_saver' && rawU8.byteLength <= 24 * 1024 * 1024;
    const effectiveOpts = { ...opts, qualityMode, preserveOriginal: Boolean(opts.preserveOriginal) || qualityMode === 'original' || keepAsIsFresh };

    async function computeOrigProfileFromRaw(u8, mimeType) {
      if (!window.__chatwWasmMedia || typeof window.__chatwWasmMedia.extractQualityProfile !== 'function') return null;
      try {
        const bmp = await createImageBitmap(new Blob([u8], { type: mimeType || 'image/png' }));
        const c = document.createElement('canvas');
        c.width = bmp.width;
        c.height = bmp.height;
        const ctx = c.getContext('2d', { willReadFrequently: true });
        ctx.drawImage(bmp, 0, 0);
        bmp.close();
        const d = ctx.getImageData(0, 0, c.width, c.height).data;
        const prof = window.__chatwWasmMedia.extractQualityProfile(new Uint8Array(d.buffer, d.byteOffset, d.byteLength), c.width, c.height);
        return prof && prof.length > 2 ? prof : null;
      } catch (_) {
        return null;
      }
    }

    // Offload both native AVIF & release WASM AVIF completely off the main thread when worker supports it
    const workerRes = await optimizeInMediaWorker(rawU8, rawSniff.mime || fileOrBlob.type, effectiveOpts, 'optimize_image');
    if (workerRes && workerRes.bytesB64 && (workerRes.ext === 'avif' || effectiveOpts.preserveOriginal || workerRes.codec === 'original_passthrough')) {
      if (!effectiveOpts.preserveOriginal && !workerRes.origProfileJson && workerRes.codec === 'avif_native_canvas_worker') {
        workerRes.origProfileJson = await computeOrigProfileFromRaw(rawU8, rawSniff.mime || fileOrBlob.type);
      }
      return workerRes;
    }

    if (effectiveOpts.preserveOriginal) {
      let w = 0;
      let h = 0;
      let blurDataUrl = null;
      try {
        const bmp = await createImageBitmap(new Blob([rawU8], { type: rawSniff.mime }));
        w = bmp.width;
        h = bmp.height;
        blurDataUrl = await makeMicroBlurDataUrl(bmp, w, h);
        bmp.close();
      } catch (_) {}
      return {
        bytesB64: bytesToB64(rawU8),
        mime: rawSniff.mime !== 'application/octet-stream' ? rawSniff.mime : (fileOrBlob.type || 'application/octet-stream'),
        ext: rawSniff.ext,
        originalSize: rawU8.byteLength,
        optimizedSize: rawU8.byteLength,
        isCrispText: false,
        width: w,
        height: h,
        blurDataUrl,
        thumbB64: null,
        thumbMime: null,
        codec: 'original_passthrough',
      };
    }

    // Extract dimensions & 16x16 blur-up preview while preserving full quality metadata
    let origW = 0;
    let origH = 0;
    let blurDataUrl = null;
    let isCrisp = false;
    try {
      const bitmap = await createImageBitmap(new Blob([rawU8], { type: rawSniff.mime || fileOrBlob.type || 'image/png' }));
      origW = bitmap.width;
      origH = bitmap.height;
      blurDataUrl = await makeMicroBlurDataUrl(bitmap, origW, origH);
      const probeW = Math.min(origW, 256);
      const probeH = Math.min(origH, 256);
      const probe = await drawMultiStepResized(bitmap, probeW, probeH);
      isCrisp = detectCrispTextOrAlpha(probe).isCrisp;

      // 1. Try native canvas genuine AVIF only when the browser supports image/avif export
      if (!rawSniff.animated && nativeCanvasAvifSupport !== false) {
        const maxEdge = opts.maxDim || adaptiveMaxEdge(origW, origH, isCrisp, qualityMode);
        let tw = origW;
        let th = origH;
        if (maxEdge && (tw > maxEdge || th > maxEdge)) {
          const r = Math.min(maxEdge / tw, maxEdge / th);
          tw = Math.max(1, Math.round(tw * r));
          th = Math.max(1, Math.round(th * r));
        }
        const canvas = await drawMultiStepResized(bitmap, tw, th);
        const q = isCrisp ? 0.82 : (qualityMode === 'data_saver' ? 0.65 : 0.80);
        const nativeAvif = await tryNativeCanvasAvif(canvas, q);
        if (nativeAvif) {
          nativeCanvasAvifSupport = true;
        } else {
          nativeCanvasAvifSupport = false;
        }
        if (nativeAvif && nativeAvif.u8.byteLength < rawU8.byteLength) {
          let origProfileJson = null;
          if (window.__chatwWasmMedia && typeof window.__chatwWasmMedia.extractQualityProfile === 'function') {
            try {
              const oc = document.createElement('canvas');
              oc.width = origW;
              oc.height = origH;
              const octx = oc.getContext('2d', { willReadFrequently: true });
              octx.drawImage(bitmap, 0, 0);
              const od = octx.getImageData(0, 0, origW, origH).data;
              origProfileJson = window.__chatwWasmMedia.extractQualityProfile(new Uint8Array(od.buffer, od.byteOffset, od.byteLength), origW, origH) || null;
            } catch (_) {}
          }
          bitmap.close();
          return {
            bytesB64: bytesToB64(nativeAvif.u8),
            mime: 'image/avif',
            ext: 'avif',
            originalSize: rawU8.byteLength,
            optimizedSize: nativeAvif.u8.byteLength,
            isCrispText: isCrisp,
            width: tw,
            height: th,
            targetWidth: origW,
            targetHeight: origH,
            superResRecommended: tw < origW,
            origProfileJson,
            blurDataUrl,
            thumbB64: null,
            thumbMime: null,
            codec: 'avif_native_canvas',
          };
        }
      }
      bitmap.close();
    } catch (_) {}

    // 2. Pure-Rust WASM `ravif`/`rav1e` Adaptive AVIF encoder (yield one frame first so UI loading badge paints!)
    if (window.__chatwWasmMedia && typeof window.__chatwWasmMedia.encodeAvifAdaptive === 'function') {
      try {
        await new Promise((r) => setTimeout(r, 0));
        const wasmMode = qualityMode === 'data_saver' ? 'aggressive' : (origW * origH > 400_000 ? 'fast' : 'ultra');
        const res = window.__chatwWasmMedia.encodeAvifAdaptive(rawU8, wasmMode);
        if (res && res.ok && res.bytes && res.bytes.byteLength > 0) {
          const avifU8 = res.bytes instanceof Uint8Array ? res.bytes : new Uint8Array(res.bytes);
          const avifSniff = await sniffBytesFormat(avifU8);
          if (avifSniff.ext === 'avif' && (avifU8.byteLength < rawU8.byteLength || rawSniff.animated)) {
            return {
              bytesB64: bytesToB64(avifU8),
              mime: 'image/avif',
              ext: 'avif',
              originalSize: rawU8.byteLength,
              optimizedSize: avifU8.byteLength,
              isCrispText: Boolean(res.isCrispText ?? isCrisp),
              width: res.width || origW,
              height: res.height || origH,
              targetWidth: res.targetWidth || origW,
              targetHeight: res.targetHeight || origH,
              superResRecommended: Boolean(res.superResRecommended ?? ((res.width || origW) < origW)),
              origProfileJson: res.origProfileJson || null,
              blurDataUrl,
              thumbB64: null,
              thumbMime: null,
              codec: rawSniff.animated ? 'avis_wasm_rav1e' : 'avif_wasm_ravif',
              estimatedSsim: res.estimatedSsim || null,
              aiRecommended: Boolean(res.aiRecommended),
            };
          }
        }
      } catch (_) {}
    }

    // 3. Honest passthrough: NEVER silently fall back to JPEG/WebP/PNG if AVIF encoder unavailable or larger!
    return {
      bytesB64: bytesToB64(rawU8),
      mime: rawSniff.mime !== 'application/octet-stream' ? rawSniff.mime : (fileOrBlob.type || 'application/octet-stream'),
      ext: rawSniff.ext,
      originalSize: rawU8.byteLength,
      optimizedSize: rawU8.byteLength,
      isCrispText: isCrisp,
      width: origW,
      height: origH,
      blurDataUrl,
      thumbB64: null,
      thumbMime: null,
      codec: 'original_passthrough',
    };
  }

  let activeVoiceRecorder = null;
  let activeVoiceChunks = [];
  let activeVoiceStartedAt = 0;

  // WebRTC DataChannel peer file transfer with ordered delivery, backpressure & SHA-256 verification.
  const peerConnections = new Map();

  function getOrCreatePeer(requestId, targetUserId, sendSignalFn) {
    let entry = peerConnections.get(requestId);
    if (entry) return entry;
    const pc = new RTCPeerConnection({
      iceServers: [{ urls: 'stun:stun.l.google.com:19302' }, { urls: 'stun:stun1.l.google.com:19302' }],
    });
    entry = { pc, dc: null, incomingMeta: null, incomingChunks: [], incomingBytes: 0, onProgress: null, onComplete: null };
    pc.onicecandidate = (e) => {
      if (e.candidate && sendSignalFn) {
        sendSignalFn(requestId, 'ice_candidate', e.candidate);
      }
    };
    pc.ondatachannel = (ev) => {
      attachDataChannel(requestId, entry, ev.channel);
    };
    peerConnections.set(requestId, entry);
    return entry;
  }

  function attachDataChannel(requestId, entry, dc) {
    entry.dc = dc;
    dc.binaryType = 'arraybuffer';
    dc.bufferedAmountLowThreshold = 64 * 1024;
    dc.onmessage = async (ev) => {
      if (typeof ev.data === 'string') {
        try {
          const msg = JSON.parse(ev.data);
          if (msg.kind === 'meta') {
            entry.incomingMeta = msg;
            entry.incomingChunks = [];
            entry.incomingBytes = 0;
          } else if (msg.kind === 'done' && entry.incomingMeta) {
            const total = new Uint8Array(entry.incomingBytes);
            let off = 0;
            for (const c of entry.incomingChunks) {
              const u8 = new Uint8Array(c);
              total.set(u8, off);
              off += u8.byteLength;
            }
            const actualSha = window.__chatwStorage ? await window.__chatwStorage.sha256Hex(total) : '';
            const verified = !entry.incomingMeta.sha256 || entry.incomingMeta.sha256 === actualSha;
            if (verified && window.__chatwStorage) {
              await window.__chatwStorage.putObject({
                ref_id: entry.incomingMeta.ref_id,
                scope_type: 'dm',
                file_name: entry.incomingMeta.file_name || 'restored.bin',
                mime_type: entry.incomingMeta.mime_type || 'application/octet-stream',
                sha256_plain: actualSha,
                availability_state: 'restored_from_peer',
              }, total);
            }
            window.dispatchEvent(new CustomEvent('chatw:p2p-recovery-finished', {
              detail: { request_id: requestId, ref_id: entry.incomingMeta.ref_id, ok: verified, sha256: actualSha, byte_size: total.byteLength },
            }));
          }
        } catch (_) {}
        return;
      }
      entry.incomingChunks.push(ev.data);
      entry.incomingBytes += ev.data.byteLength;
    };
  }

  // ---------------------------------------------------------------------------
  // Spec 111: Hyper-Dedup Local Authorized Cross-Chat Index + Pure-Rust WASM Fingerprinting
  // ---------------------------------------------------------------------------
  const LOCAL_DEDUP_STORAGE_KEY = 'chatw_dedup_local_v1';
  const MAX_LOCAL_DEDUP_ENTRIES = 256;
  const localDedupEntries = new Map(); // key = refId -> entry
  let lastCopiedHint = null; // { refId, at }

  function loadLocalDedupFromStorage() {
    try {
      const raw = localStorage.getItem(LOCAL_DEDUP_STORAGE_KEY);
      if (!raw) return;
      const arr = JSON.parse(raw);
      if (Array.isArray(arr)) {
        for (const e of arr) {
          if (e && e.refId) localDedupEntries.set(e.refId, e);
        }
      }
    } catch (_) {}
  }

  function saveLocalDedupToStorage() {
    try {
      const arr = Array.from(localDedupEntries.values()).slice(-MAX_LOCAL_DEDUP_ENTRIES);
      localStorage.setItem(LOCAL_DEDUP_STORAGE_KEY, JSON.stringify(arr));
    } catch (_) {}
  }

  loadLocalDedupFromStorage();

  function extractRefIdFromUrl(u) {
    if (!u || typeof u !== 'string') return '';
    const m = u.match(/\/api\/attachments\/([0-9a-f-]{36})/i);
    return m ? m[1] : '';
  }

  function registerLocalDedupEntry(entry) {
    if (!entry || !entry.refId) return;
    const prev = localDedupEntries.get(entry.refId) || {};
    const merged = {
      ...prev,
      ...entry,
      refId: entry.refId,
      updatedAt: Date.now(),
    };
    localDedupEntries.delete(entry.refId);
    localDedupEntries.set(entry.refId, merged);
    while (localDedupEntries.size > MAX_LOCAL_DEDUP_ENTRIES) {
      const oldest = localDedupEntries.keys().next().value;
      localDedupEntries.delete(oldest);
    }
    saveLocalDedupToStorage();
  }

  async function computeImagePdqFromBytes(u8, mime) {
    const wasm = window.__chatwWasmMedia;
    if (!wasm || typeof wasm.computeImagePdq !== 'function') return null;
    try {
      const bmp = await createImageBitmap(new Blob([u8], { type: mime || 'image/png' }));
      const origW = bmp.width;
      const origH = bmp.height;
      if (origW < 8 || origH < 8) {
        bmp.close();
        return null;
      }
      // PDQ downsamples to 64x64 internally; scaling to <= 256px preserves exact aspect & runs in < 2ms!
      const scale = Math.min(1, 256 / Math.max(origW, origH));
      const pw = Math.max(16, Math.round(origW * scale));
      const ph = Math.max(16, Math.round(origH * scale));
      const c = document.createElement('canvas');
      c.width = pw;
      c.height = ph;
      const ctx = c.getContext('2d', { willReadFrequently: true });
      ctx.imageSmoothingEnabled = true;
      ctx.imageSmoothingQuality = 'high';
      ctx.drawImage(bmp, 0, 0, pw, ph);
      bmp.close();
      const d = ctx.getImageData(0, 0, pw, ph).data;
      const pdq = wasm.computeImagePdq(new Uint8Array(d.buffer, d.byteOffset, d.byteLength), pw, ph);
      return {
        pdqHex: pdq.pdqHex || '',
        pdqQuality: Number(pdq.quality || 0),
        captionBandHash: String(pdq.captionBandHash || '0'),
        colorMomentHash: String(pdq.colorMomentHash || '0'),
        aspectRatioMilli: Number(pdq.aspectRatioMilli || Math.round((origW * 1000) / Math.max(1, origH))),
        width: origW,
        height: origH,
      };
    } catch (_) {
      return null;
    }
  }

  async function extractVideoFingerprint(file) {
    const wasm = window.__chatwWasmMedia;
    if (!wasm || typeof wasm.computeImagePdq !== 'function') return null;
    return await new Promise((resolve) => {
      let settled = false;
      const objUrl = URL.createObjectURL(file);
      const v = document.createElement('video');
      v.muted = true;
      v.playsInline = true;
      v.preload = 'auto';
      const done = (val) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        try { URL.revokeObjectURL(objUrl); } catch (_) {}
        resolve(val);
      };
      const timer = setTimeout(() => done(null), 1200);
      v.onerror = () => done(null);
      v.onloadedmetadata = async () => {
        const dur = Number(v.duration || 0);
        const w = v.videoWidth || 0;
        const h = v.videoHeight || 0;
        if (!dur || !Number.isFinite(dur) || w < 8 || h < 8) { done(null); return; }
        const pts = [0.20, 0.70].map((r) => Math.max(0.05, Math.min(dur - 0.05, dur * r)));
        const pw = Math.max(32, Math.min(160, w));
        const ph = Math.max(32, Math.round(pw * (h / Math.max(1, w))));
        const c = document.createElement('canvas');
        c.width = pw; c.height = ph;
        const ctx = c.getContext('2d', { willReadFrequently: true });
        const hashes = [];
        for (const t of pts) {
          const ok = await new Promise((res) => {
            const onSeek = () => { v.removeEventListener('seeked', onSeek); res(true); };
            v.addEventListener('seeked', onSeek, { once: true });
            try { v.currentTime = t; } catch (_) { res(false); }
            setTimeout(() => res(false), 350);
          });
          if (!ok) continue;
          try {
            ctx.drawImage(v, 0, 0, pw, ph);
            const d = ctx.getImageData(0, 0, pw, ph).data;
            const pdq = wasm.computeImagePdq(new Uint8Array(d.buffer, d.byteOffset, d.byteLength), pw, ph);
            if (pdq && pdq.pdqHex && pdq.pdqHex.length === 64) {
              const bytes = [];
              for (let i = 0; i < 64; i += 2) bytes.push(parseInt(pdq.pdqHex.slice(i, i + 2), 16));
              hashes.push({
                bytes,
                quality: Number(pdq.quality || 0),
                caption_band_hash: Number(BigInt(pdq.captionBandHash || '0') & 0x1fffffffffffffn),
                color_moment_hash: Number(BigInt(pdq.colorMomentHash || '0') & 0x1fffffffffffffn),
                aspect_ratio_milli: Number(pdq.aspectRatioMilli || 0),
              });
            }
          } catch (_) {}
        }
        if (hashes.length === 0) { done(null); return; }
        done(JSON.stringify({
          duration_ms: Math.round(dur * 1000),
          frame_count: hashes.length,
          width: w,
          height: h,
          keyframe_hashes: hashes,
        }));
      };
      v.src = objUrl;
    });
  }

  async function extractAudioFingerprint(rawBytes) {
    const wasm = window.__chatwWasmMedia;
    const AC = window.AudioContext || window.webkitAudioContext;
    if (!wasm || typeof wasm.computeAudioFingerprint !== 'function' || !AC) return null;
    try {
      const ac = new AC();
      const decoded = await ac.decodeAudioData(rawBytes.slice(0).buffer);
      const ch = decoded.getChannelData(0);
      const durMs = Math.round((decoded.duration || 0) * 1000);
      const stride = Math.max(1, Math.floor(ch.length / 32000));
      const down = new Float32Array(Math.ceil(ch.length / stride));
      for (let i = 0, j = 0; i < ch.length && j < down.length; i += stride, j++) down[j] = ch[i];
      try { await ac.close(); } catch (_) {}
      const fp = wasm.computeAudioFingerprint(down, durMs);
      return fp && fp.json ? fp.json : null;
    } catch (_) {
      return null;
    }
  }

  function findLocalAuthorizedMatch({ origBlake3, wireBlake3, sha256, pdq }) {
    const wasm = window.__chatwWasmMedia;
    // 1. Exact cryptographic match first (BLAKE3 or SHA-256 across ALL media types!)
    for (const e of localDedupEntries.values()) {
      if (!e.refId) continue;
      if (origBlake3 && (e.origBlake3 === origBlake3 || e.wireBlake3 === origBlake3)) return { ...e, matchKind: 'local_exact_blake3', hamming: 0 };
      if (wireBlake3 && (e.wireBlake3 === wireBlake3 || e.origBlake3 === wireBlake3)) return { ...e, matchKind: 'local_exact_blake3', hamming: 0 };
      if (sha256 && e.sha256 && e.sha256 === sha256) return { ...e, matchKind: 'local_exact_sha256', hamming: 0 };
    }
    // 2. Perceptual Pure-Rust PDQ 256-bit + Triple Guard (catches right-click Copy Image -> clipboard PNG, blurry AVIF, 4x enhanced!)
    if (pdq && pdq.pdqHex && pdq.pdqQuality >= 18 && wasm && typeof wasm.comparePdqLocal === 'function') {
      let best = null;
      // Check recently right-clicked/copied hint first for priority tie-breaking
      const ordered = Array.from(localDedupEntries.values()).reverse();
      if (lastCopiedHint && Date.now() - lastCopiedHint.at < 120_000 && localDedupEntries.has(lastCopiedHint.refId)) {
        ordered.unshift(localDedupEntries.get(lastCopiedHint.refId));
      }
      for (const e of ordered) {
        if (!e || !e.refId || !e.pdqHex) continue;
        const cmp = wasm.comparePdqLocal(pdq, e);
        if (cmp && cmp.matched) {
          const ham = Number(cmp.hamming || 0);
          if (!best || ham < best.hamming) {
            best = { ...e, matchKind: 'local_pdq_cross_chat', hamming: ham };
            if (ham <= 4) break;
          }
        }
      }
      if (best) return best;
    }
    return null;
  }

  // Track Right-Click / Copy / Drag on any rendered chat image or Lightbox image!
  function recordDomMediaHint(target) {
    if (!target || typeof target.closest !== 'function') return;
    const img = target.closest('img[data-media-ref], [data-media-ref]');
    if (!img) return;
    const refId = img.getAttribute('data-media-ref') || '';
    if (refId) lastCopiedHint = { refId, at: Date.now() };
  }
  if (typeof document !== 'undefined') {
    document.addEventListener('contextmenu', (e) => recordDomMediaHint(e.target), true);
    document.addEventListener('dragstart', (e) => recordDomMediaHint(e.target), true);
    document.addEventListener('copy', (e) => {
      recordDomMediaHint(e.target);
      const sel = window.getSelection && window.getSelection();
      if (sel && sel.anchorNode && sel.anchorNode.nodeType === 1) recordDomMediaHint(sel.anchorNode);
    }, true);
  }

  function removeLocalDedupEntry(refId) {
    if (!refId) return false;
    const existed = localDedupEntries.delete(refId);
    if (lastCopiedHint && lastCopiedHint.refId === refId) {
      lastCopiedHint = null;
    }
    if (existed) saveLocalDedupToStorage();
    return existed;
  }

  window.__chatwDedupLocal = {
    registerMedia: registerLocalDedupEntry,
    removeMedia: removeLocalDedupEntry,
    clear: () => {
      localDedupEntries.clear();
      lastCopiedHint = null;
      saveLocalDedupToStorage();
    },
    findMatch: findLocalAuthorizedMatch,
    listCount: () => localDedupEntries.size,
    async registerLoadedImageFromBlobUrl(blobUrl, meta = {}) {
      const refId = meta.refId || extractRefIdFromUrl(meta.url || '');
      if (!refId || !blobUrl) return { ok: false };
      const existing = localDedupEntries.get(refId);
      if (existing && existing.pdqHex && existing.keyB64 === (meta.keyB64 || '')) {
        // Update blurry/qp flags if newly known
        registerLocalDedupEntry({ ...existing, ...meta, refId });
        return { ok: true, cached: true };
      }
      try {
        const resp = await fetch(blobUrl);
        if (!resp.ok) return { ok: false };
        const u8 = new Uint8Array(await resp.arrayBuffer());
        const b3 = window.__chatwWasmMedia && window.__chatwWasmMedia.blake3Hex
          ? window.__chatwWasmMedia.blake3Hex(u8)
          : '';
        const pdq = await computeImagePdqFromBytes(u8, meta.mime || 'image/avif');
        registerLocalDedupEntry({
          refId,
          url: meta.url || `/api/attachments/${refId}`,
          keyB64: meta.keyB64 || '',
          nonceB64: meta.nonceB64 || '',
          mime: meta.mime || 'image/avif',
          fileName: stripFileExtension(meta.fileName || 'image'),
          mediaKind: 'image',
          sha256: meta.sha256 || '',
          wireBlake3: b3,
          origBlake3: meta.origBlake3 || b3,
          pdqHex: pdq ? pdq.pdqHex : '',
          pdqQuality: pdq ? pdq.pdqQuality : 0,
          captionBandHash: pdq ? pdq.captionBandHash : '0',
          colorMomentHash: pdq ? pdq.colorMomentHash : '0',
          aspectRatioMilli: pdq ? pdq.aspectRatioMilli : 0,
          width: meta.width || (pdq ? pdq.width : 0),
          height: meta.height || (pdq ? pdq.height : 0),
          origW: meta.origW || meta.width || (pdq ? pdq.width : 0),
          origH: meta.origH || meta.height || (pdq ? pdq.height : 0),
          qp: meta.qp || '',
          isAiRecovery: Boolean(meta.qp) || Boolean(meta.isAiRecovery) || (meta.origW > meta.width),
          isSenderOrigLocal: Boolean(meta.isSenderOrigLocal),
        });
        return { ok: true, refId, pdqHex: pdq ? pdq.pdqHex : '' };
      } catch (_) {
        return { ok: false };
      }
    },
  };

  // Spec 126: Omit raw file extensions when saving media filenames to system/chat payloads.
  function stripFileExtension(fileName) {
    if (!fileName || typeof fileName !== 'string') return '';
    const idx = fileName.lastIndexOf('.');
    return idx > 0 ? fileName.slice(0, idx) : fileName;
  }

  // Spec 115: Strictly block `.svg` / `.svgz` / `image/svg+xml` / XML-SVG payloads on all frontend pickers & paste paths.
  function isBlockedSvgFile(fileName, mime, bytesU8) {
    const fn = String(fileName || '').trim().toLowerCase();
    if (fn.endsWith('.svg') || fn.endsWith('.svgz')) return true;
    const mt = String(mime || '').trim().toLowerCase();
    if (mt.includes('svg')) return true;
    if (bytesU8 && bytesU8.length > 0) {
      let off = 0;
      if (bytesU8.length >= 3 && bytesU8[0] === 0xEF && bytesU8[1] === 0xBB && bytesU8[2] === 0xBF) off = 3;
      while (
        off < bytesU8.length &&
        (bytesU8[off] === 0x20 || bytesU8[off] === 0x09 || bytesU8[off] === 0x0A || bytesU8[off] === 0x0D || bytesU8[off] === 0x00)
      ) {
        off++;
      }
      if (off < bytesU8.length && bytesU8[off] === 0x3C) {
        const end = Math.min(bytesU8.length, off + 4096);
        let head = '';
        try {
          head = new TextDecoder('utf-8', { fatal: false }).decode(bytesU8.subarray(off, end)).toLowerCase();
        } catch (_) {
          for (let i = off; i < end; i++) head += String.fromCharCode(bytesU8[i]);
          head = head.toLowerCase();
        }
        if (
          /^<svg(?:\s|>|\/|:|$)/.test(head) ||
          /<svg(?:\s|>|\/|:)/.test(head) ||
          head.includes('<!doctype svg') ||
          head.includes('http://www.w3.org/2000/svg') ||
          head.includes('https://www.w3.org/2000/svg')
        ) {
          return true;
        }
      }
    }
    return false;
  }

  async function isBlockedSvgBlob(file) {
    if (!file) return false;
    if (isBlockedSvgFile(file.name, file.type, null)) return true;
    try {
      const headBuf = await file.slice(0, 4096).arrayBuffer();
      return isBlockedSvgFile(file.name, file.type, new Uint8Array(headBuf));
    } catch (_) {
      return false;
    }
  }

  function inferMediaKind(file, sniff = null) {
    const mt = String((file && file.type) || '').toLowerCase();
    const sm = String((sniff && sniff.mime) || '').toLowerCase();
    const ext = String((file && file.name && file.name.split('.').pop()) || '').toLowerCase();
    if (mt.startsWith('image/') || sm.startsWith('image/') || ['png', 'jpg', 'jpeg', 'webp', 'avif', 'gif'].includes(ext)) return 'image';
    if (mt.startsWith('video/') || sm.startsWith('video/') || ['mp4', 'webm', 'mov', 'mkv', 'm4v'].includes(ext)) return 'video';
    if (mt.startsWith('audio/') || sm.startsWith('audio/') || ['mp3', 'ogg', 'opus', 'wav', 'flac', 'm4a'].includes(ext)) return 'audio';
    return 'file';
  }

  async function processSingleFile(file, preserveOriginal = false, existingPreviewUrl = null, skipLocalDedup = false) {
    if (await isBlockedSvgBlob(file)) {
      return { error: 'svg_not_allowed' };
    }
    const previewUrl = existingPreviewUrl || URL.createObjectURL(file);
    const rawArr = new Uint8Array(await file.arrayBuffer());
    if (isBlockedSvgFile(file.name, file.type, rawArr)) {
      if (!existingPreviewUrl) { try { URL.revokeObjectURL(previewUrl); } catch (_) {} }
      return { error: 'svg_not_allowed' };
    }
    const sniffedKind = inferMediaKind(file, await sniffBytesFormat(rawArr));
    const isImageKind = sniffedKind === 'image';

    // Run BLAKE3 + SHA-256 + PDQ off-main-thread in worker first when available (0ms UI block!).
    let origBlake3 = '';
    let shaHexRaw = '';
    let pdq = null;
    const fpWorker = rawArr.byteLength <= 24 * 1024 * 1024
      ? await optimizeInMediaWorker(rawArr, file.type || (isImageKind ? 'image/png' : 'application/octet-stream'), { isImage: isImageKind }, 'compute_pdq_blake3')
      : null;
    if (fpWorker && fpWorker.ok) {
      origBlake3 = fpWorker.origBlake3 || '';
      shaHexRaw = fpWorker.sha256 || '';
      pdq = fpWorker.pdq || null;
    } else {
      origBlake3 = window.__chatwWasmMedia && window.__chatwWasmMedia.blake3Hex
        ? window.__chatwWasmMedia.blake3Hex(rawArr)
        : '';
      shaHexRaw = window.__chatwStorage ? await window.__chatwStorage.sha256Hex(rawArr) : '';
      if (isImageKind) {
        pdq = await computeImagePdqFromBytes(rawArr, file.type);
      }
    }

    if (isImageKind) {
      try {
        // 1. Check local authorized cross-chat dedup registry FIRST (catches copied/pasted/saved blurry OR enhanced images -> 0ms encode, 0B upload!)
        const localHit = skipLocalDedup ? null : findLocalAuthorizedMatch({ origBlake3, wireBlake3: origBlake3, sha256: shaHexRaw, pdq });
        if (localHit && localHit.refId) {
          const needsSenderAi = Boolean(localHit.isAiRecovery || localHit.qp || !localHit.isSenderOrigLocal);
          // Always retain source file handle for automatic claim-miss fallback while preserving sender-original safety flag.
          stashPreviewFile(previewUrl, file, localHit.origW || (pdq ? pdq.width : 0), localHit.origH || (pdq ? pdq.height : 0), {
            preserveOriginal,
            allowSenderOriginal: !needsSenderAi,
          });
          return {
            bytesB64: '',
            mime: localHit.mime || 'image/avif',
            ext: (localHit.mime || '').includes('avif') ? 'avif' : 'bin',
            fileName: stripFileExtension(file.name && file.name !== 'image.png' ? file.name : (localHit.fileName || file.name || 'image')),
            originalSize: rawArr.byteLength,
            optimizedSize: 0,
            isCrispText: false,
            mediaKind: 'image',
            sha256: localHit.sha256 || shaHexRaw,
            origBlake3,
            pdqHex: pdq ? pdq.pdqHex : (localHit.pdqHex || ''),
            pdqQuality: pdq ? pdq.pdqQuality : (localHit.pdqQuality || 0),
            captionBandHash: pdq ? pdq.captionBandHash : (localHit.captionBandHash || '0'),
            colorMomentHash: pdq ? pdq.colorMomentHash : (localHit.colorMomentHash || '0'),
            aspectRatioMilli: pdq ? pdq.aspectRatioMilli : (localHit.aspectRatioMilli || 0),
            width: localHit.width || (pdq ? pdq.width : 0),
            height: localHit.height || (pdq ? pdq.height : 0),
            targetWidth: localHit.origW || localHit.width || (pdq ? pdq.width : 0),
            targetHeight: localHit.origH || localHit.height || (pdq ? pdq.height : 0),
            origProfileJson: localHit.qp || '',
            aiRecommended: needsSenderAi,
            copiedFromRefId: localHit.refId,
            copiedKeyB64: localHit.keyB64 || '',
            copiedNonceB64: localHit.nonceB64 || '',
            needsSenderAi,
            instantPreviewUrl: previewUrl,
          };
        }

        const res = await optimizeImageBlob(file, { preserveOriginal });
        stashPreviewFile(previewUrl, file, res.targetWidth || res.width || 0, res.targetHeight || res.height || 0, {
          preserveOriginal,
          allowSenderOriginal: true,
        });
        return {
          ...res,
          fileName: stripFileExtension(file.name),
          mediaKind: 'image',
          origBlake3,
          pdqHex: pdq ? pdq.pdqHex : '',
          pdqQuality: pdq ? pdq.pdqQuality : 0,
          captionBandHash: pdq ? pdq.captionBandHash : '0',
          colorMomentHash: pdq ? pdq.colorMomentHash : '0',
          aspectRatioMilli: pdq ? pdq.aspectRatioMilli : 0,
          aiRecommended: Boolean(res.aiRecommended || res.origProfileJson || (res.targetWidth && res.width && res.targetWidth > res.width)),
          instantPreviewUrl: previewUrl,
        };
      } catch (e) {
        return { error: String(e) };
      }
    }

    const raw = rawArr;
    const shaHex = shaHexRaw;
    const mediaKind = sniffedKind;

    // Check exact cross-chat dedup for videos, audio & general files too!
    const localFileHit = skipLocalDedup ? null : findLocalAuthorizedMatch({ origBlake3, wireBlake3: origBlake3, sha256: shaHex, pdq: null });
    if (localFileHit && localFileHit.refId) {
      stashPreviewFile(previewUrl, file, localFileHit.width || 0, localFileHit.height || 0, {
        preserveOriginal,
        allowSenderOriginal: true,
      });
      return {
        bytesB64: '',
        mime: localFileHit.mime || file.type || 'application/octet-stream',
        ext: (file.name.split('.').pop() || 'bin').toLowerCase().slice(0, 12),
        fileName: stripFileExtension(file.name || localFileHit.fileName || 'attachment'),
        originalSize: raw.byteLength,
        optimizedSize: 0,
        sha256: localFileHit.sha256 || shaHex,
        origBlake3,
        width: localFileHit.width || 0,
        height: localFileHit.height || 0,
        mediaKind,
        copiedFromRefId: localFileHit.refId,
        copiedKeyB64: localFileHit.keyB64 || '',
        copiedNonceB64: localFileHit.nonceB64 || '',
        instantPreviewUrl: previewUrl,
      };
    }

    const poster = mediaKind === 'video' ? await extractVideoPosterMeta(file) : { width: 0, height: 0, blurDataUrl: null };
    const videoFingerprintJson = mediaKind === 'video' ? await extractVideoFingerprint(file) : null;
    const audioFingerprintJson = mediaKind === 'audio' ? await extractAudioFingerprint(raw) : null;

    // Lossless Smart File Compression (`CWCMP1` ZSTD/Brotli/LZMA2) off-main-thread for general files
    let uploadBytes = raw;
    let fileCodec = 'raw';
    if (mediaKind === 'file' && !preserveOriginal) {
      const wCmp = raw.byteLength <= 16 * 1024 * 1024
        ? await optimizeInMediaWorker(raw, file.type || 'application/octet-stream', {}, 'compress_file')
        : null;
      if (wCmp && wCmp.ok && wCmp.compressed && wCmp.bytesB64) {
        uploadBytes = b64ToBytes(wCmp.bytesB64);
        fileCodec = wCmp.codec || 'cwcmp1';
      } else if (!wCmp && window.__chatwWasmMedia && typeof window.__chatwWasmMedia.compressFileSmart === 'function') {
        try {
          const cmp = window.__chatwWasmMedia.compressFileSmart(raw);
          if (cmp && cmp.ok && cmp.compressed && cmp.bytes && cmp.bytes.byteLength < raw.byteLength) {
            uploadBytes = cmp.bytes instanceof Uint8Array ? cmp.bytes : new Uint8Array(cmp.bytes);
            fileCodec = cmp.codec || 'cwcmp1';
          }
        } catch (_) {}
      }
    }

    return {
      bytesB64: bytesToB64(uploadBytes),
      mime: file.type || 'application/octet-stream',
      ext: (file.name.split('.').pop() || 'bin').toLowerCase().slice(0, 12),
      fileName: stripFileExtension(file.name),
      originalSize: raw.byteLength,
      optimizedSize: uploadBytes.byteLength,
      sha256: shaHex,
      origBlake3,
      width: poster.width,
      height: poster.height,
      blurDataUrl: poster.blurDataUrl,
      videoFingerprintJson,
      audioFingerprintJson,
      mediaKind,
      codec: fileCodec,
      instantPreviewUrl: previewUrl,
    };
  }

  window.__chatwMediaOpt = {
    stripFileExtension,
    optimizeImageBlob,
    probeVideoCodecs,
    isBlockedSvgFile,
    isBlockedSvgBlob,

    // Phase 1 of Instant Sender Original Preview: returns crisp local `blob:` URL in < 20ms!
    async pickInstantPreview(inputId) {
      const inp = document.getElementById(inputId);
      if (!inp || !inp.files || !inp.files[0]) return { error: 'no_file' };
      const file = inp.files[0];
      inp.value = '';
      if (await isBlockedSvgBlob(file)) {
        return { error: 'svg_not_allowed' };
      }
      const handleId = `pend_${Date.now().toString(36)}_${nextHandleSeq++}`;
      const previewUrl = URL.createObjectURL(file);
      const mediaKind = inferMediaKind(file);
      let width = 0;
      let height = 0;
      if (mediaKind === 'image') {
        try {
          const bmp = await createImageBitmap(file);
          width = bmp.width;
          height = bmp.height;
          bmp.close();
        } catch (_) {}
      }
      pendingSenderFiles.set(handleId, { file, previewUrl, mediaKind, width, height });
      stashPreviewFile(previewUrl, file, width, height);
      return {
        handleId,
        instantPreviewUrl: previewUrl,
        fileName: stripFileExtension(file.name),
        mime: file.type || 'application/octet-stream',
        originalSize: file.size,
        mediaKind,
        width,
        height,
      };
    },

    registerSenderOriginalSync(previewUrl, downloadUrl) {
      const entry = previewUrl ? previewUrlToFile.get(previewUrl) : null;
      if (!entry || !entry.file || !downloadUrl || entry.allowSenderOriginal === false) return false;
      try {
        if (senderOriginalSession.has(downloadUrl)) return true;
        const origBlob = entry.file;
        const stableUrl = URL.createObjectURL(origBlob);
        senderOriginalSession.set(downloadUrl, {
          url: stableUrl,
          width: entry.width || 0,
          height: entry.height || 0,
          byteSize: origBlob.size,
          mime: origBlob.type || 'image/png',
          provenance: 'sender_original',
        });
        while (senderOriginalSession.size > 48) {
          const [oldK, oldV] = senderOriginalSession.entries().next().value;
          senderOriginalSession.delete(oldK);
          if (oldV && oldV.url) { try { URL.revokeObjectURL(oldV.url); } catch (_) {} }
        }
        return true;
      } catch (_) {
        return false;
      }
    },

    // After upload succeeds, copy sender's 100% untouched original file into sender's own local app storage
    // so the sender ALWAYS views their crisp original file immediately & on future reloads without downloading!
    async commitSenderOriginal(previewUrl, downloadUrl, keyB64 = '', nonceB64 = '', refId = '') {
      const entry = previewUrl ? previewUrlToFile.get(previewUrl) : null;
      if (!entry || !entry.file || !downloadUrl || entry.allowSenderOriginal === false) return { ok: false };
      try {
        const origBlob = entry.file;
        let rec = senderOriginalSession.get(downloadUrl);
        if (!rec) {
          const stableUrl = URL.createObjectURL(origBlob);
          rec = {
            url: stableUrl,
            width: entry.width || 0,
            height: entry.height || 0,
            byteSize: origBlob.size,
            mime: origBlob.type || 'image/png',
            provenance: 'sender_original',
          };
          senderOriginalSession.set(downloadUrl, rec);
          while (senderOriginalSession.size > 48) {
            const [oldK, oldV] = senderOriginalSession.entries().next().value;
            senderOriginalSession.delete(oldK);
            if (oldV && oldV.url) { try { URL.revokeObjectURL(oldV.url); } catch (_) {} }
          }
        }
        // Persist untouched sender original in IndexedDB so even after full reload sender sees crisp original!
        if (window.__chatwStorage && typeof window.__chatwStorage.putAiOutput === 'function') {
          await window.__chatwStorage.putAiOutput(`sender_orig:${downloadUrl}`, origBlob, {
            media_id: refId || downloadUrl,
            model_id: 'sender_original',
            model_label: 'Ảnh gốc trên máy',
            backend: 'local_original',
            width: entry.width || 0,
            height: entry.height || 0,
            src_bytes: origBlob.size,
            super_res: 0,
          }).catch(() => {});
        }
        return { ok: true, ...rec };
      } catch (e) {
        return { ok: false, error: String(e) };
      }
    },

    async getSenderOriginalFromCache(downloadUrl) {
      if (!downloadUrl) return { found: false };
      if (senderOriginalSession.has(downloadUrl)) {
        return { found: true, ...senderOriginalSession.get(downloadUrl) };
      }
      if (window.__chatwStorage && typeof window.__chatwStorage.getAiOutput === 'function') {
        try {
          const hit = await window.__chatwStorage.getAiOutput(`sender_orig:${downloadUrl}`);
          if (hit && hit.found && hit.blob) {
            const url = URL.createObjectURL(hit.blob);
            const rec = {
              url,
              width: hit.width || 0,
              height: hit.height || 0,
              byteSize: hit.blob.size,
              mime: hit.blob.type || 'image/png',
              provenance: 'sender_original',
            };
            senderOriginalSession.set(downloadUrl, rec);
            return { found: true, ...rec };
          }
        } catch (_) {}
      }
      return { found: false };
    },

    // Phase 2: Compress/encode the stashed sender file off-main-thread while sender already sees crisp original!
    async processPendingHandle(handleId, preserveOriginal = false) {
      const entry = pendingSenderFiles.get(handleId);
      if (!entry) return { error: 'pending_handle_not_found' };
      const res = await processSingleFile(entry.file, preserveOriginal, entry.previewUrl);
      return { ...res, handleId };
    },

    // Recover automatically when a cached local dedup reference misses or fails authorization on the server.
    async reencodeFromPreviewUrl(previewUrl, preserveOriginal = null) {
      const entry = previewUrl ? previewUrlToFile.get(previewUrl) : null;
      if (!entry || !entry.file) return { error: 'preview_source_not_found' };
      const pres = preserveOriginal === null ? Boolean(entry.preserveOriginal) : Boolean(preserveOriginal);
      return await processSingleFile(entry.file, pres, previewUrl, true);
    },

    // Return sender's original crisp local bytes so sender's local cache stores the true original!
    async getPendingOriginalB64(handleId) {
      const entry = pendingSenderFiles.get(handleId);
      if (!entry) return null;
      const raw = new Uint8Array(await entry.file.arrayBuffer());
      return {
        bytesB64: bytesToB64(raw),
        mime: entry.file.type || 'application/octet-stream',
        fileName: stripFileExtension(entry.file.name),
        byteSize: raw.byteLength,
        previewUrl: entry.previewUrl,
      };
    },

    releasePendingHandle(handleId, revokeUrl = false) {
      const entry = pendingSenderFiles.get(handleId);
      if (!entry) return;
      pendingSenderFiles.delete(handleId);
      if (revokeUrl && entry.previewUrl) {
        try { URL.revokeObjectURL(entry.previewUrl); } catch (_) {}
      }
    },

    async optimizeFromInput(inputId, preserveOriginal = false) {
      const inp = document.getElementById(inputId);
      if (!inp || !inp.files || !inp.files[0]) return { error: 'no_file' };
      const file = inp.files[0];
      inp.value = '';
      if (await isBlockedSvgBlob(file)) {
        return { error: 'svg_not_allowed' };
      }
      return await processSingleFile(file, preserveOriginal, null);
    },

    async optimizePastedBytes(b64, mime = 'image/png', fileName = 'image.png') {
      const u8 = b64ToBytes(b64);
      if (isBlockedSvgFile(fileName, mime, u8)) {
        return { error: 'svg_not_allowed' };
      }
      const blob = new Blob([u8], { type: mime || 'image/png' });
      let file;
      try {
        file = new File([blob], fileName || 'image.png', { type: mime || 'image/png' });
      } catch (_) {
        file = blob;
        file.name = fileName || 'image.png';
      }
      return await processSingleFile(file, false, null);
    },

    async startVoiceRecording(mode = 'voice') {
      const isMusic = mode === 'music';
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: isMusic
          ? { echoCancellation: false, noiseSuppression: false, autoGainControl: false, channelCount: 2 }
          : { echoCancellation: true, noiseSuppression: true, autoGainControl: true, channelCount: 1 },
      });
      const mime = MediaRecorder.isTypeSupported('audio/webm;codecs=opus') ? 'audio/webm;codecs=opus' :
        MediaRecorder.isTypeSupported('audio/ogg;codecs=opus') ? 'audio/ogg;codecs=opus' : '';
      activeVoiceChunks = [];
      activeVoiceStartedAt = Date.now();
      const bps = isMusic ? 64000 : 28000;
      activeVoiceRecorder = mime ? new MediaRecorder(stream, { mimeType: mime, audioBitsPerSecond: bps }) : new MediaRecorder(stream);
      activeVoiceRecorder.ondataavailable = (e) => { if (e.data && e.data.size > 0) activeVoiceChunks.push(e.data); };
      activeVoiceRecorder.start(200);
      return { ok: true, mime: activeVoiceRecorder.mimeType, bitrateBps: bps };
    },
    async stopVoiceRecording() {
      if (!activeVoiceRecorder) return { error: 'not_recording' };
      const rec = activeVoiceRecorder;
      activeVoiceRecorder = null;
      await new Promise((res) => { rec.onstop = res; rec.stop(); });
      rec.stream.getTracks().forEach((t) => t.stop());
      const blob = new Blob(activeVoiceChunks, { type: rec.mimeType || 'audio/webm' });
      const u8 = new Uint8Array(await blob.arrayBuffer());
      const waveform = await extractVoiceWaveform(u8, 48);
      return {
        bytesB64: bytesToB64(u8),
        mime: blob.type || 'audio/webm',
        durationMs: Date.now() - activeVoiceStartedAt,
        byteSize: u8.byteLength,
        waveform,
      };
    },
  };

  window.__chatwPeerTransfer = {
    async initiateOffer(requestId) {
      const sendSig = (rid, signalType, data) => {
        const body = JSON.stringify({ signal_type: signalType, data });
        if (window.__chatwCore && window.__chatwCore.isConnected()) {
          return window.__chatwCore.rpc('POST', `/api/recovery/requests/${rid}/signal`, body, 'application/json').catch(() => {});
        }
        return fetch(`/api/recovery/requests/${rid}/signal`, {
          method: 'POST',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body,
        }).catch(() => {});
      };
      const entry = getOrCreatePeer(requestId, null, sendSig);
      const dc = entry.pc.createDataChannel('chatw-file', { ordered: true });
      attachDataChannel(requestId, entry, dc);
      const offer = await entry.pc.createOffer();
      await entry.pc.setLocalDescription(offer);
      await sendSig(requestId, 'offer', offer);
      return { ok: true };
    },
    async handleSignal(requestId, signalType, data) {
      const sendSig = (rid, st, d) => {
        const body = JSON.stringify({ signal_type: st, data: d });
        if (window.__chatwCore && window.__chatwCore.isConnected()) {
          return window.__chatwCore.rpc('POST', `/api/recovery/requests/${rid}/signal`, body, 'application/json').catch(() => {});
        }
        return fetch(`/api/recovery/requests/${rid}/signal`, {
          method: 'POST',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body,
        }).catch(() => {});
      };
      const entry = getOrCreatePeer(requestId, null, sendSig);
      if (signalType === 'offer') {
        await entry.pc.setRemoteDescription(new RTCSessionDescription(data));
        const ans = await entry.pc.createAnswer();
        await entry.pc.setLocalDescription(ans);
        await sendSig(requestId, 'answer', ans);
      } else if (signalType === 'answer') {
        await entry.pc.setRemoteDescription(new RTCSessionDescription(data));
      } else if (signalType === 'ice_candidate') {
        try { await entry.pc.addIceCandidate(new RTCIceCandidate(data)); } catch (_) {}
      }
      return { ok: true };
    },
    async sendObjectOverPeer(requestId, refId) {
      const entry = peerConnections.get(requestId);
      if (!entry || !entry.dc || entry.dc.readyState !== 'open') return { ok: false, error: 'datachannel_not_open' };
      const obj = window.__chatwStorage ? await window.__chatwStorage.getObject(refId) : { found: false };
      if (!obj.found) return { ok: false, error: 'local_object_missing' };
      const bytes = b64ToBytes(obj.bytesB64);
      const dc = entry.dc;
      dc.send(JSON.stringify({ kind: 'meta', ref_id: refId, file_name: obj.file_name, mime_type: obj.mime_type, size: bytes.byteLength, sha256: obj.sha256_plain }));
      const CHUNK = 32 * 1024;
      const HIGH = 256 * 1024;
      for (let off = 0; off < bytes.byteLength; off += CHUNK) {
        if (dc.bufferedAmount > HIGH) {
          await new Promise((r) => {
            if (dc.bufferedAmount <= dc.bufferedAmountLowThreshold) r();
            else dc.onbufferedamountlow = () => { dc.onbufferedamountlow = null; r(); };
          });
        }
        dc.send(bytes.slice(off, off + CHUNK));
      }
      dc.send(JSON.stringify({ kind: 'done', ref_id: refId }));
      return { ok: true, sent_bytes: bytes.byteLength };
    },
  };
})();
