import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec143';
fs.mkdirSync(OUT_DIR, { recursive: true });

const PORTRAIT_PATH =
  '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/.user_uploaded/media_1795549648014_4417d69b.jpg';
const portraitB64 = fs.readFileSync(PORTRAIT_PATH).toString('base64');

const browser = await chromium.launch({
  headless: true,
  executablePath: '/usr/bin/google-chrome',
  args: ['--no-sandbox', '--disable-setuid-sandbox', '--use-gl=angle', '--use-angle=gl', '--enable-webgl', '--ignore-gpu-blocklist'],
});

try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const pageErrors = [];
  page.on('pageerror', (e) => pageErrors.push(String(e)));

  await page.goto('http://127.0.0.1:8085/', { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(
    () => Boolean(window.__chatwWasmMedia && window.__chatwWasmMedia.getModelRegistryJson && window.__chatwAiRestorer),
    { timeout: 25000 },
  );

  const report = await page.evaluate(async ({ portraitB64 }) => {
    const b64ToU8 = (s) => Uint8Array.from(atob(s), (c) => c.charCodeAt(0));
    const wasm = window.__chatwWasmMedia;
    const restorer = window.__chatwAiRestorer;

    // 1. Hardware & Registry Audit
    const gpuProbe = wasm.probeRealEsrganGpu();
    const hasWebGpu = Boolean(navigator.gpu);
    const registry = JSON.parse(wasm.getModelRegistryJson());
    const codeformerBlocked = !wasm.isModelLicenseAllowed('codeformer');
    const nomosAllowed = wasm.isModelLicenseAllowed('nomosuni_span_x4');
    const clearAllowed = wasm.isModelLicenseAllowed('clearreality_span_x4');

    const computePsnr = (refU8, tstU8) => {
      let mse = 0;
      const n = Math.min(refU8.length, tstU8.length) / 4;
      for (let i = 0; i < n; i++) {
        const dr = refU8[i * 4] - tstU8[i * 4];
        const dg = refU8[i * 4 + 1] - tstU8[i * 4 + 1];
        const db = refU8[i * 4 + 2] - tstU8[i * 4 + 2];
        mse += (dr * dr + dg * dg + db * db) / 3;
      }
      mse /= Math.max(1, n);
      return mse <= 1e-9 ? 99.0 : Number((10 * Math.log10((255 * 255) / mse)).toFixed(2));
    };

    const drawToRgba = async (url, w, h) => {
      const b = await (await fetch(url)).blob();
      const bmp = await createImageBitmap(b);
      const c = document.createElement('canvas');
      c.width = w; c.height = h;
      const ctx = c.getContext('2d');
      ctx.imageSmoothingEnabled = true; ctx.imageSmoothingQuality = 'high';
      ctx.drawImage(bmp, 0, 0, w, h);
      bmp.close();
      return ctx.getImageData(0, 0, w, h).data;
    };

    const cropFacePng = async (url) => {
      const b = await (await fetch(url)).blob();
      const bmp = await createImageBitmap(b);
      const fc = document.createElement('canvas');
      const sx = Math.round(bmp.width * 0.31), sy = Math.round(bmp.height * 0.16);
      const sw = Math.round(bmp.width * 0.36), sh = Math.round(bmp.height * 0.50);
      fc.width = Math.max(sw, 512); fc.height = Math.round(fc.width * sh / sw);
      const ctx = fc.getContext('2d');
      ctx.imageSmoothingEnabled = true; ctx.imageSmoothingQuality = 'high';
      ctx.drawImage(bmp, sx, sy, sw, sh, 0, 0, fc.width, fc.height);
      bmp.close();
      return fc.toDataURL('image/png').split(',')[1];
    };

    // Case A: Heavily Compressed 512x384 AVIF Portrait (Reproduces user's old chat photo!)
    const origBytes = b64ToU8(portraitB64);
    const origBlob = new Blob([origBytes], { type: 'image/jpeg' });
    const origBmp = await createImageBitmap(origBlob);
    const c512 = document.createElement('canvas');
    c512.width = 512; c512.height = 384;
    const ctx512 = c512.getContext('2d');
    ctx512.imageSmoothingEnabled = true; ctx512.imageSmoothingQuality = 'high';
    ctx512.drawImage(origBmp, 0, 0, 512, 384);
    const ref512U8 = ctx512.getImageData(0, 0, 512, 384).data;
    const png512Blob = await new Promise((r) => c512.toBlob(r, 'image/png'));
    const png512U8 = new Uint8Array(await png512Blob.arrayBuffer());

    const tEncA = performance.now();
    const avifA = wasm.encodeAvifAdaptive(png512U8, 'ultra');
    const encMsA = Math.round(performance.now() - tEncA);
    const avifUrlA = URL.createObjectURL(new Blob([avifA.bytes], { type: 'image/avif' }));

    let stage1MsA = null;
    const tAiA = performance.now();
    const aiA = await restorer.restoreImageFromUrl(avifUrlA, {
      mediaId: 'spec143_crushed_512_' + Date.now(),
      origProfileJson: avifA.origProfileJson || '',
      forceRun: true,
      onStage1: () => { stage1MsA = Math.round(performance.now() - tAiA); },
    });
    const aiMsA = Math.round(performance.now() - tAiA);
    const psnrA = computePsnr(ref512U8, await drawToRgba(aiA.url, 512, 384));
    const faceA = await cropFacePng(aiA.url);

    // Case B: High-Res 1024x768 AVIF Portrait (New Spec 136+ chat photo!)
    const c1024 = document.createElement('canvas');
    c1024.width = 1024; c1024.height = 768;
    const ctx1024 = c1024.getContext('2d');
    ctx1024.drawImage(origBmp, 0, 0, 1024, 768);
    const ref1024U8 = ctx1024.getImageData(0, 0, 1024, 768).data;

    const tEncB = performance.now();
    const avifB = wasm.encodeAvifAdaptive(origBytes, 'ultra');
    const encMsB = Math.round(performance.now() - tEncB);
    const avifUrlB = URL.createObjectURL(new Blob([avifB.bytes], { type: 'image/avif' }));

    let stage1MsB = null;
    const tAiB = performance.now();
    const aiB = await restorer.restoreImageFromUrl(avifUrlB, {
      mediaId: 'spec143_hires_1024_' + Date.now(),
      origProfileJson: avifB.origProfileJson || '',
      forceRun: true,
      onStage1: () => { stage1MsB = Math.round(performance.now() - tAiB); },
    });
    const aiMsB = Math.round(performance.now() - tAiB);
    const psnrB = computePsnr(ref1024U8, await drawToRgba(aiB.url, 1024, 768));
    const faceB = await cropFacePng(aiB.url);

    // Case C: Crisp Text / Document / QR Guard (Must SKIP neural SR to protect exact text & QR!)
    const cDoc = document.createElement('canvas');
    cDoc.width = 320; cDoc.height = 240;
    const ctxDoc = cDoc.getContext('2d');
    ctxDoc.fillStyle = '#ffffff';
    ctxDoc.fillRect(0, 0, 320, 240);
    ctxDoc.fillStyle = '#111111';
    ctxDoc.font = 'bold 16px monospace';
    for (let i = 0; i < 10; i++) {
      ctxDoc.fillText(`INVOICE #2026-00${i}  TOTAL: $1,240.00`, 16, 28 + i * 21);
    }
    const docBlob = await new Promise((r) => cDoc.toBlob(r, 'image/png'));
    const docUrl = URL.createObjectURL(docBlob);
    const aiDoc = await restorer.restoreImageFromUrl(docUrl, {
      mediaId: 'spec143_doc_qr_' + Date.now(),
      forceRun: true,
    });

    // Case D: AbortSignal Cancellation Guard
    const ac = new AbortController();
    ac.abort();
    const aiCancelled = await restorer.restoreImageFromUrl(avifUrlA, {
      mediaId: 'spec143_abort_' + Date.now(),
      forceRun: true,
      signal: ac.signal,
    });

    origBmp.close();
    return {
      platform: navigator.userAgent,
      gpuProbe,
      hasWebGpu,
      registryCount: registry.length,
      codeformerBlocked,
      nomosAllowed,
      clearAllowed,
      caseA_crushed_512_avif: {
        origPngBytes: png512U8.byteLength,
        avifBytes: avifA.bytes.byteLength,
        ratio: Number((png512U8.byteLength / avifA.bytes.byteLength).toFixed(2)),
        encMs: encMsA,
        modelId: aiA.modelId,
        modelLabel: aiA.modelLabel,
        backend: aiA.backend,
        outW: aiA.width, outH: aiA.height,
        stage1Ms: stage1MsA,
        totalAiMs: aiMsA,
        ssim: aiA.ssim,
        psnrDb: psnrA,
      },
      caseB_hires_1024_avif: {
        origJpgBytes: origBytes.byteLength,
        avifBytes: avifB.bytes.byteLength,
        ratio: Number((origBytes.byteLength / avifB.bytes.byteLength).toFixed(2)),
        encMs: encMsB,
        modelId: aiB.modelId,
        modelLabel: aiB.modelLabel,
        backend: aiB.backend,
        outW: aiB.width, outH: aiB.height,
        stage1Ms: stage1MsB,
        totalAiMs: aiMsB,
        ssim: aiB.ssim,
        psnrDb: psnrB,
      },
      caseC_crisp_text_qr: {
        enhanced: aiDoc.enhanced,
        skipped: aiDoc.skipped,
        reason: aiDoc.reason,
      },
      caseD_abort_cancel: {
        enhanced: aiCancelled.enhanced,
        skipped: aiCancelled.skipped,
        reason: aiCancelled.reason,
      },
      faceA,
      faceB,
    };
  }, { portraitB64 });

  fs.writeFileSync(path.join(OUT_DIR, '01_crushed_512_avif_nomosuni_face.png'), Buffer.from(report.faceA, 'base64'));
  fs.writeFileSync(path.join(OUT_DIR, '02_hires_1024_avif_clearreality_face.png'), Buffer.from(report.faceB, 'base64'));
  delete report.faceA;
  delete report.faceB;
  report.pageErrors = pageErrors;

  console.log('SPEC 143 REAL BENCHMARK:', JSON.stringify(report, null, 2));

  if (!report.codeformerBlocked) throw new Error('CodeFormer commercial license gate failed');
  if (report.caseA_crushed_512_avif.modelId !== 'nomosuni_span_x4') {
    throw new Error(`Expected nomosuni_span_x4 for crushed 512 AVIF, got ${report.caseA_crushed_512_avif.modelId}`);
  }
  if (report.caseB_hires_1024_avif.modelId !== 'clearreality_span_x4') {
    throw new Error(`Expected clearreality_span_x4 for clean 1024 AVIF, got ${report.caseB_hires_1024_avif.modelId}`);
  }
  if (report.caseC_crisp_text_qr.enhanced) {
    throw new Error('Crisp text/document was NOT skipped by text/QR protection guard!');
  }
  if (!report.caseD_abort_cancel.skipped) {
    throw new Error('AbortSignal cancellation guard failed!');
  }
  if (pageErrors.length > 0) throw new Error('Page errors: ' + pageErrors.join('; '));
  console.log('ALL SPEC 143 BENCHMARK ASSERTIONS PASSED!');
} finally {
  await browser.close();
}
