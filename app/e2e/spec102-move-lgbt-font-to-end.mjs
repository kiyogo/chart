import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_102';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 102 Move LGBT Font to End of Menu Verification...');
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

  const uname = `v102_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Open Settings -> Appearance (Giao diện)
  await page.locator('.app-rail button[aria-label="Cài đặt"], .app-rail button[title="Cài đặt"], .app-rail button[aria-label="Settings"]').first().click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.waitForTimeout(300);
  await page.locator('.settings-nav-item', { hasText: /Giao diện|Appearance/i }).first().click();
  await page.waitForSelector('select[data-font-picker="true"]', { timeout: 10000 });
  await page.waitForTimeout(400);

  // Scroll font selector into view & open custom dropdown
  const fontSel = page.locator('select[data-font-picker="true"]').first();
  await fontSel.scrollIntoViewIfNeeded();
  await page.waitForTimeout(200);
  await fontSel.click();
  await page.waitForSelector('.cw-select-menu[data-font-menu="true"] .cw-select-option', { timeout: 5000 });
  await page.waitForTimeout(500);

  const dropdownAudit = await page.evaluate(() => {
    const opts = Array.from(document.querySelectorAll('.cw-select-menu[data-font-menu="true"] .cw-select-option'));
    return {
      count: opts.length,
      labels: opts.map((o) => o.querySelector('.cw-select-option-label')?.textContent?.trim() || ''),
      values: opts.map((o) => o.getAttribute('data-font-preview') || ''),
      firstOption: {
        label: opts[0]?.querySelector('.cw-select-option-label')?.textContent?.trim() || '',
        value: opts[0]?.getAttribute('data-font-preview') || '',
      },
      lastOption: {
        label: opts[opts.length - 1]?.querySelector('.cw-select-option-label')?.textContent?.trim() || '',
        value: opts[opts.length - 1]?.getAttribute('data-font-preview') || '',
      },
      secondOption: {
        label: opts[1]?.querySelector('.cw-select-option-label')?.textContent?.trim() || '',
        value: opts[1]?.getAttribute('data-font-preview') || '',
      },
    };
  });

  console.log('Dropdown audit:', JSON.stringify(dropdownAudit, null, 2));

  // Screenshot open dropdown menu
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_lgbt_font_at_end_of_menu.png') });

  // Scroll down the dropdown menu to clearly show LGBT at the very bottom
  await page.evaluate(() => {
    const list = document.querySelector('.cw-select-menu[data-font-menu="true"] .cw-select-list, .cw-select-menu .cw-select-list');
    if (list) {
      list.scrollTop = list.scrollHeight;
    }
  });
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01b_lgbt_font_scrolled_to_bottom.png') });

  // Verification checks:
  if (dropdownAudit.lastOption.value !== 'lgbt' || dropdownAudit.lastOption.label !== 'LGBT') {
    throw new Error(`Expected last option to be LGBT, got: ${JSON.stringify(dropdownAudit.lastOption)}`);
  }
  if (dropdownAudit.secondOption.value === 'lgbt') {
    throw new Error(`LGBT should NOT be the 2nd option anymore!`);
  }
  if (dropdownAudit.firstOption.value !== 'system') {
    throw new Error(`Expected first option to be system, got: ${JSON.stringify(dropdownAudit.firstOption)}`);
  }

  // Click the last option (LGBT)
  const lastOption = page.locator('.cw-select-menu[data-font-menu="true"] .cw-select-option').last();
  await lastOption.click();
  await page.waitForTimeout(500);

  // Take screenshot with LGBT font selected
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_lgbt_font_selected.png') });

  const fontAttr = await page.evaluate(() => document.documentElement.getAttribute('data-font'));
  console.log('Selected data-font:', fontAttr);
  if (fontAttr !== 'lgbt') {
    throw new Error(`Expected data-font to be 'lgbt', got: ${fontAttr}`);
  }

  console.log('✅ Spec 102 verified successfully: LGBT font moved to end of list and functions perfectly!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
