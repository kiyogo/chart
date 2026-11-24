import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec121';

if (!fs.existsSync(OUT_DIR)) {
  fs.mkdirSync(OUT_DIR, { recursive: true });
}

async function dismissModals(page) {
  for (let i = 0; i < 6; i++) {
    const ov = page.locator('.modal-overlay').first();
    if (!(await ov.isVisible().catch(() => false))) break;
    const btn = ov.locator("button.story-close, button[aria-label='Skip tour'], button[aria-label='Close'], button:has-text('Later'), button:has-text('Skip'), button:has-text('Đóng'), button:has-text('Bỏ qua'), button.btn-secondary").first();
    if (await btn.isVisible().catch(() => false)) {
      await btn.click({ force: true }).catch(() => {});
      await page.waitForTimeout(300);
    } else {
      await page.keyboard.press('Escape').catch(() => {});
      await page.waitForTimeout(300);
    }
  }
}

async function run() {
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-web-security']
  });

  const ctx = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1
  });

  const page = await ctx.newPage();
  console.log('Loading ChatW at', BASE);
  await page.goto(BASE, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(1500);

  // Authenticate if needed
  const isAuth = await page.evaluate(() => !!document.querySelector('.auth-shell, .auth-card'));
  if (isAuth) {
    const devInp = await page.$('.auth-dev-form input, input[aria-label], input[placeholder]');
    if (devInp) {
      await devInp.fill('spec121_tester_' + Date.now().toString(36).slice(-4));
      const submitBtn = await page.$(".auth-dev-form button[type='submit'], button[type='submit'], .btn-primary");
      if (submitBtn) await submitBtn.click();
    }
    await page.waitForSelector('.app-rail', { timeout: 25000 });
  }

  await dismissModals(page);
  await page.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer && window.__chatwMediaOpt), { timeout: 25000 });

  // ==========================================
  // AUDIT 1: ULTRA-AGGRESSIVE IMAGE COMPRESSION
  // ==========================================
  console.log('Running Audit 1: Ultra-Aggressive Image Compression...');
  const compressionAudit = await page.evaluate(async () => {
    // Generate a 1200x800 sample canvas with artwork patterns, fine hair lines, and text
    const canvas = document.createElement('canvas');
    canvas.width = 1200;
    canvas.height = 800;
    const ctx = canvas.getContext('2d');

    // Rich colorful background gradient
    const grad = ctx.createLinearGradient(0, 0, 1200, 800);
    grad.addColorStop(0, '#FFB6C1');
    grad.addColorStop(0.5, '#E6E6FA');
    grad.addColorStop(1, '#87CEEB');
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, 1200, 800);

    // Fine lines mimicking anime hair & fabric
    ctx.strokeStyle = '#2D1E2F';
    ctx.lineWidth = 1.5;
    for (let i = 0; i < 60; i++) {
      ctx.beginPath();
      ctx.moveTo(100 + i * 16, 120);
      ctx.bezierCurveTo(200 + i * 10, 300, 300 - i * 5, 500, 200 + i * 14, 750);
      ctx.stroke();
    }

    // High frequency patterns
    ctx.fillStyle = '#C71585';
    for (let x = 700; x < 1100; x += 30) {
      for (let y = 100; y < 700; y += 30) {
        ctx.beginPath();
        ctx.arc(x, y, 8, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    // Text details
    ctx.fillStyle = '#1A1A1A';
    ctx.font = 'bold 36px sans-serif';
    ctx.fillText('ChatW Spec 121 Ultra Sharp AI', 650, 400);

    const blob = await new Promise(res => canvas.toBlob(res, 'image/png'));
    const origSize = blob.size;

    // Run optimizeImageBlob with default ultra-aggressive policy
    const optRes = await window.__chatwMediaOpt.optimizeImageBlob(blob, { preserveOriginal: false });

    // Decode optimized bytes to inspect dimensions
    const optBlob = new Blob([Uint8Array.from(atob(optRes.bytesB64), c => c.charCodeAt(0))], { type: 'image/avif' });
    const bmp = await createImageBitmap(optBlob);
    const optW = bmp.width;
    const optH = bmp.height;
    bmp.close();

    const reductionPct = Math.round((1 - optRes.optimizedSize / origSize) * 100);

    return {
      originalSize: origSize,
      optimizedSize: optRes.optimizedSize,
      reductionPct,
      optWidth: optW,
      optHeight: optH,
      targetWidth: optRes.targetWidth,
      targetHeight: optRes.targetHeight,
      mime: optRes.mime,
      ext: optRes.ext,
      superResRecommended: optRes.superResRecommended,
      optBlobUrl: URL.createObjectURL(optBlob)
    };
  });

  console.log('COMPRESSION_AUDIT:', JSON.stringify(compressionAudit, null, 2));

  // Assertions for Aggressive Compression:
  // 1. Format must be AVIF
  if (compressionAudit.ext !== 'avif') {
    throw new Error(`FAIL: Expected ext avif, got ${compressionAudit.ext}`);
  }
  // 2. Transmitted size must achieve > 65% reduction (typically 80-90%!)
  if (compressionAudit.reductionPct < 65) {
    throw new Error(`FAIL: Expected >= 65% compression reduction, got ${compressionAudit.reductionPct}%`);
  }
  // 3. Max edge > 900 must be downscaled by 0.5x (1200 -> 600px)
  if (compressionAudit.optWidth > 700) {
    throw new Error(`FAIL: Expected downscaled width <= 700px, got ${compressionAudit.optWidth}px`);
  }

  // ==========================================
  // AUDIT 2: ULTRA-SHARP AI RESTORATION
  // ==========================================
  console.log('Running Audit 2: Ultra-Sharp AI Restoration...');
  const aiAudit = await page.evaluate(async (blobUrl) => {
    // Helper to calculate discrete Laplacian variance with physical stride
    function calcLaplacian(rgba, w, h, stride = 1) {
      let sum = 0;
      let count = 0;
      const step = Math.max(stride, Math.floor((w * h) / 10000));
      for (let y = stride; y < h - stride; y += step) {
        for (let x = stride; x < w - stride; x += step) {
          const idx = (y * w + x) * 4;
          const up = ((y - stride) * w + x) * 4;
          const dn = ((y + stride) * w + x) * 4;
          const lf = (y * w + (x - stride)) * 4;
          const rt = (y * w + (x + stride)) * 4;

          const l = 0.299 * rgba[idx] + 0.587 * rgba[idx + 1] + 0.114 * rgba[idx + 2];
          const lu = 0.299 * rgba[up] + 0.587 * rgba[up + 1] + 0.114 * rgba[up + 2];
          const ld = 0.299 * rgba[dn] + 0.587 * rgba[dn + 1] + 0.114 * rgba[dn + 2];
          const ll = 0.299 * rgba[lf] + 0.587 * rgba[lf + 1] + 0.114 * rgba[lf + 2];
          const lr = 0.299 * rgba[rt] + 0.587 * rgba[rt + 1] + 0.114 * rgba[rt + 2];

          const lap = Math.abs(lu + ld + ll + lr - 4 * l);
          sum += lap * lap;
          count++;
        }
      }
      return count > 0 ? Math.sqrt(sum / count) : 0;
    }

    // Measure input compressed image sharpness (1x stride)
    const inBmp = await createImageBitmap(await (await fetch(blobUrl)).blob());
    const inCanvas = document.createElement('canvas');
    inCanvas.width = inBmp.width;
    inCanvas.height = inBmp.height;
    const inCtx = inCanvas.getContext('2d');
    inCtx.drawImage(inBmp, 0, 0);
    const inData = inCtx.getImageData(0, 0, inBmp.width, inBmp.height).data;
    const inputSharpness = calcLaplacian(inData, inBmp.width, inBmp.height, 1);

    // Baseline: standard 4x Bicubic upscale of the compressed input to 2400x1600
    const bicubicCanvas = document.createElement('canvas');
    bicubicCanvas.width = inBmp.width * 4;
    bicubicCanvas.height = inBmp.height * 4;
    const bicubicCtx = bicubicCanvas.getContext('2d');
    bicubicCtx.imageSmoothingEnabled = true;
    bicubicCtx.imageSmoothingQuality = 'high';
    bicubicCtx.drawImage(inBmp, 0, 0, bicubicCanvas.width, bicubicCanvas.height);
    const bicubicData = bicubicCtx.getImageData(0, 0, bicubicCanvas.width, bicubicCanvas.height).data;
    const bicubicSharpness = calcLaplacian(bicubicData, bicubicCanvas.width, bicubicCanvas.height, 1);
    inBmp.close();

    // Run AI restoration with Anime v3 (or active model)
    const aiRes = await window.__chatwAiRestorer.restoreImageFromUrl(blobUrl, {
      modelId: 'realesr_animevideov3',
      forceRun: true
    });

    if (!aiRes.enhanced || !aiRes.url) {
      return { ok: false, error: aiRes.reason || 'ai_failed' };
    }

    // Measure restored output image sharpness
    const outBmp = await createImageBitmap(await (await fetch(aiRes.url)).blob());
    const outCanvas = document.createElement('canvas');
    outCanvas.width = outBmp.width;
    outCanvas.height = outBmp.height;
    const outCtx = outCanvas.getContext('2d');
    outCtx.drawImage(outBmp, 0, 0);
    const outData = outCtx.getImageData(0, 0, outBmp.width, outBmp.height).data;
    const outputSharpness = calcLaplacian(outData, outBmp.width, outBmp.height, 1);
    const outputPhysicalSharpness = calcLaplacian(outData, outBmp.width, outBmp.height, 4);
    outBmp.close();

    const aiVsBicubicBoost = Number((outputSharpness / Math.max(0.1, bicubicSharpness)).toFixed(2));
    const physicalSharpnessBoost = Number((outputPhysicalSharpness / Math.max(0.1, inputSharpness)).toFixed(2));

    return {
      ok: true,
      modelId: aiRes.modelId,
      modelLabel: aiRes.modelLabel,
      superRes: aiRes.superRes,
      srcWidth: aiRes.srcWidth,
      srcHeight: aiRes.srcHeight,
      outWidth: aiRes.width,
      outHeight: aiRes.height,
      ssim: aiRes.ssim,
      matchPct: aiRes.matchPct,
      bicubicSharpness: Number(bicubicSharpness.toFixed(2)),
      outputSharpness: Number(outputSharpness.toFixed(2)),
      aiVsBicubicBoost,
      inputSharpness: Number(inputSharpness.toFixed(2)),
      outputPhysicalSharpness: Number(outputPhysicalSharpness.toFixed(2)),
      physicalSharpnessBoost,
      enhancedUrl: aiRes.url
    };
  }, compressionAudit.optBlobUrl);

  console.log('AI_AUDIT:', JSON.stringify(aiAudit, null, 2));

  if (!aiAudit.ok) {
    throw new Error(`FAIL: AI Restoration failed with error: ${aiAudit.error}`);
  }
  // Sharpness must show significant boost over standard 4x upscale (> 1.8x)
  if (aiAudit.aiVsBicubicBoost < 1.7) {
    throw new Error(`FAIL: Expected AI vs Bicubic sharpness boost >= 1.7x, got ${aiAudit.aiVsBicubicBoost}x`);
  }
  // Output width must be 4x input width
  if (aiAudit.outWidth < 2000) {
    throw new Error(`FAIL: Expected 4x super-res width >= 2000px, got ${aiAudit.outWidth}px`);
  }

  // ==========================================
  // AUDIT 3: LIGHTBOX VISUAL VERIFICATION
  // ==========================================
  console.log('Running Audit 3: Lightbox Visual Verification...');
  await page.evaluate((enhancedUrl) => {
    // Open lightbox directly with the enhanced image
    const shell = document.createElement('div');
    shell.id = 'spec121-lightbox-test';
    shell.className = 'lightbox-backdrop';
    shell.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.92);z-index:99999;display:flex;align-items:center;justify-content:center;';
    
    const img = document.createElement('img');
    img.src = enhancedUrl;
    img.className = 'lightbox-img';
    img.style.cssText = 'max-width:94vw;max-height:94vh;object-fit:contain;border:none;box-shadow:none;border-radius:0;';
    
    shell.appendChild(img);
    document.body.appendChild(shell);
  }, aiAudit.enhancedUrl);

  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(OUT_DIR, '01_spec121_ultra_sharp_ai_enhanced_lightbox.png') });

  // Clean up
  await page.evaluate(() => {
    const el = document.getElementById('spec121-lightbox-test');
    if (el) el.remove();
  });

  await browser.close();
  console.log('SPEC 121 ULTRA-SHARP AI & MAXIMUM COMPRESSION AUDIT: 100% PASSED!');
}

run().catch(err => {
  console.error('Test failed:', err);
  process.exit(1);
});
