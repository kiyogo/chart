// ChatW Dedicated Media Processing Web Worker (Spec 071 + Spec 110 + Spec 153)
// - Runs 100% OFF the Main UI Thread: BLAKE3, SHA-256, PDQ 256-bit hashing, CWCMP1 compression,
//   and release-compiled Rust WASM `ravif`/`rav1e` AVIF encoding (`0ms` UI freeze!).
// - Automatic EXIF/GPS stripping + 16x16 progressive micro-blur preview off the Main Thread.
import initWorkerWasm, {
  encodeRgbaAvifAdaptive,
  encodeBytesAvifAdaptive,
  computeImagePdq,
  blake3Hex,
  compressFileSmart,
} from '/pkg/chatw_media_worker.js?v=__BUILD_TS__';

const BUILD_VER = '__BUILD_TS__';
const CWRT1_MAGIC = [0x43, 0x57, 0x52, 0x54, 0x31, 0x01, 0x00, 0x00];
let workerWasmPromise = null;
let nativeAvifSupported = null;

async function unpackCwrt1(packed) {
  const bytes = packed instanceof Uint8Array ? packed : new Uint8Array(packed);
  if (bytes.byteLength < 16) throw new Error('cwrt1_truncated');
  for (let i = 0; i < 8; i++) {
    if (bytes[i] !== CWRT1_MAGIC[i]) throw new Error('cwrt1_bad_magic');
  }
  const m0 = bytes[8], m1 = bytes[9], m2 = bytes[10], m3 = bytes[11];
  const payloadLen = bytes.byteLength - 12;
  const unmasked = new Uint8Array(payloadLen);
  let i = 0;
  for (; i + 3 < payloadLen; i += 4) {
    unmasked[i] = bytes[i + 12] ^ m0;
    unmasked[i + 1] = bytes[i + 13] ^ m1;
    unmasked[i + 2] = bytes[i + 14] ^ m2;
    unmasked[i + 3] = bytes[i + 15] ^ m3;
  }
  for (; i < payloadLen; i++) {
    const mask = (i & 3) === 0 ? m0 : (i & 3) === 1 ? m1 : (i & 3) === 2 ? m2 : m3;
    unmasked[i] = bytes[i + 12] ^ mask;
  }
  const ds = new DecompressionStream('gzip');
  const stream = new Blob([unmasked]).stream().pipeThrough(ds);
  return await new Response(stream).arrayBuffer();
}

function ensureWorkerWasm() {
  if (!workerWasmPromise) {
    workerWasmPromise = (async () => {
      const resp = await fetch('/pkg/media_worker_wasm.bin?v=' + encodeURIComponent(BUILD_VER), {
        credentials: 'same-origin',
        cache: 'force-cache',
      });
      if (!resp.ok) throw new Error('worker_wasm_http_' + resp.status);
      const rawBuf = await unpackCwrt1(await resp.arrayBuffer());
      await initWorkerWasm({ module_or_path: rawBuf });
      return true;
    })().catch((err) => {
      workerWasmPromise = null;
      throw err;
    });
  }
  return workerWasmPromise;
}

// Warm up worker WASM proactively right after spawn so first photo encode has 0 fetch delay.
ensureWorkerWasm().catch(() => {});

function bytesToB64(u8) {
  let s = '';
  for (let i = 0; i < u8.length; i += 0x8000) {
    s += String.fromCharCode.apply(null, u8.subarray(i, i + 0x8000));
  }
  return btoa(s);
}

async function sha256Hex(u8) {
  const buf = await crypto.subtle.digest('SHA-256', u8);
  const arr = new Uint8Array(buf);
  let out = '';
  for (let i = 0; i < arr.length; i++) out += arr[i].toString(16).padStart(2, '0');
  return out;
}

function sniffBytesFormat(u8) {
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
  }
  return { ext: 'bin', mime: 'application/octet-stream', animated: false };
}

