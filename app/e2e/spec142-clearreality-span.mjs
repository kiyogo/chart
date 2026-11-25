import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec142';
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
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));

  await page.goto('http://127.0.0.1:8085/', { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(
    () => Boolean(window.__chatwWasmMedia && window.__chatwWasmMedia.restoreImageRealEsrgan && window.__chatwAiRestorer),
    { timeout: 25000 },
  );

  const result = await page.evaluate(async ({ portraitB64 }) => {
    const b64ToU8 = (s) => Uint8Array.from(atob(s), (c) => c.charCodeAt(0));

    // 1. Verify bundled CWSP1 weight file served over HTTP
    const headResp = await fetch('/models/clearreality_v1_x4.cwsp', { cache: 'no-store' });
    const cwspBuf = await headResp.arrayBuffer();
    const licAllowed = window.__chatwWasmMedia.isModelLicenseAllowed('clearreality_span_x4');

    // Helper: draw blob URL onto 2D canvas and extract full JPG + 400%-zoom face crop PNG
    const snapshotUrl = async (url) => {
      const resp = await fetch(url);
      const blob = await resp.blob();
      const bmp = await createImageBitmap(blob);
      const c = document.createElement('canvas');
      c.width = bmp.width;
      c.height = bmp.height;
      c.getContext('2d').drawImage(bmp, 0, 0);
      const fullJpg = c.toDataURL('image/jpeg', 0.94).split(',')[1];

      const fc = document.createElement('canvas');
      const sx = Math.round(bmp.width * 0.31);
      const sy = Math.round(bmp.height * 0.16);
      const sw = Math.round(bmp.width * 0.36);
      const sh = Math.round(bmp.height * 0.50);
      fc.width = sw;
      fc.height = sh;
      fc.getContext('2d').drawImage(bmp, sx, sy, sw, sh, 0, 0, sw, sh);
      const facePng = fc.toDataURL('image/png').split(',')[1];
      bmp.close();
      return { w: c.width, h: c.height, fullJpg, facePng };
    };

    // 2. Test Case 1: Crushed 512x384 q=0.45 Portrait (single-pass 6-MRT FP16 SPAN 4x!)
    const origBlob = new Blob([b64ToU8(portraitB64)], { type: 'image/jpeg' });
    const origBmp = await createImageBitmap(origBlob);
    const c512 = document.createElement('canvas');
    c512.width = 512;
    c512.height = 384;
    const ctx512 = c512.getContext('2d');
    ctx512.imageSmoothingEnabled = true;
    ctx512.imageSmoothingQuality = 'high';
    ctx512.drawImage(origBmp, 0, 0, 512, 384);
    const crushedBlob = await new Promise((r) => c512.toBlob(r, 'image/jpeg', 0.45));
    const crushedUrl = URL.createObjectURL(crushedBlob);

    let stage1AtMs = null;
    let stage1Url = null;
    const t0 = performance.now();
    const res512 = await window.__chatwAiRestorer.restoreImageFromUrl(crushedUrl, {
      mediaId: 'spec142_test_512_' + Date.now(),
      forceRun: true,
      onStage1: (u) => {
        stage1AtMs = Math.round(performance.now() - t0);
        stage1Url = u;
      },
    });
    const total512Ms = Math.round(performance.now() - t0);
    const snap512S1 = stage1Url ? await snapshotUrl(stage1Url) : null;
    const snap512S2 = res512 && res512.url ? await snapshotUrl(res512.url) : null;

    // 3. Test Case 2: Pristine 1024x768 Portrait -> 4K (tests multi-tile halo-tiled SPAN 4x!)
    const origUrl = URL.createObjectURL(origBlob);
    let stage1HiMs = null;
    const t1 = performance.now();
    const res1024 = await window.__chatwAiRestorer.restoreImageFromUrl(origUrl, {
      mediaId: 'spec142_test_1024_' + Date.now(),
      forceRun: true,
      onStage1: () => {
        stage1HiMs = Math.round(performance.now() - t1);
      },
    });
    const total1024Ms = Math.round(performance.now() - t1);
    const snap1024S2 = res1024 && res1024.url ? await snapshotUrl(res1024.url) : null;

    origBmp.close();
    return {
      cwspStatus: headResp.status,
      cwspBytes: cwspBuf.byteLength,
      licAllowed,
      res512: {
        enhanced: res512.enhanced,
        modelId: res512.modelId,
        modelLabel: res512.modelLabel,
        backend: res512.backend,
        width: res512.width,
        height: res512.height,
        ssim: res512.ssim,
        matchPct: res512.matchPct,
        stage1AtMs,
        total512Ms,
      },
      res1024: {
        enhanced: res1024.enhanced,
        modelId: res1024.modelId,
        modelLabel: res1024.modelLabel,
        backend: res1024.backend,
        width: res1024.width,
        height: res1024.height,
        ssim: res1024.ssim,
        matchPct: res1024.matchPct,
        stage1HiMs,
        total1024Ms,
      },
      snap512S1,
      snap512S2,
      snap1024S2,
    };
  }, { portraitB64 });

  if (result.snap512S1) {
    fs.writeFileSync(path.join(OUT_DIR, '01_512_stage1_alg_full.jpg'), Buffer.from(result.snap512S1.fullJpg, 'base64'));
  }
  if (result.snap512S2) {
    fs.writeFileSync(path.join(OUT_DIR, '02_512_stage2_clearreality_full.jpg'), Buffer.from(result.snap512S2.fullJpg, 'base64'));
    fs.writeFileSync(path.join(OUT_DIR, '03_512_stage2_clearreality_face_400pct.png'), Buffer.from(result.snap512S2.facePng, 'base64'));
  }
  if (result.snap1024S2) {
    fs.writeFileSync(path.join(OUT_DIR, '04_1024_stage2_clearreality_4k_full.jpg'), Buffer.from(result.snap1024S2.fullJpg, 'base64'));
    fs.writeFileSync(path.join(OUT_DIR, '05_1024_stage2_clearreality_4k_face_400pct.png'), Buffer.from(result.snap1024S2.facePng, 'base64'));
  }

  console.log('SPEC 142 E2E SUMMARY:', JSON.stringify({
    cwspStatus: result.cwspStatus,
    cwspBytes: result.cwspBytes,
    licAllowed: result.licAllowed,
    res512: result.res512,
    res1024: result.res1024,
    pageErrors: errors,
  }, null, 2));

  if (result.cwspStatus !== 200 || result.cwspBytes !== 852596) throw new Error('CWSP1 HTTP fetch mismatch');
  if (!result.licAllowed) throw new Error('License gate rejected clearreality_span_x4');
  if (!result.res512.enhanced || result.res512.modelId !== 'clearreality_span_x4') throw new Error('512 did not run clearreality_span_x4');
  if (result.res512.stage1AtMs === null) throw new Error('Stage 1 algorithmic callback did not fire');
  if (!result.res1024.enhanced || result.res1024.modelId !== 'clearreality_span_x4') throw new Error('1024 did not run clearreality_span_x4');
  if (errors.length > 0) throw new Error('Page errors: ' + errors.join('; '));
  console.log('ALL SPEC 142 ASSERTIONS PASSED!');
} finally {
  await browser.close();
}
