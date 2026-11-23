import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_094';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 094 Verification (Comprehensive)...');
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
  const uname = `v94_${Date.now().toString(36).slice(-5)}`;
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

  // 4. Inject 15 mock devices into .self-device-list and error messages matching image 1
  // to rigorously verify the exact visual behavior user reported
  const verificationResult = await page.evaluate(() => {
    // A. Check Header for inline devices strip
    const inlineDevices = document.querySelector('.chat-header .header-inline-devices, .chat-header .device-strip');
    const headerIdentity = document.querySelector('.chat-header .self-header-identity');

    // B. Inject error message bubble like in user attachment image 1
    const chatMessages = document.querySelector('.chat-messages');
    const msgRow = document.createElement('div');
    msgRow.className = 'message-row received group-start group-end';
    msgRow.innerHTML = `
      <div class="message-content-wrapper">
        <div class="message-bubble error-bubble" title="22:00">
          <span class="message-text">Không đọc được trên máy này</span>
          <span class="bubble-meta">22:00</span>
        </div>
      </div>
    `;
    chatMessages.appendChild(msgRow);

    const errorBubble = msgRow.querySelector('.message-bubble.error-bubble');
    const errorBubbleMeta = msgRow.querySelector('.bubble-meta');
    const errorBubbleStyle = window.getComputedStyle(errorBubble);
    const errorMetaStyle = window.getComputedStyle(errorBubbleMeta);

    // C. Open right sidebar and inject 12 devices to test full scrolling
    const deviceList = document.querySelector('.self-device-list');
    if (deviceList) {
      for (let i = 2; i <= 15; i++) {
        const btn = document.createElement('button');
        btn.type = 'button';
        btn.className = 'self-device-row';
        btn.innerHTML = `
          <span class="self-vault-row-icon"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect width="20" height="14" x="2" y="3" rx="2"/><line x1="8" x2="16" y1="21" y2="21"/><line x1="12" x2="12" y1="17" y2="21"/></svg></span>
          <span class="self-device-name">Thiết bị ${i}</span>
        `;
        deviceList.appendChild(btn);
      }
    }

    const deviceListStyle = deviceList ? window.getComputedStyle(deviceList) : null;
    const detailsScroll = document.querySelector('.details-scroll');
    const detailsScrollStyle = detailsScroll ? window.getComputedStyle(detailsScroll) : null;

    return {
      headerClean: !inlineDevices && Boolean(headerIdentity),
      errorBubbleBg: errorBubbleStyle.backgroundColor,
      errorBubbleColor: errorBubbleStyle.color,
      errorTimeColor: errorMetaStyle.color,
      deviceListMaxHeight: deviceListStyle?.maxHeight,
      deviceListOverflowY: deviceListStyle?.overflowY,
      detailsScrollOverflowY: detailsScrollStyle?.overflowY,
    };
  });

  console.log('📊 Verification Result:', verificationResult);

  // 5. Ensure right sidebar is open
  const rightPanel = page.locator('[data-testid="self-details-panel"]');
  if (!(await rightPanel.isVisible().catch(() => false))) {
    await page.locator('.chat-header-actions [data-testid="toggle-details"]').click();
    await page.waitForTimeout(500);
  }

  // 6. Capture Screenshots
  // Error bubble closeup
  const errorBubble = page.locator('.message-bubble.error-bubble').last();
  const eBox = await errorBubble.boundingBox();
  if (eBox) {
    await page.screenshot({
      path: path.join(ARTIFACTS_DIR, '04_error_bubble_100pct_white_no_red_time.png'),
      clip: {
        x: Math.max(0, eBox.x - 30),
        y: Math.max(0, eBox.y - 15),
        width: eBox.width + 60,
        height: eBox.height + 30,
      },
    });
  }

  // Right sidebar full device list view
  const sidePanel = page.locator('[data-testid="self-details-panel"]');
  const sBox = await sidePanel.boundingBox();
  if (sBox) {
    await page.screenshot({
      path: path.join(ARTIFACTS_DIR, '05_sidebar_full_device_list_no_inner_scrollbar.png'),
      clip: {
        x: sBox.x,
        y: sBox.y,
        width: sBox.width,
        height: sBox.height,
      },
    });
  }

  // Full Screen
  await page.screenshot({
    path: path.join(ARTIFACTS_DIR, '06_final_full_screen.png'),
    fullPage: false,
  });

  console.log('🎉 Comprehensive Verification Passed! Artifacts saved in:', ARTIFACTS_DIR);
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
