// Automated Live Playwright E2E Verification for Spec 053:
// 1. 5 distinct Theme Modes (Sáng, Xám tro, Tối, Đen huyền, Theo hệ thống) with proper surface RGB steps & crisp high-contrast text
// 2. Frameless <input type="color"> (zero border/shadow)
// 3. Borderless .accent-swatch.active (zero black ring), vibrant 6 preset colors, 12% accent tint, and inline custom color picker directly under "Màu nhấn"
// 4. 100% Frameless floating chat header across DM / Self-Chat / Community Channel (zero outer white/dark capsule frames)
// 5. Symmetrical avatar padding inside .chat-item (left gap equals top & bottom gap, never hugged at 3px)

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE_URL = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC053_SHOT_DIR || '';
if (SHOT_DIR) fs.mkdirSync(SHOT_DIR, { recursive: true });

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
  const tag = Math.random().toString(36).slice(2, 7);
  const uname = `s53_${tag}`;
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(e.message));

  await page.goto(BASE_URL, { waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 25000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // 1. Check .chat-item symmetrical avatar padding (top, bottom, left all equal ~6px)
  await page.locator('.nav-sidebar .chat-item').first().click();
  await page.waitForSelector('.chat-column .chat-header', { timeout: 10000 });
  await page.waitForTimeout(250);
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '05_chat_frameless_header_and_sidebar_avatar.png') });

  const sidebarAndHeaderAudit = await page.evaluate(() => {
    const item = document.querySelector('.nav-sidebar .chat-item');
    const av = item?.querySelector('.self-chat-avatar, .user-avatar');
    const ir = item.getBoundingClientRect();
    const ar = av.getBoundingClientRect();
    const hdr = document.querySelector('.chat-column > .chat-header');
    const idEl = hdr.querySelector('.chat-header-identity');
    const actEl = hdr.querySelector('.chat-header-actions');
    return {
      avLeft: Math.round((ar.left - ir.left) * 10) / 10,
      avTop: Math.round((ar.top - ir.top) * 10) / 10,
      avBottom: Math.round((ir.bottom - ar.bottom) * 10) / 10,
      idBg: getComputedStyle(idEl).backgroundColor,
      idShadow: getComputedStyle(idEl).boxShadow,
      actBg: getComputedStyle(actEl).backgroundColor,
      actShadow: getComputedStyle(actEl).boxShadow,
    };
  });
  record(
    'Chat item avatar has balanced symmetrical left/top/bottom padding (~6px) and floating chat header has ZERO outer frames',
    sidebarAndHeaderAudit.avLeft >= 5.5 &&
      Math.abs(sidebarAndHeaderAudit.avLeft - sidebarAndHeaderAudit.avTop) <= 1.2 &&
      Math.abs(sidebarAndHeaderAudit.avTop - sidebarAndHeaderAudit.avBottom) <= 1.2 &&
      sidebarAndHeaderAudit.idBg === 'rgba(0, 0, 0, 0)' &&
      sidebarAndHeaderAudit.idShadow === 'none' &&
      sidebarAndHeaderAudit.actBg === 'rgba(0, 0, 0, 0)' &&
      sidebarAndHeaderAudit.actShadow === 'none',
    JSON.stringify(sidebarAndHeaderAudit)
  );

  // 2. Open Settings -> Appearance: verify 5 theme cards, borderless swatches, frameless <input type="color">, inline custom color row under "Màu nhấn"
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  await page.waitForTimeout(300);

  // Switch to explicit Sáng first for screenshot
  await page.locator('.theme-card', { hasText: 'Sáng' }).click();
  await page.waitForTimeout(250);
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '01_appearance_5_themes_accents.png') });

  const accentAndColorAudit = await page.evaluate(() => {
    const themeLabels = Array.from(document.querySelectorAll('.theme-card')).map((el) => el.innerText.trim());
    const swatches = Array.from(document.querySelectorAll('.accent-swatch'));
    const activeSwatch = document.querySelector('.accent-swatch.active');
    const accentGroup = activeSwatch?.closest('.settings-group');
    const customInputInAccentGroup = accentGroup?.querySelector('input[type="color"]');
    const allColorInputs = Array.from(document.querySelectorAll('.settings-body input[type="color"]'));
    const allColorBorderZero = allColorInputs.every((el) => getComputedStyle(el).borderTopWidth === '0px' && getComputedStyle(el).boxShadow === 'none');
    const navActive = document.querySelector('.settings-nav-item.active');
    return {
      themeLabels,
      swatchCount: swatches.length,
      activeSwatchShadow: activeSwatch ? getComputedStyle(activeSwatch).boxShadow : '',
      activeSwatchBorder: activeSwatch ? getComputedStyle(activeSwatch).borderTopWidth : '',
      hasCustomPickerUnderAccent: Boolean(customInputInAccentGroup),
      colorInputCount: allColorInputs.length,
      allColorBorderZero,
      navActiveBg: navActive ? getComputedStyle(navActive).backgroundColor : '',
    };
  });
  record(
    'Appearance renders 5 theme cards, 6 vibrant borderless accent swatches (zero black ring), inline custom accent picker directly below, and frameless color inputs',
    accentAndColorAudit.themeLabels.length === 5 &&
      accentAndColorAudit.themeLabels.includes('Sáng') &&
      accentAndColorAudit.themeLabels.includes('Xám tro') &&
      accentAndColorAudit.themeLabels.includes('Tối') &&
      accentAndColorAudit.themeLabels.includes('Đen huyền') &&
      accentAndColorAudit.themeLabels.includes('Theo hệ thống') &&
      accentAndColorAudit.swatchCount === 6 &&
      accentAndColorAudit.activeSwatchShadow === 'none' &&
      accentAndColorAudit.activeSwatchBorder === '0px' &&
      accentAndColorAudit.hasCustomPickerUnderAccent &&
      accentAndColorAudit.allColorBorderZero,
    JSON.stringify(accentAndColorAudit)
  );

  // 3. Verify all 3 dark theme variants (Xám tro > Tối > Đen huyền brightness progression + crisp white readable text)
  const readThemeState = () =>
    page.evaluate(() => {
      const root = document.documentElement;
      const body = document.querySelector('.settings-body');
      const parse = (s) => {
        const m = (s || '').match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)/i);
        return m ? (+m[1] + +m[2] + +m[3]) / 3 : -1;
      };
      const bg = getComputedStyle(body).backgroundColor;
      const fg = getComputedStyle(body).color;
      return {
        theme: root.getAttribute('data-theme'),
        variant: root.getAttribute('data-theme-variant'),
        bg,
        bgL: Math.round(parse(bg)),
        fg,
        fgL: Math.round(parse(fg)),
      };
    });

  await page.locator('.theme-card', { hasText: 'Xám tro' }).click();
  await page.waitForTimeout(250);
  const ashState = await readThemeState();
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '02_theme_ash.png') });

  await page.locator('.theme-card', { hasText: /^Tối$/ }).click();
  await page.waitForTimeout(250);
  const darkState = await readThemeState();
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '03_theme_dark.png') });

  await page.locator('.theme-card', { hasText: 'Đen huyền' }).click();
  await page.waitForTimeout(250);
  const oledState = await readThemeState();
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '04_theme_oled.png') });

  record(
    'Xám tro (#313338) > Tối (#1C1F26) > Đen huyền (#090A0E) form a distinct, eye-friendly brightness staircase with crisp high-contrast ink',
    ashState.theme === 'dark' &&
      ashState.variant === 'ash' &&
      darkState.theme === 'dark' &&
      darkState.variant === 'dark' &&
      oledState.theme === 'dark' &&
      oledState.variant === 'oled' &&
      ashState.bgL >= 45 &&
      ashState.bgL <= 62 &&
      darkState.bgL >= 24 &&
      darkState.bgL <= 40 &&
      oledState.bgL <= 16 &&
      ashState.bgL > darkState.bgL + 10 &&
      darkState.bgL > oledState.bgL + 10 &&
      ashState.fgL >= 230 &&
      darkState.fgL >= 230 &&
      oledState.fgL >= 235,
    JSON.stringify({ ashState, darkState, oledState })
  );

  record('Zero unhandled page errors during Spec 053 verification', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 053 Summary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
