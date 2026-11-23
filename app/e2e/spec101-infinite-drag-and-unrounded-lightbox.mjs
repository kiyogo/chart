import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_101';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 101 Infinite Drag & Unrounded Lightbox Image Verification...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });

  const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
  const page = await context.newPage();

  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'light');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  const uname = `v101_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Open Self Chat
  const selfChat = page.locator('.nav-sidebar .conv-row, .nav-sidebar .chat-item').filter({ hasText: /Chính mình|Note to self/i }).first();
  await selfChat.waitFor({ state: 'visible', timeout: 5000 });
  await selfChat.click();
  await page.waitForTimeout(600);

  // Generate sample image
  const samplePngBase64 = await page.evaluate(() => {
    const c = document.createElement('canvas');
    c.width = 800;
    c.height = 600;
    const g = c.getContext('2d');
    const grad = g.createLinearGradient(0, 0, 800, 600);
    grad.addColorStop(0, '#f43f5e');
    grad.addColorStop(0.5, '#a855f7');
    grad.addColorStop(1, '#3b82f6');
    g.fillStyle = grad;
    g.fillRect(0, 0, 800, 600);
    g.fillStyle = '#ffffff';
    g.font = 'bold 36px sans-serif';
    g.fillText('Spec 101 Unrounded Photo 800x600', 80, 300);
    return c.toDataURL('image/png').split(',')[1];
  });
  const tmpPng = path.join(ARTIFACTS_DIR, 'test_image.png');
  fs.writeFileSync(tmpPng, Buffer.from(samplePngBase64, 'base64'));

  // Attach and send image
  await page.locator('.composer .icon-btn').first().click();
  await page.waitForTimeout(250);
  await page.locator('input[type="file"][accept*="image"]').first().setInputFiles(tmpPng);
  await page.waitForSelector('.rich-image-btn', { timeout: 15000 });
  await page.waitForTimeout(500);

  // Open Lightbox
  await page.locator('.rich-image-btn').first().click();
  await page.waitForSelector('.lightbox .lightbox-img', { timeout: 5000 });
  await page.waitForTimeout(400);

  // 1. Verify unrounded corners: border-radius must be 0px
  const borderRadius = await page.evaluate(() => {
    const img = document.querySelector('.lightbox-img');
    return window.getComputedStyle(img).borderRadius;
  });
  console.log('Lightbox image border-radius:', borderRadius);
  if (borderRadius !== '0px') {
    throw new Error(`❌ Lightbox image is still rounded! border-radius = ${borderRadius}`);
  }
  console.log('✅ Confirmed: Lightbox image has zero rounded corners (border-radius: 0px)!');
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_unrounded_lightbox_image.png') });

  // 2. Test infinite drag at 100% (1.0x) zoom
  const imgBox = await page.locator('.lightbox-img').boundingBox();
  const startX = imgBox.x + imgBox.width / 2;
  const startY = imgBox.y + imgBox.height / 2;

  console.log(`Dragging from (${startX}, ${startY}) to (${startX + 220}, ${startY + 140})...`);
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX + 220, startY + 140, { steps: 5 });
  await page.waitForTimeout(100);

  // Check that the image followed the drag (pan_x ~220, pan_y ~140)
  const transformDuringDrag = await page.evaluate(() => {
    return document.querySelector('.lightbox-img').style.transform;
  });
  console.log('Transform during drag:', transformDuringDrag);
  if (!transformDuringDrag.includes('translate3d(220') && !transformDuringDrag.includes('translate3d(219') && !transformDuringDrag.includes('translate3d(221')) {
    throw new Error(`❌ Image did not follow drag! transform: ${transformDuringDrag}`);
  }
  console.log('✅ Confirmed: Image followed drag freely at 1.0x zoom (infinite drag)!');
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_dragging_infinitely.png') });

  // Release mouse -> it must snap back to 0px, 0px
  await page.mouse.up();
  // Wait for 300ms transition
  await page.waitForTimeout(450);

  const transformAfterSnap = await page.evaluate(() => {
    return document.querySelector('.lightbox-img').style.transform;
  });
  console.log('Transform after snap back:', transformAfterSnap);
  if (!transformAfterSnap.includes('translate3d(0px, 0px') && !transformAfterSnap.includes('translate3d(0.0px, 0.0px')) {
    throw new Error(`❌ Image failed to snap back to original position! transform: ${transformAfterSnap}`);
  }
  console.log('✅ Confirmed: Image smoothly glided/snapped back to original position (0, 0)!');
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '03_snapped_back_to_place.png') });

  // 3. Test double-click to zoom and drag when zoomed
  await page.locator('.lightbox-img').dblclick();
  await page.waitForTimeout(300);
  const zoomText = await page.locator('.lightbox-zoom-readout').innerText();
  console.log('Zoom after dblclick:', zoomText);

  // Drag when zoomed
  const zoomedBox = await page.locator('.lightbox-img').boundingBox();
  const zStartX = zoomedBox.x + zoomedBox.width / 2;
  const zStartY = zoomedBox.y + zoomedBox.height / 2;
  await page.mouse.move(zStartX, zStartY);
  await page.mouse.down();
  await page.mouse.move(zStartX + 400, zStartY + 300, { steps: 5 });
  await page.waitForTimeout(100);

  const zoomedDragTransform = await page.evaluate(() => {
    return document.querySelector('.lightbox-img').style.transform;
  });
  console.log('Zoomed drag transform:', zoomedDragTransform);
  if (!zoomedDragTransform.includes('translate3d(400') && !zoomedDragTransform.includes('translate3d(399') && !zoomedDragTransform.includes('translate3d(401')) {
    throw new Error(`❌ Zoomed image did not follow drag! transform: ${zoomedDragTransform}`);
  }
  console.log('✅ Confirmed: Zoomed image can be dragged infinitely past edges!');
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '04_zoomed_infinite_drag.png') });

  // Release mouse -> springs back to clamped bounds
  await page.mouse.up();
  await page.waitForTimeout(450);

  const zoomedSnapTransform = await page.evaluate(() => {
    return document.querySelector('.lightbox-img').style.transform;
  });
  console.log('Zoomed snap transform:', zoomedSnapTransform);
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '05_zoomed_snapped_back.png') });

  // 4. Test close with Escape
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  const lbCount = await page.locator('.lightbox').count();
  if (lbCount !== 0) {
    throw new Error(`❌ Lightbox did not close with Escape! count = ${lbCount}`);
  }
  console.log('✅ Confirmed: Lightbox closed cleanly with Escape!');

  console.log('🎉 Spec 101 Verification finished successfully!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
