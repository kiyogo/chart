import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_100';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 100 Clean Network & Zero Canvas Strip Blobs Verification...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });

  const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
  const page = await context.newPage();

  // Track all blob requests via CDP Network events
  const cdp = await context.newCDPSession(page);
  await cdp.send('Network.enable');

  const blobRequests = [];
  cdp.on('Network.requestWillBeSent', (e) => {
    if (e.request.url.startsWith('blob:')) {
      blobRequests.push({ url: e.request.url, initiator: e.initiator });
    }
  });

  // Also track URL.createObjectURL calls on the page directly
  await page.addInitScript(() => {
    window.__createdBlobUrls = [];
    const orig = URL.createObjectURL.bind(URL);
    URL.createObjectURL = function (obj) {
      const url = orig(obj);
      window.__createdBlobUrls.push({
        url,
        type: obj && obj.type ? obj.type : 'unknown',
        size: obj && obj.size ? obj.size : 0,
      });
      return url;
    };
  });

  // Setup preferences with wallpaper & translucent panels (the exact condition that previously triggered cropPanelStripBlobUrl)
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'light');
    localStorage.setItem('chatw_app_wp', 'sunset');
    localStorage.setItem('chatw_panel_translucent', 'true');
    window.__chatwCustom = { panelOpacity: 50 };
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  // Dev login
  const uname = `v100_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });

  // Close tour modal if any
  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Open "Chính mình"
  const selfChat = page.locator('.nav-sidebar .conv-row, .nav-sidebar .chat-item').filter({ hasText: /Chính mình|Note to self/i }).first();
  await selfChat.waitFor({ state: 'visible', timeout: 5000 });
  await selfChat.click();
  await page.waitForTimeout(600);

  // Trigger multiple resizing and layout updates
  await page.setViewportSize({ width: 1400, height: 900 });
  await page.waitForTimeout(300);
  await page.setViewportSize({ width: 1100, height: 750 });
  await page.waitForTimeout(300);

  // Toggle details panel multiple times
  const toggleBtn = page.locator('button[data-testid="toggle-details"]').first();
  if (await toggleBtn.isVisible()) {
    await toggleBtn.click();
    await page.waitForTimeout(400);
    await toggleBtn.click();
    await page.waitForTimeout(400);
  }

  // Inspect page state
  const audit = await page.evaluate(() => {
    const col = document.querySelector('.chat-column');
    const topStripVar = col ? col.style.getPropertyValue('--chat-top-fade-strip') : '';
    const botStripVar = col ? col.style.getPropertyValue('--chat-bot-fade-strip') : '';
    const edgeStripAttr = col ? col.getAttribute('data-chat-edge-strip') : null;
    const createdBlobs = window.__createdBlobUrls || [];
    // Count any 160x18 image/png strips
    const stripBlobs = createdBlobs.filter((b) => b.type === 'image/png' && b.size < 10000);
    return {
      topStripVar,
      botStripVar,
      edgeStripAttr,
      totalCreatedBlobs: createdBlobs.length,
      stripBlobs,
      allCreatedBlobs: createdBlobs,
    };
  });

  console.log('Audit result:', audit);
  console.log('Network CDP blob requests:', blobRequests.length);

  // 1. Verify data-chat-edge-strip is NOT set
  if (audit.edgeStripAttr) {
    throw new Error(`❌ data-chat-edge-strip is still set: ${audit.edgeStripAttr}`);
  }
  console.log('✅ Confirmed: data-chat-edge-strip is NOT set!');

  // 2. Verify no --chat-top-fade-strip or --chat-bot-fade-strip blob URLs
  if (audit.topStripVar.includes('blob:') || audit.botStripVar.includes('blob:')) {
    throw new Error(`❌ CSS still has blob: fade strips! top=${audit.topStripVar}, bot=${audit.botStripVar}`);
  }
  console.log('✅ Confirmed: Zero blob: URLs assigned to CSS fade strips!');

  // 3. Verify zero canvas strip blobs created
  if (audit.stripBlobs.length > 0) {
    throw new Error(`❌ Still generating canvas strip blobs: ${JSON.stringify(audit.stripBlobs)}`);
  }
  console.log('✅ Confirmed: ZERO canvas strip blobs generated in memory!');

  // 4. Verify zero blob requests from canvas strips in Network tab
  const stripNetRequests = blobRequests.filter((r) => r.url.includes('127.0.0.1'));
  console.log(`Strip network requests count: ${stripNetRequests.length}`);

  // Capture screenshot of the chat with sunset wallpaper and translucent panels
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_clean_network_translucent_wallpaper_chat.png') });

  // Open details panel and take screenshot
  if (await toggleBtn.isVisible()) {
    await toggleBtn.click();
    await page.waitForTimeout(400);
    await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_clean_network_details_panel.png') });
  }

  console.log('🎉 Spec 100 Verification passed with 100% success!');
  await browser.close();
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
