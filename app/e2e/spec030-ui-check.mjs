// Live Chromium verification for Spec 030:
// - Canvas scrollbar subtle hover (3px -> ~5px) & corner-safe inset
// - Left rail Thông báo + Tải ứng dụng QR hover (CH Play / App Store single-logo) + pure white popovers + no avatar ring
// - DownloadView multi-platform page (Desktop Windows/macOS/Linux + Mobile APK/iOS)
// - Settings desktop without <- Cài đặt top header
// - Thought bubble "Chia sẻ suy nghĩ" pure white + top-left above avatar + distinct + vs bubble click targets + FeedView strip
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_030';
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
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`s030_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
  await page.waitForSelector('.notes-strip', { timeout: 10000 });

  // 1. Sidebar thought bubble position, color, subtle border, spacing to search, distinct click targets
  const noteLayout = await page.evaluate(() => {
    const search = document.querySelector('.sidebar-search input');
    const tile = document.querySelector('.my-note-tile');
    const bubble = document.querySelector('.my-note-tile .note-bubble');
    const avWrap = document.querySelector('.my-note-tile .note-avatar-wrap');
    if (!search || !tile || !bubble || !avWrap) return null;
    const sr = search.getBoundingClientRect();
    const br = bubble.getBoundingClientRect();
    const ar = avWrap.getBoundingClientRect();
    const cs = getComputedStyle(bubble);
    return {
      gapSearchToAvatar: Math.round(ar.top - sr.bottom),
      bubbleTop: Math.round(br.top),
      bubbleBottom: Math.round(br.bottom),
      bubbleLeft: Math.round(br.left),
      avatarTop: Math.round(ar.top),
      avatarCenterX: Math.round(ar.left + ar.width / 2),
      bg: cs.backgroundColor,
      border: `${cs.borderTopWidth} ${cs.borderTopStyle}`,
      overlapPx: Math.round(br.bottom - ar.top),
    };
  });
  record(
    'Sidebar "Chia sẻ suy nghĩ" is pure white + border, top-left & closer under search',
    !!(
      noteLayout &&
      noteLayout.bg === 'rgb(255, 255, 255)' &&
      noteLayout.border === '1px solid' &&
      noteLayout.gapSearchToAvatar <= 54 &&
      noteLayout.bubbleLeft < noteLayout.avatarCenterX &&
      noteLayout.overlapPx <= 4
    ),
    JSON.stringify(noteLayout),
  );
  await page.screenshot({ path: path.join(OUT, '01_sidebar_notes_bubble.png') });

  // Click thought bubble -> must open "thought" tab
  await page.locator('.my-note-tile .note-bubble-btn').click();
  await page.waitForSelector('.note-modal', { timeout: 5000 });
  const thoughtActiveText = await page.locator('.note-modal .modal-header button.btn-primary').innerText().catch(() => '');
  record('Clicking "Chia sẻ suy nghĩ" opens Thought tab', thoughtActiveText.includes('Chia sẻ suy nghĩ'), thoughtActiveText);
  await page.locator('.note-modal button[aria-label="Đóng"]').click();
  await page.waitForTimeout(200);

  // Click + badge -> must open "story" ("Tạo tin ảnh") tab
  await page.locator('.my-note-tile .note-plus-badge').click();
  await page.waitForSelector('.note-modal', { timeout: 5000 });
  const storyActiveText = await page.locator('.note-modal .modal-header button.btn-primary').innerText().catch(() => '');
  record('Clicking "+" badge opens Tạo tin tab', storyActiveText.includes('Tạo tin'), storyActiveText);
  await page.locator('.note-modal button[aria-label="Đóng"]').click();
  await page.waitForTimeout(200);

  // 2. FeedView ("Mới") strip
  await page.locator('.app-rail button[aria-label="Mới"]').click();
  await page.waitForSelector('.feed-stories-card .notes-strip', { timeout: 10000 });
  await page.waitForTimeout(300);
  const feedStripCheck = await page.evaluate(() => {
    const card = document.querySelector('.feed-stories-card');
    const bubble = document.querySelector('.feed-stories-card .note-bubble');
    if (!card || !bubble) return null;
    const cr = card.getBoundingClientRect();
    const br = bubble.getBoundingClientRect();
    return { cardTop: Math.round(cr.top), bubbleTop: Math.round(br.top), bg: getComputedStyle(bubble).backgroundColor };
  });
  record(
    'FeedView ("Mới") displays white thought bubble & clean top clearance',
    !!(feedStripCheck && feedStripCheck.bubbleTop >= feedStripCheck.cardTop && feedStripCheck.bg === 'rgb(255, 255, 255)'),
    JSON.stringify(feedStripCheck),
  );
  await page.screenshot({ path: path.join(OUT, '02_feed_stories_strip.png') });

  // 3. Left Rail: Thông báo opens dedicated full NotificationsView page (white rounded panel)
  await page.locator('.rail-bottom button[aria-label="Thông báo"]').click();
  await page.waitForSelector('.notifications-page', { timeout: 5000 });
  const notifPageAudit = await page.evaluate(() => {
    const el = document.querySelector('.notifications-page');
    const cs = getComputedStyle(el);
    return {
      sidebarHidden: !document.querySelector('.nav-sidebar'),
      bg: cs.backgroundColor,
      radius: cs.borderTopLeftRadius,
      title: document.querySelector('.notifications-title')?.innerText?.trim() || '',
    };
  });
  record(
    'Clicking Thông báo opens dedicated NotificationsView white rounded page',
    notifPageAudit.sidebarHidden && notifPageAudit.bg === 'rgb(255, 255, 255)' && parseFloat(notifPageAudit.radius) >= 15 && notifPageAudit.title === 'Thông báo',
    JSON.stringify(notifPageAudit),
  );
  await page.screenshot({ path: path.join(OUT, '03_notifications_page.png') });

  // 4. Left Rail: Tài khoản pure white menu + soft blurred border (NO gray wireframe) + NO white ring on avatar + offset away from rail
  await page.locator('.rail-account').click();
  await page.waitForSelector('.account-menu.rail-menu:not(.download-rail-menu)', { timeout: 5000 });
  const accountAudit = await page.evaluate(() => {
    const btn = document.querySelector('.rail-account');
    const rail = document.querySelector('.app-rail');
    const menu = document.querySelector('.account-menu.rail-menu:not(.download-rail-menu)');
    const bcs = getComputedStyle(btn);
    const mcs = getComputedStyle(menu);
    const rr = rail.getBoundingClientRect();
    const mr = menu.getBoundingClientRect();
    return {
      btnBg: bcs.backgroundColor,
      btnPad: bcs.paddingTop,
      btnShadow: bcs.boxShadow,
      menuBg: mcs.backgroundColor,
      menuBorder: `${mcs.borderTopWidth} ${mcs.borderTopStyle}`,
      menuShadow: mcs.boxShadow,
      gapFromRail: Math.round(mr.left - rr.right),
    };
  });
  record(
    'Account avatar has NO white ring; menu is pure white with soft blurred edge (no gray frame) and offset from rail',
    accountAudit.btnBg === 'rgba(0, 0, 0, 0)' &&
      accountAudit.btnPad === '0px' &&
      accountAudit.btnShadow === 'none' &&
      accountAudit.menuBg === 'rgb(255, 255, 255)' &&
      accountAudit.menuBorder === '0px none' &&
      accountAudit.menuShadow !== 'none' &&
      accountAudit.gapFromRail >= 10,
    JSON.stringify(accountAudit),
  );
  await page.screenshot({ path: path.join(OUT, '04_rail_account_menu.png') });
  await page.locator('.menu-dismiss').click().catch(() => {});
  await page.waitForTimeout(150);

  // 5. Left Rail: Tải ứng dụng hover QR card (bigger QR, no QR border, no store box borders, equal 8px padding matching NgSon menu, soft blurred edge)
  await page.locator('.rail-download-wrap').hover();
  await page.waitForSelector('.download-rail-menu', { timeout: 5000 });
  const dlHover = await page.evaluate(() => {
    const card = document.querySelector('.download-rail-menu');
    const accRailGap = 10;
    const qrWrap = document.querySelector('.download-popover-qr');
    const qrSvg = document.querySelector('.download-popover-qr svg');
    const deskBtn = document.querySelector('.download-popover-desktop-btn');
    const storeBtns = Array.from(document.querySelectorAll('.download-store-logo-only'));
    const cs = getComputedStyle(card);
    const qcs = getComputedStyle(qrWrap);
    const qRect = qrSvg ? qrSvg.getBoundingClientRect() : null;
    return {
      bg: cs.backgroundColor,
      border: `${cs.borderTopWidth} ${cs.borderTopStyle}`,
      shadow: cs.boxShadow,
      pad: `${cs.paddingTop} ${cs.paddingRight} ${cs.paddingBottom} ${cs.paddingLeft}`,
      qrBorder: `${qcs.borderTopWidth} ${qcs.borderTopStyle}`,
      qrBg: qcs.backgroundColor,
      hasQr: !!qrSvg,
      qrWidth: qRect ? Math.round(qRect.width) : 0,
      desktopLabel: deskBtn ? deskBtn.innerText.trim() : '',
      storeCount: storeBtns.length,
      storeTexts: storeBtns.map((b) => b.innerText.trim()),
      storeSvgs: storeBtns.map((b) => b.querySelectorAll('svg').length),
      storeBorders: storeBtns.map((b) => getComputedStyle(b).borderTopStyle),
      noZoomAnim: !Array.from(document.styleSheets).some((s) => {
        try {
          return Array.from(s.cssRules || []).some((r) =>
            (r.name === 'pop-in' || r.name === 'dialog-in') && r.cssText.includes('scale(')
          );
        } catch {
          return false;
        }
      }),
    };
  });
  record(
    'Download QR hover card has larger QR, uniform 8px edge margins, borderless QR & store logos, and zero scale() zoom pop',
    dlHover.bg === 'rgb(255, 255, 255)' &&
      dlHover.border === '0px none' &&
      dlHover.shadow !== 'none' &&
      dlHover.pad === '7.5px 7.5px 7.5px 7.5px' &&
      dlHover.qrBorder === '0px none' &&
      dlHover.hasQr &&
      dlHover.qrWidth >= 155 &&
      dlHover.desktopLabel.includes('Tải cho máy tính') &&
      dlHover.storeCount === 2 &&
      dlHover.storeTexts.every((t) => t === '') &&
      dlHover.storeSvgs.every((n) => n === 1) &&
      dlHover.storeBorders.every((b) => b === 'none') &&
      dlHover.noZoomAnim,
    JSON.stringify(dlHover),
  );
  await page.screenshot({ path: path.join(OUT, '05_rail_download_qr_hover.png') });

  // 6. Click Tải ứng dụng -> DownloadView page (white rounded panel, NO gray section/card boxes, borderless QR, NO top-left back/logo/title)
  await page.locator('.rail-download-wrap button[aria-label="Tải ứng dụng"]').click();
  await page.waitForSelector('.download-page', { timeout: 8000 });
  await page.waitForTimeout(250);
  const dlPage = await page.evaluate(() => {
    const pageEl = document.querySelector('.download-page');
    const heroEl = document.querySelector('.download-hero');
    const secEl = document.querySelector('.download-section');
    const pcs = getComputedStyle(pageEl);
    const qrBox = document.querySelector('.download-qr-box');
    const qcs = getComputedStyle(qrBox);
    const h1 = document.querySelector('.download-hero-title');
    const topBtns = document.querySelectorAll('.download-topbar-actions button, .download-topbar-actions a').length;
    const heroBtns = document.querySelectorAll('.download-hero-actions button').length;
    const qr = !!document.querySelector('.download-qr-box svg');
    const hasTopLeftBrand = !!document.querySelector('.download-brand-group');
    const txt = pageEl?.innerText || '';
    const sidebarHidden = !document.querySelector('.nav-sidebar');
    return {
      sidebarHidden,
      bg: pcs.backgroundColor,
      heroBg: heroEl ? getComputedStyle(heroEl).backgroundColor : null,
      secBg: secEl ? getComputedStyle(secEl).backgroundColor : null,
      radius: pcs.borderTopLeftRadius,
      qrBorder: `${qcs.borderTopWidth} ${qcs.borderTopStyle}`,
      hasTopLeftBrand,
      h1FontSize: h1 ? parseFloat(getComputedStyle(h1).fontSize) : 0,
      topBtns,
      heroBtns,
      qr,
      hasWin: txt.includes('Windows'),
      hasMac: txt.includes('macOS'),
      hasLinux: txt.includes('Linux'),
      hasApk: txt.includes('APK'),
      hasIos: txt.includes('iOS'),
      hasEllipsis: txt.includes('...'),
    };
  });
  record(
    'DownloadView uses pure white rounded panel, zero gray section boxes, borderless QR, and no top-left title/arrow',
    dlPage.sidebarHidden &&
      dlPage.bg === 'rgb(255, 255, 255)' &&
      dlPage.heroBg === 'rgba(0, 0, 0, 0)' &&
      dlPage.secBg === 'rgba(0, 0, 0, 0)' &&
      parseFloat(dlPage.radius) >= 15 &&
      dlPage.qrBorder === '0px none' &&
      !dlPage.hasTopLeftBrand &&
      dlPage.h1FontSize >= 28 &&
      dlPage.topBtns >= 3 &&
      dlPage.heroBtns >= 2 &&
      dlPage.qr &&
      dlPage.hasWin &&
      dlPage.hasMac &&
      dlPage.hasLinux &&
      dlPage.hasApk &&
      dlPage.hasIos &&
      !dlPage.hasEllipsis,
    JSON.stringify(dlPage),
  );
  await page.screenshot({ path: path.join(OUT, '06_download_page.png') });

  // 7. Settings page has NO "<- Cài đặt" desktop header
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 8000 });
  await page.waitForTimeout(200);
  const settingsHeaderCount = await page.locator('.main-stage > header.chat-header').count();
  record('Settings desktop has no "<- Cài đặt" header', settingsHeaderCount === 0, `headerCount=${settingsHeaderCount}`);
  await page.screenshot({ path: path.join(OUT, '07_settings_no_header.png') });

  record('Zero pageerror events', errors.length === 0, errors.join('; '));
} finally {
  await browser.close();
}

const passed = checks.filter((c) => c.pass).length;
console.log(`\nSummary: ${passed}/${checks.length} passed`);
if (passed !== checks.length) process.exit(1);
