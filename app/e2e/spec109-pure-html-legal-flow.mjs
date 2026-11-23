import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_109';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 109 Verification: Pure HTML Legal Flow & Strip "Thứ... ." Prefixes...');
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

  // 2. Click "Điều khoản dịch vụ" on login screen
  console.log('Step 1: Check Terms modal on Login screen...');
  const termsBtn = page.locator('.auth-legal-links .auth-legal-btn').filter({ hasText: /Điều khoản dịch vụ/i }).first();
  await termsBtn.click();
  await page.waitForSelector('.legal-modal-card', { timeout: 5000 });
  await page.waitForTimeout(400);

  const modalText = await page.locator('.legal-modal-scrollable').innerText();
  console.log('Sample modal text:\n', modalText.slice(0, 300));

  // Verify NO "Thứ nhất.", "Thứ hai.", etc. at the start of any line
  if (/^\s*Thứ\s+(nhất|hai|ba|tư|năm|sáu|bảy|tám|chín|mười)/im.test(modalText)) {
    throw new Error(`Ordinal prefix "Thứ..." at beginning of item must NOT exist in Terms! Found in: ${modalText}`);
  }

  if (!modalText.includes('Quyền riêng tư là tuyệt đối vĩnh viễn và không thể đảo ngược!')) {
    throw new Error('Terms missing title: "Quyền riêng tư là tuyệt đối vĩnh viễn và không thể đảo ngược!"');
  }

  // Verify computed styles: NO card background, NO border
  const firstItem = page.locator('.legal-modal-scrollable .legal-simple-item').first();
  const styles = await firstItem.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return {
      borderWidth: cs.borderWidth,
      borderStyle: cs.borderStyle,
      borderRadius: cs.borderRadius,
      backgroundColor: cs.backgroundColor,
    };
  });
  console.log('Item computed styles:', styles);
  if (styles.borderStyle !== 'none' && styles.borderWidth !== '0px') {
    throw new Error(`Item should have no border, got style=${styles.borderStyle}, width=${styles.borderWidth}`);
  }

  // Hover test: verify hover does NOT add any border box
  await firstItem.hover();
  await page.waitForTimeout(100);
  const hoverStyles = await firstItem.evaluate((el) => {
    const cs = window.getComputedStyle(el);
    return {
      borderWidth: cs.borderWidth,
      borderStyle: cs.borderStyle,
    };
  });
  if (hoverStyles.borderStyle !== 'none' && hoverStyles.borderWidth !== '0px') {
    throw new Error(`Item on hover should NOT have border, got style=${hoverStyles.borderStyle}`);
  }

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_login_terms_pure_html.png') });

  // Close modal
  await page.locator('.legal-modal-close-btn').click();
  await page.waitForSelector('.legal-modal-card', { state: 'detached', timeout: 5000 });
  await page.waitForTimeout(200);

  // 3. Sign In to App
  console.log('Step 2: Sign in and verify Settings -> About legal subtabs...');
  const uname = `v109_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Open Settings -> Giới thiệu (About)
  await page.locator('.app-rail button[aria-label="Cài đặt"], .app-rail button[title="Cài đặt"], .app-rail button[aria-label="Settings"]').first().click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.waitForTimeout(300);

  const aboutTab = page.locator('.settings-nav-item', { hasText: /Giới thiệu|About/i }).first();
  await aboutTab.click();
  await page.waitForSelector('.settings-subtabs', { timeout: 10000 });
  await page.waitForTimeout(300);

  const subtabs = page.locator('.settings-subtabs .settings-subtab');

  // Verify Terms tab
  console.log('Verifying Settings -> Điều khoản dịch vụ tab...');
  await subtabs.filter({ hasText: /Điều khoản/i }).first().click();
  await page.waitForTimeout(400);

  const panelTermsText = await page.locator('.settings-legal-panel').innerText();
  if (/^\s*Thứ\s+(nhất|hai|ba|tư|năm|sáu|bảy|tám|chín|mười)/im.test(panelTermsText)) {
    throw new Error(`Ordinal prefix "Thứ..." must NOT exist in Settings Terms! Found: ${panelTermsText}`);
  }
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_settings_terms_pure_html.png') });

  // Verify Privacy tab
  console.log('Verifying Settings -> Chính sách bảo mật tab...');
  await subtabs.filter({ hasText: /Chính sách/i }).first().click();
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '03_settings_privacy_pure_html.png') });

  // Verify Community tab
  console.log('Verifying Settings -> Tiêu chuẩn cộng đồng tab...');
  await subtabs.filter({ hasText: /Tiêu chuẩn/i }).first().click();
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '04_settings_community_pure_html.png') });

  // Verify Product Vision tab
  console.log('Verifying Settings -> Tầm nhìn sản phẩm tab...');
  await subtabs.filter({ hasText: /Tầm nhìn/i }).first().click();
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '05_settings_vision_pure_html.png') });

  console.log('✅ All Spec 109 checks PASSED successfully!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Test failed:', err);
  process.exit(1);
});
