import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_104';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 104 Terms, Privacy & Community Standards Verification...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });

  const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
  const page = await context.newPage();

  // 1. Visit Login Page
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'light');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  console.log('Step 1: Check legal links on Login screen...');
  const legalLinks = page.locator('.auth-legal-links .auth-legal-btn');
  const countLinks = await legalLinks.count();
  if (countLinks < 3) {
    throw new Error(`Expected at least 3 legal links on login, found ${countLinks}`);
  }

  // Check Terms of Service modal on Login screen
  console.log('Step 2: Open Terms of Service modal on Login screen...');
  await legalLinks.filter({ hasText: /Điều khoản dịch vụ/i }).click();
  await page.waitForSelector('.legal-modal-card', { timeout: 5000 });
  await page.waitForTimeout(300);

  const termsModalText = await page.locator('.legal-modal-card').innerText();
  if (!termsModalText.includes('Quyền riêng tư là tuyệt đối vĩnh viễn và không thể đảo ngược!')) {
    throw new Error('Terms of Service modal missing "Thứ nhất : Quyền riêng tư là tuyệt đối..."');
  }
  if (!termsModalText.includes('Từ chối cung cấp bất kì thông tin cho bên thứ ba nào.')) {
    throw new Error('Terms of Service modal missing "Thứ hai : Từ chối cung cấp..."');
  }
  if (!termsModalText.includes('Toàn bộ dữ liệu máy chủ luôn được mã hóa')) {
    throw new Error('Terms of Service modal missing "Thứ năm : Toàn bộ dữ liệu máy chủ luôn được mã hóa..."');
  }

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_login_terms_modal.png') });
  await page.locator('.legal-modal-close-btn').click();
  await page.waitForSelector('.legal-modal-card', { state: 'detached', timeout: 5000 });
  await page.waitForTimeout(200);

  // Check Privacy Policy modal on Login screen
  console.log('Step 3: Open Privacy Policy modal on Login screen...');
  await legalLinks.filter({ hasText: /Chính sách bảo mật/i }).click();
  await page.waitForSelector('.legal-modal-card', { timeout: 5000 });
  await page.waitForTimeout(300);
  const privModalText = await page.locator('.legal-modal-card').innerText();
  if (!privModalText.includes('Zero Data Collection') && !privModalText.includes('Thu thập dữ liệu: Bằng 0')) {
    throw new Error('Privacy Policy modal missing Zero Data Collection commitment');
  }
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_login_privacy_modal.png') });
  await page.locator('.legal-modal-close-btn').click();
  await page.waitForSelector('.legal-modal-card', { state: 'detached', timeout: 5000 });
  await page.waitForTimeout(200);

  // Check Community Standards modal on Login screen
  console.log('Step 4: Open Community Standards modal on Login screen...');
  await legalLinks.filter({ hasText: /Tiêu chuẩn cộng đồng/i }).click();
  await page.waitForSelector('.legal-modal-card', { timeout: 5000 });
  await page.waitForTimeout(300);
  const commModalText = await page.locator('.legal-modal-card').innerText();
  if (!commModalText.includes('Bảo vệ trẻ em tuyệt đối') && !commModalText.includes('trẻ em')) {
    throw new Error('Community standards modal missing child safety commitment');
  }
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '03_login_community_modal.png') });
  await page.locator('.legal-modal-ok-btn').click();
  await page.waitForSelector('.legal-modal-card', { state: 'detached', timeout: 5000 });
  await page.waitForTimeout(200);

  // 2. Sign In to App
  console.log('Step 5: Sign in and verify Settings -> About subtabs...');
  const uname = `v104_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Open Settings -> About
  await page.locator('.app-rail button[aria-label="Cài đặt"], .app-rail button[title="Cài đặt"], .app-rail button[aria-label="Settings"]').first().click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.waitForTimeout(300);
  await page.locator('.settings-nav-item', { hasText: /Giới thiệu|About/i }).first().click();
  await page.waitForSelector('.settings-subtabs', { timeout: 10000 });
  await page.waitForTimeout(300);

  // 3. Subtabs in About: Overview, Terms, Privacy, Community, Guide
  console.log('Step 6: Verify Settings -> About subtabs...');
  const subtabs = page.locator('.settings-subtabs .settings-subtab');
  const subtabTexts = await subtabs.allInnerTexts();
  console.log('About subtabs:', subtabTexts);

  // Click Terms subtab
  console.log('Step 7: Click Terms subtab in Settings...');
  const termsTab = subtabs.filter({ hasText: /Điều khoản|Terms/i }).first();
  await termsTab.click();
  await page.waitForSelector('.legal-items-grid', { timeout: 5000 });
  await page.waitForTimeout(400);

  const termsCardsCount = await page.locator('.legal-item-card').count();
  console.log('Terms cards count:', termsCardsCount);
  if (termsCardsCount < 10) {
    throw new Error(`Expected at least 10 steel principles in Terms of Service, got ${termsCardsCount}`);
  }
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '04_settings_about_terms_tab.png') });

  // Click Privacy subtab
  console.log('Step 8: Click Privacy subtab in Settings...');
  const privTab = subtabs.filter({ hasText: /Bảo mật|Chính sách|Privacy/i }).first();
  await privTab.click();
  await page.waitForSelector('.legal-items-grid', { timeout: 5000 });
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '05_settings_about_privacy_tab.png') });

  // Click Community subtab
  console.log('Step 9: Click Community subtab in Settings...');
  const commTab = subtabs.filter({ hasText: /Tiêu chuẩn|Cộng đồng|Community/i }).first();
  await commTab.click();
  await page.waitForSelector('.legal-items-grid', { timeout: 5000 });
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '06_settings_about_community_tab.png') });

  console.log('✅ Spec 104 verified successfully: Terms, Privacy & Community standards are fully functional, verified on Login screen and Settings!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
