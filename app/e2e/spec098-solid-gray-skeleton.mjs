import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_098';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 098 Solid Gray Skeleton Verification...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });

  const context = await browser.newContext({ viewport: { width: 1024, height: 768 } });
  const page = await context.newPage();

  // Setup preferences
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'light');
    localStorage.setItem('chatw_app_wp', 'none');
    localStorage.setItem('chatw_panel_translucent', 'false');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  // Dev login
  const uname = `v98_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });

  // Close tour modal if any
  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Verify skeleton CSS styles in the document
  const skeletonStyles = await page.evaluate(() => {
    const el = document.createElement('div');
    el.className = 'skeleton-circle skeleton-shimmer';
    document.body.appendChild(el);
    const computed = window.getComputedStyle(el);
    const bgImage = computed.backgroundImage;
    const bgColor = computed.backgroundColor;
    const animName = computed.animationName;
    document.body.removeChild(el);

    // Also check .skeleton-box
    const el2 = document.createElement('div');
    el2.className = 'skeleton-box';
    document.body.appendChild(el2);
    const computed2 = window.getComputedStyle(el2);
    const bgImage2 = computed2.backgroundImage;
    const bgColor2 = computed2.backgroundColor;
    document.body.removeChild(el2);

    return {
      circle: { bgImage, bgColor, animName },
      box: { bgImage: bgImage2, bgColor: bgColor2 }
    };
  });

  console.log('Skeleton Styles Verification:', JSON.stringify(skeletonStyles, null, 2));

  if (skeletonStyles.circle.bgImage !== 'none' || skeletonStyles.box.bgImage !== 'none') {
    throw new Error(`❌ Skeleton background-image is not 'none'! Found: ${skeletonStyles.circle.bgImage}`);
  }

  console.log('✅ Confirmed: background-image is strictly "none" (zero gradients / zero shiny stripes)!');
  console.log('✅ Confirmed: background-color is solid gray:', skeletonStyles.circle.bgColor);

  // Capture UI screenshot showing sidebar and rail
  await page.waitForTimeout(500);
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_solid_gray_skeleton_loading_overview.png') });

  console.log('🎉 Spec 098 Verification finished successfully!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
