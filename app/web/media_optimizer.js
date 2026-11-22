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

  async function encodeCanvasBestFormat(canvas, preferredMimeList, quality) {
    for (const mime of preferredMimeList) {
      try {
        const blob = await canvasToBlobPromise(canvas, mime, quality);
        if (blob && blob.size > 0 && blob.type === mime) {
          const u8 = new Uint8Array(await blob.arrayBuffer());
          const sniffed = await sniffBytesFormat(u8);
          if (sniffed.mime === mime) {
            return { u8, sniffed };
          }
        }
      } catch (_) {}
    }
    const fallback = await canvasToBlobPromise(canvas, 'image/png', 1.0);
    const u8 = new Uint8Array(await fallback.arrayBuffer());
    return { u8, sniffed: await sniffBytesFormat(u8) };
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

  // Adaptive compression preserving readable text & never inflating small files.
  async function optimizeImageBlob(fileOrBlob, opts = {}) {
    const rawU8 = new Uint8Array(await fileOrBlob.arrayBuffer());
    const rawSniff = await sniffBytesFormat(rawU8);
    if (rawSniff.animated || opts.preserveOriginal) {
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
      };
    }

    const bitmap = await createImageBitmap(new Blob([rawU8], { type: rawSniff.mime }));
    const maxDim = opts.maxDim || 1920;
    let tw = bitmap.width;
    let th = bitmap.height;
    if (tw > maxDim || th > maxDim) {
      const r = Math.min(maxDim / tw, maxDim / th);
      tw = Math.max(1, Math.round(tw * r));
      th = Math.max(1, Math.round(th * r));
    }

    const canvas = await drawMultiStepResized(bitmap, tw, th);
    const { isCrisp } = detectCrispTextOrAlpha(canvas);
    const targetBytes = opts.targetMaxBytes || (isCrisp ? 900 * 1024 : 520 * 1024);

    let best;
    if (isCrisp) {
      // Preserve UI/document sharpness: try high-Q AVIF or WebP or PNG
      const avifHq = await encodeCanvasBestFormat(canvas, ['image/avif', 'image/webp', 'image/png'], 0.88);
      const pngRes = await encodeCanvasBestFormat(canvas, ['image/png'], 1.0);
      best = avifHq.u8.byteLength <= pngRes.u8.byteLength ? avifHq : pngRes;
    } else {
      // Binary search adaptive quality in AVIF -> WebP -> JPEG
      let lo = 0.45, hi = 0.88, q = 0.76;
      best = await encodeCanvasBestFormat(canvas, ['image/avif', 'image/webp', 'image/jpeg'], q);
      for (let i = 0; i < 3; i++) {
        if (best.u8.byteLength <= targetBytes && q >= 0.72) break;
        if (best.u8.byteLength > targetBytes) {
          hi = q;
          q = (lo + q) / 2;
        } else {
          lo = q;
          q = (q + hi) / 2;
        }
        const trial = await encodeCanvasBestFormat(canvas, ['image/avif', 'image/webp', 'image/jpeg'], q);
        if (trial.u8.byteLength < best.u8.byteLength || (trial.u8.byteLength <= targetBytes && q > 0.65)) {
          best = trial;
        }
      }
    }

    // Generate lightweight <= 320px thumbnail AND tiny 16x16 progressive blur-up preview (~250B)
    let thumbB64 = null;
    let thumbMime = null;
    let blurDataUrl = null;
    try {
      blurDataUrl = await makeMicroBlurDataUrl(bitmap, tw, th);
      const tScale = Math.min(320 / tw, 320 / th, 1);
      const thumbCanvas = await drawMultiStepResized(bitmap, Math.max(1, Math.round(tw * tScale)), Math.max(1, Math.round(th * tScale)));
      const thRes = await encodeCanvasBestFormat(thumbCanvas, ['image/webp', 'image/jpeg'], 0.68);
      thumbB64 = bytesToB64(thRes.u8);
      thumbMime = thRes.sniffed.mime;
    } catch (_) {}

    const origW = bitmap.width;
    const origH = bitmap.height;
    bitmap.close();

    const useOrig = best.u8.byteLength >= rawU8.byteLength && origW <= maxDim && origH <= maxDim;
    const outU8 = useOrig ? rawU8 : best.u8;
    const outSniff = useOrig ? rawSniff : best.sniffed;

    return {
      bytesB64: bytesToB64(outU8),
      mime: outSniff.mime,
      ext: outSniff.ext,
      originalSize: rawU8.byteLength,
      optimizedSize: outU8.byteLength,
      isCrispText: isCrisp,
      width: tw,
      height: th,
      blurDataUrl,
      thumbB64,
      thumbMime,
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

  window.__chatwMediaOpt = {
    optimizeImageBlob,
    async optimizeFromInput(inputId, preserveOriginal = false) {
      const inp = document.getElementById(inputId);
      if (!inp || !inp.files || !inp.files[0]) return { error: 'no_file' };
      const file = inp.files[0];
      inp.value = '';
      if (file.type.startsWith('image/')) {
        try {
          const res = await optimizeImageBlob(file, { preserveOriginal });
          return { ...res, fileName: file.name, mediaKind: 'image' };
        } catch (e) {
          return { error: String(e) };
        }
      }
      const raw = new Uint8Array(await file.arrayBuffer());
      const shaHex = window.__chatwStorage ? await window.__chatwStorage.sha256Hex(raw) : '';
      const mediaKind = file.type.startsWith('video/') ? 'video' : file.type.startsWith('audio/') ? 'audio' : 'file';
      const poster = mediaKind === 'video' ? await extractVideoPosterMeta(file) : { width: 0, height: 0, blurDataUrl: null };
      return {
        bytesB64: bytesToB64(raw),
        mime: file.type || 'application/octet-stream',
        ext: (file.name.split('.').pop() || 'bin').toLowerCase().slice(0, 12),
        fileName: file.name,
        originalSize: raw.byteLength,
        optimizedSize: raw.byteLength,
        sha256: shaHex,
        width: poster.width,
        height: poster.height,
        blurDataUrl: poster.blurDataUrl,
        mediaKind,
      };
    },
    async startVoiceRecording() {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: true } });
      const mime = MediaRecorder.isTypeSupported('audio/webm;codecs=opus') ? 'audio/webm;codecs=opus' :
        MediaRecorder.isTypeSupported('audio/ogg;codecs=opus') ? 'audio/ogg;codecs=opus' : '';
      activeVoiceChunks = [];
      activeVoiceStartedAt = Date.now();
      activeVoiceRecorder = mime ? new MediaRecorder(stream, { mimeType: mime, audioBitsPerSecond: 32000 }) : new MediaRecorder(stream);
      activeVoiceRecorder.ondataavailable = (e) => { if (e.data && e.data.size > 0) activeVoiceChunks.push(e.data); };
      activeVoiceRecorder.start(200);
      return { ok: true, mime: activeVoiceRecorder.mimeType };
    },
    async stopVoiceRecording() {
      if (!activeVoiceRecorder) return { error: 'not_recording' };
      const rec = activeVoiceRecorder;
      activeVoiceRecorder = null;
      await new Promise((res) => { rec.onstop = res; rec.stop(); });
      rec.stream.getTracks().forEach((t) => t.stop());
      const blob = new Blob(activeVoiceChunks, { type: rec.mimeType || 'audio/webm' });
      const u8 = new Uint8Array(await blob.arrayBuffer());
      return {
        bytesB64: bytesToB64(u8),
        mime: blob.type || 'audio/webm',
        durationMs: Date.now() - activeVoiceStartedAt,
        byteSize: u8.byteLength,
      };
    },
  };

  window.__chatwPeerTransfer = {
    async initiateOffer(requestId) {
      const sendSig = (rid, signalType, data) =>
        fetch(`/api/recovery/requests/${rid}/signal`, {
          method: 'POST',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ signal_type: signalType, data }),
        }).catch(() => {});
      const entry = getOrCreatePeer(requestId, null, sendSig);
      const dc = entry.pc.createDataChannel('chatw-file', { ordered: true });
      attachDataChannel(requestId, entry, dc);
      const offer = await entry.pc.createOffer();
      await entry.pc.setLocalDescription(offer);
      await sendSig(requestId, 'offer', offer);
      return { ok: true };
    },
    async handleSignal(requestId, signalType, data) {
      const sendSig = (rid, st, d) =>
        fetch(`/api/recovery/requests/${rid}/signal`, {
          method: 'POST',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ signal_type: st, data: d }),
        }).catch(() => {});
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
