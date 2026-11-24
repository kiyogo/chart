// Thin Browser DOM/IndexedDB Shell for ChatW's 100% Pure-Rust Real-ESRGAN / Upscayl Engine (`window.__chatwWasmMedia`).
// ZERO neural network math, ZERO shaders, ZERO weight parsing, ZERO tiling, ZERO quality gate in JS!
// All Real-ESRGAN 4× (`SRVGGNetCompact`) GPU/CPU execution, model selection, color lock, license checks,
// and SSIM quality verification live strictly in Rust (`crates/chatw-media-core/src/ai/realesrgan.rs` & `app/src/storage/realesrgan_gpu.rs`).
(() => {
  const STORAGE_PREF_AI_ENABLED = 'chatw_ai_enhance_enabled';
  const PIPELINE_VERSION = 'rust-realesrgan-upscayl-v8';
  const BLOCKED_LICENSES_MODELS = Object.freeze(['codeformer', 'rvrt', 'apisr', 'upscayl']);
  const APPROVED_MODELS = Object.freeze({
    realesr_animevideov3: { id: 'realesr_animevideov3', cwsrUrl: '/models/realesr_animevideov3.cwsr', license: 'BSD-3-Clause', scale: 4 },
    realesr_general_x4v3: { id: 'realesr_general_x4v3', cwsrUrl: '/models/realesr_general_x4v3.cwsr', license: 'BSD-3-Clause', scale: 4 },
    spab_lite_builtin: { id: 'spab_lite_builtin', license: 'Apache-2.0', scale: 1 },
    span_x2_s: { id: 'span_x2_s', license: 'Apache-2.0', scale: 2 },
    realesrgan_x2plus: { id: 'realesrgan_x2plus', license: 'BSD-3-Clause', scale: 2 },
    realcugan_2x: { id: 'realcugan_2x', license: 'MIT', scale: 2 },
    nanovsr_lite: { id: 'nanovsr_lite', license: 'MIT', scale: 2 },
  });

  function isModelLicenseAllowed(modelId) {
    if (window.__chatwWasmMedia && typeof window.__chatwWasmMedia.isModelLicenseAllowed === 'function') {
      return Boolean(window.__chatwWasmMedia.isModelLicenseAllowed(String(modelId || '')));
    }
    const k = String(modelId || '').toLowerCase();
    return Boolean(k && !BLOCKED_LICENSES_MODELS.some((b) => k.includes(b)) && APPROVED_MODELS[modelId]);
  }

  function isAiEnabled() {
    const v = localStorage.getItem(STORAGE_PREF_AI_ENABLED);
    return v === null ? true : v !== 'false';
  }

  function setAiEnabled(enabled) {
    localStorage.setItem(STORAGE_PREF_AI_ENABLED, enabled ? 'true' : 'false');
    return Boolean(enabled);
  }

  let queue = Promise.resolve();
  function enqueueExclusive(fn) {
    const next = queue.then(fn, fn);
    queue = next.catch(() => {});
    return next;
  }

  const sessionResultCache = new Map();
  const modelBytesMemCache = new Map();
  const MAX_SESSION_URLS = 48;

  function rememberSessionResult(cacheKey, entry) {
    if (sessionResultCache.has(cacheKey)) sessionResultCache.delete(cacheKey);
    sessionResultCache.set(cacheKey, entry);
    while (sessionResultCache.size > MAX_SESSION_URLS) {
      const [oldKey, oldEntry] = sessionResultCache.entries().next().value;
      sessionResultCache.delete(oldKey);
      if (oldEntry && oldEntry.url) {
        try { URL.revokeObjectURL(oldEntry.url); } catch (_) {}
      }
    }
  }

  async function loadModelBytesCached(modelId, cwsrUrl, version) {
    if (modelBytesMemCache.has(modelId)) return modelBytesMemCache.get(modelId);
    if (window.__chatwStorage && typeof window.__chatwStorage.getAiModel === 'function') {
      try {
        const hit = await window.__chatwStorage.getAiModel(modelId);
        if (hit && hit.byteLength > 100000) {
          modelBytesMemCache.set(modelId, hit);
          return hit;
        }
      } catch (_) {}
    }
    const resp = await fetch(cwsrUrl, { cache: 'force-cache' });
    if (!resp.ok) throw new Error(`Model fetch failed (${resp.status}): ${cwsrUrl}`);
    const bytes = new Uint8Array(await resp.arrayBuffer());
    modelBytesMemCache.set(modelId, bytes);
    if (window.__chatwStorage && typeof window.__chatwStorage.putAiModel === 'function') {
      window.__chatwStorage.putAiModel(modelId, bytes, { version: version || '0.2.5.0' }).catch(() => {});
    }
    return bytes;
  }

  async function detectHardware() {
    const wasm = window.__chatwWasmMedia;
    const probe = wasm && typeof wasm.probeRealEsrganGpu === 'function'
      ? wasm.probeRealEsrganGpu()
      : { webgl2Available: false, isSoftware: true, renderer: 'Rust WASM', backend: 'rust_wasm_cpu_realesrgan_srvgg' };
    let lowBattery = false;
    let batteryLevel = null;
    try {
      if (typeof navigator.getBattery === 'function') {
        const bat = await navigator.getBattery();
        batteryLevel = bat.level;
        lowBattery = !bat.charging && bat.level <= 0.15;
      }
    } catch (_) {}
    return {
      backend: probe.backend,
      webgl2Available: Boolean(probe.webgl2Available),
      webgl2Software: Boolean(probe.isSoftware),
      gpuAdapter: probe.renderer || 'Rust GPU',
      cores: Math.max(1, Number(navigator.hardwareConcurrency || 4)),
      ramGb: Number(navigator.deviceMemory || 4),
      batteryLevel,
      lowBattery,
      aiEnabled: isAiEnabled(),
    };
  }

  // Persistent background OffscreenCanvas Worker for zero-jank blob->RGBA decode and 4K JPEG/WebP encode!
  let aiMediaWorker = null;
  let aiMediaWorkerFailed = false;
  let aiWorkerSeq = 1;
  const aiWorkerPending = new Map();

  function getAiMediaWorker() {
    if (aiMediaWorkerFailed || typeof Worker === 'undefined' || typeof OffscreenCanvas === 'undefined') return null;
    if (aiMediaWorker) return aiMediaWorker;
    try {
      const workerCode = `
        self.onmessage = async (ev) => {
          const msg = ev.data || {};
          const id = msg.id;
          try {
            if (msg.type === 'decode') {
              const bmp = await createImageBitmap(msg.blob);
              const rawW = bmp.width;
              const rawH = bmp.height;
              if (rawW < 16 || rawH < 16 || rawW * rawH > 8000000) {
                bmp.close();
                self.postMessage({ id, ok: false, reason: 'dimensions_out_of_ai_bounds' });
                return;
              }
              const maxInEdge = msg.maxInEdge || 1080;
              let procW = rawW;
              let procH = rawH;
              if (Math.max(rawW, rawH) > maxInEdge) {
                const r = maxInEdge / Math.max(rawW, rawH);
                procW = Math.max(32, Math.round(rawW * r));
                procH = Math.max(32, Math.round(rawH * r));
              }
              const c = new OffscreenCanvas(procW, procH);
              const ctx = c.getContext('2d', { willReadFrequently: true });
              ctx.imageSmoothingEnabled = true;
              ctx.imageSmoothingQuality = 'high';
              ctx.drawImage(bmp, 0, 0, procW, procH);
              bmp.close();
              const imgData = ctx.getImageData(0, 0, procW, procH);
              const u8 = new Uint8Array(imgData.data.buffer, imgData.data.byteOffset, imgData.data.byteLength);
              self.postMessage({ id, ok: true, rawW, rawH, procW, procH, rgba: u8 }, [u8.buffer]);
              return;
            }
            if (msg.type === 'encode') {
              const bmp = msg.bitmap;
              let finalW = msg.width || bmp.width;
              let finalH = msg.height || bmp.height;
              const maxOutEdge = msg.maxOutEdge || 3840;
              if (Math.max(finalW, finalH) > maxOutEdge) {
                const r = maxOutEdge / Math.max(finalW, finalH);
                finalW = Math.round(finalW * r);
                finalH = Math.round(finalH * r);
              }
              const c = new OffscreenCanvas(finalW, finalH);
              const ctx = c.getContext('2d');
              ctx.imageSmoothingEnabled = true;
              ctx.imageSmoothingQuality = 'high';
              ctx.drawImage(bmp, 0, 0, finalW, finalH);
              bmp.close();
              let outBlob = null;
              if (!msg.hasAlpha) {
                try {
                  const j = await c.convertToBlob({ type: 'image/jpeg', quality: 0.93 });
                  if (j && j.size > 0 && j.type === 'image/jpeg') outBlob = j;
                } catch (_) {}
              }
              if (!outBlob) {
                try {
                  const w = await c.convertToBlob({ type: 'image/webp', quality: 0.88 });
                  if (w && w.size > 0 && w.type === 'image/webp') outBlob = w;
                } catch (_) {}
              }
              if (!outBlob) {
                outBlob = await c.convertToBlob({ type: 'image/png' });
              }
              self.postMessage({ id, ok: Boolean(outBlob && outBlob.size > 0), blob: outBlob, width: finalW, height: finalH });
              return;
            }
            self.postMessage({ id, ok: false, reason: 'unknown_cmd' });
          } catch (err) {
            self.postMessage({ id, ok: false, reason: String(err) });
          }
        };
      `;
      const blobUrl = URL.createObjectURL(new Blob([workerCode], { type: 'application/javascript' }));
      const w = new Worker(blobUrl);
      w.onmessage = (ev) => {
        const d = ev.data || {};
        const p = aiWorkerPending.get(d.id);
        if (p) {
          aiWorkerPending.delete(d.id);
          p.resolve(d);
        }
      };
      w.onerror = () => {
        aiMediaWorkerFailed = true;
        for (const p of aiWorkerPending.values()) p.reject(new Error('ai_media_worker_failed'));
        aiWorkerPending.clear();
        try { w.terminate(); } catch (_) {}
        aiMediaWorker = null;
      };
      aiMediaWorker = w;
      return w;
    } catch (_) {
      aiMediaWorkerFailed = true;
      return null;
    }
  }

  function callAiMediaWorker(payload, transfers = []) {
    const w = getAiMediaWorker();
    if (!w) return Promise.resolve(null);
    return new Promise((resolve) => {
      const id = aiWorkerSeq++;
      aiWorkerPending.set(id, {
        resolve,
        reject: () => resolve(null),
      });
      try {
        w.postMessage({ ...payload, id }, transfers);
      } catch (_) {
        aiWorkerPending.delete(id);
        resolve(null);
      }
    });
  }

  function yieldUiSlice() {
    return new Promise((resolve) => setTimeout(resolve, 0));
  }

  // Idle GPU shader + cached model pre-warm so clicking an unenhanced image has 0ms compile hitch!
  let idleWarmupScheduled = false;
  function scheduleIdleGpuWarmup() {
    if (idleWarmupScheduled) return;
    idleWarmupScheduled = true;
    const runWarm = async () => {
      try {
        if (!isAiEnabled()) return;
        const wasm = window.__chatwWasmMedia;
        if (wasm && typeof wasm.warmupRealEsrganGpu === 'function') {
          wasm.warmupRealEsrganGpu();
        }
        // Also warm IndexedDB model bytes into RAM if already cached locally (0 network cost!)
        if (window.__chatwStorage && typeof window.__chatwStorage.getAiModel === 'function') {
          for (const mid of ['realesr_animevideov3', 'realesr_general_x4v3']) {
            if (!modelBytesMemCache.has(mid)) {
              const hit = await window.__chatwStorage.getAiModel(mid).catch(() => null);
              if (hit && hit.byteLength > 100000) {
                modelBytesMemCache.set(mid, hit);
              }
            }
          }
        }
      } catch (_) {}
    };
    const trigger = () => {
      if (!window.__chatwWasmMedia || typeof window.__chatwWasmMedia.warmupRealEsrganGpu !== 'function') {
        setTimeout(trigger, 120);
        return;
      }
      if (typeof requestIdleCallback === 'function') {
        requestIdleCallback(() => { runWarm(); }, { timeout: 800 });
      } else {
        setTimeout(() => { runWarm(); }, 150);
      }
    };
    setTimeout(trigger, 120);
  }
  scheduleIdleGpuWarmup();

  async function predecodeImageUrl(url) {
    if (!url || typeof Image === 'undefined') return;
    try {
      const img = new Image();
      img.decoding = 'async';
      img.src = url;
      if (typeof img.decode === 'function') {
        await img.decode();
      }
    } catch (_) {}
  }

  async function encodeCanvasBlob(canvas, hasAlpha) {
    // Opaque 4× images use hardware-SIMD `image/jpeg@0.93` (~8× faster than software WebP on 4K canvases,
    // while 4× super-res gives full original 1:1 chroma resolution!). Transparent images keep lossless-alpha WebP.
    if (!hasAlpha) {
      try {
        const jpeg = await new Promise((resolve) => canvas.toBlob((b) => resolve(b), 'image/jpeg', 0.93));
        if (jpeg && jpeg.size > 0 && jpeg.type === 'image/jpeg') return jpeg;
      } catch (_) {}
    }
    try {
      const webp = await new Promise((resolve) => canvas.toBlob((b) => resolve(b), 'image/webp', 0.88));
      if (webp && webp.size > 0 && webp.type === 'image/webp') return webp;
    } catch (_) {}
    return await new Promise((resolve) => canvas.toBlob((b) => resolve(b), 'image/png'));
  }

  async function restoreImageFromUrl(imageUrl, meta = {}) {
    if (!isAiEnabled()) return { enhanced: false, skipped: true, reason: 'ai_disabled_by_user', url: imageUrl };
    if (meta.provenance === 'original' || meta.provenance === 'sender_original' || meta.isAnimated) {
      return { enhanced: false, skipped: true, reason: 'already_original_or_animated', url: imageUrl };
    }
    if (meta.modelId && !isModelLicenseAllowed(meta.modelId)) {
      return { enhanced: false, skipped: true, reason: 'model_license_blocked', url: imageUrl };
    }

    const origProfileJson = typeof meta.origProfileJson === 'string' ? meta.origProfileJson : '';
    const mediaId = String(meta.mediaId || meta.refId || imageUrl).slice(0, 128);
    const contentHash = String(meta.contentHash || meta.sha256 || 'v1').slice(0, 64);
    const profTag = origProfileJson ? `qp${origProfileJson.length}` : 'noqp';
    // Deduplicate on-device AI outputs across chats by canonical contentHash whenever a real hash is present!
    const hasCanonicalHash = /^[0-9a-f]{32,}$/i.test(contentHash);
    const cacheKey = hasCanonicalHash
      ? `ai:hash:${contentHash}:v7rs:${meta.modelId || 'auto'}:${profTag}:${PIPELINE_VERSION}`
      : `ai:${mediaId}:v7rs:${contentHash}:${meta.modelId || 'auto'}:${profTag}:${PIPELINE_VERSION}`;

    if (sessionResultCache.has(cacheKey)) {
      return { ...sessionResultCache.get(cacheKey), enhanced: true, cacheHit: true, cacheTier: 'memory', provenance: 'ai_restored' };
    }

    if (window.__chatwStorage && typeof window.__chatwStorage.getAiOutput === 'function') {
      try {
        const hit = await window.__chatwStorage.getAiOutput(cacheKey);
        if (hit && hit.found && hit.blob) {
          const entry = {
            enhanced: true,
            cacheHit: true,
            cacheTier: 'indexeddb',
            url: URL.createObjectURL(hit.blob),
            modelId: hit.model_id || 'realesr_animevideov3',
            modelLabel: hit.model_label || 'Real-ESRGAN Anime v3 (4×)',
            backend: hit.backend || 'cached',
            ssim: hit.ssim || null,
            width: hit.width || 0,
            height: hit.height || 0,
            srcWidth: hit.src_width || 0,
            srcHeight: hit.src_height || 0,
            srcBytes: hit.src_bytes || 0,
            outBytes: hit.blob.size,
            superRes: hit.super_res || 4,
            matchPct: hit.match_pct != null ? hit.match_pct : null,
            provenance: 'ai_restored',
          };
          await predecodeImageUrl(entry.url);
          rememberSessionResult(cacheKey, entry);
          return entry;
        }
      } catch (_) {}
    }

    // Fast cache-only lookup for historical / new-device images (shows sleek corner SVG icon until clicked!)
    if (meta.cacheOnly) {
      return { enhanced: false, cacheMiss: true, url: imageUrl };
    }

    const reportProgress = (pct) => {
      if (typeof meta.onProgress === 'function') {
        try { meta.onProgress(Math.max(0, Math.min(100, Math.round(pct)))); } catch (_) {}
      }
    };

    return await enqueueExclusive(async () => {
      if (sessionResultCache.has(cacheKey)) {
        reportProgress(100);
        return { ...sessionResultCache.get(cacheKey), enhanced: true, cacheHit: true, cacheTier: 'memory', provenance: 'ai_restored' };
      }

      const wasm = window.__chatwWasmMedia;
      if (!wasm || typeof wasm.restoreImageRealEsrgan !== 'function') {
        return { enhanced: false, skipped: true, reason: 'rust_wasm_not_ready', url: imageUrl };
      }

      const t0 = performance.now();
      reportProgress(2);
      const hw = await detectHardware();
      if (hw.lowBattery && !meta.forceRun) {
        return { enhanced: false, skipped: true, reason: 'low_battery_power_saver', url: imageUrl };
      }

      let srcBytes = 0;
      let rawW = 0;
      let rawH = 0;
      let procW = 0;
      let procH = 0;
      let srcU8 = null;
      const maxInEdge = hw.webgl2Available && !hw.webgl2Software ? 1080 : 960;
      try {
        const resp = await fetch(imageUrl);
        if (!resp.ok) return { enhanced: false, skipped: true, reason: 'fetch_failed', url: imageUrl };
        const blob = await resp.blob();
        srcBytes = blob.size;
        // Offload decode + downscale + getImageData completely off the main thread!
        const wDec = await callAiMediaWorker({ type: 'decode', blob, maxInEdge });
        if (wDec && wDec.ok && wDec.rgba) {
          rawW = wDec.rawW;
          rawH = wDec.rawH;
          procW = wDec.procW;
          procH = wDec.procH;
          srcU8 = wDec.rgba;
        } else if (wDec && wDec.reason === 'dimensions_out_of_ai_bounds') {
          return { enhanced: false, skipped: true, reason: 'dimensions_out_of_ai_bounds', url: imageUrl };
        } else {
          const bitmap = await createImageBitmap(blob);
          rawW = bitmap.width;
          rawH = bitmap.height;
          if (rawW < 16 || rawH < 16 || rawW * rawH > 8_000_000) {
            bitmap.close();
            return { enhanced: false, skipped: true, reason: 'dimensions_out_of_ai_bounds', url: imageUrl };
          }
          procW = rawW;
          procH = rawH;
          if (Math.max(rawW, rawH) > maxInEdge) {
            const r = maxInEdge / Math.max(rawW, rawH);
            procW = Math.max(32, Math.round(rawW * r));
            procH = Math.max(32, Math.round(rawH * r));
          }
          const srcCanvas = document.createElement('canvas');
          srcCanvas.width = procW;
          srcCanvas.height = procH;
          const srcCtx = srcCanvas.getContext('2d', { willReadFrequently: true });
          srcCtx.imageSmoothingEnabled = true;
          srcCtx.imageSmoothingQuality = 'high';
          srcCtx.drawImage(bitmap, 0, 0, procW, procH);
          bitmap.close();
          const imgData = srcCtx.getImageData(0, 0, procW, procH);
          srcU8 = new Uint8Array(imgData.data.buffer, imgData.data.byteOffset, imgData.data.byteLength);
        }
      } catch (err) {
        return { enhanced: false, skipped: true, reason: 'decode_failed', error: String(err), url: imageUrl };
      }

      await yieldUiSlice();

      // 1. Model selection strictly in Pure Rust (`selectRealEsrganModel`)
      const sel = wasm.selectRealEsrganModel(srcU8, procW, procH, origProfileJson, meta.modelId || '');
      if (!sel || sel.skip) {
        return { enhanced: false, skipped: true, reason: (sel && sel.reason) || 'skipped', url: imageUrl };
      }

      // 2. Load compact FP16 Real-ESRGAN weights (cached permanently in IndexedDB `ai_models`)
      reportProgress(4);
      let modelBytes;
      try {
        modelBytes = await loadModelBytesCached(sel.modelId, sel.cwsrUrl, sel.version);
      } catch (err) {
        return { enhanced: false, skipped: true, reason: 'model_download_failed', error: String(err), url: imageUrl };
      }

      await yieldUiSlice();

      // 3. Execute 100% Pure-Rust FP16 8-MRT Real-ESRGAN 4× + Zero-Alteration Color Anchor + AMD CAS!
      // All GPU layer batches & quality-gate PBO readback use non-blocking WebGL2 FenceSync!
      const tGpu0 = performance.now();
      const res = await wasm.restoreImageRealEsrgan(srcU8, procW, procH, sel.modelId, modelBytes, origProfileJson, reportProgress);
      const gpuMs = Math.round(performance.now() - tGpu0);
      if (!res || !res.ok || (!res.canvas && !res.rgba)) {
        return { enhanced: false, skipped: true, reason: (res && res.error) || 'rust_realesrgan_failed', url: imageUrl };
      }

      // 4. Zero-stall off-main-thread 4K JPEG/WebP encoding via OffscreenCanvas Web Worker!
      const tEnc0 = performance.now();
      let finalW = res.width;
      let finalH = res.height;
      let outBlob = null;
      if (getAiMediaWorker()) {
        try {
          let bmp = null;
          if (res.canvas) {
            bmp = await createImageBitmap(res.canvas);
          } else if (res.rgba) {
            bmp = await createImageBitmap(
              new ImageData(new Uint8ClampedArray(res.rgba.buffer, res.rgba.byteOffset, res.rgba.byteLength), res.width, res.height),
            );
          }
          if (bmp) {
            const wEnc = await callAiMediaWorker(
              { type: 'encode', bitmap: bmp, width: res.width, height: res.height, hasAlpha: Boolean(res.hasAlpha), maxOutEdge: 3840 },
              [bmp],
            );
            if (wEnc && wEnc.ok && wEnc.blob) {
              outBlob = wEnc.blob;
              finalW = wEnc.width || finalW;
              finalH = wEnc.height || finalH;
            }
          }
        } catch (_) {}
      }

      if (!outBlob) {
        let finalCanvas = res.canvas || null;
        if (!finalCanvas) {
          const outCanvas = document.createElement('canvas');
          outCanvas.width = res.width;
          outCanvas.height = res.height;
          const outCtx = outCanvas.getContext('2d');
          outCtx.putImageData(
            new ImageData(new Uint8ClampedArray(res.rgba.buffer, res.rgba.byteOffset, res.rgba.byteLength), res.width, res.height),
            0,
            0,
          );
          finalCanvas = outCanvas;
        }
        if (Math.max(finalW, finalH) > 3840) {
          const r = 3840 / Math.max(finalW, finalH);
          const cw = Math.round(finalW * r);
          const ch = Math.round(finalH * r);
          const fit = document.createElement('canvas');
          fit.width = cw;
          fit.height = ch;
          const fctx = fit.getContext('2d');
          fctx.imageSmoothingEnabled = true;
          fctx.imageSmoothingQuality = 'high';
          fctx.drawImage(finalCanvas, 0, 0, cw, ch);
          finalCanvas = fit;
          finalW = cw;
          finalH = ch;
        }
        outBlob = await encodeCanvasBlob(finalCanvas, Boolean(res.hasAlpha));
      }
      const encodeMs = Math.round(performance.now() - tEnc0);
      if (!outBlob || outBlob.size === 0) {
        return { enhanced: false, skipped: true, reason: 'output_encode_failed', url: imageUrl };
      }
      const outUrl = URL.createObjectURL(outBlob);
      await predecodeImageUrl(outUrl);
      reportProgress(100);

      const elapsedMs = Math.round(performance.now() - t0);
      const entry = {
        enhanced: true,
        cacheHit: false,
        cacheTier: 'computed',
        url: outUrl,
        modelId: res.modelId,
        modelLabel: res.modelLabel,
        modelVersion: sel.version,
        backend: res.backend,
        elapsedMs,
        gpuMs,
        encodeMs,
        ssim: Number(Number(res.ssim || 0).toFixed(4)),
        meanColorDrift: Number(Number(res.meanColorDrift || 0).toFixed(4)),
        matchPct: Number(Number(res.matchPct || 99).toFixed(1)),
        guidedByOriginal: Boolean(origProfileJson),
        width: finalW,
        height: finalH,
        srcWidth: rawW,
        srcHeight: rawH,
        srcBytes,
        outBytes: outBlob.size,
        superRes: res.scale || 4,
        provenance: 'ai_restored',
      };
      rememberSessionResult(cacheKey, entry);

      if (window.__chatwStorage && typeof window.__chatwStorage.putAiOutput === 'function') {
        window.__chatwStorage
          .putAiOutput(cacheKey, outBlob, {
            media_id: mediaId,
            model_id: entry.modelId,
            model_label: entry.modelLabel,
            model_version: entry.modelVersion,
            backend: entry.backend,
            ssim: entry.ssim,
            compute_cost_ms: elapsedMs,
            width: finalW,
            height: finalH,
            src_width: rawW,
            src_height: rawH,
            src_bytes: srcBytes,
            super_res: entry.superRes,
            match_pct: entry.matchPct,
          })
          .catch(() => {});
      }
      return entry;
    });
  }

  window.__chatwAiRestorer = {
    PIPELINE_VERSION,
    APPROVED_MODELS,
    BLOCKED_LICENSES_MODELS,
    isModelLicenseAllowed,
    isAiEnabled,
    setAiEnabled,
    detectHardware,
    restoreImageFromUrl,
    async getStatus() {
      const hw = await detectHardware();
      let cacheStats = null;
      if (window.__chatwStorage && typeof window.__chatwStorage.getSmartCacheStats === 'function') {
        cacheStats = await window.__chatwStorage.getSmartCacheStats().catch(() => null);
      }
      return {
        ...hw,
        pipelineVersion: PIPELINE_VERSION,
        approvedModels: Object.keys(APPROVED_MODELS),
        blockedModels: BLOCKED_LICENSES_MODELS,
        sessionCacheCount: sessionResultCache.size,
        cacheStats,
      };
    },
    async clearAiCache() {
      for (const e of sessionResultCache.values()) {
        if (e && e.url) {
          try { URL.revokeObjectURL(e.url); } catch (_) {}
        }
      }
      sessionResultCache.clear();
      if (window.__chatwStorage && typeof window.__chatwStorage.clearAiCache === 'function') {
        return await window.__chatwStorage.clearAiCache();
      }
      return { removed: 0, freed_bytes: 0 };
    },
  };
})();
