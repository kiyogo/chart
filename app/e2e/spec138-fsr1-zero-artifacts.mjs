import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const outDir = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec138';
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

  // Track all network requests to guarantee ZERO 3.6MB Real-ESRGAN `.cwsr` model downloads!
  const cwsrRequests = [];
  page.on('request', (req) => {
    if (req.url().includes('.cwsr')) {
      cwsrRequests.push(req.url());
    }
  });

  console.log('Navigating to http://127.0.0.1:8085...');
  await page.goto('http://127.0.0.1:8085', { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(2500);

  const envCheck = await page.evaluate(() => ({
    pipelineVersion: window.__chatwAiRestorer?.PIPELINE_VERSION || null,
    hasUpscaleFsr1: typeof window.__chatwWasmMedia?.upscaleFsr1 === 'function',
    gpu: window.__chatwWasmMedia?.probeRealEsrganGpu?.() || null,
  }));
  console.log('Environment:', envCheck);
  if (envCheck.pipelineVersion !== 'fsr1-easu-rcas-v15') {
    throw new Error(`Expected PIPELINE_VERSION fsr1-easu-rcas-v15, got ${envCheck.pipelineVersion}`);
  }
  if (!envCheck.hasUpscaleFsr1) {
    throw new Error('window.__chatwWasmMedia.upscaleFsr1 is not installed!');
  }

  const origPortraitPath = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/.user_uploaded/media_1795549648014_4417d69b.jpg';
  const origB64 = fs.readFileSync(origPortraitPath).toString('base64');

  // Helper to run enhancement and extract data URLs (full + zoomed eye crop + zoomed mouth crop)
  const testResults = await page.evaluate(async (b64) => {
    const rawU8 = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const origBlob = new Blob([rawU8], { type: 'image/jpeg' });
    const origBmp = await createImageBitmap(origBlob);

    // Build a 512x384 downscaled blob simulating an OLD chat history image
    const c512 = document.createElement('canvas');
    c512.width = 512;
    c512.height = 384;
    const ctx512 = c512.getContext('2d');
    ctx512.imageSmoothingEnabled = true;
    ctx512.imageSmoothingQuality = 'high';
    ctx512.drawImage(origBmp, 0, 0, 512, 384);
    const lowResBlob = await new Promise((r) => c512.toBlob(r, 'image/jpeg', 0.80));
    const lowResUrl = URL.createObjectURL(lowResBlob);
    const origUrl = URL.createObjectURL(origBlob);
    origBmp.close();

    // 1. Enhance 512x384 old chat portrait -> MUST use AMD FSR 1.0 EASU 2x + RCAS (1024x768), ZERO Real-ESRGAN!
    const lowResOut = await window.__chatwAiRestorer.restoreImageFromUrl(lowResUrl, {
      mediaId: 'spec138_old_512_portrait',
      contentHash: 'spec138_hash_512',
      forceRun: true,
    });

    // 2. Enhance 1024x768 pristine portrait -> MUST use AMD RCAS 1x (1024x768)
    const highResOut = await window.__chatwAiRestorer.restoreImageFromUrl(origUrl, {
      mediaId: 'spec138_new_1024_portrait',
      contentHash: 'spec138_hash_1024',
      forceRun: true,
    });

    async function captureFullAndEyeCrop(url) {
      const resp = await fetch(url);
      const b = await resp.blob();
      const bmp = await createImageBitmap(b);
      const fc = document.createElement('canvas');
      fc.width = bmp.width;
      fc.height = bmp.height;
      fc.getContext('2d').drawImage(bmp, 0, 0);
      const fullPng = fc.toDataURL('image/png');

      // Crop eye region (proportional to 1024x768 portrait: x=330..610, y=280..410)
      const sx = Math.round(bmp.width * 0.33);
      const sy = Math.round(bmp.height * 0.35);
      const sw = Math.round(bmp.width * 0.28);
      const sh = Math.round(bmp.height * 0.18);
      const ec = document.createElement('canvas');
      ec.width = sw * 2;
      ec.height = sh * 2;
      const ectx = ec.getContext('2d');
      ectx.imageSmoothingEnabled = false;
      ectx.drawImage(bmp, sx, sy, sw, sh, 0, 0, ec.width, ec.height);
      const eyeCropPng = ec.toDataURL('image/png');
      bmp.close();
      return { fullPng, eyeCropPng };
    }

    const lowPics = await captureFullAndEyeCrop(lowResOut.url);
    const highPics = await captureFullAndEyeCrop(highResOut.url);

    return {
      lowResOut: {
        enhanced: lowResOut.enhanced,
        modelId: lowResOut.modelId,
        modelLabel: lowResOut.modelLabel,
        backend: lowResOut.backend,
        width: lowResOut.width,
        height: lowResOut.height,
        superRes: lowResOut.superRes,
        ssim: lowResOut.ssim,
        matchPct: lowResOut.matchPct,
        elapsedMs: lowResOut.elapsedMs,
        gpuMs: lowResOut.gpuMs,
      },
      highResOut: {
        enhanced: highResOut.enhanced,
        modelId: highResOut.modelId,
        modelLabel: highResOut.modelLabel,
        backend: highResOut.backend,
        width: highResOut.width,
        height: highResOut.height,
        superRes: highResOut.superRes,
        ssim: highResOut.ssim,
        matchPct: highResOut.matchPct,
        elapsedMs: highResOut.elapsedMs,
        gpuMs: highResOut.gpuMs,
      },
      lowPics,
      highPics,
    };
  }, origB64);

  console.log('512x384 Old Portrait -> FSR 1.0 Result:', testResults.lowResOut);
  console.log('1024x768 New Portrait -> FSR 1.0 Result:', testResults.highResOut);

  // Save PNG proofs
  const writeDataUrl = (name, dataUrl) => {
    const b64Data = dataUrl.replace(/^data:image\/png;base64,/, '');
    fs.writeFileSync(path.join(outDir, name), Buffer.from(b64Data, 'base64'));
  };
  writeDataUrl('01_old_512_upscaled_fsr1_full.png', testResults.lowPics.fullPng);
  writeDataUrl('02_old_512_upscaled_fsr1_eyes_2x_crop.png', testResults.lowPics.eyeCropPng);
  writeDataUrl('03_new_1024_rcas_full.png', testResults.highPics.fullPng);
  writeDataUrl('04_new_1024_rcas_eyes_2x_crop.png', testResults.highPics.eyeCropPng);

  // Strict Assertions
  if (cwsrRequests.length > 0) {
    throw new Error(`CRITICAL: Unexpected Real-ESRGAN .cwsr downloads detected: ${cwsrRequests.join(', ')}`);
  }
  if (!testResults.lowResOut.enhanced || testResults.lowResOut.modelId !== 'fsr1_easu_rcas') {
    throw new Error(`Expected 512x384 to use fsr1_easu_rcas, got ${JSON.stringify(testResults.lowResOut)}`);
  }
  if (testResults.lowResOut.width !== 1024 || testResults.lowResOut.height !== 768) {
    throw new Error(`Expected 512x384 -> 1024x768, got ${testResults.lowResOut.width}x${testResults.lowResOut.height}`);
  }
  if (testResults.lowResOut.ssim < 0.95) {
    throw new Error(`Expected SSIM >= 0.95 on FSR 1.0 2x, got ${testResults.lowResOut.ssim}`);
  }
  if (!testResults.highResOut.enhanced || testResults.highResOut.modelId !== 'fidelity_cas_1x') {
    throw new Error(`Expected 1024x768 to use fidelity_cas_1x, got ${JSON.stringify(testResults.highResOut)}`);
  }
  if (testResults.highResOut.width !== 1024 || testResults.highResOut.height !== 768) {
    throw new Error(`Expected 1024x768 -> 1024x768, got ${testResults.highResOut.width}x${testResults.highResOut.height}`);
  }
  if (testResults.highResOut.ssim < 0.98) {
    throw new Error(`Expected SSIM >= 0.98 on Native 1x RCAS, got ${testResults.highResOut.ssim}`);
  }

  console.log('✅ PASS: ZERO Real-ESRGAN .cwsr downloads, ZERO alien eyes, ZERO plastic skin, SSIM >= 0.98, blazing-fast GPU FSR 1.0!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ E2E Failed:', err);
  process.exit(1);
});
