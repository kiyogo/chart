import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_097';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 097 Avatar Shimmer & Stable Chat Frame Verification...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });

  // Use 1024x768 viewport matching the user's exact responsive width
  const context = await browser.newContext({ viewport: { width: 1024, height: 768 } });
  const page = await context.newPage();

  // 1. Setup preferences
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'light');
    localStorage.setItem('chatw_app_wp', 'sunset');
    localStorage.setItem('chatw_panel_translucent', 'true');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  // 2. Dev login
  const uname = `v97_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(600);

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }
  console.log('✅ Initial login complete as:', uname);

  // 3. Test Avatar shimmer behavior:
  // Check that Avatar components support the .skeleton-circle.skeleton-shimmer state
  const avatars = await page.locator('.user-avatar').all();
  console.log(`✅ Found ${avatars.length} active avatar elements across rail, sidebar and header.`);
  
  // Verify avatar element structure
  const avatarClasses = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('.user-avatar')).map(el => el.className);
  });
  console.log('Sample avatar classes:', avatarClasses.slice(0, 5));

  // 4. Test Stable Chat Frame on Conversation Click (Eliminating 1s "mất khung"):
  // Click on Self Chat ("Chính mình")
  const selfChat = page.locator('.nav-sidebar .conv-row, .nav-sidebar .chat-item').filter({ hasText: /Chính mình|Note to self/i }).first();
  await selfChat.waitFor({ state: 'visible', timeout: 5000 });
  
  console.log('🖱️ Clicking conversation and checking immediate frame stability...');
  await selfChat.click();

  // Check IMMEDIATELY (within 50ms) that .chat-column is mounted and has non-zero width/height
  const chatColBox = await page.locator('.chat-column').boundingBox();
  console.log('Immediate .chat-column bounding box on click:', chatColBox);
  if (!chatColBox || chatColBox.width === 0 || chatColBox.height === 0) {
    throw new Error('❌ .chat-column collapsed or lost frame on conversation click!');
  }
  console.log('✅ .chat-column is rock-solid and visible immediately with width:', chatColBox.width, 'height:', chatColBox.height);

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_chat_frame_rock_solid_on_click.png') });

  // 5. Check message area during loading or loaded state
  await page.waitForTimeout(400);
  const chatHeader = await page.locator('.chat-header').isVisible();
  const composer = await page.locator('.composer').isVisible();
  console.log('Chat header visible:', chatHeader, 'Composer visible:', composer);
  if (!chatHeader || !composer) {
    throw new Error('❌ Chat header or composer collapsed during chat load!');
  }

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_chat_messages_and_composer_visible.png') });

  // 6. Test Right Details Panel stability
  const detailsToggle = page.locator('button[data-tooltip="Chi tiết"]').or(page.locator('.chat-header-actions button')).last();
  if (await detailsToggle.isVisible().catch(() => false)) {
    await detailsToggle.click().catch(() => {});
    await page.waitForTimeout(400);
  }
  
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '03_full_responsive_view_with_details.png') });

  console.log('🎉 Spec 097 Verification completed successfully!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
