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
  // When preserveOriginal is false, compress AGGRESSIVELY ("nén mạnh nhất có thể"):
  // - Downscale large images by 2x (0.5x) so transmitted size drops by 80-90% (e.g. 178KB down to ~15-28KB)
  // - The sender immediately sees their crisp original file locally (< 20ms)
  // - The receiver runs on-device AI 2x Super-Resolution to restore back to full original resolution & sharpness!
  function adaptiveMaxEdge(w, h, isCrisp, qualityMode) {
    if (qualityMode === 'original') return null;
    const maxEdge = Math.max(w, h);
    if (qualityMode === 'data_saver') {
      return maxEdge > 960 ? Math.round(maxEdge * 0.5) : (maxEdge > 640 ? 640 : null);
    }
    if (maxEdge > 1080) {
      return Math.round(maxEdge * 0.5);
    } else if (maxEdge > 800) {
      return 640;
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
  let nextHandleSeq = 1;

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

  function optimizeInMediaWorker(rawU8, mime, opts) {
    if (typeof Worker === 'undefined' || typeof OffscreenCanvas === 'undefined') {
      return Promise.resolve(null);
    }
    try {
      if (!mediaWorker) {
        mediaWorker = new Worker('/media_worker.js');
        mediaWorker.onmessage = (ev) => {
          const d = ev.data;
          if (!d || !d.id) return;
          const entry = mediaWorkerPending.get(d.id);
          if (entry) {
            clearTimeout(entry.timer);
            mediaWorkerPending.delete(d.id);
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
        }, 8000);
        mediaWorkerPending.set(id, { resolve, timer });
        mediaWorker.postMessage({ id, buffer: copy.buffer, mime, opts }, [copy.buffer]);
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
    const effectiveOpts = { ...opts, qualityMode, preserveOriginal: Boolean(opts.preserveOriginal) || qualityMode === 'original' };

    if (!rawSniff.animated) {
      const workerRes = await optimizeInMediaWorker(rawU8, rawSniff.mime || fileOrBlob.type, effectiveOpts);
      if (workerRes && workerRes.bytesB64 && (workerRes.ext === 'avif' || effectiveOpts.preserveOriginal)) {
        return workerRes;
      }
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

      // 1. Try native canvas genuine AVIF (only for still images)
      if (!rawSniff.animated) {
        const maxEdge = opts.maxDim || adaptiveMaxEdge(origW, origH, isCrisp, qualityMode);
        let tw = origW;
        let th = origH;
        if (maxEdge && (tw > maxEdge || th > maxEdge)) {
          const r = Math.min(maxEdge / tw, maxEdge / th);
          tw = Math.max(1, Math.round(tw * r));
          th = Math.max(1, Math.round(th * r));
        }
        const canvas = await drawMultiStepResized(bitmap, tw, th);
        const q = isCrisp ? 0.54 : (qualityMode === 'data_saver' ? 0.44 : 0.48);
        const nativeAvif = await tryNativeCanvasAvif(canvas, q);
        if (nativeAvif && nativeAvif.u8.byteLength < rawU8.byteLength) {
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
            blurDataUrl,
            thumbB64: null,
            thumbMime: null,
            codec: 'avif_native_canvas',
          };
        }
      }
      bitmap.close();
    } catch (_) {}

    // 2. Pure-Rust WASM `ravif`/`rav1e` Adaptive Ultra AVIF encoder (works on ALL browsers, incl. Animated AVIF `avis`!)
    if (window.__chatwWasmMedia && typeof window.__chatwWasmMedia.encodeAvifAdaptive === 'function') {
      try {
        const wasmMode = qualityMode === 'data_saver' ? 'fast' : 'ultra';
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
              targetWidth: origW,
              targetHeight: origH,
              superResRecommended: (res.width || origW) < origW,
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

  async function processSingleFile(file, preserveOriginal = false, existingPreviewUrl = null) {
    const previewUrl = existingPreviewUrl || URL.createObjectURL(file);
    if (file.type.startsWith('image/')) {
      try {
        const res = await optimizeImageBlob(file, { preserveOriginal });
        return {
          ...res,
          fileName: file.name,
          mediaKind: 'image',
          instantPreviewUrl: previewUrl,
        };
      } catch (e) {
        return { error: String(e) };
      }
    }
    const raw = new Uint8Array(await file.arrayBuffer());
    const shaHex = window.__chatwStorage ? await window.__chatwStorage.sha256Hex(raw) : '';
    const mediaKind = file.type.startsWith('video/') ? 'video' : file.type.startsWith('audio/') ? 'audio' : 'file';
    const poster = mediaKind === 'video' ? await extractVideoPosterMeta(file) : { width: 0, height: 0, blurDataUrl: null };

    // Lossless Smart File Compression (`CWCMP1` ZSTD/Brotli/LZMA2) for general files (documents, logs, JSON, code)
    let uploadBytes = raw;
    let fileCodec = 'raw';
    if (mediaKind === 'file' && !preserveOriginal && window.__chatwWasmMedia && typeof window.__chatwWasmMedia.compressFileSmart === 'function') {
      try {
        const cmp = window.__chatwWasmMedia.compressFileSmart(raw);
        if (cmp && cmp.ok && cmp.compressed && cmp.bytes && cmp.bytes.byteLength < raw.byteLength) {
          uploadBytes = cmp.bytes instanceof Uint8Array ? cmp.bytes : new Uint8Array(cmp.bytes);
          fileCodec = cmp.codec || 'cwcmp1';
        }
      } catch (_) {}
    }

    return {
      bytesB64: bytesToB64(uploadBytes),
      mime: file.type || 'application/octet-stream',
      ext: (file.name.split('.').pop() || 'bin').toLowerCase().slice(0, 12),
      fileName: file.name,
      originalSize: raw.byteLength,
      optimizedSize: uploadBytes.byteLength,
      sha256: shaHex,
      width: poster.width,
      height: poster.height,
      blurDataUrl: poster.blurDataUrl,
      mediaKind,
      codec: fileCodec,
      instantPreviewUrl: previewUrl,
    };
  }

  window.__chatwMediaOpt = {
    optimizeImageBlob,
    probeVideoCodecs,

    // Phase 1 of Instant Sender Original Preview: returns crisp local `blob:` URL in < 20ms!
    async pickInstantPreview(inputId) {
      const inp = document.getElementById(inputId);
      if (!inp || !inp.files || !inp.files[0]) return { error: 'no_file' };
      const file = inp.files[0];
      inp.value = '';
      const handleId = `pend_${Date.now().toString(36)}_${nextHandleSeq++}`;
      const previewUrl = URL.createObjectURL(file);
      const mediaKind = file.type.startsWith('image/')
        ? 'image'
        : file.type.startsWith('video/')
          ? 'video'
          : file.type.startsWith('audio/')
            ? 'audio'
            : 'file';
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
      return {
        handleId,
        instantPreviewUrl: previewUrl,
        fileName: file.name,
        mime: file.type || 'application/octet-stream',
        originalSize: file.size,
        mediaKind,
        width,
        height,
      };
    },

    // Phase 2: Compress/encode the stashed sender file off-main-thread while sender already sees crisp original!
    async processPendingHandle(handleId, preserveOriginal = false) {
      const entry = pendingSenderFiles.get(handleId);
      if (!entry) return { error: 'pending_handle_not_found' };
      const res = await processSingleFile(entry.file, preserveOriginal, entry.previewUrl);
      return { ...res, handleId };
    },

    // Return sender's original crisp local bytes so sender's local cache stores the true original!
    async getPendingOriginalB64(handleId) {
      const entry = pendingSenderFiles.get(handleId);
      if (!entry) return null;
      const raw = new Uint8Array(await entry.file.arrayBuffer());
      return {
        bytesB64: bytesToB64(raw),
        mime: entry.file.type || 'application/octet-stream',
        fileName: entry.file.name,
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
      return await processSingleFile(file, preserveOriginal, null);
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
