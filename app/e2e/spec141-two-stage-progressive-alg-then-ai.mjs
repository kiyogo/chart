import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const outDir = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec141';
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
  console.log('Navigating to http://127.0.0.1:8085...');
  await page.goto('http://127.0.0.1:8085', { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(2500);

  const origPortraitPath = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/.user_uploaded/media_1795549648014_4417d69b.jpg';
  const origB64 = fs.readFileSync(origPortraitPath).toString('base64');

  const res = await page.evaluate(async (b64) => {
    const rawU8 = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const origBlob = new Blob([rawU8], { type: 'image/jpeg' });
    const origBmp = await createImageBitmap(origBlob);

    const c512 = document.createElement('canvas');
    c512.width = 512;
    c512.height = 384;
    const ctx512 = c512.getContext('2d');
    ctx512.imageSmoothingEnabled = true;
    ctx512.imageSmoothingQuality = 'high';
    ctx512.drawImage(origBmp, 0, 0, 512, 384);
    origBmp.close();
    const lowBlob = await new Promise((r) => c512.toBlob(r, 'image/jpeg', 0.65));
    const lowUrl = URL.createObjectURL(lowBlob);

    const tStart = performance.now();
    let stage1Url = null;
    let stage1Ms = null;
    const progressEvents = [];

    const finalOut = await window.__chatwAiRestorer.restoreImageFromUrl(lowUrl, {
      mediaId: 'spec141_two_stage_512',
      contentHash: 'spec141_hash_512',
      forceRun: true,
      onStage1: (u) => {
        stage1Url = u;
        stage1Ms = Math.round(performance.now() - tStart);
      },
      onProgress: (p) => progressEvents.push(p),
    });

    async function captureFullAndFace(url) {
      const resp = await fetch(url);
      const b = await resp.blob();
      const bmp = await createImageBitmap(b);
      const c = document.createElement('canvas');
      c.width = bmp.width;
      c.height = bmp.height;
      c.getContext('2d').drawImage(bmp, 0, 0);
      const fullPng = c.toDataURL('image/png');

      const fx = Math.round(bmp.width * 0.28);
      const fy = Math.round(bmp.height * 0.30);
      const fw = Math.round(bmp.width * 0.38);
      const fh = Math.round(bmp.height * 0.42);
      const fc = document.createElement('canvas');
      fc.width = fw;
      fc.height = fh;
      fc.getContext('2d').drawImage(c, fx, fy, fw, fh, 0, 0, fw, fh);
      const facePng = fc.toDataURL('image/png');
      const dims = { w: bmp.width, h: bmp.height };
      bmp.close();
      return { dims, fullPng, facePng };
    }

    const s1Cap = stage1Url ? await captureFullAndFace(stage1Url) : null;
    const s2Cap = finalOut?.url ? await captureFullAndFace(finalOut.url) : null;

    return {
      pipelineVersion: window.__chatwAiRestorer.PIPELINE_VERSION,
      stage1Fired: Boolean(stage1Url),
      stage1Ms,
      stage1Dims: s1Cap?.dims || null,
      stage2: {
        enhanced: finalOut.enhanced,
        modelId: finalOut.modelId,
        modelLabel: finalOut.modelLabel,
        width: finalOut.width,
        height: finalOut.height,
        ssim: finalOut.ssim,
        gpuMs: finalOut.gpuMs,
        elapsedMs: finalOut.elapsedMs,
      },
      progressEvents,
      s1Full: s1Cap?.fullPng || null,
      s2Full: s2Cap?.fullPng || null,
      s2Face: s2Cap?.facePng || null,
    };
  }, origB64);

  console.log('Pipeline:', res.pipelineVersion);
  console.log('Stage 1 (Algorithmic First):', { fired: res.stage1Fired, ms: res.stage1Ms, dims: res.stage1Dims });
  console.log('Stage 2 (AI 4x + Post-AI Optical RL):', res.stage2);

  const save = (n, u) => fs.writeFileSync(path.join(outDir, n), Buffer.from(u.replace(/^data:image\/png;base64,/, ''), 'base64'));
  if (res.s1Full) save('01_stage1_algorithmic_full.png', res.s1Full);
  if (res.s2Full) save('02_stage2_ai_plus_rl_full.png', res.s2Full);
  if (res.s2Face) save('03_stage2_ai_plus_rl_face_400pct.png', res.s2Face);

  if (res.pipelineVersion !== 'progressive-alg-then-lsdir3-rl-v18') {
    throw new Error(`Expected progressive-alg-then-lsdir3-rl-v18, got ${res.pipelineVersion}`);
  }
  if (!res.stage1Fired || !res.stage1Dims || res.stage1Dims.w !== 1024) {
    throw new Error(`Stage 1 Algorithmic failed to fire properly: ${JSON.stringify(res.stage1Dims)}`);
  }
  if (!res.stage2.enhanced || res.stage2.modelId !== 'lsdir_compact3_x4' || !res.stage2.modelLabel.includes('Optical RL')) {
    throw new Error(`Stage 2 AI + Optical RL failed: ${JSON.stringify(res.stage2)}`);
  }
  if (res.stage2.width !== 2048 || res.stage2.height !== 1536) {
    throw new Error(`Expected Stage 2 2048x1536, got ${res.stage2.width}x${res.stage2.height}`);
  }
  if (res.stage2.ssim < 0.98) {
    throw new Error(`Expected SSIM >= 0.98, got ${res.stage2.ssim}`);
  }

  console.log('✅ PASS: 2-Stage Progressive Pipeline (Algorithmic First -> 4× AI + Optical RL Clarity) verified end-to-end!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ E2E Failed:', err);
  process.exit(1);
});
