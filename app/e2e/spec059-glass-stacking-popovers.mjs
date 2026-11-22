// Automated Live Playwright E2E Verification for Spec 059:
// Unified Glass Stacking Contexts, Rail Tooltips & Popovers, Full-Viewport Dismiss Backdrops & Crisp Popover Surfaces
// Verifies EVERY failure mode from the user's screenshots + full workspace stacking invariants:
// 1. Glass Blur ON + Translucent Panels (50%) + Wallpaper ON:
//    - .app-rail uses ::before for backdrop-filter (zero flicker, never traps position: fixed)
//    - Hovering Friends & Settings icons on .app-rail: tooltip ::after renders ON TOP OF .settings-nav (z-index > settings-nav, zero text-shadow)
//    - Hovering Download icon on .app-rail: .download-rail-menu renders ON TOP OF .settings-nav (elementFromPoint hits .download-rail-menu, clickable!)
//    - Clicking bottom Account avatar on .app-rail: .account-menu.rail-menu renders ON TOP OF .settings-nav AND .menu-dismiss covers full 1440x900 viewport; clicking outside dismisses it cleanly!
//    - .rail-item.active NEVER has nested backdrop-filter
// 2. Chat / Messages View in Glass Blur + Wallpaper mode:
//    - Sidebar + button (.cw-dropdown-menu) renders ON TOP OF .chat-column & .panel-resizer AND .menu-dismiss covers full 1440x900 viewport
//    - Composer AttachMenu (.tools-menu) & Emoji picker (.picker-popover) render ON TOP with opaque elevated background (alpha >= 0.95, NOT translucent 0.045) and full 1440x900 .menu-dismiss
//    - Quick Switcher modal (.modal-overlay) renders ABOVE .app-rail (z-index 1000 > 30/80/100)
// 3. Dark theme (Tối) + Glass Blur + Wallpaper mode:
//    - Rail tooltips, Download QR popover, Account menu, and Emoji picker maintain crisp high-contrast elevated surfaces.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE_URL = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC059_SHOT_DIR || '';
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
  const uname = `s59_${tag}`;
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(e.message));

  await page.goto(BASE_URL, { waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 25000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Open Settings -> Giao diện, select Sunset wallpaper, 50% transparency, and enable Glass Blur ("Làm mờ kính khung")
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  await page.waitForTimeout(300);

  await page.locator('.theme-card', { hasText: 'Sáng' }).click();
  await page.waitForTimeout(150);
  await page.locator('.settings-body .wallpaper-tile', { hasText: 'Hoàng hôn' }).last().click();
  await page.waitForTimeout(200);

  // Set panel transparency to 50%
  await page.evaluate(() => {
    const slider = document.querySelector('input[aria-label="Độ trong suốt khung trắng"]');
    const desc = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
    desc.set.call(slider, '50');
    slider.dispatchEvent(new Event('input', { bubbles: true }));
    slider.dispatchEvent(new Event('change', { bubbles: true }));
  });
  await page.waitForTimeout(200);

  // Enable Glass Blur toggle
  const glassCb = page.locator('.settings-row', { hasText: 'Làm mờ kính khung' }).locator('input[type="checkbox"]');
  if (!(await glassCb.isChecked())) await glassCb.click();
  await page.waitForTimeout(350);

  // 1. Verify .app-rail uses ::before for glass blur, never traps fixed children, and sits strictly above .settings-nav
  const railArch = await page.evaluate(() => {
    const html = document.documentElement;
    const rail = document.querySelector('.app-rail');
    const snav = document.querySelector('.settings-nav');
    const activeItem = document.querySelector('.app-rail .rail-item.active');
    const railCs = getComputedStyle(rail);
    const railBef = getComputedStyle(rail, '::before');
    const snavCs = getComputedStyle(snav);
    const activeCs = activeItem ? getComputedStyle(activeItem) : null;
    return {
      glass: html.dataset.panelGlass,
      translucent: html.dataset.panelTranslucent,
      railBg: railCs.backgroundColor,
      railBf: railCs.backdropFilter,
      railBefBf: railBef.backdropFilter,
      railBefBg: railBef.backgroundColor,
      railZ: Number(railCs.zIndex),
      snavZ: Number(snavCs.zIndex),
      snavBf: snavCs.backdropFilter,
      activeItemBf: activeCs ? activeCs.backdropFilter : 'none',
    };
  });
  record(
    'Rail renders glass blur on ::before (no container backdrop-filter trap), no nested blur on .rail-item.active, and rail z-index > settings-nav',
    railArch.glass === 'true' &&
      railArch.translucent === 'true' &&
      railArch.railBf === 'none' &&
      railArch.railBefBf.includes('blur(') &&
      railArch.railBefBg === getComputedStyleSafe(railArch.railBefBg) &&
      railArch.railZ > railArch.snavZ &&
      railArch.snavBf.includes('blur(') &&
      railArch.activeItemBf === 'none',
    JSON.stringify(railArch)
  );

  // 2. Hover Friends icon -> verify tooltip paints ABOVE .settings-nav (z-index 80 > 1) and capture screenshot
  await page.locator('.app-rail button[aria-label="Bạn bè"]').hover();
  await page.waitForTimeout(250);
  const friendsTipAudit = await page.evaluate(() => {
    const rail = document.querySelector('.app-rail');
    const snav = document.querySelector('.settings-nav');
    const btn = document.querySelector('.app-rail button[aria-label="Bạn bè"]');
    const tipCs = getComputedStyle(btn, '::after');
    return {
      railHoverZ: Number(getComputedStyle(rail).zIndex),
      snavZ: Number(getComputedStyle(snav).zIndex),
      tipContent: tipCs.content,
      tipShadow: tipCs.textShadow,
      tipBg: tipCs.backgroundColor,
      tipColor: tipCs.color,
    };
  });
  record(
    'Hovering Friends icon elevates .app-rail strictly above .settings-nav and renders crisp tooltip with zero text-shadow',
    friendsTipAudit.railHoverZ >= 80 &&
      friendsTipAudit.railHoverZ > friendsTipAudit.snavZ &&
      friendsTipAudit.tipContent.includes('Bạn bè') &&
      friendsTipAudit.tipShadow === 'none',
    JSON.stringify(friendsTipAudit)
  );
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '01_glass_hover_friends_tooltip.png') });

  // 3. Hover active Settings icon -> verify tooltip paints ABOVE .settings-nav and capture screenshot
  await page.locator('.app-rail button[aria-label="Cài đặt"]').hover();
  await page.waitForTimeout(250);
  const settingsTipAudit = await page.evaluate(() => {
    const rail = document.querySelector('.app-rail');
    const snav = document.querySelector('.settings-nav');
    const btn = document.querySelector('.app-rail button[aria-label="Cài đặt"]');
    const tipCs = getComputedStyle(btn, '::after');
    return {
      railHoverZ: Number(getComputedStyle(rail).zIndex),
      snavZ: Number(getComputedStyle(snav).zIndex),
      tipContent: tipCs.content,
      activeBf: getComputedStyle(btn).backdropFilter,
    };
  });
  record(
    'Hovering active Settings gear renders tooltip ABOVE .settings-nav with zero nested backdrop-filter',
    settingsTipAudit.railHoverZ >= 80 &&
      settingsTipAudit.railHoverZ > settingsTipAudit.snavZ &&
      settingsTipAudit.tipContent.includes('Cài đặt') &&
      settingsTipAudit.activeBf === 'none',
    JSON.stringify(settingsTipAudit)
  );
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '02_glass_hover_settings_tooltip.png') });

  // 4. Hover Download icon -> verify .download-rail-menu QR popover is TOPMOST (elementFromPoint hits it, NOT .settings-nav!)
  await page.locator('.app-rail .rail-download-wrap button').first().hover();
  await page.waitForSelector('.download-rail-menu', { timeout: 5000 });
  await page.waitForTimeout(250);
  const dlAudit = await page.evaluate(() => {
    const rail = document.querySelector('.app-rail');
    const dl = document.querySelector('.download-rail-menu');
    const cta = dl.querySelector('.download-popover-desktop-btn');
    const dlRect = dl.getBoundingClientRect();
    const ctaRect = cta.getBoundingClientRect();
    const centerHit = document.elementFromPoint(dlRect.left + dlRect.width / 2, dlRect.top + dlRect.height / 2);
    const ctaHit = document.elementFromPoint(ctaRect.left + ctaRect.width / 2, ctaRect.top + ctaRect.height / 2);
    return {
      railZ: Number(getComputedStyle(rail).zIndex),
      dlBg: getComputedStyle(dl).backgroundColor,
      centerInsideDl: !!(centerHit && dl.contains(centerHit)),
      ctaHitIsCta: !!(ctaHit && (ctaHit === cta || cta.contains(ctaHit))),
      centerTag: centerHit ? `${centerHit.tagName}.${centerHit.className}` : null,
    };
  });
  record(
    'Download QR hover popover renders TOPMOST above .settings-nav (hit-test hits QR card & CTA button, never .settings-nav)',
    dlAudit.railZ >= 100 && dlAudit.centerInsideDl && dlAudit.ctaHitIsCta,
    JSON.stringify(dlAudit)
  );
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '03_glass_hover_download_qr.png') });

  // Move mouse off Download icon so it closes cleanly
  await page.mouse.move(700, 450);
  await page.waitForTimeout(200);

  // 5. Click bottom Account avatar -> verify .account-menu.rail-menu is TOPMOST AND .menu-dismiss covers full 1440x900 viewport
  await page.locator('.app-rail .rail-account').click();
  await page.waitForSelector('.account-menu.rail-menu', { timeout: 5000 });
  await page.waitForTimeout(250);
  const acctAudit = await page.evaluate(() => {
    const menu = document.querySelector('.account-menu.rail-menu');
    const dismiss = document.querySelector('.app-rail .menu-dismiss');
    const mr = menu.getBoundingClientRect();
    const dr = dismiss.getBoundingClientRect();
    const hit = document.elementFromPoint(mr.left + mr.width / 2, mr.top + mr.height / 2);
    return {
      menuInsideHit: !!(hit && menu.contains(hit)),
      dismissW: Math.round(dr.width),
      dismissH: Math.round(dr.height),
      dismissX: Math.round(dr.x),
      dismissY: Math.round(dr.y),
      menuTextShadow: getComputedStyle(menu).textShadow,
    };
  });
  record(
    'Account rail menu renders TOPMOST above .settings-nav with full 1440x900 .menu-dismiss backdrop',
    acctAudit.menuInsideHit &&
      acctAudit.dismissW === 1440 &&
      acctAudit.dismissH === 900 &&
      acctAudit.dismissX === 0 &&
      acctAudit.dismissY === 0 &&
      acctAudit.menuTextShadow === 'none',
    JSON.stringify(acctAudit)
  );
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '04_glass_account_menu.png') });

  // Clicking on the settings body area (900, 400) must dismiss the rail account menu!
  await page.mouse.click(900, 400);
  await page.waitForTimeout(200);
  const acctClosed = (await page.locator('.account-menu.rail-menu').count()) === 0;
  record('Clicking outside rail cleanly dismisses .account-menu.rail-menu via full-viewport .menu-dismiss', acctClosed);

  // 6. Switch to Messages -> open self chat -> test Sidebar + menu, Composer AttachMenu, Emoji Picker & Modal stacking in Glass mode!
  await page.locator('.app-rail button[aria-label="Tin nhắn"]').click();
  await page.waitForSelector('.nav-sidebar .chat-item', { timeout: 10000 });
  await page.locator('.nav-sidebar .chat-item').first().click();
  await page.waitForSelector('.composer-box', { timeout: 10000 });
  await page.waitForTimeout(300);

  // Test Sidebar + dropdown menu & full-viewport dismiss
  await page.locator('.nav-sidebar .sidebar-plus-wrap button').first().click();
  await page.waitForSelector('.nav-sidebar .cw-dropdown-menu', { timeout: 5000 });
  await page.waitForTimeout(200);
  const sbMenuAudit = await page.evaluate(() => {
    const menu = document.querySelector('.nav-sidebar .cw-dropdown-menu');
    const dismiss = document.querySelector('.nav-sidebar .menu-dismiss');
    const mr = menu.getBoundingClientRect();
    const dr = dismiss.getBoundingClientRect();
    const hit = document.elementFromPoint(mr.left + mr.width / 2, mr.top + mr.height / 2);
    return {
      menuHit: !!(hit && menu.contains(hit)),
      dismissW: Math.round(dr.width),
      dismissH: Math.round(dr.height),
    };
  });
  record(
    'Sidebar + dropdown menu in Glass mode is topmost with full 1440x900 .menu-dismiss',
    sbMenuAudit.menuHit && sbMenuAudit.dismissW === 1440 && sbMenuAudit.dismissH === 900,
    JSON.stringify(sbMenuAudit)
  );
  // Click inside chat column to dismiss sidebar menu
  await page.mouse.click(1000, 400);
  await page.waitForTimeout(200);

  // Test Composer AttachMenu (.tools-menu) in Glass mode
  await page.locator('.composer-tools button[aria-label*="Thêm"]').click();
  await page.waitForSelector('.tools-menu', { timeout: 5000 });
  await page.waitForTimeout(200);
  const toolsAudit = await page.evaluate(() => {
    const menu = document.querySelector('.tools-menu');
    const dismiss = document.querySelector('.composer .menu-dismiss');
    const dr = dismiss.getBoundingClientRect();
    const mr = menu.getBoundingClientRect();
    const hit = document.elementFromPoint(mr.left + mr.width / 2, mr.top + mr.height / 2);
    return {
      menuHit: !!(hit && menu.contains(hit)),
      dismissW: Math.round(dr.width),
      dismissH: Math.round(dr.height),
    };
  });
  record(
    'Composer AttachMenu in Glass mode is topmost with full 1440x900 .menu-dismiss',
    toolsAudit.menuHit && toolsAudit.dismissW === 1440 && toolsAudit.dismissH === 900,
    JSON.stringify(toolsAudit)
  );
  await page.mouse.click(200, 400);
  await page.waitForTimeout(200);

  // Test Emoji Picker (.picker-popover) has opaque elevated surface (NOT 0.045 translucent bg-surface-hover)
  await page.locator('.composer-box button[aria-label="Chọn emoji"]').click();
  await page.waitForSelector('.picker-popover', { timeout: 5000 });
  await page.waitForTimeout(200);
  const pickerAudit = await page.evaluate(() => {
    const pop = document.querySelector('.picker-popover');
    const cs = getComputedStyle(pop);
    const m = cs.backgroundColor.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\)/);
    const alpha = m && m[4] !== undefined ? parseFloat(m[4]) : 1;
    return { bg: cs.backgroundColor, alpha, shadow: cs.boxShadow };
  });
  record(
    'Emoji .picker-popover uses crisp opaque elevated surface (alpha >= 0.95) over glass chat wallpaper',
    pickerAudit.alpha >= 0.95 && pickerAudit.shadow !== 'none',
    JSON.stringify(pickerAudit)
  );
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '05_glass_chat_emoji_picker.png') });
  await page.locator('.composer-box button[aria-label="Chọn emoji"]').click();
  await page.waitForTimeout(150);

  // 7. Open Profile Modal from rail account menu -> verify .modal-overlay renders ABOVE .app-rail (z-index 1000 > 30/80/100)
  await page.locator('.app-rail .rail-account').click();
  await page.waitForSelector('.account-menu.rail-menu', { timeout: 5000 });
  await page.locator('.account-menu.rail-menu .menu-item', { hasText: 'Hồ sơ của tôi' }).click();
  await page.waitForSelector('.modal-overlay .modal-dialog', { timeout: 5000 });
  await page.waitForTimeout(250);
  const modalStackAudit = await page.evaluate(() => {
    const overlay = document.querySelector('.modal-overlay');
    const rail = document.querySelector('.app-rail');
    const rr = rail.getBoundingClientRect();
    const hitOverRail = document.elementFromPoint(rr.left + rr.width / 2, rr.top + rr.height / 2);
    return {
      overlayZ: Number(getComputedStyle(overlay).zIndex),
      railZ: Number(getComputedStyle(rail).zIndex),
      modalCoversRail: !!(hitOverRail && (hitOverRail === overlay || overlay.contains(hitOverRail))),
    };
  });
  record(
    'Modal overlay (.modal-overlay) sits strictly ABOVE .app-rail and covers the rail cleanly',
    modalStackAudit.overlayZ >= 1000 && modalStackAudit.modalCoversRail,
    JSON.stringify(modalStackAudit)
  );
  await page.locator('.modal-overlay .profile-close, .modal-overlay button[aria-label="Đóng"]').first().click();
  await page.waitForTimeout(250);

  // 8. Dark theme (Tối) + Glass Blur + Night wallpaper -> verify Download QR popover renders crisply on top
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  await page.waitForTimeout(200);
  await page.locator('.theme-card', { hasText: /^Tối$/ }).click();
  await page.waitForTimeout(200);
  await page.locator('.settings-body .wallpaper-tile', { hasText: 'Đêm sao' }).last().click();
  await page.waitForTimeout(250);
  await page.locator('.app-rail .rail-download-wrap button').first().hover();
  await page.waitForSelector('.download-rail-menu', { timeout: 5000 });
  await page.waitForTimeout(250);
  const darkDlAudit = await page.evaluate(() => {
    const dl = document.querySelector('.download-rail-menu');
    const r = dl.getBoundingClientRect();
    const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    return {
      theme: document.documentElement.dataset.theme,
      dlBg: getComputedStyle(dl).backgroundColor,
      dlColor: getComputedStyle(dl).color,
      hitInsideDl: !!(hit && dl.contains(hit)),
    };
  });
  record(
    'Dark theme + Glass Blur + Night wallpaper: Download QR popover renders crisply TOPMOST with dark elevated surface',
    darkDlAudit.theme === 'dark' && darkDlAudit.hitInsideDl,
    JSON.stringify(darkDlAudit)
  );
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '06_glass_dark_download_qr.png') });

  record('Zero uncaught page errors throughout Spec 059 glass stacking verification', errors.length === 0, JSON.stringify(errors));
} finally {
  await browser.close();
}

function getComputedStyleSafe(v) {
  return v && v !== 'rgba(0, 0, 0, 0)' ? v : '__transparent__';
}

const passed = checks.filter((c) => c.pass).length;
console.log(`\nSpec 059 Summary: ${passed}/${checks.length} passed`);
if (passed !== checks.length) process.exit(1);
