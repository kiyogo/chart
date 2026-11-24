import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec131';

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
  console.log('--- Spec 131 E2E Verification: Pristine AI Enhancement Quality & Model Routing ---');
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
  page.on('console', msg => console.log('PAGE LOG:', msg.text()));

  console.log('Loading ChatW at', BASE);
  await page.goto(BASE, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(1500);

  // Authenticate if needed
  const isAuth = await page.evaluate(() => !!document.querySelector('.auth-shell, .auth-card'));
  if (isAuth) {
    const devInp = await page.$('.auth-dev-form input, input[aria-label], input[placeholder]');
    if (devInp) {
      await devInp.fill('spec131_tester_' + Date.now().toString(36).slice(-4));
      const submitBtn = await page.$(".auth-dev-form button[type='submit'], button[type='submit'], .btn-primary");
      if (submitBtn) await submitBtn.click();
    }
    await page.waitForSelector('.app-rail', { timeout: 25000 });
  }

  await dismissModals(page);
  await page.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer), { timeout: 25000 });

  // 1. Audit Model Selection Routing (Portraits -> GeneralX4V3, Anime -> AnimeVideoV3)
  console.log('Testing Intelligent Model Selection in Rust WASM...');
  const routingResult = await page.evaluate(async () => {
    const wasm = window.__chatwWasmMedia;

    // A. Create a Portrait Test Image (cosplay / selfie skin tones with eye shadows)
    const cPort = document.createElement('canvas');
    cPort.width = 160;
    cPort.height = 160;
    const ctxP = cPort.getContext('2d');
    // Mall background (blurred / bokeh)
    ctxP.fillStyle = '#89A0C0';
    ctxP.fillRect(0, 0, 160, 160);
    // Face skin tone (#FAD6C0)
    ctxP.fillStyle = '#FAD6C0';
    ctxP.beginPath();
    ctxP.arc(80, 80, 50, 0, Math.PI * 2);
    ctxP.fill();
    // Eyes with subtle natural shadow
    ctxP.fillStyle = '#6E453B';
    ctxP.beginPath();
    ctxP.arc(60, 75, 8, 0, Math.PI * 2);
    ctxP.arc(100, 75, 8, 0, Math.PI * 2);
    ctxP.fill();
    // Eyelid crease shadow (natural gradient)
    ctxP.fillStyle = '#D4A894';
    ctxP.fillRect(52, 64, 16, 3);
    ctxP.fillRect(92, 64, 16, 3);

    const imgDataP = ctxP.getImageData(0, 0, 160, 160);
    const u8P = new Uint8Array(imgDataP.data.buffer, imgDataP.data.byteOffset, imgDataP.data.byteLength);
    const selPortrait = wasm.selectRealEsrganModel(u8P, 160, 160, '', '');

    // B. Create an Anime Test Image (flat color fills + hard black ink lines)
    const cAnime = document.createElement('canvas');
    cAnime.width = 160;
    cAnime.height = 160;
    const ctxA = cAnime.getContext('2d');
    ctxA.fillStyle = '#E8F5E9';
    ctxA.fillRect(0, 0, 160, 160);
    ctxA.fillStyle = '#FF5722';
    ctxA.beginPath();
    ctxA.arc(80, 80, 50, 0, Math.PI * 2);
    ctxA.fill();
    // Thick black anime ink outlines
    ctxA.strokeStyle = '#000000';
    ctxA.lineWidth = 4;
    ctxA.stroke();

    const imgDataA = ctxA.getImageData(0, 0, 160, 160);
    const u8A = new Uint8Array(imgDataA.data.buffer, imgDataA.data.byteOffset, imgDataA.data.byteLength);
    const selAnime = wasm.selectRealEsrganModel(u8A, 160, 160, '', '');

    return {
      portraitModel: selPortrait.modelId,
      portraitLabel: selPortrait.modelLabel,
      animeModel: selAnime.modelId,
      animeLabel: selAnime.modelLabel,
    };
  });

  console.log('Model Routing Result:', routingResult);
  if (routingResult.portraitModel !== 'realesr_general_x4v3') {
    throw new Error(`Portrait routing failed: expected realesr_general_x4v3, got ${routingResult.portraitModel}`);
  }
  if (routingResult.animeModel !== 'realesr_animevideov3') {
    throw new Error(`Anime routing failed: expected realesr_animevideov3, got ${routingResult.animeModel}`);
  }

  // 2. Test Full Portrait Restoration Quality & AMD CAS Boundary Verification
  console.log('Testing Portrait AI Enhancement with AMD FidelityFX CAS...');
  const portraitAiResult = await page.evaluate(async () => {
    // Generate a test face with soft eyelid gradient
    const c = document.createElement('canvas');
    c.width = 160;
    c.height = 140;
    const ctx = c.getContext('2d');
    ctx.fillStyle = '#E3E9F0';
    ctx.fillRect(0, 0, 160, 140);
    ctx.fillStyle = '#FCE0D2';
    ctx.beginPath();
    ctx.arc(80, 70, 50, 0, Math.PI * 2);
    ctx.fill();
    // Soft eye crease
    ctx.fillStyle = '#CCA492';
    ctx.fillRect(60, 60, 15, 3);
    ctx.fillRect(85, 60, 15, 3);

    const blob = await new Promise(r => c.toBlob(r, 'image/jpeg', 0.90));
    const url = URL.createObjectURL(blob);

    const res = await window.__chatwAiRestorer.restoreImageFromUrl(url, {
      forceRun: true,
      mediaId: 'test_portrait_face_' + Date.now(),
    });

    if (!res || !res.enhanced || !res.url) {
      return { ok: false, error: 'Enhancement failed' };
    }

    // Check enhanced pixels: verify NO necrotic / extreme black craters exist in the eye crease
    const bmp = await createImageBitmap(await (await fetch(res.url)).blob());
    const cOut = document.createElement('canvas');
    cOut.width = bmp.width;
    cOut.height = bmp.height;
    const ctxOut = cOut.getContext('2d');
    ctxOut.drawImage(bmp, 0, 0);
    const data = ctxOut.getImageData(0, 0, bmp.width, bmp.height).data;

    // Sample the eye region (scaled 4x)
    let minLumaInCrease = 255;
    for (let y = Math.round(60 * 4); y < Math.round(63 * 4); y++) {
      for (let x = Math.round(60 * 4); x < Math.round(75 * 4); x++) {
        const idx = (y * bmp.width + x) * 4;
        const luma = 0.299 * data[idx] + 0.587 * data[idx + 1] + 0.114 * data[idx + 2];
        if (luma < minLumaInCrease) minLumaInCrease = luma;
      }
    }

    return {
      ok: true,
      modelId: res.modelId,
      width: bmp.width,
      height: bmp.height,
      minLumaInCrease, // Under broken unsharp mask with 0.65 clamp, this dropped to < 50; with CAS it stays healthy (~140-180)
    };
  });

  console.log('Portrait Enhancement Result:', portraitAiResult);
  if (!portraitAiResult.ok) {
    throw new Error('Portrait AI restoration failed: ' + portraitAiResult.error);
  }
  console.log('Eyelid crease minimum luma (healthy):', portraitAiResult.minLumaInCrease);
  if (portraitAiResult.minLumaInCrease < 90) {
    throw new Error('Eye shadow abnormally dark / cratered: ' + portraitAiResult.minLumaInCrease);
  }

  // 3. UI Lightbox & Chat Verification
  console.log('Testing Chat & Lightbox UI with Note to Self...');
  const noteSelfBtn = page.locator(".chat-item:has-text('Chính mình'), .chat-item:has-text('Note to self'), button.chat-item:has(.self-chat-avatar), button.chat-item").first();
  await noteSelfBtn.waitFor({ state: 'visible', timeout: 15000 });
  await noteSelfBtn.click();
  await page.waitForSelector('.composer-box, textarea.composer-input', { timeout: 15000 });
  await page.waitForTimeout(500);
  await dismissModals(page);

  // Send a test image
  const TEST_IMAGE_PATH = '/tmp/test_spec131_portrait.jpg';

  const addBtn = page.locator('.composer-tools button, .composer-box button.icon-btn').first();
  await addBtn.click();
  await page.waitForTimeout(400);

  const fileInput = page.locator("input[type='file'][accept*='image']").first();
  await fileInput.setInputFiles(TEST_IMAGE_PATH);
  await page.waitForSelector('.rich-image-btn', { timeout: 20000 });
  await page.waitForTimeout(600);

  const latestImage = page.locator('.rich-image-btn').last();
  await latestImage.click();
  await page.waitForSelector('.lightbox', { timeout: 10000 });
  await page.waitForTimeout(600);

  const wandBtn = page.locator('button.lightbox-enhance-toggle');
  await wandBtn.waitFor({ state: 'visible', timeout: 15000 });

  // Wait for AI enhancement to complete (button becomes active and enabled)
  const activeWandBtn = page.locator('button.lightbox-enhance-toggle.is-active:not([disabled])');
  try {
    await activeWandBtn.waitFor({ state: 'visible', timeout: 25000 });
  } catch (_) {
    await wandBtn.click({ force: true });
    await activeWandBtn.waitFor({ state: 'visible', timeout: 25000 });
  }

  console.log('Capturing Lightbox screenshot...');
  await page.screenshot({ path: path.join(OUT_DIR, '01_pristine_lightbox_wand_active.png') });

  // Toggle to original
  console.log('Testing wand toggle to original...');
  await activeWandBtn.click();
  await page.waitForTimeout(500);
  await page.screenshot({ path: path.join(OUT_DIR, '02_lightbox_toggled_original.png') });

  // Toggle back to enhanced
  console.log('Testing wand toggle back to enhanced...');
  await wandBtn.click();
  await page.waitForTimeout(500);
  await page.screenshot({ path: path.join(OUT_DIR, '03_lightbox_toggled_enhanced.png') });

  console.log('--- Verification Complete: Spec 131 Successfully Verified! ---');
  await browser.close();
}

run().catch(err => {
  console.error('Verification failed:', err);
  process.exit(1);
});
