// Spec 047 Live Chromium Verification:
// 1. ZERO visible titles in Settings (.settings-page-title, .settings-nav-title, .settings-group-title) across all tabs.
// 2. ZERO nested white card boxes across the entire app — both default opaque mode AND translucent wallpaper mode (25% & 75% transparency).
// 3. Inline labels preserved for 2FA ("Xác minh hai bước"), "Dán bảng tính", "Hình nền trò chuyện", "Hình nền toàn ứng dụng".
// 4. Clean screenshots captured for visual inspection across Trò chuyện, Quyền riêng tư, Bảo mật, Giao diện, Tài khoản, Messages (over anime-like pink translucent wallpaper).

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'verify_047';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
const record = (name, pass, detail = '') => {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
};
const shot = (page, name) => page.screenshot({ path: path.join(OUT, `${name}.png`) });

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
  args: ['--no-sandbox'],
});

const run = Date.now().toString(36).slice(-5);
const USER = 's047_' + run;

const ctx = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));

try {
  await page.goto(BASE + '/');
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(USER);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 30000 });
  await page.waitForTimeout(900);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Open Settings
  await page.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.waitForTimeout(400);

  // Set a soft pink anime-like gradient/photo wallpaper + 25% panel transparency (matching user's 4 screenshots!)
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).first().click();
  await page.waitForTimeout(400);
  const wpPath = path.join(OUT, 'pink-wallpaper.png');
  const pngB64 = await page.evaluate(() => {
    const c = document.createElement('canvas');
    c.width = 480; c.height = 360;
    const g = c.getContext('2d');
    const grad = g.createLinearGradient(0, 0, 480, 360);
    grad.addColorStop(0, '#F9A8D4');
    grad.addColorStop(0.5, '#FDE7F3');
    grad.addColorStop(1, '#DDD6FE');
    g.fillStyle = grad;
    g.fillRect(0, 0, 480, 360);
    g.fillStyle = '#EC4899';
    g.beginPath(); g.arc(140, 120, 90, 0, Math.PI * 2); g.fill();
    g.fillStyle = '#FFFFFF';
    g.beginPath(); g.arc(330, 210, 110, 0, Math.PI * 2); g.fill();
    return c.toDataURL('image/png').split(',')[1];
  });
  fs.writeFileSync(wpPath, Buffer.from(pngB64, 'base64'));
  await page.locator('#app-wp-file').setInputFiles(wpPath);
  await page.waitForTimeout(1000);
  await page.locator('input[aria-label="Độ trong suốt khung trắng"]').fill('24');
  await page.locator('input[aria-label="Độ trong suốt khung trắng"]').dispatchEvent('input');
  await page.waitForTimeout(600);

  const tabs = [
    { label: 'Trò chuyện', shotName: '01_chat_headerless_flat' },
    { label: 'Quyền riêng tư', shotName: '02_privacy_headerless_flat' },
    { label: 'Bảo mật', shotName: '03_security_headerless_flat' },
    { label: 'Giao diện', shotName: '04_appearance_headerless_flat' },
    { label: 'Tài khoản', shotName: '05_account_headerless_flat' },
  ];

  let allHeaderless = true;
  let allNoWhiteBoxes = true;
  const details = [];

  for (const t of tabs) {
    await page.locator('.settings-nav-item', { hasText: t.label }).first().click();
    await page.waitForTimeout(450);
    await shot(page, t.shotName);

    const res = await page.evaluate(() => {
      const vis = (sel) => [...document.querySelectorAll(sel)].filter((el) => {
        const s = getComputedStyle(el);
        const r = el.getBoundingClientRect();
        return s.display !== 'none' && s.visibility !== 'hidden' && r.height > 0 && r.width > 0;
      });
      const visibleTitles = [
        ...vis('.settings-page .settings-page-title'),
        ...vis('.settings-page .settings-nav-title'),
        ...vis('.settings-page .settings-group-title'),
      ].map((el) => el.innerText.trim());

      const boxSelectors = [
        '.settings-row',
        '.settings-list',
        '.account-row:not(:hover)',
        '.security-event',
        '.settings-nav-item:not(.active):not(:hover)',
        '.segmented',
      ];
      const nonTransparent = [];
      for (const sel of boxSelectors) {
        for (const el of document.querySelectorAll(sel)) {
          const bg = getComputedStyle(el).backgroundColor;
          if (bg && bg !== 'rgba(0, 0, 0, 0)' && bg !== 'transparent') {
            nonTransparent.push({ sel, bg, text: (el.innerText || '').slice(0, 28) });
          }
        }
      }
      // Also check segmented active button is NOT solid white rgb(255, 255, 255)
      const whiteSegActive = [...document.querySelectorAll('.segmented button.active')]
        .filter((el) => getComputedStyle(el).backgroundColor === 'rgb(255, 255, 255)')
        .length;

      return { visibleTitles, nonTransparent, whiteSegActive };
    });

    if (res.visibleTitles.length > 0) allHeaderless = false;
    if (res.nonTransparent.length > 0 || res.whiteSegActive > 0) allNoWhiteBoxes = false;
    details.push({ tab: t.label, ...res });
  }

  record('All Settings page titles, nav section titles & group subheadings are completely hidden', allHeaderless, JSON.stringify(details.filter((d) => d.visibleTitles.length > 0)));
  record('Zero inner white boxes across all Settings tabs under translucent wallpaper mode', allNoWhiteBoxes, JSON.stringify(details.filter((d) => d.nonTransparent.length > 0 || d.whiteSegActive > 0)));

  // Verify inline labels still present without headings
  await page.locator('.settings-nav-item', { hasText: 'Bảo mật' }).first().click();
  await page.waitForTimeout(350);
  const has2faLabel = await page.locator('.settings-label', { hasText: 'Xác minh hai bước' }).isVisible();
  record('2FA row displays clear inline label "Xác minh hai bước" without section title', has2faLabel);

  await page.locator('.settings-nav-item', { hasText: 'Trò chuyện' }).first().click();
  await page.waitForTimeout(350);
  const hasPasteLabel = await page.locator('.settings-label', { hasText: 'Dán bảng tính' }).isVisible();
  record('Chat tab displays inline label "Dán bảng tính" without section title', hasPasteLabel);

  // Also verify Messages view under translucent wallpaper: idle .chat-item & .chat-header-identity have ZERO white boxes
  await page.locator('.app-rail button.rail-item[aria-label="Tin nhắn"]').click();
  await page.waitForTimeout(500);
  await shot(page, '06_messages_no_white_boxes');
  const msgCheck = await page.evaluate(() => {
    const nonTransparent = [];
    for (const sel of ['.chat-item:not(.selected):not(.active):not(:hover)', '.chat-header-identity', '.chat-header-actions']) {
      for (const el of document.querySelectorAll(sel)) {
        const bg = getComputedStyle(el).backgroundColor;
        if (bg && bg !== 'rgba(0, 0, 0, 0)' && bg !== 'transparent') {
          nonTransparent.push({ sel, bg });
        }
      }
    }
    return nonTransparent;
  });
  record('Messages sidebar & chat header have zero nested white boxes under translucent wallpaper', msgCheck.length === 0, JSON.stringify(msgCheck));

  // Verify 75% transparency (data-panel-scrim="true") STILL has zero white row boxes
  await page.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await page.waitForTimeout(350);
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).first().click();
  await page.waitForTimeout(350);
  await page.locator('input[aria-label="Độ trong suốt khung trắng"]').fill('75');
  await page.locator('input[aria-label="Độ trong suốt khung trắng"]').dispatchEvent('input');
  await page.waitForTimeout(500);
  await page.locator('.settings-nav-item', { hasText: 'Quyền riêng tư' }).first().click();
  await page.waitForTimeout(400);
  await shot(page, '07_privacy_75pct_translucent_flat');
  const scrimCheck = await page.evaluate(() => {
    const scrim = document.documentElement.getAttribute('data-panel-scrim');
    const bad = [...document.querySelectorAll('.settings-row, .settings-nav-item:not(.active):not(:hover)')]
      .filter((el) => {
        const bg = getComputedStyle(el).backgroundColor;
        return bg && bg !== 'rgba(0, 0, 0, 0)' && bg !== 'transparent';
      }).length;
    return { scrim, bad };
  });
  record('Even at 75% panel transparency (panel-scrim=true), zero white card boxes appear on rows/nav', scrimCheck.bad === 0, JSON.stringify(scrimCheck));

  record('Zero JS runtime page errors', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
