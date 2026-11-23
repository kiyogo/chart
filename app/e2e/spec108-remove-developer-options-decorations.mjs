import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_108';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 108 Verification: Remove Developer Options Decorations...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });

  const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
  const page = await context.newPage();

  // 1. Visit Login Page & Setup Vietnamese language
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'light');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  // 2. Sign In
  const uname = `v108_${Date.now().toString(36).slice(-5)}`;
  console.log(`Signing in as ${uname}...`);
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // 3. Open Settings -> About
  console.log('Opening Settings -> Giới thiệu (About)...');
  await page.locator('.app-rail button[aria-label="Cài đặt"], .app-rail button[title="Cài đặt"], .app-rail button[aria-label="Settings"]').first().click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.waitForTimeout(300);

  const aboutTab = page.locator('.settings-nav-item', { hasText: /Giới thiệu|About/i }).first();
  await aboutTab.click();
  await page.waitForSelector('.about-dev-card', { timeout: 10000 });
  await page.waitForTimeout(500);

  console.log('Verifying elements with Developer mode OFF...');
  // Check group title "Tùy chọn nhà phát triển" does not exist
  const groupTitleCount = await page.locator('.settings-group-title').filter({ hasText: /Tùy chọn nhà phát triển|Developer options/i }).count();
  if (groupTitleCount > 0) {
    throw new Error(`"Tùy chọn nhà phát triển" header must NOT exist, but found ${groupTitleCount}`);
  }

  // Check bolt icon does not exist
  const iconCount = await page.locator('.about-dev-icon').count();
  if (iconCount > 0) {
    throw new Error(`.about-dev-icon must NOT exist, but found ${iconCount}`);
  }

  // Check hint text does not exist
  const hintCount = await page.locator('.about-dev-hint').count();
  if (hintCount > 0) {
    throw new Error(`.about-dev-hint must NOT exist, but found ${hintCount}`);
  }
  const hintTextCount = await page.locator('text=/Hiển thị mục Nhà phát triển gồm/i').count();
  if (hintTextCount > 0) {
    throw new Error(`Hint description text must NOT exist, but found ${hintTextCount}`);
  }

  // Check footer does not exist
  const footerCount = await page.locator('.about-dev-footer').count();
  if (footerCount > 0) {
    throw new Error(`.about-dev-footer must NOT exist, but found ${footerCount}`);
  }

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_settings_about_dev_off.png') });

  // 4. Toggle Developer mode ON
  console.log('Toggling Developer mode ON...');
  const devSwitch = page.locator('.about-dev-card input.switch').first();
  await devSwitch.click();
  await page.waitForTimeout(500);

  console.log('Verifying elements with Developer mode ON...');
  // Verify footer, "Đang bật", "Mở Nhà phát triển", grey line are still NOT present!
  const footerCountOn = await page.locator('.about-dev-footer').count();
  if (footerCountOn > 0) {
    throw new Error(`.about-dev-footer must NOT exist even when ON, but found ${footerCountOn}`);
  }
  const badgeCount = await page.locator('.about-dev-card .tag-active').count();
  const hasDangBat = await page.locator('.about-dev-card').getByText(/Đang bật|Enabled/i).count();
  if (badgeCount > 0 || hasDangBat > 0) {
    throw new Error(`Badge "Đang bật" must NOT exist, but found badgeCount=${badgeCount}, hasDangBat=${hasDangBat}`);
  }
  const btnCount = await page.locator('.about-dev-card button').count();
  const hasMoDev = await page.locator('.about-dev-card').getByText(/Mở Nhà phát triển|Open Developer/i).count();
  if (btnCount > 0 || hasMoDev > 0) {
    throw new Error(`Button "Mở Nhà phát triển" must NOT exist, but found btnCount=${btnCount}, hasMoDev=${hasMoDev}`);
  }

  // Verify only clean toggle row with title exists
  const devCardText = await page.locator('.about-dev-card').innerText();
  console.log(`Clean card text: "${devCardText.trim()}"`);
  if (!devCardText.includes('Chế độ nhà phát triển') && !devCardText.includes('Developer mode')) {
    throw new Error(`Card should contain "Chế độ nhà phát triển", but got: "${devCardText}"`);
  }

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_settings_about_dev_on_clean.png') });

  // Verify Developer tab appeared in settings nav
  const devTabInNav = page.locator('.settings-nav-item', { hasText: /Nhà phát triển|Developer/i }).first();
  if (!(await devTabInNav.isVisible())) {
    throw new Error('Developer tab should be visible in settings navigation when developer_mode is enabled');
  }
  console.log('Developer tab in settings nav is verified visible.');

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '03_settings_developer_tab_visible.png') });

  console.log('✅ All Spec 108 checks PASSED successfully!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Test failed:', err);
  process.exit(1);
});
