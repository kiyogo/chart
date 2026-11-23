import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_093';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 093 Settings Sub-Tabs Border Verification (with Wallpaper)...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  // 1. Navigate to app
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'light');
    localStorage.setItem('chatw_app_wp', 'sunset');
    localStorage.setItem('chatw_panel_translucent', 'true');
    document.documentElement.setAttribute('data-app-wp', 'sunset');
    document.documentElement.setAttribute('data-panel-translucent', 'true');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  // 2. Dev login
  const uname = `v93_wp_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // 3. Open Settings via app-rail
  const settingsBtn = await page.waitForSelector('button[aria-label="Cài đặt"], button[title="Cài đặt"], [data-testid="rail-settings"], .rail-btn[data-tab="settings"], button:has(.lucide-settings)');
  await settingsBtn.click();
  await page.waitForTimeout(1000);

  // 4. Click on "Riêng tư & bảo mật" in settings sidebar
  const privacyNav = await page.waitForSelector('.settings-nav-item:has-text("Riêng tư"), .settings-nav-item:has-text("Bảo mật")');
  await privacyNav.click();
  await page.waitForTimeout(1000);

  // 5. Inspect computed border styles of .settings-subtabs
  const subtabs = await page.waitForSelector('.settings-subtabs');
  const borderBottomInfo = await page.evaluate(() => {
    const el = document.querySelector('.settings-subtabs');
    if (!el) return null;
    const style = window.getComputedStyle(el);
    return {
      borderBottomWidth: style.borderBottomWidth,
      borderBottomStyle: style.borderBottomStyle,
      borderBottomColor: style.borderBottomColor,
    };
  });
  console.log('📊 Computed .settings-subtabs borderBottom (with wallpaper):', borderBottomInfo);

  // 6. Screenshot Settings View with Wallpaper
  await page.screenshot({
    path: path.join(ARTIFACTS_DIR, '04_wallpaper_settings_privacy_no_grey_bar.png'),
    fullPage: false,
  });

  // 7. Closeup screenshot of the subtabs area with wallpaper
  const box = await subtabs.boundingBox();
  if (box) {
    await page.screenshot({
      path: path.join(ARTIFACTS_DIR, '05_wallpaper_closeup_subtabs_no_grey_bar.png'),
      clip: {
        x: Math.max(0, box.x - 20),
        y: Math.max(0, box.y - 15),
        width: Math.min(800, box.width + 40),
        height: box.height + 90,
      },
    });
  }

  console.log('🎉 Verification completed successfully! All screenshots saved.');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
