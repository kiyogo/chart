// Live Chromium verification for Spec 025: Modal & Popover full-viewport placement
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_025';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

const browser = await chromium.launch({
  executablePath: '/usr/bin/google-chrome',
  headless: true,
  args: ['--no-sandbox', '--disable-gpu'],
});

const errors = [];
try {
  // Desktop 1440x900
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(`desktop:${e.message}`));

  const suffix = `${Date.now().toString(36).slice(-5)}${Math.floor(Math.random() * 90 + 10)}`;
  await page.goto(`${BASE}/`);
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Test username"]').fill(`modal_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Skip tour"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Open self chat or first conversation with composer
  await page.locator('.conv-row .chat-item').first().click();
  await page.waitForSelector('.composer-box', { timeout: 10000 });
  await page.waitForTimeout(300);

  // 1. Check composer-box backdrop-filter moved cleanly to ::before (does not trap fixed children)
  const composerBoxInfo = await page.evaluate(() => {
    const box = document.querySelector('.composer-box');
    const cs = getComputedStyle(box);
    const bef = getComputedStyle(box, '::before');
    return {
      boxBackdrop: cs.backdropFilter || cs.webkitBackdropFilter || 'none',
      beforeBackdrop: bef.backdropFilter || bef.webkitBackdropFilter || 'none',
      beforeBg: bef.backgroundColor,
    };
  });
  record(
    'Composer box keeps frosted backdrop-filter on ::before instead of container',
    composerBoxInfo.boxBackdrop === 'none' && composerBoxInfo.beforeBackdrop.includes('blur'),
    JSON.stringify(composerBoxInfo),
  );

  // 2. Open AttachMenu ("Add") -> verify menu-dismiss covers 1440x900 and tools-menu is visible
  await page.locator('.composer-tools button[aria-label*="Add"]').click();
  await page.waitForSelector('.tools-menu', { timeout: 5000 });
  await page.waitForTimeout(200);
  const dismissBox = await page.locator('.composer .menu-dismiss').boundingBox();
  record(
    'Attach menu dismiss backdrop covers full 1440x900 viewport',
    !!(dismissBox && Math.round(dismissBox.width) === 1440 && Math.round(dismissBox.height) === 900),
    JSON.stringify(dismissBox),
  );
  await page.screenshot({ path: path.join(OUT, '01_attach_menu.png') });

  // 3. Test "Create poll", "Create note", "Create reminder"
  for (const [label, expectedTitle, fileName] of [
    ['Poll', 'Create poll', '02_poll_dialog'],
    ['Notes', 'Create note', '03_note_dialog'],
    ['Appointment reminder', 'Create reminder', '04_reminder_dialog'],
  ]) {
    if (!(await page.locator('.tools-menu').isVisible().catch(() => false))) {
      await page.locator('.composer-tools button[aria-label*="Add"]').click();
      await page.waitForSelector('.tools-menu', { timeout: 5000 });
    }
    await page.locator('.tools-menu button', { hasText: label }).click();
    await page.waitForSelector('.modal-overlay .modal-dialog', { timeout: 5000 });
    await page.waitForTimeout(300);

    const overlayBox = await page.locator('.modal-overlay').boundingBox();
    const dialogBox = await page.locator('.modal-dialog').boundingBox();
    const titleText = await page.locator('.modal-dialog .modal-title').innerText();
    const centeredAndUnclipped =
      overlayBox &&
      dialogBox &&
      Math.round(overlayBox.x) === 0 &&
      Math.round(overlayBox.y) === 0 &&
      Math.round(overlayBox.width) === 1440 &&
      Math.round(overlayBox.height) === 900 &&
      dialogBox.y > 40 &&
      dialogBox.y + dialogBox.height < 860 &&
      Math.abs(dialogBox.x + dialogBox.width / 2 - 720) < 10 &&
      titleText.trim() === expectedTitle;

    record(
      `Dialog "${expectedTitle}" renders full-viewport centered & unclipped`,
      !!centeredAndUnclipped,
      `overlay=${JSON.stringify(overlayBox)}, dialog=${JSON.stringify(dialogBox)}`,
    );
    await page.screenshot({ path: path.join(OUT, `${fileName}.png`) });
    await page.locator('.modal-dialog button[aria-label="Close"]').click();
    await page.waitForTimeout(200);
  }

  // 4. Test Mobile viewport (390x844) Emoji picker-popover (uses position: fixed on <=767px)
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForTimeout(300);
  await page.locator('.composer-box button[aria-label="Choose emoji"]').click();
  await page.waitForSelector('.picker-popover', { timeout: 5000 });
  await page.waitForTimeout(300);
  const mobilePickerBox = await page.locator('.picker-popover').boundingBox();
  const pickerNotTrapped =
    mobilePickerBox &&
    mobilePickerBox.width > 340 &&
    mobilePickerBox.height > 200 &&
    mobilePickerBox.y > 100 &&
    mobilePickerBox.y + mobilePickerBox.height < 840;
  record(
    'Mobile (390x844) fixed picker-popover sits cleanly above composer (not trapped)',
    !!pickerNotTrapped,
    JSON.stringify(mobilePickerBox),
  );
  await page.screenshot({ path: path.join(OUT, '05_mobile_picker.png') });

  record('No uncaught page errors', errors.length === 0, errors.join('; '));
  await ctx.close();
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