function detectCrispTextOrAlpha(ctx, w, h) {
  const pw = Math.min(256, w);
  const ph = Math.min(256, h);
  if (pw < 4 || ph < 4) return { isCrisp: false, hasAlpha: false };
  const probe = new OffscreenCanvas(pw, ph);
  const pctx = probe.getContext('2d');
  pctx.drawImage(ctx.canvas, 0, 0, pw, ph);
  const data = pctx.getImageData(0, 0, pw, ph).data;
  let sharp = 0;
  let flat = 0;
  let hasAlpha = false;
  const total = (pw - 1) * (ph - 1);
  for (let y = 0; y < ph - 1; y++) {
    for (let x = 0; x < pw - 1; x++) {
      const i = (y * pw + x) * 4;
      if (data[i + 3] < 250) hasAlpha = true;
      const ir = i + 4;
      const id = i + pw * 4;
      const dh = Math.abs(data[i] - data[ir]) + Math.abs(data[i + 1] - data[ir + 1]) + Math.abs(data[i + 2] - data[ir + 2]);
      const dv = Math.abs(data[i] - data[id]) + Math.abs(data[i + 1] - data[id + 1]) + Math.abs(data[i + 2] - data[id + 2]);
      if (dh > 120 || dv > 120) sharp++;
      else if (dh <= 4 && dv <= 4) flat++;
    }
  }
  return { isCrisp: hasAlpha || (flat / total > 0.45 && sharp / total > 0.006), hasAlpha };
}

function adaptiveMaxEdge(w, h, qualityMode) {
  if (qualityMode === 'original') return null;
  const maxEdge = Math.max(w, h);
  if (qualityMode === 'data_saver') return maxEdge > 1280 ? 1280 : null;
  if (maxEdge > 2048) return 1920;
  return null;
}

async function tryGenuineAvif(canvas, quality) {
  if (nativeAvifSupported === false) return null;
  try {
    const blob = await canvas.convertToBlob({ type: 'image/avif', quality });
    if (blob && blob.size > 0 && blob.type === 'image/avif') {
      const u8 = new Uint8Array(await blob.arrayBuffer());
      const sniff = sniffBytesFormat(u8);
      if (sniff.ext === 'avif') {
        nativeAvifSupported = true;
        return { u8, sniff };
      }
    }
  } catch (_) {}
  nativeAvifSupported = false;
  return null;
}

async function makeMicroBlur(bitmap, srcW, srcH) {
  try {
    const bw = Math.max(4, Math.min(18, Math.round(srcW >= srcH ? 16 : 16 * (srcW / Math.max(1, srcH)))));
    const bh = Math.max(4, Math.min(18, Math.round(srcH > srcW ? 16 : 16 * (srcH / Math.max(1, srcW)))));
    const c = new OffscreenCanvas(bw, bh);
    const ctx = c.getContext('2d');
    ctx.imageSmoothingEnabled = true;
    ctx.drawImage(bitmap, 0, 0, bw, bh);
    for (const mime of ['image/avif', 'image/webp', 'image/jpeg']) {
      try {
        const b = await c.convertToBlob({ type: mime, quality: 0.35 });
        if (b && b.size > 0 && b.size <= 1024) {
          const u8 = new Uint8Array(await b.arrayBuffer());
          const dataUrl = `data:${b.type};base64,${bytesToB64(u8)}`;
          if (dataUrl.length <= 1400) return dataUrl;
        }
      } catch (_) {}
    }
  } catch (_) {}
  return null;
}

function computePdqFromBitmap(bitmap, origW, origH) {
  if (origW < 8 || origH < 8) return null;
  const scale = Math.min(1, 256 / Math.max(origW, origH));
  const pw = Math.max(16, Math.round(origW * scale));
  const ph = Math.max(16, Math.round(origH * scale));
  const pc = new OffscreenCanvas(pw, ph);
  const pctx = pc.getContext('2d', { willReadFrequently: true });
  pctx.imageSmoothingEnabled = true;
  pctx.imageSmoothingQuality = 'high';
  pctx.drawImage(bitmap, 0, 0, pw, ph);
  const d = pctx.getImageData(0, 0, pw, ph).data;
  const pdq = computeImagePdq(new Uint8Array(d.buffer, d.byteOffset, d.byteLength), pw, ph);
  return {
    pdqHex: pdq.pdqHex || '',
    pdqQuality: Number(pdq.quality || 0),
    captionBandHash: String(pdq.captionBandHash || '0'),
    colorMomentHash: String(pdq.colorMomentHash || '0'),
    aspectRatioMilli: Number(pdq.aspectRatioMilli || Math.round((origW * 1000) / Math.max(1, origH))),
    width: origW,
    height: origH,
  };
}

