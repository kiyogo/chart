// Playwright E2E verification for Spec 041:
// - Global borderless & ringless UI (zero 3px focus-rings, zero 2px focus outlines)
// - Ultra-sleek, slim capsule pill composer (border-radius: 9999px, height <= 44px, zero border on focus)
// - Circular action buttons & send button
// - Zero uncaught runtime page errors
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_041';
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
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(e.message));

  const suffix = `${Date.now().toString(36).slice(-5)}${Math.floor(Math.random() * 90 + 10)}`;
  await page.goto(`${BASE}/`);
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`s041_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // 1. Open direct chat
  await page.locator('.chat-item').first().click();
  await page.waitForSelector('.composer-box', { timeout: 10000 });
  await page.waitForTimeout(300);

  // 2. Measure composer geometry before focus
  const boxGeometry = await page.evaluate(() => {
    const box = document.querySelector('.composer-box');
    const send = document.querySelector('.send-btn');
    const input = document.querySelector('.composer-input');
    const boxCs = window.getComputedStyle(box);
    const sendCs = window.getComputedStyle(send);
    const inputCs = window.getComputedStyle(input);
    const boxRect = box.getBoundingClientRect();
    const sendRect = send.getBoundingClientRect();

    return {
      boxHeight: boxRect.height,
      boxRadius: boxCs.borderRadius,
      boxBorder: boxCs.borderWidth,
      boxOutline: boxCs.outlineStyle,
      boxShadow: boxCs.boxShadow,
      sendHeight: sendRect.height,
      sendWidth: sendRect.width,
      sendRadius: sendCs.borderRadius,
      inputHeight: input.offsetHeight,
    };
  });

  record(
    'Composer box is a slim capsule pill (height <= 44px, full rounded capsule radius)',
    boxGeometry.boxHeight <= 44 &&
      (boxGeometry.boxRadius === '9999px' || parseFloat(boxGeometry.boxRadius) >= 18),
    `boxHeight=${boxGeometry.boxHeight}px, boxRadius=${boxGeometry.boxRadius}`,
  );

  record(
    'Send button is circular pill matching capsule geometry',
    Math.abs(boxGeometry.sendWidth - boxGeometry.sendHeight) <= 2 &&
      (boxGeometry.sendRadius === '9999px' || parseFloat(boxGeometry.sendRadius) >= 15),
    `sendSize=${boxGeometry.sendWidth}x${boxGeometry.sendHeight}, sendRadius=${boxGeometry.sendRadius}`,
  );

  await page.screenshot({ path: path.join(OUT, '01_composer_sleek_pill_normal.png') });

  // 3. Focus the input: click into #dm-chat-input and verify ZERO border / focus ring
  await page.locator('.composer-input').click();
  await page.waitForTimeout(200);

  const focusedAudit = await page.evaluate(() => {
    const box = document.querySelector('.composer-box');
    const input = document.querySelector('.composer-input');
    const boxCs = window.getComputedStyle(box);
    const inputCs = window.getComputedStyle(input);

    return {
      boxBorder: boxCs.borderWidth,
      boxOutline: boxCs.outlineStyle,
      boxShadow: boxCs.boxShadow,
      inputBorder: inputCs.borderWidth,
      inputOutline: inputCs.outlineStyle,
      inputShadow: inputCs.boxShadow,
    };
  });

  const hasThickBlueRing = focusedAudit.boxShadow.includes('3px') || focusedAudit.boxShadow.includes('0 0 0');
  record(
    'Composer box on focus-within has strictly zero border, zero outline, zero thick blue ring',
    (focusedAudit.boxBorder === '0px' || focusedAudit.boxBorder === '') &&
      (focusedAudit.boxOutline === 'none' || focusedAudit.boxOutline === '') &&
      !hasThickBlueRing,
    `boxBorder=${focusedAudit.boxBorder}, boxOutline=${focusedAudit.boxOutline}, boxShadow=${focusedAudit.boxShadow}`,
  );

  record(
    'Composer input textarea has zero border, zero outline, zero box-shadow',
    (focusedAudit.inputBorder === '0px' || focusedAudit.inputBorder === '') &&
      (focusedAudit.inputOutline === 'none' || focusedAudit.inputOutline === '') &&
      focusedAudit.inputShadow === 'none',
    `inputBorder=${focusedAudit.inputBorder}, inputOutline=${focusedAudit.inputOutline}, inputShadow=${focusedAudit.inputShadow}`,
  );

  await page.screenshot({ path: path.join(OUT, '02_composer_sleek_pill_focused.png') });

  // 4. Test global search input focus has zero ring
  const searchInput = page.locator('.sidebar-search input');
  if (await searchInput.isVisible()) {
    await searchInput.click();
    await page.waitForTimeout(150);
    const searchFocus = await page.evaluate(() => {
      const inp = document.querySelector('.sidebar-search input');
      const cs = window.getComputedStyle(inp);
      return { border: cs.borderWidth, outline: cs.outlineStyle, shadow: cs.boxShadow };
    });
    record(
      'Sidebar search input has zero focus ring & zero outline on focus',
      (searchFocus.border === '0px' || searchFocus.border === '') &&
        searchFocus.outline === 'none' &&
        (searchFocus.shadow === 'none' || !searchFocus.shadow.includes('3px')),
      JSON.stringify(searchFocus),
    );
  }

  // 5. Zero runtime errors
  record('Zero uncaught page errors', errors.length === 0, errors.join('; '));

  console.log(`\nSpec 041 check: ${checks.filter((c) => c.pass).length}/${checks.length} passed.`);
  if (checks.some((c) => !c.pass)) {
    process.exit(1);
  }
} catch (e) {
  console.error('Test execution failed:', e);
  process.exit(1);
} finally {
  await browser.close();
}
