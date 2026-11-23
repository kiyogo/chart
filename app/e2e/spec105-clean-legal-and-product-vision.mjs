import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_105';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 105 Clean Simple Legal & Product Vision Verification...');
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

  console.log('Step 1: Check 4 legal links on Login screen...');
  const legalLinks = page.locator('.auth-legal-links .auth-legal-btn');
  const countLinks = await legalLinks.count();
  if (countLinks < 4) {
    throw new Error(`Expected at least 4 legal links on login (including Tầm nhìn sản phẩm), found ${countLinks}`);
  }

  // Check Product Vision modal
  console.log('Step 2: Open "Tầm nhìn sản phẩm" modal on Login screen...');
  await legalLinks.filter({ hasText: /Tầm nhìn sản phẩm/i }).click();
  await page.waitForSelector('.legal-modal-card', { timeout: 5000 });
  await page.waitForTimeout(300);

  const heroInVision = await page.locator('.legal-modal-card .legal-doc-hero').count();
  if (heroInVision > 0) {
    throw new Error('Hero header banner (.legal-doc-hero) should NOT exist!');
  }

  const visionText = await page.locator('.legal-modal-scrollable').innerText();
  if (visionText.includes(':')) {
    throw new Error(`Vision text contains unwanted colon ':' in: ${visionText}`);
  }
  if (/\([^\)]*\)/.test(visionText)) {
    throw new Error(`Vision text contains unwanted parentheses '(...)': ${visionText}`);
  }

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_login_vision_modal.png') });
  await page.locator('.legal-modal-close-btn').click();
  await page.waitForSelector('.legal-modal-card', { state: 'detached', timeout: 5000 });
  await page.waitForTimeout(200);

  // Check Terms of Service clean modal
  console.log('Step 3: Open Terms of Service modal on Login screen...');
  await legalLinks.filter({ hasText: /Điều khoản dịch vụ/i }).click();
  await page.waitForSelector('.legal-modal-card', { timeout: 5000 });
  await page.waitForTimeout(300);

  const termsText = await page.locator('.legal-modal-scrollable').innerText();
  if (termsText.includes(':')) {
    throw new Error(`Terms text contains unwanted colon ':' in: ${termsText}`);
  }
  if (/\([^\)]*\)/.test(termsText)) {
    throw new Error(`Terms text contains unwanted parentheses '(...)': ${termsText}`);
  }
  if (!termsText.includes('Quyền riêng tư là tuyệt đối vĩnh viễn và không thể đảo ngược!')) {
    throw new Error('Terms missing "Thứ nhất. Quyền riêng tư là tuyệt đối..."');
  }

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_login_terms_clean_modal.png') });
  await page.locator('.legal-modal-close-btn').click();
  await page.waitForSelector('.legal-modal-card', { state: 'detached', timeout: 5000 });
  await page.waitForTimeout(200);

  // 2. Sign In to App
  console.log('Step 4: Sign in and verify Settings -> About subtabs...');
  const uname = `v105_${Date.now().toString(36).slice(-5)}`;
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

  const subtabs = page.locator('.settings-subtabs .settings-subtab');
  const subtabTexts = await subtabs.allInnerTexts();
  console.log('About subtabs:', subtabTexts);

  if (!subtabTexts.some((t) => t.includes('Tầm nhìn sản phẩm'))) {
    throw new Error(`Expected "Tầm nhìn sản phẩm" subtab in About! Got: ${JSON.stringify(subtabTexts)}`);
  }

  // Click "Tầm nhìn sản phẩm" subtab
  console.log('Step 5: Click "Tầm nhìn sản phẩm" subtab...');
  const visionTab = subtabs.filter({ hasText: /Tầm nhìn sản phẩm/i }).first();
  await visionTab.click();
  await page.waitForSelector('.legal-simple-doc', { timeout: 5000 });
  await page.waitForTimeout(400);

  const heroInPanel = await page.locator('.settings-legal-panel .legal-doc-hero').count();
  if (heroInPanel > 0) {
    throw new Error('Hero header banner (.legal-doc-hero) should NOT exist in Settings tab!');
  }
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '03_settings_vision_tab.png') });

  // Click "Điều khoản dịch vụ" subtab
  console.log('Step 6: Click "Điều khoản dịch vụ" subtab...');
  const termsTab = subtabs.filter({ hasText: /Điều khoản/i }).first();
  await termsTab.click();
  await page.waitForSelector('.legal-simple-doc', { timeout: 5000 });
  await page.waitForTimeout(400);

  const termsDocText = await page.locator('.settings-legal-panel .legal-simple-doc').innerText();
  if (termsDocText.includes(':') || /\([^\)]*\)/.test(termsDocText)) {
    throw new Error('Terms in Settings contains colon or parentheses');
  }
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '04_settings_terms_clean_tab.png') });

  // Click "Chính sách bảo mật" subtab
  console.log('Step 7: Click "Chính sách bảo mật" subtab...');
  const privTab = subtabs.filter({ hasText: /Chính sách bảo mật|Bảo mật/i }).first();
  await privTab.click();
  await page.waitForSelector('.legal-simple-doc', { timeout: 5000 });
  await page.waitForTimeout(400);

  const privDocText = await page.locator('.settings-legal-panel .legal-simple-doc').innerText();
  if (privDocText.includes(':') || /\([^\)]*\)/.test(privDocText)) {
    throw new Error('Privacy in Settings contains colon or parentheses');
  }
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '05_settings_privacy_clean_tab.png') });

  // Click "Tiêu chuẩn cộng đồng" subtab
  console.log('Step 8: Click "Tiêu chuẩn cộng đồng" subtab...');
  const commTab = subtabs.filter({ hasText: /Tiêu chuẩn cộng đồng/i }).first();
  await commTab.click();
  await page.waitForSelector('.legal-simple-doc', { timeout: 5000 });
  await page.waitForTimeout(400);

  const commDocText = await page.locator('.settings-legal-panel .legal-simple-doc').innerText();
  if (commDocText.includes(':') || /\([^\)]*\)/.test(commDocText)) {
    throw new Error('Community in Settings contains colon or parentheses');
  }
  if (commDocText.includes('Chuẩn mực 1') || commDocText.includes('Chuẩn mực chung')) {
    throw new Error('Community text contains "Chuẩn mực" label which should have been removed');
  }
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '06_settings_community_clean_tab.png') });

  console.log('✅ Spec 105 verified successfully: Hero headers removed, all colons and parentheses eliminated, and Product Vision working perfectly!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
