import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec145';
fs.mkdirSync(OUT_DIR, { recursive: true });

const IMG_PATH =
  '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/.user_uploaded/media_1795549648014_4417d69b.jpg';
const imgB64 = fs.readFileSync(IMG_PATH).toString('base64');

const browser = await chromium.launch({
  headless: true,
  executablePath: '/usr/bin/google-chrome',
  args: [
    '--no-sandbox',
    '--disable-setuid-sandbox',
    '--use-gl=angle',
    '--use-angle=gl',
    '--enable-webgl',
    '--enable-unsafe-webgpu',
    '--enable-features=Vulkan',
    '--ignore-gpu-blocklist',
  ],
});

try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const pageErrors = [];
  page.on('pageerror', (e) => pageErrors.push(String(e)));

  await page.goto('http://127.0.0.1:8085/', { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(
    () => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer && window.__chatwFaceRestorer),
    { timeout: 25000 },
  );

  const report = await page.evaluate(async ({ imgB64 }) => {
    const b64ToU8 = (s) => Uint8Array.from(atob(s), (c) => c.charCodeAt(0));
    const wasm = window.__chatwWasmMedia;
    const restorer = window.__chatwAiRestorer;
    const webgpuOk = await window.__chatwFaceRestorer.isWebGpuAvailable();

    const snapshot = async (url) => {
      const b = await (await fetch(url)).blob();
      const bmp = await createImageBitmap(b);
      const c = document.createElement('canvas');
      c.width = bmp.width; c.height = bmp.height;
      c.getContext('2d').drawImage(bmp, 0, 0);
      const fc = document.createElement('canvas');
      const sx = Math.round(bmp.width * 0.30), sy = Math.round(bmp.height * 0.16);
      const sw = Math.round(bmp.width * 0.38), sh = Math.round(bmp.height * 0.56);
      fc.width = Math.max(sw, 640); fc.height = Math.round(fc.width * sh / sw);
      fc.getContext('2d').drawImage(c, sx, sy, sw, sh, 0, 0, fc.width, fc.height);
      const out = { w: bmp.width, h: bmp.height, facePng: fc.toDataURL('image/png').split(',')[1] };
      bmp.close();
      return out;
    };

    const origBytes = b64ToU8(imgB64);
    const origBmp = await createImageBitmap(new Blob([origBytes], { type: 'image/jpeg' }));

    // Case 1: Crushed 512x384 AVIF (14 KB — 29x smaller than PNG!) -> NomosUni SPAN 4x + GFPGANv1.4 WebGPU!
    const c512 = document.createElement('canvas'); c512.width = 512; c512.height = 384;
    const ctx512 = c512.getContext('2d'); ctx512.imageSmoothingEnabled = true; ctx512.imageSmoothingQuality = 'high';
    ctx512.drawImage(origBmp, 0, 0, 512, 384);
    const png512 = new Uint8Array(await (await new Promise((r) => c512.toBlob(r, 'image/png'))).arrayBuffer());
    const avif512 = wasm.encodeAvifAdaptive(png512, 'ultra');
    const url512 = URL.createObjectURL(new Blob([avif512.bytes], { type: 'image/avif' }));

    let stage1Ms512 = null;
    const t0 = performance.now();
    const res512 = await restorer.restoreImageFromUrl(url512, {
      mediaId: 'spec145_512_' + Date.now(),
      origProfileJson: avif512.origProfileJson || '',
      forceRun: true,
      onStage1: () => { stage1Ms512 = Math.round(performance.now() - t0); },
    });
    const totalMs512 = Math.round(performance.now() - t0);
    const snap512 = await snapshot(res512.url);

    // Case 2: High-Res 1024x768 AVIF (33 KB) -> NomosUni SPAN 2x + GFPGANv1.4 WebGPU (warm cached session!)
    const avif1024 = wasm.encodeAvifAdaptive(origBytes, 'ultra');
    const url1024 = URL.createObjectURL(new Blob([avif1024.bytes], { type: 'image/avif' }));

    let stage1Ms1024 = null;
    const t1 = performance.now();
    const res1024 = await restorer.restoreImageFromUrl(url1024, {
      mediaId: 'spec145_1024_' + Date.now(),
      origProfileJson: avif1024.origProfileJson || '',
      forceRun: true,
      onStage1: () => { stage1Ms1024 = Math.round(performance.now() - t1); },
    });
    const totalMs1024 = Math.round(performance.now() - t1);
    const snap1024 = await snapshot(res1024.url);

    origBmp.close();
    return {
      webgpuOk,
      case1_crushed_512_avif: {
        avifBytes: avif512.bytes.byteLength,
        modelId: res512.modelId,
        modelLabel: res512.modelLabel,
        outRes: `${snap512.w}x${snap512.h}`,
        stage1Ms: stage1Ms512,
        totalMs: totalMs512,
        facePng: snap512.facePng,
      },
      case2_hires_1024_avif: {
        avifBytes: avif1024.bytes.byteLength,
        modelId: res1024.modelId,
        modelLabel: res1024.modelLabel,
        outRes: `${snap1024.w}x${snap1024.h}`,
        stage1Ms: stage1Ms1024,
        totalMs: totalMs1024,
        facePng: snap1024.facePng,
      },
    };
  }, { imgB64 });

  fs.writeFileSync(path.join(OUT_DIR, '01_crushed_512_hybrid_gfpgan_400pct.png'), Buffer.from(report.case1_crushed_512_avif.facePng, 'base64'));
  fs.writeFileSync(path.join(OUT_DIR, '02_hires_1024_hybrid_gfpgan_400pct.png'), Buffer.from(report.case2_hires_1024_avif.facePng, 'base64'));
  delete report.case1_crushed_512_avif.facePng;
  delete report.case2_hires_1024_avif.facePng;

  console.log('SPEC 145 HYBRID FACE E2E REPORT:', JSON.stringify({ report, pageErrors }, null, 2));

  if (!report.webgpuOk) throw new Error('WebGPU not enabled in Chrome E2E');
  if (!report.case1_crushed_512_avif.modelLabel.includes('GFPGANv1.4')) {
    throw new Error('Case 1 did not apply GFPGANv1.4 hybrid face restoration');
  }
  if (!report.case2_hires_1024_avif.modelLabel.includes('GFPGANv1.4')) {
    throw new Error('Case 2 did not apply GFPGANv1.4 hybrid face restoration');
  }
  if (pageErrors.length > 0) throw new Error('Page errors: ' + pageErrors.join('; '));
  console.log('ALL SPEC 145 HYBRID FACE E2E ASSERTIONS PASSED!');
} finally {
  await browser.close();
}
