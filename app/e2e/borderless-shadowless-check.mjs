// Live Chromium verification for Spec 028: Remove Faint Dark Borders & Blur Shadows App-Wide
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_028';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

const browser = await chromium.launch({
  executablePath: '/usr/bin/google-chrome',
  headless: true,
  args: ['--no-sandbox', '--disable-gpu'],
});

const errors = [];
try {
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(e.message));

  const suffix = `${Date.now().toString(36).slice(-5)}${Math.floor(Math.random() * 90 + 10)}`;
  await page.goto(`${BASE}/`);
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`flat_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // 1. MeetView (from user screenshot)
  await page.locator('.app-rail button[aria-label="Họp nhóm"]').click();
  await page.waitForSelector('.meet-hero-card', { timeout: 10000 });
  await page.waitForTimeout(300);
  const meetAudit = await page.evaluate(() => {
    const col = document.querySelector('.meet-column');
    const cards = Array.from(document.querySelectorAll('.meet-column .feed-card'));
    return {
      colBg: col ? getComputedStyle(col).backgroundColor : null,
      cardCount: cards.length,
      cardShadows: cards.map((c) => getComputedStyle(c).boxShadow),
      cardBorders: cards.map((c) => getComputedStyle(c).borderTopWidth + '/' + getComputedStyle(c).borderTopStyle),
      cardBgs: cards.map((c) => getComputedStyle(c).backgroundColor),
    };
  });
  record(
    'MeetView cards shadowless & borderless on tonal canvas',
    meetAudit.cardCount === 4 &&
      meetAudit.cardShadows.every((s) => s === 'none') &&
      meetAudit.cardBorders.every((b) => b.endsWith('none') || b.startsWith('0px')) &&
      meetAudit.colBg !== meetAudit.cardBgs[0],
    JSON.stringify(meetAudit)
  );
  await page.screenshot({ path: path.join(OUT, '01_meet_view.png') });

  // 2. FeedView
  await page.locator('.app-rail button[aria-label="Mới"]').click();
  await page.waitForSelector('.feed-column .feed-card', { timeout: 10000 });
  await page.waitForTimeout(300);
  const feedAudit = await page.evaluate(() => {
    const cards = Array.from(document.querySelectorAll('.feed-column .feed-card'));
    return {
      cardCount: cards.length,
      shadows: cards.map((c) => getComputedStyle(c).boxShadow),
      borders: cards.map((c) => getComputedStyle(c).borderTopStyle),
    };
  });
  record(
    'FeedView cards shadowless & borderless',
    feedAudit.cardCount > 0 &&
      feedAudit.shadows.every((s) => s === 'none') &&
      feedAudit.borders.every((b) => b === 'none'),
    JSON.stringify(feedAudit)
  );
  await page.screenshot({ path: path.join(OUT, '02_feed_view.png') });

  // 3. Chat & Sidebar Notes + Composer
  await page.locator('.app-rail button[aria-label="Tin nhắn"]').click();
  await page.waitForSelector('.chat-item', { timeout: 10000 });
  await page.locator('.chat-item').first().click();
  await page.waitForSelector('.composer-box', { timeout: 10000 });
  await page.waitForTimeout(300);
  const chatAudit = await page.evaluate(() => {
    const comp = document.querySelector('.composer-box');
    const railAct = document.querySelector('.rail-item.active');
    const secAct = document.querySelector('.section-tab.active');
    const noteBubble = document.querySelector('.note-bubble');
    return {
      composerShadow: comp ? getComputedStyle(comp).boxShadow : null,
      railShadow: railAct ? getComputedStyle(railAct).boxShadow : null,
      secTabShadow: secAct ? getComputedStyle(secAct).boxShadow : null,
      noteBubbleShadow: noteBubble ? getComputedStyle(noteBubble).boxShadow : null,
      noteBubbleBg: noteBubble ? getComputedStyle(noteBubble).backgroundColor : null,
    };
  });
  record(
    'Chat composer, active tabs/rail, and note bubble shadowless',
    chatAudit.composerShadow === 'none' &&
      chatAudit.railShadow === 'none' &&
      (!chatAudit.secTabShadow || chatAudit.secTabShadow === 'none') &&
      (!chatAudit.noteBubbleShadow || chatAudit.noteBubbleShadow === 'none'),
    JSON.stringify(chatAudit)
  );
  await page.screenshot({ path: path.join(OUT, '03_chat_view.png') });

  // 4. Settings -> Storage (settings-select borderless) + Appearance (segmented active shadowless)
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav button', { hasText: 'Giao diện' }).click().catch(() => {});
  await page.waitForTimeout(250);
  const segShadow = await page.evaluate(() => {
    const b = document.querySelector('.segmented button.active');
    return b ? getComputedStyle(b).boxShadow : 'n/a';
  });
  await page.locator('.settings-nav button', { hasText: 'Lưu trữ' }).click();
  await page.waitForSelector('.settings-select', { timeout: 10000 });
  await page.waitForTimeout(300);
  const storageAudit = await page.evaluate(() => {
    const selects = Array.from(document.querySelectorAll('.settings-select'));
    return {
      count: selects.length,
      borders: selects.map((s) => getComputedStyle(s).borderTopStyle),
      shadows: selects.map((s) => getComputedStyle(s).boxShadow),
    };
  });
  record(
    'Settings segmented active & .settings-select borderless/shadowless',
    (segShadow === 'none' || segShadow === 'n/a') &&
      storageAudit.count > 0 &&
      storageAudit.borders.every((b) => b === 'none') &&
      storageAudit.shadows.every((s) => s === 'none'),
    `seg=${segShadow} storage=${JSON.stringify(storageAudit)}`
  );
  await page.screenshot({ path: path.join(OUT, '04_settings_storage.png') });

  record('Zero runtime pageerror events', errors.length === 0, errors.join('; '));
} finally {
  await browser.close();
}

const passed = checks.filter((c) => c.pass).length;
console.log(`\nSummary: ${passed}/${checks.length} passed`);
if (passed !== checks.length) process.exit(1);
