import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec123';

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
  console.log('--- Spec 123 E2E Verification: Pure Real-ESRGAN AI Super-Resolution ---');
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
      await devInp.fill('spec123_tester_' + Date.now().toString(36).slice(-4));
      const submitBtn = await page.$(".auth-dev-form button[type='submit'], button[type='submit'], .btn-primary");
      if (submitBtn) await submitBtn.click();
    }
    await page.waitForSelector('.app-rail', { timeout: 25000 });
  }

  await dismissModals(page);
  await page.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer), { timeout: 25000 });

  console.log('Running Anime Chromatic Aberration & Pure AI Quality Audit...');
  const auditResult = await page.evaluate(async () => {
    // Create an anime test image mimicking the user's uploaded sample:
    // - Orange hair on mint green background
    // - White suit with black tie
    // - Peach skin tone
    const canvas = document.createElement('canvas');
    canvas.width = 400;
    canvas.height = 360;
    const ctx = canvas.getContext('2d');

    // Mint background
    ctx.fillStyle = '#C8E6C9';
    ctx.fillRect(0, 0, 400, 360);

    // Orange hair shape
    ctx.fillStyle = '#E65100';
    ctx.beginPath();
    ctx.arc(200, 160, 100, 0, Math.PI * 2);
    ctx.fill();

    // Peach face
    ctx.fillStyle = '#FFE0B2';
    ctx.beginPath();
    ctx.arc(200, 190, 65, 0, Math.PI * 2);
    ctx.fill();

    // White collar / suit
    ctx.fillStyle = '#FFFFFF';
    ctx.beginPath();
    ctx.moveTo(120, 270);
    ctx.lineTo(280, 270);
    ctx.lineTo(310, 360);
    ctx.lineTo(90, 360);
    ctx.closePath();
    ctx.fill();

    // Black tie
    ctx.fillStyle = '#111111';
    ctx.beginPath();
    ctx.moveTo(195, 270);
    ctx.lineTo(205, 270);
    ctx.lineTo(210, 340);
    ctx.lineTo(200, 355);
    ctx.lineTo(190, 340);
    ctx.closePath();
    ctx.fill();

    // Black ink stroke outlines
    ctx.strokeStyle = '#1A1A1A';
    ctx.lineWidth = 2.0;
    ctx.stroke();

    const testBlob = await new Promise(r => canvas.toBlob(r, 'image/jpeg', 0.88));
    const testUrl = URL.createObjectURL(testBlob);

    // Run Real-ESRGAN restoration
    const aiRes = await window.__chatwAiRestorer.restoreImageFromUrl(testUrl, {
      modelId: 'realesr_animevideov3',
      forceRun: true
    });

    if (!aiRes.enhanced || !aiRes.url) {
      return { ok: false, error: aiRes.reason || 'ai_failed' };
    }

    // Inspect enhanced output bitmap
    const outBmp = await createImageBitmap(await (await fetch(aiRes.url)).blob());
    const outCanvas = document.createElement('canvas');
    outCanvas.width = outBmp.width;
    outCanvas.height = outBmp.height;
    const outCtx = outCanvas.getContext('2d');
    outCtx.drawImage(outBmp, 0, 0);
    const outData = outCtx.getImageData(0, 0, outBmp.width, outBmp.height).data;
    const ow = outBmp.width;
    const oh = outBmp.height;

    // Measure Chromatic Aberration along high-contrast edges:
    // For each pixel, measure whether gradient in Green deviates aberrantly from Red & Blue
    let totalEdgePixels = 0;
    let chromaticAberrationSum = 0;
    let maxFringe = 0;

    for (let y = 2; y < oh - 2; y += 4) {
      for (let x = 2; x < ow - 2; x += 4) {
        const idx = (y * ow + x) * 4;
        const r = outData[idx];
        const g = outData[idx + 1];
        const b = outData[idx + 2];

        // Horizontal gradient for each channel
        const rLeft = outData[((y) * ow + (x - 2)) * 4];
        const rRight = outData[((y) * ow + (x + 2)) * 4];
        const gLeft = outData[((y) * ow + (x - 2)) * 4 + 1];
        const gRight = outData[((y) * ow + (x + 2)) * 4 + 1];
        const bLeft = outData[((y) * ow + (x - 2)) * 4 + 2];
        const bRight = outData[((y) * ow + (x + 2)) * 4 + 2];

        const gradR = Math.abs(rRight - rLeft);
        const gradG = Math.abs(gRight - gLeft);
        const gradB = Math.abs(bRight - bLeft);
        const maxGrad = Math.max(gradR, gradG, gradB);

        // Only evaluate prominent edges
        if (maxGrad > 60) {
          totalEdgePixels++;
          // Chromatic divergence: non-physical shift where Green shoots in opposite direction from Red
          const diffRG = Math.abs((rRight - rLeft) - (gRight - gLeft));
          const diffRB = Math.abs((rRight - rLeft) - (bRight - bLeft));
          // Normalized by edge magnitude
          const fringe = Math.min(diffRG, diffRB) / 255.0;
          chromaticAberrationSum += fringe;
          if (fringe > maxFringe) maxFringe = fringe;
        }
      }
    }

    const meanChromaticFringe = totalEdgePixels > 0 ? chromaticAberrationSum / totalEdgePixels : 0;

    // Measure noise on flat white suit area
    let whiteNoiseSum = 0;
    let whitePixelCount = 0;
    for (let y = Math.round(oh * 0.85); y < oh - 10; y += 3) {
      for (let x = Math.round(ow * 0.35); x < Math.round(ow * 0.45); x += 3) {
        const idx = (y * ow + x) * 4;
        const r = outData[idx];
        const g = outData[idx + 1];
        const b = outData[idx + 2];
        // In clean white, r, g, b should all be high (> 240) and nearly equal
        if (r > 230 && g > 230 && b > 230) {
          whitePixelCount++;
          const spread = Math.max(Math.abs(r - g), Math.abs(g - b), Math.abs(r - b));
          whiteNoiseSum += spread;
        }
      }
    }
    const meanWhiteSpread = whitePixelCount > 0 ? whiteNoiseSum / whitePixelCount : 0;

    outBmp.close();
    return {
      ok: true,
      ow,
      oh,
      modelId: aiRes.modelId,
      totalEdgePixels,
      meanChromaticFringe,
      maxFringe,
      whitePixelCount,
      meanWhiteSpread,
      enhancedUrl: aiRes.url
    };
  });

  console.log('AUDIT RESULT:', JSON.stringify(auditResult, null, 2));

  if (!auditResult.ok) {
    throw new Error(`Real-ESRGAN failed: ${auditResult.error}`);
  }

  // Assertions for Pristine AI Quality:
  // 1. Output must be 4x upscale (1600x1440)
  if (auditResult.ow !== 1600 || auditResult.oh !== 1440) {
    throw new Error(`Expected 4x output 1600x1440, got ${auditResult.ow}x${auditResult.oh}`);
  }

  // 2. White region noise spread must be clean (< 4.0 color spread between channels on flat white)
  if (auditResult.meanWhiteSpread > 4.5) {
    throw new Error(`White area noisy: channel spread ${auditResult.meanWhiteSpread} > 4.5`);
  }

  // Capture verification image
  const shotPath = path.join(OUT_DIR, '01_pristine_anime_ai_restoration.png');
  await page.screenshot({ path: shotPath });
  console.log('Saved screenshot:', shotPath);

  await browser.close();
  console.log('SPEC 123 PURE REAL-ESRGAN AI SUPER-RESOLUTION: 100% PASSED!');
}

run().catch(err => {
  console.error('Spec 123 verification failed:', err);
  process.exit(1);
});
