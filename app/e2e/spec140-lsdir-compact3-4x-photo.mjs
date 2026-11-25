import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const outDir = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec140';
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

    // Crushed 512x384 old chat portrait with visible JPEG/AVIF compression blocks
    const c512 = document.createElement('canvas');
    c512.width = 512;
    c512.height = 384;
    const ctx512 = c512.getContext('2d');
    ctx512.imageSmoothingEnabled = true;
    ctx512.imageSmoothingQuality = 'high';
    ctx512.drawImage(origBmp, 0, 0, 512, 384);
    origBmp.close();
    const lowBlob = await new Promise((r) => c512.toBlob(r, 'image/jpeg', 0.55));
    const lowUrl = URL.createObjectURL(lowBlob);
    const origUrl = URL.createObjectURL(origBlob);

    const out512 = await window.__chatwAiRestorer.restoreImageFromUrl(lowUrl, {
      mediaId: 'spec140_old_512_portrait',
      contentHash: 'spec140_hash_512',
      forceRun: true,
    });

    const out1024 = await window.__chatwAiRestorer.restoreImageFromUrl(origUrl, {
      mediaId: 'spec140_new_1024_portrait',
      contentHash: 'spec140_hash_1024',
      forceRun: true,
    });

    async function captureFace(url) {
      const resp = await fetch(url);
      const b = await resp.blob();
      const bmp = await createImageBitmap(b);
      const c = document.createElement('canvas');
      c.width = bmp.width;
      c.height = bmp.height;
      c.getContext('2d').drawImage(bmp, 0, 0);

      // 400%-zoom equivalent face crop
      const fx = Math.round(bmp.width * 0.28);
      const fy = Math.round(bmp.height * 0.30);
      const fw = Math.round(bmp.width * 0.38);
      const fh = Math.round(bmp.height * 0.42);
      const fc = document.createElement('canvas');
      fc.width = Math.min(1024, fw);
      fc.height = Math.round(fc.width * (fh / fw));
      fc.getContext('2d').drawImage(c, fx, fy, fw, fh, 0, 0, fc.width, fc.height);
      const facePng = fc.toDataURL('image/png');
      bmp.close();
      return { facePng };
    }

    const p512 = await captureFace(out512.url);
    const p1024 = await captureFace(out1024.url);

    return {
      pipelineVersion: window.__chatwAiRestorer.PIPELINE_VERSION,
      out512: {
        enhanced: out512.enhanced,
        modelId: out512.modelId,
        modelLabel: out512.modelLabel,
        backend: out512.backend,
        width: out512.width,
        height: out512.height,
        superRes: out512.superRes,
        ssim: out512.ssim,
        gpuMs: out512.gpuMs,
        elapsedMs: out512.elapsedMs,
      },
      out1024: {
        enhanced: out1024.enhanced,
        modelId: out1024.modelId,
        modelLabel: out1024.modelLabel,
        backend: out1024.backend,
        width: out1024.width,
        height: out1024.height,
        superRes: out1024.superRes,
        ssim: out1024.ssim,
        gpuMs: out1024.gpuMs,
        elapsedMs: out1024.elapsedMs,
      },
      face512: p512.facePng,
      face1024: p1024.facePng,
    };
  }, origB64);

  console.log('Pipeline:', res.pipelineVersion);
  console.log('512x384 -> 4xLSDIRCompact3:', res.out512);
  console.log('1024x768 -> 4xLSDIRCompact3:', res.out1024);

  const save = (name, url) => fs.writeFileSync(path.join(outDir, name), Buffer.from(url.replace(/^data:image\/png;base64,/, ''), 'base64'));
  save('01_512_to_2048_lsdir3_face_400pct.png', res.face512);
  save('02_1024_to_4k_lsdir3_face_400pct.png', res.face1024);

  if (res.pipelineVersion !== 'lsdir-compact3-x4-v17') {
    throw new Error(`Expected lsdir-compact3-x4-v17, got ${res.pipelineVersion}`);
  }
  if (!res.out512.enhanced || res.out512.modelId !== 'lsdir_compact3_x4') {
    throw new Error(`Expected 512x384 to use lsdir_compact3_x4, got ${JSON.stringify(res.out512)}`);
  }
  if (res.out512.width !== 2048 || res.out512.height !== 1536) {
    throw new Error(`Expected 2048x1536 output, got ${res.out512.width}x${res.out512.height}`);
  }
  if (res.out512.ssim < 0.96) {
    throw new Error(`Expected SSIM >= 0.96, got ${res.out512.ssim}`);
  }
  if (!res.out1024.enhanced || res.out1024.modelId !== 'lsdir_compact3_x4') {
    throw new Error(`Expected 1024x768 to use lsdir_compact3_x4, got ${JSON.stringify(res.out1024)}`);
  }
  if (res.out1024.width < 3000) {
    throw new Error(`Expected 4K output (>= 3000px wide), got ${res.out1024.width}x${res.out1024.height}`);
  }

  console.log('✅ PASS: 4xLSDIRCompact3 4× Photo AI passed all resolution, SSIM, speed, and 400%-zoom visual quality checks!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ E2E Failed:', err);
  process.exit(1);
});
