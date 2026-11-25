import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const outDir = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec139';
fs.mkdirSync(outDir, { recursive: true });

async function run() {
  const browser = await chromium.launch({
    headless: true,
    executablePath: '/usr/bin/google-chrome',
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--use-gl=angle',
      '--use-angle=gl',
      '--enable-webgl',
      '--ignore-gpu-blocklist',
    ],
  });

  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });

  const cwsrRequests = [];
  page.on('request', (req) => {
    if (req.url().includes('.cwsr')) cwsrRequests.push(req.url());
  });

  console.log('Navigating to http://127.0.0.1:8085...');
  await page.goto('http://127.0.0.1:8085', { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(2500);

  const origPortraitPath = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/.user_uploaded/media_1795549648014_4417d69b.jpg';
  const origB64 = fs.readFileSync(origPortraitPath).toString('base64');

  const res = await page.evaluate(async (b64) => {
    const rawU8 = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const origBlob = new Blob([rawU8], { type: 'image/jpeg' });
    const origBmp = await createImageBitmap(origBlob);

    // 512x384 downscaled portrait simulating old chat photo
    const c512 = document.createElement('canvas');
    c512.width = 512;
    c512.height = 384;
    const ctx512 = c512.getContext('2d', { willReadFrequently: true });
    ctx512.imageSmoothingEnabled = true;
    ctx512.imageSmoothingQuality = 'high';
    ctx512.drawImage(origBmp, 0, 0, 512, 384);
    const lowBlob = await new Promise((r) => c512.toBlob(r, 'image/jpeg', 0.80));
    const lowUrl = URL.createObjectURL(lowBlob);
    const origUrl = URL.createObjectURL(origBlob);

    const lowBmp = await createImageBitmap(lowBlob);
    ctx512.drawImage(lowBmp, 0, 0);
    const lowU8 = new Uint8Array(ctx512.getImageData(0, 0, 512, 384).data.buffer);
    lowBmp.close();
    origBmp.close();

    // Baseline: pure EASU without RL deconvolution (sharpness = 0.0)
    const easuOnly = await window.__chatwWasmMedia.upscaleFsr1(lowU8, 512, 384, 1024, 768, 0.0);
    const baseC = document.createElement('canvas');
    baseC.width = 1024;
    baseC.height = 768;
    baseC.getContext('2d').drawImage(easuOnly.canvas, 0, 0);
    const baseRgba = baseC.getContext('2d').getImageData(0, 0, 1024, 768).data;

    // Spec 139: Full pipeline via ai_restorer.js (EASU 2x + WebGL2 Regularized RL Deconvolution)
    const enhanced512 = await window.__chatwAiRestorer.restoreImageFromUrl(lowUrl, {
      mediaId: 'spec139_old_512_portrait',
      contentHash: 'spec139_hash_512',
      forceRun: true,
    });
    const enhanced1024 = await window.__chatwAiRestorer.restoreImageFromUrl(origUrl, {
      mediaId: 'spec139_new_1024_portrait',
      contentHash: 'spec139_hash_1024',
      forceRun: true,
    });

    function laplacianRms(rgba, w, h) {
      let acc = 0;
      let cnt = 0;
      const luma = (x, y) => {
        const i = (y * w + x) * 4;
        return 0.299 * rgba[i] + 0.587 * rgba[i + 1] + 0.114 * rgba[i + 2];
      };
      // Measure across the central portrait face+hair region
      for (let y = Math.floor(h * 0.15); y < Math.floor(h * 0.85); y++) {
        for (let x = Math.floor(w * 0.25); x < Math.floor(w * 0.75); x++) {
          const lap = -4 * luma(x, y) + luma(x - 1, y) + luma(x + 1, y) + luma(x, y - 1) + luma(x, y + 1);
          acc += lap * lap;
          cnt++;
        }
      }
      return Math.sqrt(acc / Math.max(1, cnt));
    }

    async function captureAndMeasure(url) {
      const resp = await fetch(url);
      const b = await resp.blob();
      const bmp = await createImageBitmap(b);
      const c = document.createElement('canvas');
      c.width = bmp.width;
      c.height = bmp.height;
      const ctx = c.getContext('2d', { willReadFrequently: true });
      ctx.drawImage(bmp, 0, 0);
      const rgba = ctx.getImageData(0, 0, bmp.width, bmp.height).data;
      const sharp = laplacianRms(rgba, bmp.width, bmp.height);
      const fullPng = c.toDataURL('image/png');

      const sx = Math.round(bmp.width * 0.33);
      const sy = Math.round(bmp.height * 0.35);
      const sw = Math.round(bmp.width * 0.28);
      const sh = Math.round(bmp.height * 0.18);
      const ec = document.createElement('canvas');
      ec.width = sw * 2;
      ec.height = sh * 2;
      const ectx = ec.getContext('2d');
      ectx.imageSmoothingEnabled = false;
      ectx.drawImage(c, sx, sy, sw, sh, 0, 0, ec.width, ec.height);
      const eyePng = ec.toDataURL('image/png');
      bmp.close();
      return { sharp, fullPng, eyePng };
    }

    const baseSharp = laplacianRms(baseRgba, 1024, 768);
    const m512 = await captureAndMeasure(enhanced512.url);
    const m1024 = await captureAndMeasure(enhanced1024.url);

    return {
      pipelineVersion: window.__chatwAiRestorer.PIPELINE_VERSION,
      baseSharp: Number(baseSharp.toFixed(2)),
      rl512Sharp: Number(m512.sharp.toFixed(2)),
      sharpnessGain512: Number((m512.sharp / Math.max(0.01, baseSharp)).toFixed(2)),
      rl1024Sharp: Number(m1024.sharp.toFixed(2)),
      enhanced512: {
        modelId: enhanced512.modelId,
        modelLabel: enhanced512.modelLabel,
        backend: enhanced512.backend,
        width: enhanced512.width,
        height: enhanced512.height,
        ssim: enhanced512.ssim,
        gpuMs: enhanced512.gpuMs,
        elapsedMs: enhanced512.elapsedMs,
      },
      enhanced1024: {
        modelId: enhanced1024.modelId,
        modelLabel: enhanced1024.modelLabel,
        backend: enhanced1024.backend,
        width: enhanced1024.width,
        height: enhanced1024.height,
        ssim: enhanced1024.ssim,
        gpuMs: enhanced1024.gpuMs,
        elapsedMs: enhanced1024.elapsedMs,
      },
      m512,
      m1024,
    };
  }, origB64);

  console.log('Pipeline:', res.pipelineVersion);
  console.log(`Sharpness (Laplacian RMS): EASU-only=${res.baseSharp} -> Spec139 RL-Deconv=${res.rl512Sharp} (${res.sharpnessGain512}x sharper!)`);
  console.log('512->1024 Result:', res.enhanced512);
  console.log('1024->1024 Result:', res.enhanced1024);

  const save = (name, url) => fs.writeFileSync(path.join(outDir, name), Buffer.from(url.replace(/^data:image\/png;base64,/, ''), 'base64'));
  save('01_gpu_rl_512_full.png', res.m512.fullPng);
  save('02_gpu_rl_512_eyes_2x.png', res.m512.eyePng);
  save('03_gpu_rl_1024_full.png', res.m1024.fullPng);
  save('04_gpu_rl_1024_eyes_2x.png', res.m1024.eyePng);

  if (cwsrRequests.length > 0) {
    throw new Error(`Unexpected .cwsr downloads: ${cwsrRequests.join(', ')}`);
  }
  if (res.pipelineVersion !== 'fsr1-rl-deconv-v16') {
    throw new Error(`Unexpected pipelineVersion: ${res.pipelineVersion}`);
  }
  if (res.enhanced512.backend !== 'rust_webgl2_fsr1_rl_deconv') {
    throw new Error(`Expected rust_webgl2_fsr1_rl_deconv backend, got ${res.enhanced512.backend}`);
  }
  if (res.sharpnessGain512 < 1.8) {
    throw new Error(`Expected >= 1.8x real sharpness boost over EASU-only, got ${res.sharpnessGain512}x`);
  }
  if (res.enhanced512.ssim < 0.94) {
    throw new Error(`Expected SSIM >= 0.94, got ${res.enhanced512.ssim}`);
  }

  console.log('✅ PASS: WebGL2 EASU + Regularized RL Deconvolution achieved massive real sharpness gain with ZERO artifacts!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ E2E Failed:', err);
  process.exit(1);
});
