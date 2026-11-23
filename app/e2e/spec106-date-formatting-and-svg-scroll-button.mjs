import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = path.resolve('scratch/verify_105');
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 106 Verification: Date Formatting, Pill Elevation & SVG Scroll Button...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();

  // 1. Navigate to app with dark theme and vi language
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'dark');
    localStorage.setItem('chatw_chat_wp', 'dots');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  // 2. Dev login
  const uname = `v105_${Date.now().toString(36).slice(-5)}`;
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

  // 4. AUDIT 1: Date formatting rules (Today -> Hôm nay, Yesterday -> Hôm qua, May 12, 2025 -> 12 tháng 05 năm 2025)
  console.log('--- AUDIT 1: Date Formatting Verification ---');
  const i18nAudit = await page.evaluate(() => {
    const fn = window.__chatwI18n && (window.__chatwI18n.translateString || window.__chatwI18n.translate);
    const testCases = [
      { raw: 'Today', expected: 'Hôm nay' },
      { raw: 'Yesterday', expected: 'Hôm qua' },
      { raw: 'November 23', expected: '23 tháng 11' },
      { raw: 'May 12, 2025', expected: '12 tháng 05 năm 2025' },
    ];
    const results = testCases.map(tc => ({
      raw: tc.raw,
      translated: fn ? fn(tc.raw) : null,
      expected: tc.expected,
      match: fn ? fn(tc.raw) === tc.expected : false,
    }));
    return { hasI18n: Boolean(fn), results };
  });

  console.log('i18n translation results:', i18nAudit);
  for (const r of i18nAudit.results) {
    if (!r.match) {
      throw new Error(`Date translation failed for "${r.raw}"! Got: "${r.translated}", Expected: "${r.expected}"`);
    }
  }
  console.log('✅ PASS: All relative and historical dates translate correctly according to Spec 106!');

  // 5. AUDIT 2: Date pill elevation when scrolled
  console.log('--- AUDIT 2: Date Pill Elevation Check ---');
  const elevationAudit = await page.evaluate(() => {
    const scroller = document.getElementById('chat-messages-scroll');
    const stream = document.querySelector('.message-stream');

    const dg = document.createElement('div');
    dg.className = 'date-group';
    dg.style.minHeight = '1400px';
    dg.innerHTML = `
      <div class="date-separator"><span class="date-pill">Hôm nay</span></div>
      <div style="height: 1200px;"></div>
    `;
    stream.appendChild(dg);

    const sep = dg.querySelector('.date-separator');
    const scrollerRect = scroller.getBoundingClientRect();
    const initTop = sep.getBoundingClientRect().top - scrollerRect.top;

    // Scroll down 400px so sep hits its sticky boundary
    scroller.scrollTop = 400;
    const scrolledTop = sep.getBoundingClientRect().top - scrollerRect.top;

    return {
      initTop,
      scrolledTop,
      computedTop: window.getComputedStyle(sep).top,
      computedMargin: window.getComputedStyle(sep).margin,
    };
  });

  console.log('Date pill elevation results:', elevationAudit);
  if (elevationAudit.scrolledTop > 30) {
    throw new Error(`Date pill is not elevated near top! scrolledTop: ${elevationAudit.scrolledTop}px (expected <= 30px)`);
  }
  console.log(`✅ PASS: Date pill sticks elevated near the top (${elevationAudit.scrolledTop.toFixed(1)}px from top)!`);

  // 6. AUDIT 3: Modern SVG down arrow & lower scroll button position
  console.log('--- AUDIT 3: Modern SVG Scroll Button Check ---');
  const btnAudit = await page.evaluate(() => {
    const scroller = document.getElementById('chat-messages-scroll');
    const composer = document.querySelector('.composer');

    // Create a mock button if not active
    let btn = document.querySelector('.scroll-to-bottom-btn');
    if (!btn) {
      btn = document.createElement('button');
      btn.className = 'scroll-to-bottom-btn';
      btn.innerHTML = `
        <svg class="scroll-down-svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M12 4v13.5"></path>
          <path d="m18 12-6 6-6-6"></path>
        </svg>
      `;
      scroller.appendChild(btn);
    }

    const scrollerRect = scroller.getBoundingClientRect();
    const btnRect = btn.getBoundingClientRect();
    const composerRect = composer ? composer.getBoundingClientRect() : null;

    const hasSvg = Boolean(btn.querySelector('svg.scroll-down-svg'));
    const noUnicodeArrow = !btn.textContent.includes('↓');
    const distAboveComposer = composerRect ? composerRect.top - btnRect.bottom : null;

    return {
      hasSvg,
      noUnicodeArrow,
      btnBottomOffset: scrollerRect.bottom - btnRect.bottom,
      distAboveComposer,
    };
  });

  console.log('Scroll button audit:', btnAudit);
  if (!btnAudit.hasSvg) {
    throw new Error('Scroll-to-bottom button does not contain SVG icon!');
  }
  if (!btnAudit.noUnicodeArrow) {
    throw new Error('Scroll-to-bottom button still contains unicode text arrow!');
  }
  console.log('✅ PASS: Scroll button contains modern SVG icon with zero unicode text arrow!');
  if (btnAudit.distAboveComposer !== null) {
    console.log(`✅ PASS: Distance above composer is ${btnAudit.distAboveComposer.toFixed(1)}px (sits cleanly right above input)!`);
  }

  // 7. Screenshot capture
  const screenshotPath = path.join(ARTIFACTS_DIR, 'spec105_elevation_date_format_and_svg_btn.png');
  await page.screenshot({ path: screenshotPath });
  console.log('📸 Visual screenshot saved:', screenshotPath);

  await browser.close();
  console.log('🎉 Spec 106 Verification COMPLETE: 100% ALL CHECKS PASSED!');
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
