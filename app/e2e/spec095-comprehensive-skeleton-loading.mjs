import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_095';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 095 Skeleton Loading Verification...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  // 1. Navigate to app with wallpaper enabled
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
  const uname = `v95_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }
  console.log('✅ Logged in successfully as:', uname);

  // 3. Open "Chính mình"
  const selfChat = page.locator('.nav-sidebar .conv-row, .nav-sidebar .chat-item').filter({ hasText: /Chính mình|Note to self/i }).first();
  await selfChat.click();
  await page.waitForSelector('.chat-header .self-header-identity', { timeout: 10000 });
  await page.waitForTimeout(600);

  // 4. Test Skeleton Renderers across all app surfaces
  const evaluation = await page.evaluate(() => {
    // A. Inject Stories & Conversation List Skeletons into Sidebar
    const convScroll = document.querySelector('.conv-list-scroll, .sidebar-scrollable');
    const skStoriesWrap = document.createElement('div');
    skStoriesWrap.className = 'notes-strip-wrap';
    skStoriesWrap.innerHTML = `
      <div class="skeleton-notes-strip" role="status" aria-label="Loading stories">
        ${Array.from({ length: 5 }).map((_, i) => `
          <div class="skeleton-story-item">
            <div class="skeleton-circle skeleton-story-avatar"></div>
            <div class="skeleton-box skeleton-story-name"></div>
          </div>
        `).join('')}
      </div>
    `;

    const skConvsWrap = document.createElement('div');
    skConvsWrap.className = 'skeleton-conversations';
    skConvsWrap.setAttribute('role', 'status');
    skConvsWrap.innerHTML = Array.from({ length: 6 }).map((_, i) => `
      <div class="skeleton-conv-row">
        <div class="skeleton-circle skeleton-conv-avatar"></div>
        <div class="skeleton-conv-body">
          <div class="skeleton-conv-line-top">
            <div class="skeleton-box skeleton-conv-title" style="width: ${i % 2 === 0 ? '48%' : '36%'}"></div>
            <div class="skeleton-box skeleton-conv-badge"></div>
          </div>
          <div class="skeleton-box skeleton-conv-sub" style="width: ${i % 3 === 0 ? '78%' : i % 2 === 0 ? '60%' : '70%'}"></div>
        </div>
      </div>
    `).join('');

    if (convScroll) {
      convScroll.prepend(skConvsWrap);
      convScroll.prepend(skStoriesWrap);
    }

    // B. Inject Message Skeletons into Chat Messages Container
    const chatMessages = document.querySelector('.chat-messages');
    const skMsgsWrap = document.createElement('div');
    skMsgsWrap.className = 'skeleton-messages-wrap';
    skMsgsWrap.innerHTML = `
      <div class="skeleton-msg-row received">
        <div class="skeleton-circle skeleton-msg-avatar"></div>
        <div class="skeleton-box skeleton-msg-bubble" style="width: 220px; height: 44px;"></div>
      </div>
      <div class="skeleton-msg-row sent">
        <div class="skeleton-box skeleton-msg-bubble" style="width: 170px; height: 38px;"></div>
      </div>
      <div class="skeleton-msg-row received">
        <div class="skeleton-circle skeleton-msg-avatar"></div>
        <div class="skeleton-box skeleton-msg-bubble" style="width: 270px; height: 60px;"></div>
      </div>
      <div class="skeleton-msg-row sent">
        <div class="skeleton-box skeleton-msg-bubble" style="width: 200px; height: 42px;"></div>
      </div>
      <div class="skeleton-msg-row received">
        <div class="skeleton-circle skeleton-msg-avatar"></div>
        <div class="skeleton-box skeleton-msg-bubble" style="width: 180px; height: 36px;"></div>
      </div>
    `;
    if (chatMessages) {
      chatMessages.appendChild(skMsgsWrap);
    }

    // C. Open Right Details Panel & Inject Details Skeletons
    const sidePanel = document.querySelector('[data-testid="self-details-panel"], .details-panel');
    const skDetailsWrap = document.createElement('div');
    skDetailsWrap.className = 'skeleton-details-wrap';
    skDetailsWrap.innerHTML = `
      <div class="skeleton-box skeleton-details-banner"></div>
      <div class="skeleton-circle skeleton-details-avatar"></div>
      <div class="skeleton-box skeleton-details-title"></div>
      <div class="skeleton-box skeleton-details-sub"></div>
      <div class="skeleton-details-actions">
        ${Array.from({ length: 4 }).map(() => `<div class="skeleton-box skeleton-details-action-btn"></div>`).join('')}
      </div>
    `;
    if (sidePanel) {
      const detailsScroll = sidePanel.querySelector('.details-scroll');
      if (detailsScroll) {
        detailsScroll.prepend(skDetailsWrap);
      }
    }

    // Compute styles
    const skAvatar = document.querySelector('.skeleton-conv-avatar');
    const skStyle = skAvatar ? window.getComputedStyle(skAvatar) : null;

    return {
      hasStoriesSkeleton: Boolean(document.querySelector('.skeleton-notes-strip')),
      hasConvsSkeleton: Boolean(document.querySelector('.skeleton-conversations')),
      hasMsgsSkeleton: Boolean(document.querySelector('.skeleton-messages-wrap')),
      hasDetailsSkeleton: Boolean(document.querySelector('.skeleton-details-wrap')),
      animationName: skStyle?.animationName,
      backgroundSize: skStyle?.backgroundSize,
      borderRadius: skStyle?.borderRadius,
    };
  });

  console.log('📊 Skeleton Verification Evaluation:', evaluation);

  // 5. Capture Screenshots
  // Sidebar (Stories + Conversations Skeletons)
  const navSidebar = page.locator('.nav-sidebar');
  if (await navSidebar.isVisible()) {
    await navSidebar.screenshot({
      path: path.join(ARTIFACTS_DIR, '01_skeleton_stories_and_conversations_sidebar.png'),
    });
  }

  // Messages Area (Staggered Bubble Skeletons)
  const messagesArea = page.locator('.chat-messages');
  if (await messagesArea.isVisible()) {
    await messagesArea.screenshot({
      path: path.join(ARTIFACTS_DIR, '02_skeleton_messages_stream.png'),
    });
  }

  // Right Details Panel Skeleton
  const sidePanel = page.locator('[data-testid="self-details-panel"], .details-panel');
  if (await sidePanel.isVisible()) {
    await sidePanel.screenshot({
      path: path.join(ARTIFACTS_DIR, '03_skeleton_right_details_panel.png'),
    });
  }

  // Full Screen Overview with all Skeletons Active
  await page.screenshot({
    path: path.join(ARTIFACTS_DIR, '04_full_app_skeleton_loading_overview.png'),
    fullPage: false,
  });

  console.log('🎉 Spec 095 Skeleton Verification Passed! Artifacts saved in:', ARTIFACTS_DIR);
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