self.onmessage = async (ev) => {
  const { id, task = 'optimize_image', buffer, mime, opts = {} } = ev.data || {};
  if (!id || !buffer) {
    self.postMessage({ id, ok: false, error: 'missing_payload' });
    return;
  }
  try {
    await ensureWorkerWasm();
    const rawU8 = new Uint8Array(buffer);

    // Task 1: Off-main-thread cryptographic + perceptual fingerprint check (for local cross-chat dedup)
    if (task === 'compute_pdq_blake3') {
      const [origBlake3, sha256] = await Promise.all([
        Promise.resolve(blake3Hex(rawU8)),
        sha256Hex(rawU8).catch(() => ''),
      ]);
      let pdq = null;
      if (opts.isImage && typeof OffscreenCanvas !== 'undefined') {
        try {
          const bmp = await createImageBitmap(new Blob([rawU8], { type: mime || 'image/png' }));
          pdq = computePdqFromBitmap(bmp, bmp.width, bmp.height);
          bmp.close();
        } catch (_) {}
      }
      self.postMessage({ id, ok: true, origBlake3, sha256, pdq });
      return;
    }

    // Task 2: Off-main-thread CWCMP1 lossless document compression
    if (task === 'compress_file') {
      const cmp = compressFileSmart(rawU8);
      if (cmp && cmp.ok && cmp.compressed && cmp.bytes && cmp.bytes.byteLength < rawU8.byteLength) {
        const outU8 = cmp.bytes instanceof Uint8Array ? cmp.bytes : new Uint8Array(cmp.bytes);
        self.postMessage({
          id,
          ok: true,
          compressed: true,
          codec: cmp.codec || 'cwcmp1',
          bytesB64: bytesToB64(outU8),
          optimizedSize: outU8.byteLength,
        });
      } else {
        self.postMessage({ id, ok: true, compressed: false, codec: 'raw', bytesB64: bytesToB64(rawU8), optimizedSize: rawU8.byteLength });
      }
      return;
    }

    // Task 3: Full off-main-thread AVIF optimization (OffscreenCanvas -> Release WASM ravif!)
    if (typeof OffscreenCanvas === 'undefined') {
      self.postMessage({ id, ok: false, error: 'offscreen_unsupported' });
      return;
    }

    const rawSniff = sniffBytesFormat(rawU8);
    if (rawSniff.animated) {
      const wasmMode = opts.qualityMode === 'data_saver' ? 'aggressive' : 'fast';
      const wres = encodeBytesAvifAdaptive(rawU8, wasmMode);
      if (wres && wres.ok && wres.bytes && wres.bytes.byteLength > 0) {
        const avifU8 = wres.bytes instanceof Uint8Array ? wres.bytes : new Uint8Array(wres.bytes);
        if (avifU8.byteLength > 0) {
          self.postMessage({
            id,
            ok: true,
            bytesB64: bytesToB64(avifU8),
            mime: 'image/avif',
            ext: 'avif',
            originalSize: rawU8.byteLength,
            optimizedSize: avifU8.byteLength,
            isCrispText: Boolean(wres.isCrispText),
            width: wres.width || 0,
            height: wres.height || 0,
            targetWidth: wres.targetWidth || 0,
            targetHeight: wres.targetHeight || 0,
            superResRecommended: Boolean(wres.superResRecommended),
            origProfileJson: wres.origProfileJson || null,
            aiRecommended: Boolean(wres.aiRecommended),
            blurDataUrl: null,
            thumbB64: null,
            thumbMime: null,
            workerUsed: true,
            codec: 'avis_wasm_rav1e_worker',
          });
          return;
        }
      }
      self.postMessage({ id, ok: false, animatedNeedsFallback: true });
      return;
    }

    const bitmap = await createImageBitmap(new Blob([rawU8], { type: rawSniff.mime || mime || 'image/png' }));
    const origW = bitmap.width;
    const origH = bitmap.height;

    const probeCanvas = new OffscreenCanvas(Math.min(origW, 512), Math.min(origH, 512));
    const pctx = probeCanvas.getContext('2d');
    pctx.drawImage(bitmap, 0, 0, probeCanvas.width, probeCanvas.height);
    const { isCrisp } = detectCrispTextOrAlpha(pctx, probeCanvas.width, probeCanvas.height);
    const blurDataUrl = await makeMicroBlur(bitmap, origW, origH);

    const keepAsIsFresh = !opts.forceCompress && opts.qualityMode !== 'data_saver' && rawU8.byteLength <= 24 * 1024 * 1024;
    if (rawSniff.ext === 'avif' || opts.preserveOriginal || opts.qualityMode === 'original' || keepAsIsFresh) {
      const finalMime = rawSniff.ext === 'avif'
        ? 'image/avif'
        : (rawSniff.mime !== 'application/octet-stream' ? rawSniff.mime : (mime || 'application/octet-stream'));
      const finalExt = rawSniff.ext === 'bin' ? 'bin' : rawSniff.ext;
      // Losslessly strip JPEG EXIF/GPS APP1 & COM metadata segments without touching image pixels
      let outU8 = rawU8;
      let stripped = false;
      if (!opts.preserveOriginal && rawSniff.ext === 'jpg' && rawU8.byteLength > 4 && rawU8[0] === 0xFF && rawU8[1] === 0xD8) {
        const parts = [rawU8.subarray(0, 2)];
        let p = 2;
        let ok = true;
        while (p + 4 <= rawU8.byteLength && rawU8[p] === 0xFF) {
          const marker = rawU8[p + 1];
          if (marker === 0xDA || marker === 0xD9) {
            parts.push(rawU8.subarray(p));
            p = rawU8.byteLength;
            break;
          }
          if (marker === 0x01 || (marker >= 0xD0 && marker <= 0xD7)) {
            parts.push(rawU8.subarray(p, p + 2));
            p += 2;
            continue;
          }
          const segLen = (rawU8[p + 2] << 8) | rawU8[p + 3];
          if (segLen < 2 || p + 2 + segLen > rawU8.byteLength) { ok = false; break; }
          if (marker === 0xE1 || marker === 0xFE) {
            stripped = true;
          } else {
            parts.push(rawU8.subarray(p, p + 2 + segLen));
          }
          p += 2 + segLen;
        }
        if (ok && stripped && p === rawU8.byteLength) {
          const total = parts.reduce((acc, a) => acc + a.byteLength, 0);
          const merged = new Uint8Array(total);
          let off = 0;
          for (const part of parts) { merged.set(part, off); off += part.byteLength; }
          outU8 = merged;
        } else {
          stripped = false;
        }
      }
      bitmap.close();
      self.postMessage({
        id,
        ok: true,
        bytesB64: bytesToB64(outU8),
        mime: finalMime,
        ext: finalExt,
        originalSize: rawU8.byteLength,
        optimizedSize: outU8.byteLength,
        isCrispText: isCrisp,
        width: origW,
        height: origH,
        targetWidth: origW,
        targetHeight: origH,
        blurDataUrl,
        thumbB64: null,
        thumbMime: null,
        exifStripped: stripped,
        workerUsed: true,
        codec: rawSniff.ext === 'avif' ? 'already_avif_passthrough' : 'original_passthrough',
        estimatedSsim: 1.0,
        aiRecommended: false,
      });
      return;
    }

    const maxEdge = opts.maxDim || adaptiveMaxEdge(origW, origH, opts.qualityMode || 'ultra_optimized');
    let tw = origW;
    let th = origH;
    if (maxEdge && (tw > maxEdge || th > maxEdge)) {
      const r = Math.min(maxEdge / tw, maxEdge / th);
      tw = Math.max(1, Math.round(tw * r));
      th = Math.max(1, Math.round(th * r));
    }

    const canvas = new OffscreenCanvas(tw, th);
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(bitmap, 0, 0, tw, th);

    const q = isCrisp ? 0.82 : (opts.qualityMode === 'data_saver' ? 0.65 : 0.80);
    const avif = await tryGenuineAvif(canvas, q);
    if (avif && avif.u8.byteLength < rawU8.byteLength) {
      bitmap.close();
      self.postMessage({
        id,
        ok: true,
        bytesB64: bytesToB64(avif.u8),
        mime: 'image/avif',
        ext: 'avif',
        originalSize: rawU8.byteLength,
        optimizedSize: avif.u8.byteLength,
        isCrispText: isCrisp,
        width: tw,
        height: th,
        targetWidth: origW,
        targetHeight: origH,
        superResRecommended: tw < origW,
        blurDataUrl,
        thumbB64: null,
        thumbMime: null,
        exifStripped: true,
        workerUsed: true,
        codec: 'avif_native_canvas_worker',
      });
      return;
    }

    // Release WASM `ravif` right inside this background Web Worker thread -> ZERO main-thread freeze!
    let fullRgba;
    if (tw === origW && th === origH) {
      fullRgba = ctx.getImageData(0, 0, origW, origH).data;
    } else {
      const fullCanvas = new OffscreenCanvas(origW, origH);
      const fctx = fullCanvas.getContext('2d', { willReadFrequently: true });
      fctx.drawImage(bitmap, 0, 0);
      fullRgba = fctx.getImageData(0, 0, origW, origH).data;
    }
    bitmap.close();

    const wasmMode = opts.qualityMode === 'data_saver' ? 'aggressive' : (origW * origH > 400_000 ? 'fast' : 'ultra');
    const wres = encodeRgbaAvifAdaptive(
      new Uint8Array(fullRgba.buffer, fullRgba.byteOffset, fullRgba.byteLength),
      origW,
      origH,
      rawU8.byteLength,
      rawU8,
      wasmMode,
    );
    if (wres && wres.ok && wres.bytes && wres.bytes.byteLength > 0) {
      const avifU8 = wres.bytes instanceof Uint8Array ? wres.bytes : new Uint8Array(wres.bytes);
      const outSniff = sniffBytesFormat(avifU8);
      if (outSniff.ext === 'avif' && avifU8.byteLength < rawU8.byteLength) {
        self.postMessage({
          id,
          ok: true,
          bytesB64: bytesToB64(avifU8),
          mime: 'image/avif',
          ext: 'avif',
          originalSize: rawU8.byteLength,
          optimizedSize: avifU8.byteLength,
          isCrispText: Boolean(wres.isCrispText ?? isCrisp),
          width: wres.width || tw,
          height: wres.height || th,
          targetWidth: wres.targetWidth || origW,
          targetHeight: wres.targetHeight || origH,
          superResRecommended: Boolean(wres.superResRecommended ?? ((wres.width || origW) < origW)),
          origProfileJson: wres.origProfileJson || null,
          aiRecommended: Boolean(wres.aiRecommended),
          estimatedSsim: wres.estimatedSsim || null,
          blurDataUrl,
          thumbB64: null,
          thumbMime: null,
          exifStripped: true,
          workerUsed: true,
          codec: 'avif_wasm_ravif_worker',
        });
        return;
      }
    }

    // Honest passthrough if input is already smaller than AVIF
    self.postMessage({
      id,
      ok: true,
      bytesB64: bytesToB64(rawU8),
      mime: rawSniff.mime !== 'application/octet-stream' ? rawSniff.mime : (mime || 'application/octet-stream'),
      ext: rawSniff.ext,
      originalSize: rawU8.byteLength,
      optimizedSize: rawU8.byteLength,
      isCrispText: isCrisp,
      width: origW,
      height: origH,
      targetWidth: origW,
      targetHeight: origH,
      blurDataUrl,
      thumbB64: null,
      thumbMime: null,
      workerUsed: true,
      codec: 'original_passthrough',
    });
  } catch (err) {
    self.postMessage({ id, ok: false, error: String(err && err.message ? err.message : err) });
  }
};
