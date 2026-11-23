import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_099';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 099 Header, Details & Hover Verification...');
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
    localStorage.setItem('chatw_app_wp', 'sunset');
    localStorage.setItem('chatw_panel_translucent', 'true');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  // Dev login
  const uname = `v99_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });

  // Close tour modal if any
  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // 1. Open Self Chat ("Chính mình")
  const selfChat = page.locator('.nav-sidebar .conv-row, .nav-sidebar .chat-item').filter({ hasText: /Chính mình|Note to self/i }).first();
  await selfChat.waitFor({ state: 'visible', timeout: 5000 });
  await selfChat.click();
  await page.waitForTimeout(500);

  // 2. Verify "Chính mình" header text has NO "thiết bị"
  const metaText = await page.locator('.chat-header .chat-header-meta').innerText();
  console.log('Header meta text:', metaText);
  if (metaText.includes('thiết bị') || metaText.includes('devices')) {
    throw new Error(`❌ Header meta still contains devices count: ${metaText}`);
  }
  console.log('✅ Confirmed: Header meta does NOT contain devices count!');

  // 3. Verify spacing between title and subtitle
  const titleBox = await page.locator('.chat-header .chat-header-title').boundingBox();
  const metaBox = await page.locator('.chat-header .chat-header-meta').boundingBox();
  console.log('Title box:', titleBox, 'Meta box:', metaBox);
  const gap = metaBox.y - (titleBox.y + titleBox.height);
  console.log(`Title to subtitle gap: ${gap}px`);
  if (gap > 4) {
    throw new Error(`❌ Gap between title and meta is too large: ${gap}px`);
  }
  console.log('✅ Confirmed: Subtitle is positioned close directly beneath the name!');

  // 4. Verify chat header actions are on the far right
  const headerBox = await page.locator('.chat-header').boundingBox();
  const actionsBox = await page.locator('.chat-header-actions').boundingBox();
  console.log('Header box:', headerBox, 'Actions box:', actionsBox);
  const rightDistance = (headerBox.x + headerBox.width) - (actionsBox.x + actionsBox.width);
  console.log(`Actions distance from right edge of header: ${rightDistance}px`);
  if (rightDistance > 30) {
    throw new Error(`❌ Actions are not on the far right! Distance from right edge: ${rightDistance}px`);
  }
  console.log('✅ Confirmed: Header action buttons are positioned on the far right!');

  // 5. Verify hover on header identity has ZERO background box (frameless)
  await page.locator('.chat-header .chat-header-identity').hover();
  await page.waitForTimeout(100);
  const identityHoverBg = await page.evaluate(() => {
    const el = document.querySelector('.chat-header .chat-header-identity');
    return window.getComputedStyle(el).backgroundColor;
  });
  console.log('Identity hover background:', identityHoverBg);
  if (identityHoverBg !== 'rgba(0, 0, 0, 0)' && identityHoverBg !== 'transparent') {
    throw new Error(`❌ Identity still has a hover background box: ${identityHoverBg}`);
  }
  console.log('✅ Confirmed: Header identity has zero background box on hover!');

  // 6. Verify hover on header action buttons has ZERO background box
  const actionBtn = page.locator('.chat-header-actions .icon-btn').first();
  await actionBtn.hover();
  await page.waitForTimeout(100);
  const actionHoverBg = await page.evaluate(() => {
    const el = document.querySelector('.chat-header-actions .icon-btn');
    return window.getComputedStyle(el).backgroundColor;
  });
  console.log('Action button hover background:', actionHoverBg);
  if (actionHoverBg !== 'rgba(0, 0, 0, 0)' && actionHoverBg !== 'transparent') {
    throw new Error(`❌ Action button still has a hover background box: ${actionHoverBg}`);
  }
  console.log('✅ Confirmed: Action buttons have zero background box on hover!');

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_clean_chat_header_right_aligned_frameless.png') });

  // 7. Open Details Panel and verify Search in Chat is GONE
  const toggleBtn = page.locator('button[data-testid="toggle-details"]').first();
  await toggleBtn.click();
  await page.waitForTimeout(400);

  const searchInputCount = await page.locator('input[placeholder*="Tìm tin nhắn"], #self-details-search-input').count();
  console.log('Search input count in details panel:', searchInputCount);
  if (searchInputCount > 0) {
    throw new Error(`❌ Search in chat input still exists in details panel!`);
  }
  console.log('✅ Confirmed: Search in chat is completely removed from details panel!');

  const selfDetailsSub = await page.locator('.self-details-sub').innerText();
  console.log('Self details subtext:', selfDetailsSub);
  if (selfDetailsSub.includes('thiết bị') || selfDetailsSub.includes('Devices')) {
    throw new Error(`❌ Self details subtext still contains devices count: ${selfDetailsSub}`);
  }
  console.log('✅ Confirmed: Self details subtext does NOT contain devices count!');

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_details_panel_no_search_box.png') });

  console.log('🎉 Spec 099 Verification finished successfully!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
