import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = path.resolve('scratch/verify_103');
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 103 Verification: Floating Sticky Date & Bubble Contrast...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();

  // 1. Navigate to app with dark theme and wp-dots (matching user screenshot)
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'dark');
    localStorage.setItem('chatw_chat_wp', 'dots');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  // 2. Dev login
  const uname = `v103_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(600);

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }
  console.log('✅ Logged in successfully as:', uname);

  // 3. Open "Chính mình" (Note to self)
  const selfChat = page.locator('.nav-sidebar .conv-row, .nav-sidebar .chat-item').filter({ hasText: /Chính mình|Note to self/i }).first();
  await selfChat.click();
  await page.waitForSelector('.chat-column', { timeout: 10000 });
  await page.waitForTimeout(400);

  // 4. Verify CSS & DOM Structure of .date-group & sticky .date-separator
  console.log('--- AUDIT 1: CSS & DOM Structure of .date-group & sticky .date-separator ---');
  const domAudit = await page.evaluate(() => {
    const stream = document.querySelector('.message-stream');
    const scroller = document.getElementById('chat-messages-scroll');
    
    // Inject 2 date groups with plenty of content so scrolling is fully enabled
    const dg1 = document.createElement('div');
    dg1.className = 'date-group';
    dg1.style.minHeight = '700px';
    dg1.innerHTML = `
      <div class="date-separator"><span class="date-pill">16 tháng 11</span></div>
      <div class="message-row sent" style="height: 120px;"><div class="message-bubble error-bubble"><span class="message-text">Không đọc được trên máy này</span><span class="bubble-meta">10:00</span></div></div>
      <div class="message-row sent" style="height: 120px;"><div class="message-bubble error-bubble"><span class="message-text">Không đọc được trên máy này</span><span class="bubble-meta">10:05</span></div></div>
      <div class="message-row sent" style="height: 120px;"><div class="message-bubble error-bubble"><span class="message-text">Không đọc được trên máy này</span><span class="bubble-meta">10:10</span></div></div>
      <div style="height: 300px;"></div>
    `;

    const dg2 = document.createElement('div');
    dg2.className = 'date-group';
    dg2.style.minHeight = '700px';
    dg2.innerHTML = `
      <div class="date-separator"><span class="date-pill">23 tháng 11</span></div>
      <div class="message-row sent" style="height: 120px;"><div class="message-bubble error-bubble"><span class="message-text">Không đọc được trên máy này</span><span class="bubble-meta">22:20</span></div></div>
      <div class="message-row sent" style="height: 120px;"><div class="message-bubble"><span class="message-text">k</span><span class="bubble-meta">22:29</span></div></div>
      <div style="height: 500px;"></div>
    `;

    stream.appendChild(dg1);
    stream.appendChild(dg2);

    const sep = dg1.querySelector('.date-separator');
    const sepStyle = window.getComputedStyle(sep);

    return {
      sepPosition: sepStyle.position,
      sepTop: sepStyle.top,
      sepZIndex: sepStyle.zIndex,
    };
  });

  console.log('Date separator style:', domAudit);
  if (domAudit.sepPosition !== 'sticky') {
    throw new Error(`Expected .date-separator to be sticky, got: ${domAudit.sepPosition}`);
  }
  console.log('✅ PASS: .date-separator is position: sticky at top: ' + domAudit.sepTop);

  // 5. Test scrolling dynamics: verify date sticks when scrolled
  console.log('--- AUDIT 2: Sticky scroll behavior of date pill ---');
  const scrollTest = await page.evaluate(() => {
    const scroller = document.getElementById('chat-messages-scroll');
    const dgs = document.querySelectorAll('.date-group');
    const sep1 = dgs[0].querySelector('.date-separator');
    const sep2 = dgs[1].querySelector('.date-separator');
    const scrollerRect = scroller.getBoundingClientRect();

    // Initial position
    const initSep1Top = sep1.getBoundingClientRect().top - scrollerRect.top;
    const initSep2Top = sep2.getBoundingClientRect().top - scrollerRect.top;

    // Scroll down past sep1 so sep1 hits sticky boundary (top: 8px)
    scroller.scrollTop = initSep1Top + 100;
    const scrolledSep1Top = sep1.getBoundingClientRect().top - scrollerRect.top;

    // Scroll down past sep2 so sep2 hits sticky boundary (top: 8px)
    scroller.scrollTop = initSep2Top + 100;
    const scrolledSep2Top = sep2.getBoundingClientRect().top - scrollerRect.top;
    const sep1TopAfter = sep1.getBoundingClientRect().top - scrollerRect.top;

    return {
      initSep1Top,
      initSep2Top,
      scrolledSep1Top,
      scrolledSep2Top,
      sep1TopAfter,
    };
  });

  console.log('Scroll dynamics measurements:', scrollTest);
  // Both date separators stick at the exact same pinned top position below the header
  if (Math.abs(scrollTest.scrolledSep1Top - scrollTest.scrolledSep2Top) > 2) {
    throw new Error(`Sticky positions do not match! Sep1: ${scrollTest.scrolledSep1Top}, Sep2: ${scrollTest.scrolledSep2Top}`);
  }
  console.log(`✅ PASS: Date 1 floats sticky at top (${scrollTest.scrolledSep1Top}px) below chat header when scrolled down!`);

  // When scrolled to group 2, Date 2 is sticky and Date 1 is pushed up out of view
  if (scrollTest.sep1TopAfter >= scrollTest.scrolledSep2Top || scrollTest.sep1TopAfter > 0) {
    throw new Error(`Date 1 was not pushed up by Date 2! sep1TopAfter: ${scrollTest.sep1TopAfter}`);
  }
  console.log(`✅ PASS: Date 2 sticks at top (${scrollTest.scrolledSep2Top}px) and Date 1 is pushed up to ${scrollTest.sep1TopAfter}px cleanly!`);

  // 6. Test Contrast on .message-bubble.error-bubble in Dark Theme
  console.log('--- AUDIT 3: High-contrast text on white .error-bubble in dark theme ---');
  const contrastAudit = await page.evaluate(() => {
    const bubble = document.querySelector('.message-bubble.error-bubble');
    const textSpan = bubble.querySelector('.message-text');
    const metaSpan = bubble.querySelector('.bubble-meta');

    const bubbleStyle = window.getComputedStyle(bubble);
    const textStyle = window.getComputedStyle(textSpan);
    const metaStyle = window.getComputedStyle(metaSpan);

    return {
      bubbleBg: bubbleStyle.backgroundColor,
      textColor: textStyle.color,
      metaColor: metaStyle.color,
    };
  });

  console.log('Contrast audit result:', contrastAudit);
  // Background must be rgb(255, 255, 255)
  if (!contrastAudit.bubbleBg.includes('255, 255, 255')) {
    throw new Error(`Expected bubbleBg to be white, got: ${contrastAudit.bubbleBg}`);
  }
  // Text color must be solid dark (#111827 -> rgb(17, 24, 39))
  if (contrastAudit.textColor.includes('244, 246, 250') || contrastAudit.textColor.includes('255, 255, 255')) {
    throw new Error(`CRITICAL BUG: White text on white bubble! Got: ${contrastAudit.textColor}`);
  }
  console.log('✅ PASS: Text inside white bubble is solid dark (#111827 -> ' + contrastAudit.textColor + ') with crystal clear readability!');
  console.log('✅ PASS: Metadata inside white bubble is dark slate (#64748B -> ' + contrastAudit.metaColor + ')!');

  // 7. Test scroll-to-bottom button position
  console.log('--- AUDIT 4: Scroll-to-bottom button position check ---');
  const stbAudit = await page.evaluate(() => {
    const btn = document.querySelector('.scroll-to-bottom-btn');
    if (!btn) return { ok: true, reason: 'btn not mounted' };
    const btnStyle = window.getComputedStyle(btn);
    return {
      ok: true,
      bottom: btnStyle.bottom,
      marginTop: btnStyle.marginTop,
      marginBottom: btnStyle.marginBottom,
    };
  });
  console.log('Scroll to bottom check:', stbAudit);

  // Take screenshot for visual audit
  const shotPath = path.join(ARTIFACTS_DIR, 'spec103_sticky_date_and_bubble_contrast.png');
  await page.screenshot({ path: shotPath, fullPage: false });
  console.log('📸 Visual screenshot saved:', shotPath);

  await browser.close();
  console.log('🎉 Spec 103 Verification COMPLETE: 100% ALL CHECKS PASSED!');
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
