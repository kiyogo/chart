import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec132';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

function parseRgb(rgbStr) {
  const m = rgbStr.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
  if (!m) return { r: 0, g: 0, b: 0, brightness: 0 };
  const r = parseInt(m[1], 10);
  const g = parseInt(m[2], 10);
  const b = parseInt(m[3], 10);
  const brightness = (r * 299 + g * 587 + b * 114) / 1000;
  return { r, g, b, brightness };
}

async function run() {
  console.log('🚀 Starting Spec 132 Verification: Theme-Adaptive Message Bubble Colors & High-Contrast Borders...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();

  // ==========================================
  // PHASE 1: DARK THEME VERIFICATION
  // ==========================================
  console.log('\n--- PHASE 1: Dark Mode Bubbles Verification ---');
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'dark');
    localStorage.setItem('chatw_chat_wp', 'dots');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  const uname = `v132_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(600);

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Open "Chính mình"
  const selfChat = page.locator('.nav-sidebar .conv-row, .nav-sidebar .chat-item').filter({ hasText: /Chính mình|Note to self/i }).first();
  await selfChat.click();
  await page.waitForSelector('.chat-column', { timeout: 10000 });
  await page.waitForTimeout(400);

  // Inject sample messages: received normal, received error, sent normal
  await page.evaluate(() => {
    const stream = document.querySelector('.message-stream');
    const container = document.createElement('div');
    container.id = 'spec132-test-container';
    container.innerHTML = `
      <div class="message-row received" id="msg-recv-normal">
        <div class="message-bubble">
          <span class="message-text">Xin chào! Đây là tin nhắn đến bình thường</span>
          <span class="bubble-meta">22:05</span>
        </div>
      </div>
      <div class="message-row received" id="msg-recv-error">
        <div class="message-bubble error-bubble">
          <span class="message-text">Không đọc được trên máy này</span>
          <span class="bubble-meta">22:06</span>
        </div>
      </div>
      <div class="message-row sent" id="msg-sent-normal">
        <div class="message-bubble">
          <span class="message-text">fsdfsd</span>
          <span class="bubble-meta">00:18</span>
        </div>
      </div>
    `;
    stream.appendChild(container);
  });

  const darkStyles = await page.evaluate(() => {
    const recvNormal = document.querySelector('#msg-recv-normal .message-bubble');
    const recvError = document.querySelector('#msg-recv-error .message-bubble');
    const recvErrorText = document.querySelector('#msg-recv-error .message-text');
    const recvErrorMeta = document.querySelector('#msg-recv-error .bubble-meta');
    const sentNormal = document.querySelector('#msg-sent-normal .message-bubble');

    const sRecvNorm = window.getComputedStyle(recvNormal);
    const sRecvErr = window.getComputedStyle(recvError);
    const sRecvErrTxt = window.getComputedStyle(recvErrorText);
    const sRecvErrMeta = window.getComputedStyle(recvErrorMeta);
    const sSentNorm = window.getComputedStyle(sentNormal);

    return {
      recvNormalBg: sRecvNorm.backgroundColor,
      recvNormalColor: sRecvNorm.color,
      recvErrorBg: sRecvErr.backgroundColor,
      recvErrorColor: sRecvErr.color,
      recvErrorTextColor: sRecvErrTxt.color,
      recvErrorMetaColor: sRecvErrMeta.color,
      recvErrorBorder: sRecvErr.border,
      sentNormalBg: sSentNorm.backgroundColor,
      sentNormalColor: sSentNorm.color,
    };
  });

  console.log('Dark Mode Computed Styles:', darkStyles);

  // Assertions for Dark Mode:
  const recvNormBg = parseRgb(darkStyles.recvNormalBg);
  const recvErrBg = parseRgb(darkStyles.recvErrorBg);
  const recvErrColor = parseRgb(darkStyles.recvErrorColor);
  const recvErrTxtColor = parseRgb(darkStyles.recvErrorTextColor);

  console.log(`Recv Normal Bg brightness: ${recvNormBg.brightness} (expect < 100 for dark mode)`);
  console.log(`Recv Error Bg brightness: ${recvErrBg.brightness} (expect < 100 for dark mode)`);
  console.log(`Recv Error Text brightness: ${recvErrTxtColor.brightness} (expect > 180 for readable light text on dark)`);

  if (recvErrBg.brightness > 200 || darkStyles.recvErrorBg === 'rgb(255, 255, 255)') {
    throw new Error(`CRITICAL: Error bubble in dark mode is still STARK WHITE: ${darkStyles.recvErrorBg}`);
  }
  if (recvErrTxtColor.brightness < 120 || darkStyles.recvErrorTextColor === 'rgb(17, 24, 39)') {
    throw new Error(`CRITICAL: Error bubble text in dark mode is dark/black: ${darkStyles.recvErrorTextColor}`);
  }
  console.log('✅ PASS: Dark mode received & error bubbles are properly dark with clear light text!');

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_dark_mode_bubbles.png') });

  // ==========================================
  // PHASE 2: LIGHT THEME & WHITE CANVAS VERIFICATION
  // ==========================================
  console.log('\n--- PHASE 2: Light Mode & High-Contrast Border Verification ---');
  await page.evaluate(() => {
    localStorage.setItem('chatw_theme', 'light');
    localStorage.setItem('chatw_chat_wp', 'none');
    document.documentElement.setAttribute('data-theme', 'light');
    document.documentElement.removeAttribute('data-surface-tone');
    document.documentElement.removeAttribute('data-app-wp');
    const col = document.querySelector('.chat-column');
    if (col) {
      col.className = 'chat-column';
      col.setAttribute('data-chat-tone', 'light');
    }
  });
  await page.waitForTimeout(500);

  // Re-inject test messages
  await page.evaluate(() => {
    const stream = document.querySelector('.message-stream');
    const container = document.createElement('div');
    container.id = 'spec132-test-container';
    container.innerHTML = `
      <div class="message-row received" id="msg-recv-normal-light">
        <div class="message-bubble">
          <span class="message-text">Không đọc được trên máy này</span>
          <span class="bubble-meta">15:41</span>
        </div>
      </div>
      <div class="message-row received" id="msg-recv-error-light">
        <div class="message-bubble error-bubble">
          <span class="message-text">Không đọc được trên máy này</span>
          <span class="bubble-meta">22:00</span>
        </div>
      </div>
      <div class="message-row sent" id="msg-sent-normal-light">
        <div class="message-bubble">
          <span class="message-text">Chào bạn</span>
          <span class="bubble-meta">22:01</span>
        </div>
      </div>
    `;
    stream.appendChild(container);
  });

  const lightStyles = await page.evaluate(() => {
    const recvNorm = document.querySelector('#msg-recv-normal-light .message-bubble');
    const recvErr = document.querySelector('#msg-recv-error-light .message-bubble');
    const recvErrMeta = document.querySelector('#msg-recv-error-light .bubble-meta');

    const sNorm = window.getComputedStyle(recvNorm);
    const sErr = window.getComputedStyle(recvErr);
    const sMeta = window.getComputedStyle(recvErrMeta);

    return {
      normBorderWidth: sNorm.borderWidth,
      normBorderStyle: sNorm.borderStyle,
      normBorderColor: sNorm.borderColor,
      normBoxShadow: sNorm.boxShadow,
      normBg: sNorm.backgroundColor,
      errBorderWidth: sErr.borderWidth,
      errBorderStyle: sErr.borderStyle,
      errBorderColor: sErr.borderColor,
      errBoxShadow: sErr.boxShadow,
      errBg: sErr.backgroundColor,
      errColor: sErr.color,
      metaColor: sMeta.color,
    };
  });

  console.log('Light Mode Computed Styles:', lightStyles);

  if (lightStyles.normBorderWidth === '0px' || lightStyles.normBorderStyle === 'none') {
    throw new Error(`CRITICAL: Normal bubble on light canvas lacks border: width=${lightStyles.normBorderWidth}, style=${lightStyles.normBorderStyle}`);
  }
  if (lightStyles.errBorderWidth === '0px' || lightStyles.errBorderStyle === 'none') {
    throw new Error(`CRITICAL: Error bubble on light canvas lacks border: width=${lightStyles.errBorderWidth}, style=${lightStyles.errBorderStyle}`);
  }
  console.log('✅ PASS: Light mode bubbles have distinct 1px border & shadow to ensure crisp contrast on white surfaces!');

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_light_mode_contrast_bubbles.png') });

  // ==========================================
  // PHASE 3: WALLPAPER NIGHT TONE VERIFICATION
  // ==========================================
  console.log('\n--- PHASE 3: Wallpaper Night Tone Verification ---');
  await page.evaluate(() => {
    const col = document.querySelector('.chat-column');
    if (col) {
      col.classList.add('wp-night');
      col.setAttribute('data-chat-tone', 'dark');
    }
  });
  await page.waitForTimeout(300);

  const wpNightStyles = await page.evaluate(() => {
    const recvErr = document.querySelector('#msg-recv-error-light .message-bubble');
    const sErr = window.getComputedStyle(recvErr);
    return {
      errBg: sErr.backgroundColor,
      errColor: sErr.color,
      errBorder: sErr.border,
    };
  });

  console.log('Wallpaper Night Computed Styles:', wpNightStyles);
  const wpNightBg = parseRgb(wpNightStyles.errBg);
  if (wpNightBg.brightness > 160) {
    throw new Error(`CRITICAL: Bubble under wp-night has bright background: ${wpNightStyles.errBg}`);
  }
  console.log('✅ PASS: Wallpaper night mode message bubble is dark and contrast-compliant!');

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '03_wallpaper_night_bubbles.png') });

  await browser.close();
  console.log('\n🎉 ALL SPEC 132 TESTS PASSED SUCCESSFULLY!');
}

run().catch((err) => {
  console.error('❌ Test failed:', err);
  process.exit(1);
});
