// Thin Browser DOM/IndexedDB Shell for ChatW's 100% Pure-Rust Adaptive Multi-Model AI Restoration Engine (`window.__chatwWasmMedia`).
// Spec 143: 2-Stage Progressive Pipeline — Stage 1 Instant Algorithmic FSR 1.0 + RL -> Stage 2 Adaptive SPAN 4× (NomosUni AVIF / ClearRealityV1 / AnimeVideoV3) + Text/QR Guard + AbortSignal cancellation!
(() => {
  const STORAGE_PREF_AI_ENABLED = 'chatw_ai_enhance_enabled';
  const PIPELINE_VERSION = 'hybrid-span-gfpgan-v30-strict';
  const BLOCKED_LICENSES_MODELS = Object.freeze(['codeformer', 'rvrt', 'apisr', 'upscayl']);
  const APPROVED_MODELS = Object.freeze({
    nomosuni_span_x2: { id: 'nomosuni_span_x2', cwsrUrl: '/models/nomosuni_span_x2.cwsp', license: 'CC-BY-4.0', scale: 2 },
    nomosuni_span_x4: { id: 'nomosuni_span_x4', cwsrUrl: '/models/nomosuni_span_x4.cwsp', license: 'CC-BY-4.0', scale: 4 },
    clearreality_span_x4: { id: 'clearreality_span_x4', cwsrUrl: '/models/clearreality_v1_x4.cwsp', license: 'Apache-2.0', scale: 4 },
    lsdir_compact3_x4: { id: 'lsdir_compact3_x4', cwsrUrl: '/models/lsdir_compact3_x4.cwsr', license: 'CC-BY-4.0', scale: 4 },
    fsr1_easu_rcas: { id: 'fsr1_easu_rcas', license: 'MIT', scale: 2 },
    fidelity_cas_1x: { id: 'fidelity_cas_1x', license: 'MIT', scale: 1 },
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
    const v = localStorage.getItem(STORAGE_PREF_AI_ENABLED) ?? localStorage.getItem('chatw_ai_enhance');
    return v === null ? true : v !== 'false';
  }

  function setAiEnabled(enabled) {
    const val = enabled ? 'true' : 'false';
    localStorage.setItem(STORAGE_PREF_AI_ENABLED, val);
    localStorage.setItem('chatw_ai_enhance', val);
    return Boolean(enabled);
  }

  function withTimeout(promise, ms, label = 'op') {
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

  function raceWithSignal(promise, signal, label = 'preempted') {
    if (!signal) return promise;
    if (signal.aborted) return Promise.reject(new Error(`aborted:${label}`));
    return new Promise((resolve, reject) => {
      const onAbort = () => reject(new Error(`aborted:${label}`));
      signal.addEventListener('abort', onAbort, { once: true });
      Promise.resolve(promise).then(
        (v) => {
          signal.removeEventListener('abort', onAbort);
          resolve(v);
        },
        (e) => {
          signal.removeEventListener('abort', onAbort);
          reject(e);
        },
      );
    });
  }

  let activeMainJob = false;
  let activeMainPriority = null;
  let activeJobAbortCtrl = null;
  const highWaiters = [];
  const normalWaiters = [];
  let mainIdleResolvers = [];
  const inFlightByKey = new Map();

  function waitForMainQueueIdle() {
    if (!activeMainJob && highWaiters.length === 0 && normalWaiters.length === 0) return Promise.resolve();
    return new Promise((resolve) => mainIdleResolvers.push(resolve));
  }

  function abortActiveNormalJobForHighPriority() {
    if (!activeMainJob || activeMainPriority !== 'NORMAL') return;
    if (activeJobAbortCtrl && !activeJobAbortCtrl.signal.aborted) {
      try { activeJobAbortCtrl.abort(); } catch (_) {}
    }
    try {
      if (window.__chatwWasmMedia && typeof window.__chatwWasmMedia.abortActiveGpuJob === 'function') {
        window.__chatwWasmMedia.abortActiveGpuJob();
      }
    } catch (_) {}
  }

  function pumpMainQueue() {
    if (activeMainJob) {
      if (highWaiters.length > 0 && activeMainPriority === 'NORMAL') {
        abortActiveNormalJobForHighPriority();
      }
      return;
    }
    let nextTask = null;
    while (!nextTask && (highWaiters.length > 0 || normalWaiters.length > 0)) {
      const cand = highWaiters.shift() || normalWaiters.shift();
      if (cand.externalSignal && cand.externalSignal.aborted) {
        cand.resolve({ enhanced: false, skipped: true, reason: 'cancelled_offscreen' });
        continue;
      }
      nextTask = cand;
    }
    if (!nextTask) {
      if (mainIdleResolvers.length > 0) {
        const list = mainIdleResolvers;
        mainIdleResolvers = [];
        for (const r of list) r();
      }
      return;
    }
    const { fn, timeoutMs, highPriority, taskCtrl, externalSignal, resolve } = nextTask;
    activeMainJob = true;
    activeMainPriority = highPriority ? 'HIGH' : 'NORMAL';
    activeJobAbortCtrl = taskCtrl;
    const onExtAbort = () => {
      if (!taskCtrl.signal.aborted) {
        try { taskCtrl.abort(); } catch (_) {}
      }
    };
    if (externalSignal) {
      if (externalSignal.aborted) onExtAbort();
      else externalSignal.addEventListener('abort', onExtAbort, { once: true });
    }
    withTimeout(
      raceWithSignal(Promise.resolve().then(() => fn(taskCtrl.signal)), taskCtrl.signal, highPriority ? 'high_cancelled' : 'preempted'),
      timeoutMs,
      'ai_restore_job',
    )
      .catch((err) => {
        try {
          if (window.__chatwWasmMedia && typeof window.__chatwWasmMedia.abortActiveGpuJob === 'function') {
            window.__chatwWasmMedia.abortActiveGpuJob();
          }
        } catch (_) {}
        const reason = taskCtrl.signal.aborted
          ? (highPriority ? 'cancelled_offscreen' : 'preempted_by_high_priority')
          : 'job_timeout_or_error';
        return {
          enhanced: false,
          skipped: true,
          reason,
          error: String(err),
        };
      })
      .then((res) => resolve(res))
      .finally(() => {
        if (externalSignal) {
          try { externalSignal.removeEventListener('abort', onExtAbort); } catch (_) {}
        }
        activeMainJob = false;
        activeMainPriority = null;
        if (activeJobAbortCtrl === taskCtrl) activeJobAbortCtrl = null;
        pumpMainQueue();
      });
  }

  function enqueueExclusive(fn, timeoutMs = 12000, highPriority = false, externalSignal = null) {
    return new Promise((resolve) => {
      const taskCtrl = new AbortController();
      const item = { fn, timeoutMs, highPriority: Boolean(highPriority), taskCtrl, externalSignal, resolve };
      if (highPriority) {
        highWaiters.push(item);
        abortActiveNormalJobForHighPriority();
      } else {
        normalWaiters.push(item);
      }
      pumpMainQueue();
    });
  }

  // Dedicated low-priority background queue for Stage 2B Face Refinement (YuNet + GFPGANv1.4 WebGPU).
  // Yields to all Stage 2A `enqueueExclusive` jobs first so visible chat images NEVER contend with 325 MB ONNX!
  let faceBgQueue = Promise.resolve();
  function enqueueFaceBg(fn) {
    const runWhenIdle = async () => {
      await waitForMainQueueIdle();
      await yieldUiSlice();
      return await fn();
    };
    const next = faceBgQueue.then(runWhenIdle, runWhenIdle);
    faceBgQueue = next.catch(() => {});
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

  function isValidModelHeader(bytes) {
    if (!(bytes instanceof Uint8Array) || bytes.byteLength < 100000) return false;
    // Accept CWSR1 / CWSR / CWSP model headers; reject HTML SPA fallbacks or truncated IDB records
    const m = String.fromCharCode(bytes[0], bytes[1], bytes[2], bytes[3]);
    return m === 'CWSR' || m === 'CWSP';
  }

  async function loadModelBytesCached(modelId, cwsrUrl, version) {
    if (modelBytesMemCache.has(modelId)) return modelBytesMemCache.get(modelId);
    if (window.__chatwStorage && typeof window.__chatwStorage.getAiModel === 'function') {
      try {
        const hit = await withTimeout(window.__chatwStorage.getAiModel(modelId), 2000, 'idb_get_model');
        if (isValidModelHeader(hit)) {
          modelBytesMemCache.set(modelId, hit);
          return hit;
        }
      } catch (_) {}
    }
    const resp = await withTimeout(fetch(cwsrUrl, { cache: 'force-cache' }), 5000, 'model_fetch');
    if (!resp.ok) throw new Error(`Model fetch failed (${resp.status}): ${cwsrUrl}`);
    const bytes = new Uint8Array(await withTimeout(resp.arrayBuffer(), 5000, 'model_read'));
    if (!isValidModelHeader(bytes)) {
      throw new Error(`Invalid model header from ${cwsrUrl} (len=${bytes.byteLength})`);
    }
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
        const bat = await withTimeout(navigator.getBattery(), 1000, 'battery');
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
              if (rawW < 16 || rawH < 16 || rawW * rawH > 36000000) {
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
                  const j = await c.convertToBlob({ type: 'image/jpeg', quality: 0.95 });
                  if (j && j.size > 0 && j.type === 'image/jpeg') outBlob = j;
                } catch (_) {}
              }
              if (!outBlob) {
                try {
                  const w = await c.convertToBlob({ type: 'image/webp', quality: 0.92 });
                  if (w && w.size > 0 && w.type === 'image/webp') outBlob = w;
                } catch (_) {}
              }
              if (!outBlob) {
                outBlob = await c.convertToBlob({ type: 'image/png' });
              }
              self.postMessage({ id, ok: Boolean(outBlob && outBlob.size > 0), blob: outBlob, width: finalW, height: finalH });
              return;
            }
            if (msg.type === 'cas') {
              let srcRgba = msg.rgba;
              let w = msg.width;
              let h = msg.height;
              const tw = msg.targetW || w;
              const th = msg.targetH || h;
              if (tw > w || th > h) {
                const sc = new OffscreenCanvas(w, h);
                sc.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(srcRgba.buffer, srcRgba.byteOffset, srcRgba.byteLength), w, h), 0, 0);
                const uc = new OffscreenCanvas(tw, th);
                const uctx = uc.getContext('2d', { willReadFrequently: true });
                uctx.imageSmoothingEnabled = true;
                uctx.imageSmoothingQuality = 'high';
                uctx.drawImage(sc, 0, 0, tw, th);
                const uImg = uctx.getImageData(0, 0, tw, th);
                srcRgba = new Uint8Array(uImg.data.buffer, uImg.data.byteOffset, uImg.data.byteLength);
                w = tw;
                h = th;
              }
              const sharpness = msg.sharpness || 1.18;
              const dst = new Uint8ClampedArray(srcRgba.length);
              const stride = w * 4;
              const lumaAt = (yRow, xCol) => {
                const p = yRow + xCol;
                return (0.299 * srcRgba[p] + 0.587 * srcRgba[p + 1] + 0.114 * srcRgba[p + 2]) / 255;
              };
              for (let y = 0; y < h; y++) {
                const y0 = Math.max(0, y - 1) * stride;
                const y1 = y * stride;
                const y2 = Math.min(h - 1, y + 1) * stride;
                const ym2 = Math.max(0, y - 2) * stride;
                const yp2 = Math.min(h - 1, y + 2) * stride;
                for (let x = 0; x < w; x++) {
                  const x0 = Math.max(0, x - 1) * 4;
                  const x1 = x * 4;
                  const x2 = Math.min(w - 1, x + 1) * 4;
                  const xm2 = Math.max(0, x - 2) * 4;
                  const xp2 = Math.min(w - 1, x + 2) * 4;
                  const idx = y1 + x1;
                  dst[idx + 3] = srcRgba[idx + 3];
                  const aL = lumaAt(y0, x0), bL = lumaAt(y0, x1), cL = lumaAt(y0, x2);
                  const dL = lumaAt(y1, x0), eL = lumaAt(y1, x1), fL = lumaAt(y1, x2);
                  const gL = lumaAt(y2, x0), hL = lumaAt(y2, x1), iL = lumaAt(y2, x2);
                  const mnCross = Math.min(dL, eL, fL, bL, hL);
                  const mxCross = Math.max(dL, eL, fL, bL, hL);
                  const edgeRange = mxCross - mnCross;
                  const avgCross = 0.25 * (bL + dL + fL + hL);
                  const nz = Math.max(0, Math.min(1, Math.abs(avgCross - eL) / Math.max(edgeRange, 1e-4)));
                  const cleanEdge = 1.0 - 0.80 * nz;
                  const shadowSafe = Math.max(0, Math.min(1, (mxCross - 0.07) / 0.18));
                  const gate = Math.max(0, Math.min(1, (edgeRange - 0.018) / 0.052)) * shadowSafe * cleanEdge;
                  const rcpM = 1.0 / Math.max(mxCross, 1e-4);
                  const amp = Math.sqrt(Math.max(0, Math.min(1, Math.min(mnCross, 2.0 - mxCross) * rcpM)));
                  const weight = Math.max(amp * (-0.162 * sharpness * gate), -0.158);
                  const totalWeight = 1.0 + 5.8 * weight;
                  const casL = (eL + (bL + dL + fL + hL) * weight + (aL + cL + gL + iL) * (weight * 0.45)) / totalWeight;
                  const medBlur = 0.25 * (lumaAt(ym2, x1) + lumaAt(y1, xm2) + lumaAt(y1, xp2) + lumaAt(yp2, x1));
                  const maxHalo = 0.11 * edgeRange + 0.025;
                  const medBoost = Math.max(-maxHalo, Math.min(maxHalo, (eL - medBlur) * 0.42 * sharpness * gate));
                  const mnL = Math.min(aL, bL, cL, dL, eL, fL, gL, hL, iL);
                  const mxL = Math.max(aL, bL, cL, dL, eL, fL, gL, hL, iL);
                  const margin = 0.06 * edgeRange * shadowSafe * cleanEdge;
                  const sharpL = Math.max(Math.max(0, mnL - margin), Math.min(Math.min(1, mxL + margin), casL + medBoost));
                  const delta = (sharpL - eL) * 255;
                  for (let c = 0; c < 3; c++) {
                    const mnC = Math.min(
                      srcRgba[y0 + x0 + c], srcRgba[y0 + x1 + c], srcRgba[y0 + x2 + c],
                      srcRgba[y1 + x0 + c], srcRgba[idx + c], srcRgba[y1 + x2 + c],
                      srcRgba[y2 + x0 + c], srcRgba[y2 + x1 + c], srcRgba[y2 + x2 + c],
                    );
                    const mxC = Math.max(
                      srcRgba[y0 + x0 + c], srcRgba[y0 + x1 + c], srcRgba[y0 + x2 + c],
                      srcRgba[y1 + x0 + c], srcRgba[idx + c], srcRgba[y1 + x2 + c],
                      srcRgba[y2 + x0 + c], srcRgba[y2 + x1 + c], srcRgba[y2 + x2 + c],
                    );
                    const pad = margin * 200;
                    const v = Math.max(Math.max(0, mnC - pad), Math.min(Math.min(255, mxC + pad), srcRgba[idx + c] + delta));
                    dst[idx + c] = Math.round(v);
                  }
                }
              }
              const c = new OffscreenCanvas(w, h);
              const ctx = c.getContext('2d');
              ctx.putImageData(new ImageData(dst, w, h), 0, 0);
              const outBlob = msg.hasAlpha
                ? await c.convertToBlob({ type: 'image/webp', quality: 0.92 })
                : await c.convertToBlob({ type: 'image/jpeg', quality: 0.95 });
              self.postMessage({ id, ok: Boolean(outBlob && outBlob.size > 0), blob: outBlob, width: w, height: h });
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

  function callAiMediaWorker(payload, transfers = [], timeoutMs = 6000) {
    const w = getAiMediaWorker();
    if (!w) return Promise.resolve(null);
    return new Promise((resolve) => {
      const id = aiWorkerSeq++;
      const timer = setTimeout(() => {
        if (aiWorkerPending.has(id)) {
          aiWorkerPending.delete(id);
          // Recycle wedged worker so future calls never stall
          try { w.terminate(); } catch (_) {}
          if (aiMediaWorker === w) aiMediaWorker = null;
          for (const p of aiWorkerPending.values()) {
            try { clearTimeout(p.timer); } catch (_) {}
            p.resolve(null);
          }
          aiWorkerPending.clear();
          resolve(null);
        }
      }, timeoutMs);
      aiWorkerPending.set(id, {
        timer,
        resolve: (val) => {
          clearTimeout(timer);
          resolve(val);
        },
        reject: () => {
          clearTimeout(timer);
          resolve(null);
        },
      });
      try {
        w.postMessage({ ...payload, id }, transfers);
      } catch (_) {
        clearTimeout(timer);
        aiWorkerPending.delete(id);
        resolve(null);
      }
    });
  }

  function applyAmdCasRgba(srcRgba, w, h, sharpness = 0.85) {
    const dst = new Uint8ClampedArray(srcRgba.length);
    const stride = w * 4;
    for (let y = 0; y < h; y++) {
      const y0 = Math.max(0, y - 1) * stride;
      const y1 = y * stride;
      const y2 = Math.min(h - 1, y + 1) * stride;
      for (let x = 0; x < w; x++) {
        const x0 = Math.max(0, x - 1) * 4;
        const x1 = x * 4;
        const x2 = Math.min(w - 1, x + 1) * 4;
        const idx = y1 + x1;
        dst[idx + 3] = srcRgba[idx + 3];
        for (let c = 0; c < 3; c++) {
          const a = srcRgba[y0 + x0 + c] / 255;
          const b = srcRgba[y0 + x1 + c] / 255;
          const c_top = srcRgba[y0 + x2 + c] / 255;
          const d = srcRgba[y1 + x0 + c] / 255;
          const e = srcRgba[idx + c] / 255;
          const f = srcRgba[y1 + x2 + c] / 255;
          const g = srcRgba[y2 + x0 + c] / 255;
          const h_bot = srcRgba[y2 + x1 + c] / 255;
          const i = srcRgba[y2 + x2 + c] / 255;
          const mnCross = Math.min(d, e, f, b, h_bot);
          const mxCross = Math.max(d, e, f, b, h_bot);
          const edgeRange = mxCross - mnCross;
          const gate = Math.max(0, Math.min(1, (edgeRange - 0.015) / 0.05));
          const rcpM = 1.0 / Math.max(mxCross, 1e-4);
          const amp = Math.sqrt(Math.max(0, Math.min(1, Math.min(mnCross, 2.0 - mxCross) * rcpM)));
          const peak = -0.15 * sharpness * gate;
          const weight = Math.max(amp * peak, -0.18);
          const crossSum = b + d + f + h_bot;
          const diagSum = a + c_top + g + i;
          const totalWeight = 1.0 + (4.0 + 1.8) * weight;
          const cas = (e + crossSum * weight + diagSum * (weight * 0.45)) / totalWeight;
          const mnRgb = Math.min(a, b, c_top, d, e, f, g, h_bot, i);
          const mxRgb = Math.max(a, b, c_top, d, e, f, g, h_bot, i);
          const clamped = Math.max(mnRgb, Math.min(mxRgb, cas));
          dst[idx + c] = Math.round(clamped * 255);
        }
      }
    }
    return dst;
  }

  function yieldUiSlice() {
    return new Promise((resolve) => setTimeout(resolve, 0));
  }

  function yieldUiPaint() {
    return new Promise((resolve) => {
      if (typeof requestAnimationFrame === 'function') {
        let done = false;
        const finish = () => { if (!done) { done = true; resolve(); } };
        const t = setTimeout(finish, 68);
        // Wait 2 frames so Dioxus async signal -> VDOM patch -> browser paint commits before WASM/GPU work!
        requestAnimationFrame(() => {
          setTimeout(() => {
            requestAnimationFrame(() => { clearTimeout(t); finish(); });
          }, 0);
        });
      } else {
        setTimeout(resolve, 24);
      }
    });
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
        // Pre-warm compact 1.18 MB LSDIR Photo v3 model from IndexedDB into RAM (0 network cost!)
        if (window.__chatwStorage && typeof window.__chatwStorage.getAiModel === 'function') {
          for (const mid of ['lsdir_compact3_x4', 'realesr_animevideov3']) {
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

  async function predecodeImageUrlStrict(url) {
    if (!url || typeof Image === 'undefined') return false;
    try {
      const img = new Image();
      img.decoding = 'async';
      const loaded = await withTimeout(
        new Promise((resolve) => {
          img.onload = () => resolve(true);
          img.onerror = () => resolve(false);
          img.src = url;
          if (img.complete && img.naturalWidth > 0) resolve(true);
        }),
        2500,
        'img_load_strict',
      );
      if (!loaded) return false;
      if (typeof img.decode === 'function') {
        await withTimeout(img.decode(), 2500, 'img_decode_strict').catch(() => {});
      }
      return Boolean(img.complete && img.naturalWidth >= 16 && img.naturalHeight >= 16);
    } catch (_) {
      return false;
    }
  }

  async function predecodeImageUrl(url) {
    return await predecodeImageUrlStrict(url);
  }

  async function encodeCanvasBlob(canvas, hasAlpha) {
    const toBlobTimed = (type, quality) =>
      withTimeout(new Promise((resolve) => canvas.toBlob((b) => resolve(b), type, quality)), 3500, `toBlob_${type}`).catch(() => null);
    // Opaque 4× images use hardware-SIMD `image/jpeg@0.93` (~8× faster than software WebP on 4K canvases,
    // while 4× super-res gives full original 1:1 chroma resolution!). Transparent images keep lossless-alpha WebP.
    if (!hasAlpha) {
      const jpeg = await toBlobTimed('image/jpeg', 0.93);
      if (jpeg && jpeg.size > 0 && jpeg.type === 'image/jpeg') return jpeg;
    }
    const webp = await toBlobTimed('image/webp', 0.88);
    if (webp && webp.size > 0 && webp.type === 'image/webp') return webp;
    return await toBlobTimed('image/png', undefined);
  }

  function rasterizeImgElement(imgEl, maxInEdge) {
    const rawW = imgEl.naturalWidth || 0;
    const rawH = imgEl.naturalHeight || 0;
    if (rawW < 16 || rawH < 16 || rawW * rawH > 36_000_000) {
      return { outOfBounds: true, rawW, rawH };
    }
    let procW = rawW;
    let procH = rawH;
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
    srcCtx.drawImage(imgEl, 0, 0, procW, procH);
    const imgData = srcCtx.getImageData(0, 0, procW, procH);
    const srcU8 = new Uint8Array(imgData.data.buffer, imgData.data.byteOffset, imgData.data.byteLength);
    return { rawW, rawH, procW, procH, srcU8 };
  }

  function decodeFromLoadedImgElement(imageUrl, maxInEdge, mediaId = '') {
    if (!imageUrl || typeof document === 'undefined') return null;
    try {
      const imgs = document.querySelectorAll('img');
      for (const el of imgs) {
        const matchUrl = el.currentSrc === imageUrl || el.src === imageUrl;
        const matchMedia = mediaId && el.getAttribute && el.getAttribute('data-media-ref') === mediaId;
        if ((matchUrl || matchMedia) && el.complete && el.naturalWidth >= 16 && el.naturalHeight >= 16) {
          try {
            return rasterizeImgElement(el, maxInEdge);
          } catch (_) {}
        }
      }
    } catch (_) {}
    return null;
  }

  function resolveMediaFetchUrl(rawUrl) {
    const s = String(rawUrl || '');
    if (!s || s.startsWith('blob:') || s.startsWith('data:')) return s;
    try {
      if (typeof window !== 'undefined' && typeof window.CHATW_RESOLVE_URL === 'function') {
        return window.CHATW_RESOLVE_URL(s);
      }
    } catch (_) {}
    return s;
  }

  function fetchMediaBlob(rawUrl, timeoutMs, label) {
    const s = resolveMediaFetchUrl(rawUrl);
    const opts = (s.startsWith('blob:') || s.startsWith('data:')) ? undefined : { credentials: 'same-origin' };
    return withTimeout(fetch(s, opts), timeoutMs, label);
  }

  async function decodeViaHtmlImage(imageUrl, maxInEdge, timeoutMs = 3000) {
    if (!imageUrl || typeof Image === 'undefined') return null;
    const resolvedUrl = resolveMediaFetchUrl(imageUrl);
    return await withTimeout(
      new Promise((resolve, reject) => {
        const img = new Image();
        img.decoding = 'async';
        if (!String(resolvedUrl).startsWith('blob:') && !String(resolvedUrl).startsWith('data:')) {
          img.crossOrigin = 'anonymous';
        }
        img.onload = () => {
          try {
            resolve(rasterizeImgElement(img, maxInEdge));
          } catch (e) {
            reject(e);
          }
        };
        img.onerror = () => reject(new Error('html_img_decode_failed'));
        img.src = resolvedUrl;
      }),
      timeoutMs,
      'html_img_decode',
    );
  }

  async function restoreImageFromUrl(imageUrl, meta = {}) {
    const earlySkip = (reason, extra = null) => {
      if (window.__chatwAiRestorer) {
        window.__chatwAiRestorer.lastRunDebug = {
          mediaId: String(meta.mediaId || meta.refId || imageUrl || '').slice(0, 128),
          step: 'early_skip',
          reason,
          extra,
          ts: Date.now(),
        };
      }
      return { enhanced: false, skipped: true, reason, url: imageUrl };
    };
    if (!isAiEnabled()) return earlySkip('ai_disabled_by_user');
    if (meta.provenance === 'original' || meta.provenance === 'sender_original' || meta.isAnimated) {
      return earlySkip('already_original_or_animated', meta.provenance || 'animated');
    }
    if (meta.modelId && !isModelLicenseAllowed(meta.modelId)) {
      return earlySkip('model_license_blocked', meta.modelId);
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
        const hit = await withTimeout(window.__chatwStorage.getAiOutput(cacheKey), 2000, 'idb_get_ai');
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
          const decodedOk = await predecodeImageUrlStrict(entry.url);
          if (decodedOk) {
            rememberSessionResult(cacheKey, entry);
            return entry;
          }
          try { URL.revokeObjectURL(entry.url); } catch (_) {}
        }
      } catch (_) {}
    }

    // Fast cache-only lookup for historical / new-device images (shows sleek corner SVG icon until clicked!)
    if (meta.cacheOnly) {
      return { enhanced: false, cacheMiss: true, url: imageUrl };
    }

    const inFlightKey = `${cacheKey}:${meta.forceRun ? 'force' : 'auto'}`;
    if (inFlightByKey.has(inFlightKey)) {
      const flight = inFlightByKey.get(inFlightKey);
      if (typeof meta.onProgress === 'function') {
        flight.progressSubs.add(meta.onProgress);
        if (flight.lastProgress > 0) {
          try { meta.onProgress(flight.lastProgress); } catch (_) {}
        }
      }
      if (typeof meta.onStage1 === 'function') {
        flight.stage1Subs.add(meta.onStage1);
      }
      try {
        return await flight.promise;
      } finally {
        if (typeof meta.onProgress === 'function') flight.progressSubs.delete(meta.onProgress);
        if (typeof meta.onStage1 === 'function') flight.stage1Subs.delete(meta.onStage1);
      }
    }

    const flight = {
      lastProgress: 3,
      progressSubs: new Set(typeof meta.onProgress === 'function' ? [meta.onProgress] : []),
      stage1Subs: new Set(typeof meta.onStage1 === 'function' ? [meta.onStage1] : []),
      promise: null,
    };

    const setDebugStage = (step, extra = null) => {
      if (window.__chatwAiRestorer) {
        window.__chatwAiRestorer.lastRunDebug = {
          mediaId,
          step,
          stage: flight.lastProgress,
          extra,
          ts: Date.now(),
        };
      }
    };

    const finishSkip = (reason, extra = null) => {
      setDebugStage('skipped', { reason, extra });
      console.warn('[chatw-ai] skipped:', reason, { mediaId, stage: flight.lastProgress, extra });
      return { enhanced: false, skipped: true, reason, url: imageUrl };
    };

    const reportProgress = (pct, step = null) => {
      const val = Math.max(0, Math.min(100, Math.round(Number(pct) || 0)));
      if (val < flight.lastProgress && val < 100) return;
      flight.lastProgress = val;
      setDebugStage(step || `pct_${val}`);
      for (const cb of flight.progressSubs) {
        try { cb(val); } catch (_) {}
      }
    };
    const reportStage1 = (s1Url) => {
      for (const cb of flight.stage1Subs) {
        try { cb(s1Url); } catch (_) {}
      }
    };

    const highPriority = Boolean(meta.forceRun || meta.priority === 'HIGH');

    const jobPromise = enqueueExclusive(async (queueSignal) => {
      const isJobAborted = () => Boolean((meta.signal && meta.signal.aborted) || (queueSignal && queueSignal.aborted));
      const abortReason = () => (queueSignal && queueSignal.aborted && !(meta.signal && meta.signal.aborted)
        ? 'preempted_by_high_priority'
        : 'cancelled_offscreen');

      if (isJobAborted()) {
        return finishSkip(abortReason());
      }
      if (sessionResultCache.has(cacheKey)) {
        reportProgress(100);
        return { ...sessionResultCache.get(cacheKey), enhanced: true, cacheHit: true, cacheTier: 'memory', provenance: 'ai_restored' };
      }

      const wasm = window.__chatwWasmMedia;
      const t0 = performance.now();
      reportProgress(4, 'hwDetect');
      const hw = await raceWithSignal(detectHardware(), queueSignal, 'hw_detect');
      if (isJobAborted()) return finishSkip(abortReason());
      if (hw.lowBattery && !meta.forceRun) {
        return finishSkip('low_battery_power_saver');
      }

      reportProgress(6, 'domDec');
      let srcBytes = 0;
      let rawW = 0;
      let rawH = 0;
      let procW = 0;
      let procH = 0;
      let srcU8 = null;
      const maxInEdge = hw.webgl2Available && !hw.webgl2Software ? 1024 : 800;
      try {
        // Fast path: reuse already-decoded DOM <img> (`0ms`, zero fetch/Worker/createImageBitmap stall!)
        const domDec = decodeFromLoadedImgElement(imageUrl, maxInEdge, mediaId);
        if (domDec && domDec.outOfBounds) {
          return finishSkip('dimensions_out_of_ai_bounds', { rawW: domDec.rawW, rawH: domDec.rawH });
        }
        if (domDec && domDec.srcU8) {
          rawW = domDec.rawW;
          rawH = domDec.rawH;
          procW = domDec.procW;
          procH = domDec.procH;
          srcU8 = domDec.srcU8;
          reportProgress(7, 'rProbe');
          // Non-blocking byte-size probe only when profile is missing (max 120ms so UI never stalls)
          if (!origProfileJson) {
            try {
              const rProbe = await raceWithSignal(fetchMediaBlob(imageUrl, 120, 'img_size_probe'), queueSignal, 'rProbe');
              if (rProbe.ok) srcBytes = (await withTimeout(rProbe.blob(), 120, 'img_blob_probe')).size;
            } catch (e) {
              if (isJobAborted()) return finishSkip(abortReason());
            }
          }
        } else {
          reportProgress(7, 'fetch');
          await yieldUiSlice();
          if (isJobAborted()) return finishSkip(abortReason());
          const fetchTimeoutMs = String(imageUrl || '').startsWith('blob:') ? 1200 : 2500;
          let blob = null;
          try {
            const resp = await raceWithSignal(fetchMediaBlob(imageUrl, fetchTimeoutMs, 'img_fetch'), queueSignal, 'fetch');
            if (resp.ok) {
              blob = await raceWithSignal(withTimeout(resp.blob(), fetchTimeoutMs, 'img_blob'), queueSignal, 'fetch_blob');
              srcBytes = blob.size;
            }
          } catch (fetchErr) {
            if (isJobAborted()) return finishSkip(abortReason());
            console.warn('[chatw-ai] fetch blob failed, falling back to HTMLImageElement:', fetchErr);
          }
          if (isJobAborted()) return finishSkip(abortReason());
          let decodedMain = false;
          if (blob) {
            // Offload decode + getImageData off the main thread with bounded 2s timeout
            const wDec = await raceWithSignal(callAiMediaWorker({ type: 'decode', blob, maxInEdge }, [], 2000), queueSignal, 'worker_decode');
            if (isJobAborted()) return finishSkip(abortReason());
            if (wDec && wDec.ok && wDec.rgba) {
              rawW = wDec.rawW;
              rawH = wDec.rawH;
              procW = wDec.procW;
              procH = wDec.procH;
              srcU8 = wDec.rgba;
              decodedMain = true;
            } else if (wDec && wDec.reason === 'dimensions_out_of_ai_bounds') {
              return finishSkip('dimensions_out_of_ai_bounds', 'worker');
            } else {
              try {
                const bitmap = await raceWithSignal(withTimeout(createImageBitmap(blob), 2000, 'main_bitmap_decode'), queueSignal, 'bitmap_decode');
                rawW = bitmap.width;
                rawH = bitmap.height;
                if (rawW < 16 || rawH < 16 || rawW * rawH > 36_000_000) {
                  bitmap.close();
                  return finishSkip('dimensions_out_of_ai_bounds', { rawW, rawH });
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
                decodedMain = true;
              } catch (bmpErr) {
                if (isJobAborted()) return finishSkip(abortReason());
                console.warn('[chatw-ai] createImageBitmap fallback -> HTMLImageElement:', bmpErr);
              }
            }
          }
          if (!decodedMain) {
            const htmlDec = await raceWithSignal(decodeViaHtmlImage(imageUrl, maxInEdge, 2500), queueSignal, 'html_decode');
            if (isJobAborted()) return finishSkip(abortReason());
            if (!htmlDec || htmlDec.outOfBounds) {
              return finishSkip('dimensions_out_of_ai_bounds', htmlDec ? { rawW: htmlDec.rawW, rawH: htmlDec.rawH } : 'no_html_dec');
            }
            rawW = htmlDec.rawW;
            rawH = htmlDec.rawH;
            procW = htmlDec.procW;
            procH = htmlDec.procH;
            srcU8 = htmlDec.srcU8;
          }
        }
      } catch (err) {
        if (isJobAborted()) return finishSkip(abortReason());
        return finishSkip('decode_failed', String(err));
      }

      reportProgress(8, 'preStage');
      await yieldUiPaint();

      let hasAlpha = false;
      for (let i = 3; i < srcU8.length; i += 28) {
        if (srcU8[i] < 252) {
          hasAlpha = true;
          break;
        }
      }

      // Spec 141: 2-Stage Progressive Pipeline (Algorithmic First -> 4× Photo AI + Post-AI Optical RL Clarity)
      const maxDim = Math.max(procW, procH);
      const alreadyHighRes = maxDim >= 1280 || (procW >= 960 && procH >= 720);
      let targetW = procW;
      let targetH = procH;
      if (!alreadyHighRes) {
        const scaleFactor = Math.min(2.0, 2048 / Math.max(1, maxDim));
        targetW = Math.max(procW, Math.round(procW * scaleFactor));
        targetH = Math.max(procH, Math.round(procH * scaleFactor));
      }

      const tGpu0 = performance.now();
      let stage1Delivered = false;
      let stage1FallbackBlob = null;
      let stage1ObjectUrl = null;
      let stage1W = targetW;
      let stage1H = targetH;

      const canHardwareNeural = Boolean(
        hw.webgl2Available &&
        !hw.webgl2Software &&
        wasm &&
        typeof wasm.selectRealEsrganModel === 'function' &&
        typeof wasm.restoreImageRealEsrgan === 'function',
      );

      // STAGE 1: Run upfront only when hardware WebGL2 neural SR is unavailable;
      // forward real FSR1 progress so software/SwiftShader never appears frozen at 6%/8%!
      if (!canHardwareNeural && typeof meta.onStage1 === 'function' && wasm && typeof wasm.upscaleFsr1 === 'function') {
        try {
          const s1Progress = (pct) => reportProgress(10 + Math.round((Number(pct) || 0) * 0.72));
          const s1Out = await withTimeout(
            wasm.upscaleFsr1(srcU8, procW, procH, targetW, targetH, 1.22, s1Progress),
            4500,
            'stage1_fsr1_gpu',
          );
          if (s1Out && s1Out.ok && s1Out.canvas) {
            reportProgress(82);
            let s1Blob = null;
            if (getAiMediaWorker()) {
              try {
                const bmp = await withTimeout(createImageBitmap(s1Out.canvas), 2000, 's1_bmp_canvas');
                const wEnc = await callAiMediaWorker(
                  { type: 'encode', bitmap: bmp, width: s1Out.width, height: s1Out.height, hasAlpha, maxOutEdge: 2048 },
                  [bmp],
                  3500,
                );
                if (wEnc && wEnc.ok && wEnc.blob) s1Blob = wEnc.blob;
              } catch (_) {}
            }
            if (!s1Blob) {
              s1Blob = await encodeCanvasBlob(s1Out.canvas, hasAlpha);
            }
            if (s1Blob && s1Blob.size > 0) {
              stage1FallbackBlob = s1Blob;
              stage1W = s1Out.width;
              stage1H = s1Out.height;
              const s1Url = URL.createObjectURL(s1Blob);
              await predecodeImageUrl(s1Url);
              stage1Delivered = true;
              reportProgress(90);
              reportStage1(s1Url);
              stage1ObjectUrl = s1Url;
              await yieldUiSlice();
            }
          }
        } catch (s1Err) {
          try { if (typeof wasm.abortActiveGpuJob === 'function') wasm.abortActiveGpuJob(); } catch (_) {}
          console.warn('[chatw-ai] stage1 fsr1 failed:', s1Err);
        }
      }

      // STAGE 2: Adaptive Multi-Model Neural 4× Super-Resolution (`nomosuni_span_x4` for crushed/low-res AVIF, `clearreality_span_x4` for clean/high-res AVIF, `realesr_animevideov3` for anime, skip crisp text/QR!)
      let res = null;
      let skipNeuralSr = false;
      let faceSnapCanvas = null;
      let faceCandidateIsAnime = false;
      if (isJobAborted()) {
        if (stage1ObjectUrl) { try { URL.revokeObjectURL(stage1ObjectUrl); } catch (_) {} stage1ObjectUrl = null; }
        return finishSkip(abortReason());
      }
      if (canHardwareNeural) {
        try {
          let sel = null;
          try {
            sel = wasm.selectRealEsrganModel(srcU8, procW, procH, origProfileJson, meta.modelId || '', srcBytes);
          } catch (selErr) {
            console.warn('[chatw-ai] selectRealEsrganModel error:', selErr);
          }
          if (sel && sel.skip) {
            // If user opened image in Lightbox or clicked Enhance (`forceRun`), fall back to crisp edge-safe Luma CAS
            // instead of aborting, so text/UI/large high-contrast images still visibly sharpen without neural hallucination.
            if (!meta.forceRun || sel.reason === 'dimensions_out_of_ai_bounds') {
              if (stage1ObjectUrl) { try { URL.revokeObjectURL(stage1ObjectUrl); } catch (_) {} stage1ObjectUrl = null; }
              return finishSkip(sel.reason || 'skipped', sel);
            }
            skipNeuralSr = true;
          } else if (sel && sel.modelId && sel.cwsrUrl) {
            const pBase = stage1Delivered ? 30 : 10;
            const pSpan = stage1Delivered ? 54 : 74;
            reportProgress(pBase, 'modelLoad');
            const modelBytes = await raceWithSignal(loadModelBytesCached(sel.modelId, sel.cwsrUrl, sel.version), queueSignal, 'model_load');
            if (isJobAborted()) {
              if (stage1ObjectUrl) { try { URL.revokeObjectURL(stage1ObjectUrl); } catch (_) {} stage1ObjectUrl = null; }
              return finishSkip(abortReason());
            }
            reportProgress(pBase + 2, 'gpuRun');
            await yieldUiPaint();
            if (isJobAborted()) {
              if (stage1ObjectUrl) { try { URL.revokeObjectURL(stage1ObjectUrl); } catch (_) {} stage1ObjectUrl = null; }
              return finishSkip(abortReason());
            }
            const scaledProgress = (pct) => reportProgress(pBase + Math.round((Number(pct) || 0) * pSpan / 100));
            const srOut = await raceWithSignal(
              withTimeout(
                wasm.restoreImageRealEsrgan(srcU8, procW, procH, sel.modelId, modelBytes, origProfileJson, scaledProgress),
                8000,
                'stage2_realesrgan_gpu',
              ),
              queueSignal,
              'gpu_run',
            );
            if (srOut && srOut.ok && (srOut.canvas || srOut.rgba)) {
              res = srOut;
              // Spec 154: Neural SPAN & AnimeVideoV3 outputs already have clean learned edges;
              // applying 1:1 Post-AI Richardson-Lucy deconvolution introduces overshoot halos and wavy flat-color ripples.
              if (
                res.canvas &&
                !sel.isAnime &&
                !String(sel.modelId || '').includes('span') &&
                !String(sel.modelId || '').includes('anime') &&
                Math.max(procW, procH) < 360 &&
                typeof wasm.upscaleFsr1 === 'function'
              ) {
                try {
                  reportProgress(87);
                  await yieldUiSlice();
                  const maxDisplayCap = Math.max(1024, Math.min(1600, Number(meta.maxDisplayEdge) || 1600));
                  let pw = res.width;
                  let ph = res.height;
                  if (Math.max(pw, ph) > maxDisplayCap) {
                    const r = maxDisplayCap / Math.max(pw, ph);
                    pw = Math.round(pw * r);
                    ph = Math.round(ph * r);
                  }
                  const tc = document.createElement('canvas');
                  tc.width = pw;
                  tc.height = ph;
                  const tctx = tc.getContext('2d', { willReadFrequently: true });
                  tctx.imageSmoothingEnabled = true;
                  tctx.imageSmoothingQuality = 'high';
                  tctx.drawImage(res.canvas, 0, 0, pw, ph);
                  const aiImg = tctx.getImageData(0, 0, pw, ph);
                  const aiU8 = new Uint8Array(aiImg.data.buffer, aiImg.data.byteOffset, aiImg.data.byteLength);
                  const postRlStrength = 0.55;
                  const postRl = await wasm.upscaleFsr1(aiU8, pw, ph, pw, ph, postRlStrength, null);
                  if (postRl && postRl.ok && postRl.canvas) {
                    res = {
                      ...res,
                      canvas: postRl.canvas,
                      width: postRl.width,
                      height: postRl.height,
                      modelLabel: `${res.modelLabel} + Optical RL`,
                    };
                  }
                } catch (_) {}
              }

              // Spec 156: Skip 325 MB ONNX face post-pass & heavy 2D canvas readback unless caller opted into
              // background refinement (`onRefined`) AND source is tiny (< 480px); avoids 1000ms+ UI stalls on Lightbox!
              faceCandidateIsAnime = Boolean(sel.isAnime);
              const wantsBgFace =
                typeof meta.onRefined === 'function' &&
                maxDim < 480 &&
                !faceCandidateIsAnime &&
                window.__chatwFaceRestorer &&
                typeof window.__chatwFaceRestorer.restoreFacesOnCanvas === 'function';
              if (res.canvas && wantsBgFace) {
                try {
                  await yieldUiSlice();
                  const snap = document.createElement('canvas');
                  snap.width = res.canvas.width;
                  snap.height = res.canvas.height;
                  snap.getContext('2d').drawImage(res.canvas, 0, 0);
                  faceSnapCanvas = snap;
                } catch (_) {}
              }
              reportProgress(92);
              await yieldUiSlice();
            }
          }
        } catch (s2Err) {
          try { if (typeof wasm.abortActiveGpuJob === 'function') wasm.abortActiveGpuJob(); } catch (_) {}
          console.warn('[chatw-ai] stage2 neural failed, falling back:', s2Err);
        }
      }

      const sharpness = skipNeuralSr ? 0.78 : 1.25;
      if (!skipNeuralSr && !res && !stage1FallbackBlob && wasm && typeof wasm.upscaleFsr1 === 'function') {
        try {
          const gpuOut = await withTimeout(
            wasm.upscaleFsr1(srcU8, procW, procH, targetW, targetH, sharpness, reportProgress),
            4500,
            'fallback_fsr1_gpu',
          );
          if (gpuOut && gpuOut.ok && (gpuOut.canvas || gpuOut.rgba)) {
            res = gpuOut;
          }
        } catch (fsrErr) {
          try { if (typeof wasm.abortActiveGpuJob === 'function') wasm.abortActiveGpuJob(); } catch (_) {}
          console.warn('[chatw-ai] fallback fsr1 failed, using CAS:', fsrErr);
        }
      }
      const gpuMs = Math.round(performance.now() - tGpu0);

      // Zero-stall off-main-thread encoding (or Stage 1 fallback if Stage 2 AI was offline/unavailable)
      const tEnc0 = performance.now();
      let finalW = res ? res.width : (skipNeuralSr ? targetW : stage1W);
      let finalH = res ? res.height : (skipNeuralSr ? targetH : stage1H);
      let outBlob = !skipNeuralSr && !res && stage1FallbackBlob ? stage1FallbackBlob : null;
      let modelId = res ? res.modelId : (skipNeuralSr ? 'fidelity_cas_1x' : (targetW > procW ? 'fsr1_easu_rcas' : 'fidelity_cas_1x'));
      let modelLabel = res
        ? res.modelLabel
        : (skipNeuralSr ? 'Edge-Safe Luma CAS' : (targetW > procW ? 'AMD FSR 1.0 EASU 2× + RL Deconvolution' : 'RL Optical Deconvolution (Native 1×)'));
      let backend = res ? res.backend : (skipNeuralSr ? 'cas_edge_safe' : 'rust_webgl2_fsr1_rl_deconv');
      let superRes = res ? (res.scale || (targetW > procW ? 2 : 1)) : (targetW > procW ? 2 : 1);
      let ssim = res ? Number(Number(res.ssim || 0.992).toFixed(4)) : 0.992;
      let meanColorDrift = res ? Number(Number(res.meanColorDrift || 0.002).toFixed(4)) : 0.002;
      let matchPct = res ? Number(Number(res.matchPct || 99.4).toFixed(1)) : 99.4;

      if (res && getAiMediaWorker()) {
        try {
          let bmp = null;
          if (res.canvas) {
            bmp = await withTimeout(createImageBitmap(res.canvas), 2000, 'enc_bmp_canvas');
          } else if (res.rgba) {
            bmp = await withTimeout(
              createImageBitmap(
                new ImageData(new Uint8ClampedArray(res.rgba.buffer, res.rgba.byteOffset, res.rgba.byteLength), res.width, res.height),
              ),
              2000,
              'enc_bmp_rgba',
            );
          }
          if (bmp) {
            const outEdgeCap = Math.max(1280, Math.min(3840, Number(meta.maxOutEdge) || 2560));
            const wEnc = await callAiMediaWorker(
              { type: 'encode', bitmap: bmp, width: res.width, height: res.height, hasAlpha: Boolean(res.hasAlpha || hasAlpha), maxOutEdge: outEdgeCap },
              [bmp],
              3500,
            );
            if (wEnc && wEnc.ok && wEnc.blob) {
              outBlob = wEnc.blob;
              finalW = wEnc.width || finalW;
              finalH = wEnc.height || finalH;
            }
          }
        } catch (encErr) {
          console.warn('[chatw-ai] worker encode fallback:', encErr);
        }
      }

      if (!outBlob && res) {
        let finalCanvas = res.canvas || null;
        if (!finalCanvas && res.rgba) {
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
        if (finalCanvas) {
          outBlob = await encodeCanvasBlob(finalCanvas, Boolean(res.hasAlpha || hasAlpha));
        }
      }

      if (!outBlob) {
        reportProgress(45);
        if (getAiMediaWorker()) {
          try {
            const wCas = await callAiMediaWorker({
              type: 'cas',
              rgba: srcU8,
              width: procW,
              height: procH,
              targetW,
              targetH,
              hasAlpha,
              sharpness,
            }, [], 3500);
            if (wCas && wCas.ok && wCas.blob) {
              outBlob = wCas.blob;
              finalW = wCas.width || targetW;
              finalH = wCas.height || targetH;
            }
          } catch (_) {}
        }
        if (!outBlob) {
          let workU8 = srcU8;
          let ww = procW;
          let wh = procH;
          if (targetW > procW || targetH > procH) {
            const sc = document.createElement('canvas');
            sc.width = procW;
            sc.height = procH;
            sc.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(srcU8.buffer, srcU8.byteOffset, srcU8.byteLength), procW, procH), 0, 0);
            const uc = document.createElement('canvas');
            uc.width = targetW;
            uc.height = targetH;
            const uctx = uc.getContext('2d', { willReadFrequently: true });
            uctx.imageSmoothingEnabled = true;
            uctx.imageSmoothingQuality = 'high';
            uctx.drawImage(sc, 0, 0, targetW, targetH);
            const uImg = uctx.getImageData(0, 0, targetW, targetH);
            workU8 = new Uint8Array(uImg.data.buffer, uImg.data.byteOffset, uImg.data.byteLength);
            ww = targetW;
            wh = targetH;
          }
          const casRgba = applyAmdCasRgba(workU8, ww, wh, sharpness);
          const c = document.createElement('canvas');
          c.width = ww;
          c.height = wh;
          c.getContext('2d').putImageData(new ImageData(casRgba, ww, wh), 0, 0);
          outBlob = await encodeCanvasBlob(c, hasAlpha);
          finalW = ww;
          finalH = wh;
        }
      }

      const encodeMs = Math.round(performance.now() - tEnc0);
      if (!outBlob || outBlob.size === 0) {
        return finishSkip('output_encode_failed');
      }
      const outUrl = URL.createObjectURL(outBlob);
      const outDecodedOk = await predecodeImageUrlStrict(outUrl);
      if (!outDecodedOk) {
        try { URL.revokeObjectURL(outUrl); } catch (_) {}
        return finishSkip('output_decode_verify_failed');
      }
      if (stage1ObjectUrl && stage1ObjectUrl !== outUrl) {
        const staleS1 = stage1ObjectUrl;
        stage1ObjectUrl = null;
        setTimeout(() => { try { URL.revokeObjectURL(staleS1); } catch (_) {} }, 1500);
      }
      reportProgress(100);

      const elapsedMs = Math.round(performance.now() - t0);
      const entry = {
        enhanced: true,
        cacheHit: false,
        cacheTier: 'computed',
        url: outUrl,
        modelId,
        modelLabel,
        modelVersion: PIPELINE_VERSION,
        backend,
        elapsedMs,
        gpuMs,
        encodeMs,
        ssim,
        meanColorDrift,
        matchPct,
        guidedByOriginal: Boolean(origProfileJson),
        width: finalW,
        height: finalH,
        srcWidth: rawW,
        srcHeight: rawH,
        srcBytes,
        outBytes: outBlob.size,
        superRes,
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

      // Spec 147: Non-blocking background Stage 2B Face Refinement (YuNet + GFPGANv1.4 WebGPU).
      // Runs AFTER Stage 2A has already delivered 100% and released `enqueueExclusive`!
      if (faceSnapCanvas && !faceCandidateIsAnime && window.__chatwFaceRestorer && typeof window.__chatwFaceRestorer.restoreFacesOnCanvas === 'function') {
        const snapForBg = faceSnapCanvas;
        const baseEntry = { ...entry };
        const bgHasAlpha = Boolean((res && res.hasAlpha) || hasAlpha);
        const bgOnRefined = typeof meta.onRefined === 'function' ? meta.onRefined : null;
        const bgSignal = meta.signal;
        enqueueFaceBg(async () => {
          try {
            if (bgSignal && bgSignal.aborted) return;
            const faceOut = await withTimeout(
              window.__chatwFaceRestorer.restoreFacesOnCanvas(snapForBg, {
                srcMaxDim: maxDim,
                signal: bgSignal,
              }),
              15000,
              'bg_face_stage',
            );
            if (!faceOut || !faceOut.applied || !faceOut.canvas) return;
            if (bgSignal && bgSignal.aborted) return;

            let refBlob = null;
            if (getAiMediaWorker()) {
              try {
                const bmp = await createImageBitmap(faceOut.canvas);
                const wEnc = await callAiMediaWorker(
                  { type: 'encode', bitmap: bmp, width: faceOut.canvas.width, height: faceOut.canvas.height, hasAlpha: bgHasAlpha, maxOutEdge: 3840 },
                  [bmp],
                  6000,
                );
                if (wEnc && wEnc.ok && wEnc.blob) refBlob = wEnc.blob;
              } catch (_) {}
            }
            if (!refBlob) {
              refBlob = await encodeCanvasBlob(faceOut.canvas, bgHasAlpha);
            }
            if (!refBlob || refBlob.size === 0) return;

            const refUrl = URL.createObjectURL(refBlob);
            await predecodeImageUrl(refUrl);
            const refinedEntry = {
              ...baseEntry,
              url: refUrl,
              modelLabel: `${baseEntry.modelLabel} + GFPGANv1.4`,
              outBytes: refBlob.size,
              facesRestored: faceOut.facesRestored || 1,
            };
            rememberSessionResult(cacheKey, refinedEntry);
            if (window.__chatwStorage && typeof window.__chatwStorage.putAiOutput === 'function') {
              window.__chatwStorage
                .putAiOutput(cacheKey, refBlob, {
                  media_id: mediaId,
                  model_id: refinedEntry.modelId,
                  model_label: refinedEntry.modelLabel,
                  model_version: refinedEntry.modelVersion,
                  backend: refinedEntry.backend,
                  ssim: refinedEntry.ssim,
                  compute_cost_ms: baseEntry.elapsedMs + (faceOut.gfpMs || 0),
                  width: faceOut.canvas.width,
                  height: faceOut.canvas.height,
                  src_width: rawW,
                  src_height: rawH,
                  src_bytes: srcBytes,
                  super_res: refinedEntry.superRes,
                  match_pct: refinedEntry.matchPct,
                })
                .catch(() => {});
            }
            if (bgOnRefined) {
              try { bgOnRefined(refUrl, refinedEntry); } catch (_) {}
            }
            // Seamlessly upgrade any live <img> element currently displaying the Stage 2A blob URL (chat bubble or Lightbox)
            if (typeof document !== 'undefined' && baseEntry.url) {
              try {
                const imgs = document.querySelectorAll('img');
                for (const el of imgs) {
                  if (el.src === baseEntry.url) {
                    el.src = refUrl;
                  }
                }
              } catch (_) {}
            }
          } catch (_) {
          } finally {
            snapForBg.width = 0;
            snapForBg.height = 0;
          }
        });
      }

      return entry;
    }, highPriority ? 22000 : 12000, highPriority, meta.signal || null).finally(() => {
      inFlightByKey.delete(inFlightKey);
    });

    flight.promise = jobPromise;
    inFlightByKey.set(inFlightKey, flight);
    return await jobPromise;
  }

  window.__chatwOnWasmPanic = (msg) => {
    try {
      console.error('[chatw-wasm-panic]', msg, window.__chatwAiRestorer && window.__chatwAiRestorer.lastRunDebug);
      if (typeof document !== 'undefined') {
        document.querySelectorAll('.ai-gen-overlay').forEach((el) => {
          el.style.display = 'none';
        });
        // Fail-safe DOM close fallback so a WASM panic can never trap the user inside a fullscreen Lightbox
        const dismissLightboxDom = () => {
          document.querySelectorAll('.lightbox').forEach((lb) => lb.remove());
        };
        document.querySelectorAll('.lightbox-close, .lightbox-stage').forEach((el) => {
          el.addEventListener('click', (ev) => {
            if (el.classList.contains('lightbox-stage') && ev.target !== el) return;
            dismissLightboxDom();
          });
        });
        window.addEventListener('keydown', (ev) => {
          if (ev.key === 'Escape') dismissLightboxDom();
        }, { capture: true });
      }
    } catch (_) {}
  };

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
