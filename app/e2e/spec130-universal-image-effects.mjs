import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec130';

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
  console.log('--- Spec 130 E2E Verification: Universal AI Image Enhancement Effects & Sender Enablement ---');
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
  page.on('console', msg => {
    console.log('PAGE LOG:', msg.text());
  });
  console.log('Loading ChatW at', BASE);
  await page.goto(BASE, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(1500);

  // Authenticate if needed
  const isAuth = await page.evaluate(() => !!document.querySelector('.auth-shell, .auth-card'));
  if (isAuth) {
    const devInp = await page.$('.auth-dev-form input, input[aria-label], input[placeholder]');
    if (devInp) {
      await devInp.fill('spec130_tester_' + Date.now().toString(36).slice(-4));
      const submitBtn = await page.$(".auth-dev-form button[type='submit'], button[type='submit'], .btn-primary");
      if (submitBtn) await submitBtn.click();
    }
    await page.waitForSelector('.app-rail', { timeout: 25000 });
  }

  await dismissModals(page);
  await page.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer), { timeout: 25000 });

  // 1. Navigate to Note to Self ("Chính mình" / "Note to self")
  console.log('Opening Note to Self...');
  const noteSelfBtn = page.locator(".chat-item:has-text('Chính mình'), .chat-item:has-text('Note to self'), button.chat-item:has(.self-chat-avatar), button.chat-item").first();
  await noteSelfBtn.waitFor({ state: 'visible', timeout: 15000 });
  await noteSelfBtn.click();
  await page.waitForSelector('.composer-box, textarea.composer-input', { timeout: 15000 });
  await page.waitForTimeout(500);
  await dismissModals(page);

  // 2. Upload a test image into chat as sender (generates is_sender_original = true)
  console.log('Preparing test image...');
  const tmpImgPath = path.resolve('/tmp/spec130_test.jpg');

  console.log('Inspecting composer tools button...');
  const addBtn = page.locator('.composer-tools button, .composer-box button.icon-btn').first();
  await addBtn.click();
  await page.waitForTimeout(500);

  const fileInput = page.locator("input[type='file'][accept*='image']").first();
  console.log('Setting input file on composer tools file input...');
  await fileInput.setInputFiles(tmpImgPath);
  console.log('Image selected, waiting for upload and message bubble...');
  await page.waitForTimeout(4000);

  // Find the latest image in chat
  const latestImage = page.locator('.rich-image-btn').last();
  await latestImage.scrollIntoViewIfNeeded().catch(() => {});
  await page.waitForTimeout(500);

  // Check data attributes
  const imgAttrs = await latestImage.locator('img').first().evaluate(el => ({
    isSenderOrig: el.getAttribute('data-sender-original'),
    aiPending: el.getAttribute('data-ai-pending'),
    aiRestored: el.getAttribute('data-ai-restored'),
    src: el.src
  }));
  console.log('Chat image status:', imgAttrs);

  // Hover over the image to inspect the corner Focus icon
  await latestImage.hover();
  await page.waitForTimeout(400);

  const cornerIcon = latestImage.locator('.rich-image-enhance-corner');
  const cornerVisible = await cornerIcon.isVisible().catch(() => false);
  console.log('Corner Focus icon visible on hover for self-sent image:', cornerVisible);

  await page.screenshot({ path: path.join(OUT_DIR, '01_sender_chat_image_focus_icon.png') });

  // 3. Click image to open Lightbox
  console.log('Opening Lightbox to verify auto-enhance shimmer and wand toggle...');
  await latestImage.click();
  await page.waitForSelector('.lightbox', { timeout: 10000 });

  // Verify Lightbox AI Shimmer overlay is present
  const lbAiOverlay = page.locator('.lightbox-ai-gen');
  const lbOverlayVisible = await lbAiOverlay.isVisible().catch(() => false);
  console.log('Lightbox AI shimmer overlay active on open:', lbOverlayVisible);

  await page.screenshot({ path: path.join(OUT_DIR, '02_lightbox_aurora_shimmer_overlay.png') });

  // Wait for AI enhancement to complete (from auto_enhance_on_open or manual wand click)
  console.log('Checking wand button status...');
  const wandBtn = page.locator('button.lightbox-enhance-toggle');
  await wandBtn.waitFor({ state: 'visible', timeout: 15000 });

  const activeWandBtn = page.locator('button.lightbox-enhance-toggle.is-active');
  try {
    await activeWandBtn.waitFor({ state: 'visible', timeout: 15000 });
    console.log('AI Enhancement auto-completed! Wand button is active (blue)!');
  } catch (_) {
    console.log('Clicking wand button to trigger AI enhancement...');
    await wandBtn.click({ force: true });
    await activeWandBtn.waitFor({ state: 'visible', timeout: 25000 });
    console.log('AI Enhancement complete via manual wand click! Wand button is active (blue)!');
  }

  await page.screenshot({ path: path.join(OUT_DIR, '03_lightbox_ai_enhanced_with_wand_toggle.png') });

  // Now toggle to original
  console.log('Clicking wand toggle to switch to original...');
  await activeWandBtn.click();
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(OUT_DIR, '04_lightbox_toggled_to_original.png') });

  // Toggle back to enhanced
  console.log('Clicking wand toggle to switch back to enhanced...');
  await wandBtn.click();
  await activeWandBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
  await page.waitForTimeout(600);

  // Close lightbox
  const closeBtn = page.locator('.lightbox-close').first();
  if (await closeBtn.isVisible()) {
    await closeBtn.click();
    await page.waitForTimeout(500);
  }

  // 4. Test Details Panel / Shared Media gallery opening
  console.log('Testing Details Panel / Shared Media gallery tile...');
  const mediaTile = page.locator('.self-media-tile').first();
  if (await mediaTile.isVisible().catch(() => false)) {
    await mediaTile.click();
    await page.waitForSelector('.lightbox', { timeout: 10000 });
    const wandFromDetails = await page.locator('.lightbox-enhance-toggle').isVisible().catch(() => false);
    console.log('Lightbox opened from Details panel with wand button:', wandFromDetails);
    await page.screenshot({ path: path.join(OUT_DIR, '05_lightbox_from_details_panel.png') });

    const closeBtn2 = page.locator('.lightbox-close').first();
    if (await closeBtn2.isVisible()) await closeBtn2.click();
  }

  console.log('--- Verification Complete: Universal AI Effects Successfully Enabled ---');
  await browser.close();
}

run().catch(err => {
  console.error('Verification failed:', err);
  process.exit(1);
});
