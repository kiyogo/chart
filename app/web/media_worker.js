// ChatW Dedicated Media Processing Web Worker (Spec 071 — OffscreenCanvas)
// Runs image downscaling, AVIF/WebP adaptive compression, automatic EXIF/GPS stripping,
// and 16x16 micro-blur thumbnail generation completely off the Main Thread!
(() => {
  function bytesToB64(u8) {
    let s = '';
    for (let i = 0; i < u8.length; i += 0x8000) {
      s += String.fromCharCode.apply(null, u8.subarray(i, i + 0x8000));
    }
    return btoa(s);
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

  async function encodeOffscreen(canvas, mimes, quality) {
    for (const mime of mimes) {
      try {
        const blob = await canvas.convertToBlob({ type: mime, quality });
        if (blob && blob.size > 0 && blob.type === mime) {
          const u8 = new Uint8Array(await blob.arrayBuffer());
          const sniff = sniffBytesFormat(u8);
          if (sniff.mime === mime) return { u8, sniff };
        }
      } catch (_) {}
    }
    const pngBlob = await canvas.convertToBlob({ type: 'image/png' });
    const u8 = new Uint8Array(await pngBlob.arrayBuffer());
    return { u8, sniff: sniffBytesFormat(u8) };
  }

  async function makeMicroBlur(bitmap, srcW, srcH) {
    try {
      const bw = Math.max(4, Math.min(18, Math.round(srcW >= srcH ? 16 : 16 * (srcW / Math.max(1, srcH)))));
      const bh = Math.max(4, Math.min(18, Math.round(srcH > srcW ? 16 : 16 * (srcH / Math.max(1, srcW)))));
      const c = new OffscreenCanvas(bw, bh);
      const ctx = c.getContext('2d');
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(bitmap, 0, 0, bw, bh);
      const res = await encodeOffscreen(c, ['image/webp', 'image/jpeg'], 0.35);
      const dataUrl = `data:${res.sniff.mime};base64,${bytesToB64(res.u8)}`;
      return dataUrl.length <= 1400 ? dataUrl : null;
    } catch (_) {
      return null;
    }
  }

  self.onmessage = async (ev) => {
    const { id, buffer, mime, opts = {} } = ev.data || {};
    if (!id || !buffer || typeof OffscreenCanvas === 'undefined') {
      self.postMessage({ id, ok: false, error: 'offscreen_unsupported' });
      return;
    }
    try {
      const rawU8 = new Uint8Array(buffer);
      const rawSniff = sniffBytesFormat(rawU8);
      if (rawSniff.animated) {
        self.postMessage({
          id,
          ok: true,
          bytesB64: bytesToB64(rawU8),
          mime: rawSniff.mime,
          ext: rawSniff.ext,
          originalSize: rawU8.byteLength,
          optimizedSize: rawU8.byteLength,
          isCrispText: false,
          width: 0,
          height: 0,
          blurDataUrl: null,
          thumbB64: null,
          thumbMime: null,
          exifStripped: false,
        });
        return;
      }

      const bitmap = await createImageBitmap(new Blob([rawU8], { type: rawSniff.mime || mime || 'image/png' }));
      const maxDim = opts.maxDim || 1920;
      let tw = bitmap.width;
      let th = bitmap.height;
      if (tw > maxDim || th > maxDim) {
        const r = Math.min(maxDim / tw, maxDim / th);
        tw = Math.max(1, Math.round(tw * r));
        th = Math.max(1, Math.round(th * r));
      }

      // Drawing through OffscreenCanvas inherently strips 100% of EXIF/GPS/IPTC privacy metadata!
      const canvas = new OffscreenCanvas(tw, th);
      const ctx = canvas.getContext('2d');
      ctx.imageSmoothingEnabled = true;
      ctx.imageSmoothingQuality = 'high';
      ctx.drawImage(bitmap, 0, 0, tw, th);

      const blurDataUrl = await makeMicroBlur(bitmap, tw, th);
      const best = opts.preserveOriginal
        ? { u8: rawU8, sniff: rawSniff }
        : await encodeOffscreen(canvas, ['image/avif', 'image/webp', 'image/jpeg'], 0.80);

      let thumbB64 = null;
      let thumbMime = null;
      try {
        const tScale = Math.min(320 / tw, 320 / th, 1);
        const tc = new OffscreenCanvas(Math.max(1, Math.round(tw * tScale)), Math.max(1, Math.round(th * tScale)));
        tc.getContext('2d').drawImage(bitmap, 0, 0, tc.width, tc.height);
        const thRes = await encodeOffscreen(tc, ['image/webp', 'image/jpeg'], 0.68);
        thumbB64 = bytesToB64(thRes.u8);
        thumbMime = thRes.sniff.mime;
      } catch (_) {}

      bitmap.close();

      self.postMessage({
        id,
        ok: true,
        bytesB64: bytesToB64(best.u8),
        mime: best.sniff.mime,
        ext: best.sniff.ext,
        originalSize: rawU8.byteLength,
        optimizedSize: best.u8.byteLength,
        isCrispText: false,
        width: tw,
        height: th,
        blurDataUrl,
        thumbB64,
        thumbMime,
        exifStripped: !opts.preserveOriginal,
        workerUsed: true,
      });
    } catch (err) {
      self.postMessage({ id, ok: false, error: String(err && err.message ? err.message : err) });
    }
  };
})();
