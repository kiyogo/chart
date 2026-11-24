import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec129';

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
  console.log('--- Spec 129: Settings Column Default Width & Minimum Width Synchronization ---');
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
  console.log('1. Loading ChatW at', BASE);
  await page.goto(BASE, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(1500);

  // Authenticate if needed
  const isAuth = await page.evaluate(() => !!document.querySelector('.auth-shell, .auth-card'));
  if (isAuth) {
    const devInp = await page.$('.auth-dev-form input, input[aria-label], input[placeholder]');
    if (devInp) {
      await devInp.fill('spec129_tester_' + Date.now().toString(36).slice(-4));
      const submitBtn = await page.$(".auth-dev-form button[type='submit'], button[type='submit'], .btn-primary");
      if (submitBtn) await submitBtn.click();
    }
    await page.waitForSelector('.app-rail', { timeout: 25000 });
  }

  await dismissModals(page);
  await page.waitForSelector('.nav-sidebar', { timeout: 20000 });

  // Clear any existing localStorage override for settings nav width to test default synchronization
  await page.evaluate(() => {
    localStorage.removeItem('chatw_settings_nav_width');
  });

  // 2. Measure .nav-sidebar in messages view
  const sidebarWidth = await page.evaluate(() => {
    const el = document.querySelector('.nav-sidebar');
    return el ? Math.round(el.getBoundingClientRect().width) : null;
  });
  console.log('2. Messages column width (.nav-sidebar):', sidebarWidth + 'px');

  const shot1 = path.join(OUT_DIR, '01_messages_sidebar_width.png');
  await page.screenshot({ path: shot1 });
  console.log('Saved messages sidebar screenshot:', shot1);

  // 3. Open Settings view
  console.log('3. Opening Settings view...');
  const settingsBtn = page.locator(".rail-footer button[aria-label*='Settings'], .rail-footer button[data-tip*='Settings'], .rail-item[aria-label*='Settings'], .rail-item:has(svg.lucide-settings), .rail-footer .rail-item").first();
  if (await settingsBtn.isVisible()) {
    await settingsBtn.click({ force: true });
  } else {
    const gearBtn = page.locator(".rail-footer .rail-item, .app-rail .rail-footer button").last();
    await gearBtn.click({ force: true });
  }
  await page.waitForSelector('.settings-nav', { timeout: 15000 });
  await page.waitForTimeout(500);

  // 4. Measure default .settings-nav width
  const settingsDefaultWidth = await page.evaluate(() => {
    const el = document.querySelector('.settings-nav');
    return el ? Math.round(el.getBoundingClientRect().width) : null;
  });
  console.log('4. Settings column default width (.settings-nav):', settingsDefaultWidth + 'px');

  const shot2 = path.join(OUT_DIR, '02_settings_default_synchronized_width.png');
  await page.screenshot({ path: shot2 });
  console.log('Saved settings default width screenshot:', shot2);

  // Assertion: Settings column default width must match messages column width within 2px
  const widthDiff = Math.abs(settingsDefaultWidth - sidebarWidth);
  console.log(`Width difference between Settings (${settingsDefaultWidth}px) and Messages (${sidebarWidth}px): ${widthDiff}px`);
  if (widthDiff > 2) {
    throw new Error(`FAIL: Settings column default width (${settingsDefaultWidth}px) does not match Messages column (${sidebarWidth}px)! Diff: ${widthDiff}px`);
  }

  // 5. Test dragging the resizer to shrink .settings-nav down towards 190px
  console.log('5. Testing dragging resizer to shrink Settings column...');
  const resizer = page.locator('.panel-resizer[data-resizer="settings-nav"]').first();
  await resizer.waitFor({ state: 'visible', timeout: 5000 });
  const resizerBox = await resizer.boundingBox();
  if (!resizerBox) throw new Error('Could not find bounding box for settings-nav resizer');

  // Drag resizer to the left by 70px
  const startX = resizerBox.x + resizerBox.width / 2;
  const startY = resizerBox.y + resizerBox.height / 2;
  const targetX = startX - 70;

  await page.mouse.move(startX, startY);
  await page.mouse.down({ button: 'left' });
  for (let i = 1; i <= 15; i++) {
    const curX = startX + (targetX - startX) * (i / 15);
    await page.mouse.move(curX, startY);
    await page.waitForTimeout(16);
  }
  await page.mouse.up({ button: 'left' });
  await page.waitForTimeout(300);

  const shrunkenWidth = await page.evaluate(() => {
    const el = document.querySelector('.settings-nav');
    return el ? Math.round(el.getBoundingClientRect().width) : null;
  });
  console.log('6. Shrunken Settings column width after drag:', shrunkenWidth + 'px');

  const shot3 = path.join(OUT_DIR, '03_settings_shrunken_compact.png');
  await page.screenshot({ path: shot3 });
  console.log('Saved shrunken settings screenshot:', shot3);

  // Verify it shrunk down below 210px (previously stuck at 285px / 240px!)
  if (shrunkenWidth > 210) {
    throw new Error(`FAIL: Settings column could not shrink below 210px! Current: ${shrunkenWidth}px`);
  }

  console.log(`Successfully verified settings column can shrink down to ${shrunkenWidth}px!`);

  await browser.close();
  console.log('=== SPEC 129: SETTINGS COLUMN WIDTH SYNCHRONIZATION 100% PASSED ===');
}

run().catch(err => {
  console.error('Spec 129 verification failed:', err);
  process.exit(1);
});
