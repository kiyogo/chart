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

  // 1. Sidebar thought bubble position, pure white borderless (soft blur only), closer under search, tight avatar spacing, and equal top/bottom/left chat row inset
  const noteLayout = await page.evaluate(() => {
    const search = document.querySelector('.sidebar-search input');
    const scrollable = document.querySelector('.sidebar-scrollable');
    const strip = document.querySelector('.notes-strip');
    const tile = document.querySelector('.my-note-tile');
    const bubble = document.querySelector('.my-note-tile .note-bubble');
    const avWrap = document.querySelector('.my-note-tile .note-avatar-wrap');
    const plusBadge = document.querySelector('.my-note-tile .note-plus-badge');
    const chatItems = document.querySelectorAll('.sidebar-scrollable .chat-item');
    const chatItem = chatItems[0];
    const chatItem2 = chatItems[1];
    if (!search || !scrollable || !strip || !tile || !bubble || !avWrap || !chatItem) return null;
    const sr = search.getBoundingClientRect();
    const scr = scrollable.getBoundingClientRect();
    const br = bubble.getBoundingClientRect();
    const ar = avWrap.getBoundingClientRect();
    const cr = chatItem.getBoundingClientRect();
    const cr2 = chatItem2 ? chatItem2.getBoundingClientRect() : null;
    const cs = getComputedStyle(bubble);
    const scs = getComputedStyle(scrollable);
    const stcs = getComputedStyle(strip);
    const pcs = plusBadge ? getComputedStyle(plusBadge) : null;
    const ccs = getComputedStyle(chatItem);
    return {
      gapSearchToAvatar: Math.round(ar.top - sr.bottom),
      gapAvatarToChat: Math.round(cr.top - ar.bottom),
      bubbleTop: Math.round(br.top),
      bubbleBottom: Math.round(br.bottom),
      bubbleLeft: Math.round(br.left),
      avatarTop: Math.round(ar.top),
      avatarCenterX: Math.round(ar.left + ar.width / 2),
      bg: cs.backgroundColor,
      border: `${cs.borderTopWidth} ${cs.borderTopStyle}`,
      shadow: cs.boxShadow,
      plusBorder: pcs ? `${pcs.borderTopWidth} ${pcs.borderTopStyle}` : null,
      chatPad: `${ccs.paddingTop} ${ccs.paddingRight} ${ccs.paddingBottom} ${ccs.paddingLeft}`,
      scrollLeftPad: scs.paddingLeft,
      scrollRowGap: scs.rowGap,
      chatRowGapPx: cr2 ? Math.round(cr2.top - cr.bottom) : null,
      chatLeftInsetPx: Math.round(cr.left - scr.left),
      stripOverflowY: stcs.overflowY,
      overlapPx: Math.round(br.bottom - ar.top),
    };
  });
  record(
    'Sidebar "Chia sẻ suy nghĩ" borderless, balanced avatar spacing, no drag scrollbar, & chat row gap is exact 2/3 (4.5px)',
    !!(
      noteLayout &&
      noteLayout.bg === 'rgb(255, 255, 255)' &&
      noteLayout.border === '0px none' &&
      noteLayout.shadow !== 'none' &&
      noteLayout.plusBorder === '0px none' &&
      noteLayout.gapSearchToAvatar <= 44 &&
      noteLayout.bubbleLeft < noteLayout.avatarCenterX &&
      noteLayout.overlapPx <= 4 &&
      noteLayout.stripOverflowY === 'hidden' &&
      noteLayout.chatPad === '6.75px 6.75px 6.75px 6.75px' &&
      noteLayout.scrollLeftPad === '6.75px' &&
      noteLayout.scrollRowGap === '4.5px'
    ),
    JSON.stringify(noteLayout),
  );
  await page.screenshot({ path: path.join(OUT, '01_sidebar_notes_bubble.png') });

  // Click thought bubble -> must open "thought" modal (title: "Chia sẻ suy nghĩ", no switcher buttons)
  await page.locator('.my-note-tile .note-bubble-btn').click();
  await page.waitForSelector('.note-modal', { timeout: 5000 });
  const thoughtHeaderBtns = await page.locator('.note-modal .modal-header button:not(.icon-btn)').count();
  const thoughtTitle = await page.locator('.note-modal .modal-header .modal-title').innerText().catch(() => '');
  record('Clicking "Chia sẻ suy nghĩ" opens Thought modal with title and no header toggle buttons',
    thoughtTitle.includes('Chia sẻ suy nghĩ') && thoughtHeaderBtns === 0,
    `title="${thoughtTitle}", extraBtns=${thoughtHeaderBtns}`
  );
  await page.screenshot({ path: path.join(OUT, '01b_note_modal_thought.png') });
  await page.locator('.note-modal button[aria-label="Đóng"]').click();
  await page.waitForTimeout(200);

  // Click + badge -> must open "story" modal (title: "Tạo tin ảnh", no switcher buttons)
  await page.locator('.my-note-tile .note-plus-badge').click();
  await page.waitForSelector('.note-modal', { timeout: 5000 });
  const storyHeaderBtns = await page.locator('.note-modal .modal-header button:not(.icon-btn)').count();
  const storyTitle = await page.locator('.note-modal .modal-header .modal-title').innerText().catch(() => '');
  record('Clicking "+" badge opens Tạo tin modal with title and no header toggle buttons',
    storyTitle.includes('Tạo tin') && storyHeaderBtns === 0,
    `title="${storyTitle}", extraBtns=${storyHeaderBtns}`
  );
  await page.screenshot({ path: path.join(OUT, '01c_note_modal_story.png') });
  await page.locator('.note-modal button[aria-label="Đóng"]').click();
  await page.waitForTimeout(200);

  // 2. FeedView ("Mới"): Facebook-style compact composer bar + vertical story cards carousel
  await page.locator('.app-rail button[aria-label="Mới"]').click();
  await page.waitForSelector('.feed-compose-bar', { timeout: 10000 });
  await page.waitForSelector('.feed-story-carousel .feed-story-card.create-card', { timeout: 10000 });
  await page.waitForTimeout(300);
  const feedLayoutCheck = await page.evaluate(() => {
    const bar = document.querySelector('.feed-compose-bar');
    const pill = document.querySelector('.feed-compose-pill');
    const quickBtns = document.querySelectorAll('.feed-compose-quick-actions .feed-quick-btn');
    const carousel = document.querySelector('.feed-story-carousel');
    const createCard = document.querySelector('.feed-story-card.create-card');
    const plusBadge = document.querySelector('.feed-story-create-plus');
    const allCards = document.querySelectorAll('.feed-story-rail .feed-story-card');
    if (!bar || !pill || !carousel || !createCard || !plusBadge) return null;
    const br = bar.getBoundingClientRect();
    const cr = carousel.getBoundingClientRect();
    const cardR = createCard.getBoundingClientRect();
    return {
      barBottom: Math.round(br.bottom),
      carouselTop: Math.round(cr.top),
      pillText: pill.textContent?.trim(),
      quickBtnCount: quickBtns.length,
      cardCount: allCards.length,
      cardWidth: Math.round(cardR.width),
      cardHeight: Math.round(cardR.height),
      plusBg: getComputedStyle(plusBadge).backgroundColor,
      createLabel: createCard.querySelector('.feed-story-create-label')?.textContent?.trim(),
    };
  });
  record(
    'FeedView ("Mới") renders compact top composer bar & vertical Facebook-style Story Cards carousel',
    !!(
      feedLayoutCheck &&
      feedLayoutCheck.carouselTop >= feedLayoutCheck.barBottom &&
      feedLayoutCheck.pillText?.includes('bạn đang nghĩ gì thế?') &&
      feedLayoutCheck.quickBtnCount === 3 &&
      feedLayoutCheck.cardHeight > feedLayoutCheck.cardWidth &&
      feedLayoutCheck.createLabel === 'Tạo tin' &&
      feedLayoutCheck.plusBg === 'rgb(24, 119, 242)'
    ),
    JSON.stringify(feedLayoutCheck),
  );
  await page.screenshot({ path: path.join(OUT, '02_feed_stories_strip.png') });

  // Clicking "Tạo tin" card opens "Tạo tin ảnh" modal cleanly
  await page.locator('.feed-story-card.create-card').click();
  await page.waitForSelector('.note-modal', { timeout: 5000 });
  const feedStoryModalTitle = await page.locator('.note-modal .modal-title').innerText().catch(() => '');
  record('Clicking "Tạo tin" story card opens "Tạo tin ảnh" modal', feedStoryModalTitle.includes('Tạo tin ảnh'), feedStoryModalTitle);
  await page.locator('.note-modal button[aria-label="Đóng"]').click();
  await page.waitForTimeout(200);

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
    const panel = document.querySelector('.nav-sidebar') || document.querySelector('.main-stage > div');
    const menu = document.querySelector('.account-menu.rail-menu:not(.download-rail-menu)');
    const bcs = getComputedStyle(btn);
    const mcs = getComputedStyle(menu);
    const rr = rail.getBoundingClientRect();
    const pr = panel.getBoundingClientRect();
    const mr = menu.getBoundingClientRect();
    const leftGapToPanel = Math.round(mr.left - pr.left);
    const bottomGapToPanel = Math.round(pr.bottom - mr.bottom);
    const hasHeader = !!menu.querySelector('.menu-header');
    const hasSectionTitle = !!menu.querySelector('.menu-section-title');
    const items = Array.from(menu.querySelectorAll('.menu-item'));
    const rects = items.map((el) => el.getBoundingClientRect());
    const heights = rects.map((r) => Math.round(r.height));
    const gaps = [];
    for (let i = 1; i < rects.length; i++) {
      gaps.push(Math.round(rects[i].top - rects[i - 1].bottom));
    }
    const equalHeights = heights.length > 0 && heights.every((h) => h === heights[0]);
    const equalGaps = gaps.length > 0 && gaps.every((g) => Math.abs(g - gaps[0]) <= 1);
    return {
      btnBg: bcs.backgroundColor,
      btnPad: bcs.paddingTop,
      btnShadow: bcs.boxShadow,
      menuBg: mcs.backgroundColor,
      menuBorder: `${mcs.borderTopWidth} ${mcs.borderTopStyle}`,
      menuShadow: mcs.boxShadow,
      gapFromRail: Math.round(mr.left - rr.right),
      leftGapToPanel,
      bottomGapToPanel,
      hasHeader,
      hasSectionTitle,
      heights,
      gaps,
      equalHeights,
      equalGaps,
    };
  });
  record(
    'Account menu has no username/Tài khoản headers, evenly spaced equal buttons, pure white soft shadow, flush 0px/0px',
    accountAudit.btnBg === 'rgba(0, 0, 0, 0)' &&
      accountAudit.btnPad === '0px' &&
      accountAudit.btnShadow === 'none' &&
      accountAudit.menuBg === 'rgb(255, 255, 255)' &&
      accountAudit.menuBorder === '0px none' &&
      accountAudit.menuShadow !== 'none' &&
      !accountAudit.hasHeader &&
      !accountAudit.hasSectionTitle &&
      accountAudit.equalHeights &&
      accountAudit.equalGaps &&
      Math.abs(accountAudit.leftGapToPanel) <= 1 &&
      Math.abs(accountAudit.bottomGapToPanel) <= 1,
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
    const panel = document.querySelector('.nav-sidebar') || document.querySelector('.main-stage > div');
    const qrWrap = document.querySelector('.download-popover-qr');
    const qrSvg = document.querySelector('.download-popover-qr svg');
    const deskBtn = document.querySelector('.download-popover-desktop-btn');
    const storeBtns = Array.from(document.querySelectorAll('.download-store-logo-only'));
    const cs = getComputedStyle(card);
    const qcs = getComputedStyle(qrWrap);
    const qRect = qrSvg ? qrSvg.getBoundingClientRect() : null;
    const pr = panel.getBoundingClientRect();
    const cr = card.getBoundingClientRect();
    return {
      bg: cs.backgroundColor,
      border: `${cs.borderTopWidth} ${cs.borderTopStyle}`,
      shadow: cs.boxShadow,
      pad: `${cs.paddingTop} ${cs.paddingRight} ${cs.paddingBottom} ${cs.paddingLeft}`,
      leftGapToPanel: Math.round(cr.left - pr.left),
      bottomGapToPanel: Math.round(pr.bottom - cr.bottom),
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
    'Download QR hover card is flush with bottom-left panel corner (0px/0px), large QR, 8px edge margins, borderless QR & store logos',
    dlHover.bg === 'rgb(255, 255, 255)' &&
      dlHover.border === '0px none' &&
      dlHover.shadow !== 'none' &&
      Math.abs(dlHover.leftGapToPanel) <= 1 &&
      Math.abs(dlHover.bottomGapToPanel) <= 1 &&
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

  // 8. Community list cards match Chat item spacing/padding/inset/gap (4.5px) 100% identically
  await page.locator('.app-rail button[aria-label="Cộng đồng"]').click();
  await page.waitForSelector('.create-community-btn', { timeout: 8000 });
  await page.waitForTimeout(250);
  await page.locator('.create-community-btn').click();
  await page.waitForSelector('.modal-dialog input[type="text"]', { timeout: 5000 });
  await page.locator('.modal-dialog input[type="text"]').first().fill('123');
  await page.locator('.modal-dialog button[type="submit"]').click();
  await page.waitForSelector('.sidebar-server-header', { timeout: 8000 });
  await page.waitForTimeout(250);
  // Go back to the Community List view to inspect .community-item & .create-community-btn side by side
  await page.locator('.sidebar-server-header button').first().click();
  await page.waitForSelector('.community-item', { timeout: 8000 });
  await page.waitForTimeout(300);

  const commParity = await page.evaluate(() => {
    const scroll = document.querySelector('.sidebar-scrollable');
    const item = document.querySelector('.community-item');
    const createBtn = document.querySelector('.create-community-btn');
    if (!scroll || !item || !createBtn) return null;
    const scs = getComputedStyle(scroll);
    const ics = getComputedStyle(item);
    const ccs = getComputedStyle(createBtn);
    const sr = scroll.getBoundingClientRect();
    const ir = item.getBoundingClientRect();
    const cr = createBtn.getBoundingClientRect();
    return {
      scrollLeftPad: scs.paddingLeft,
      scrollRowGap: scs.rowGap,
      itemPad: `${ics.paddingTop} ${ics.paddingRight} ${ics.paddingBottom} ${ics.paddingLeft}`,
      createPad: `${ccs.paddingTop} ${ccs.paddingRight} ${ccs.paddingBottom} ${ccs.paddingLeft}`,
      createMarginTop: ccs.marginTop,
      itemGap: ics.columnGap,
      createGap: ccs.columnGap,
      itemHeight: Math.round(ir.height),
      createHeight: Math.round(cr.height),
      itemLeftInset: Math.round((ir.left - sr.left) * 100) / 100,
      createLeftInset: Math.round((cr.left - sr.left) * 100) / 100,
      gapBetweenCardsPx: Math.round((cr.top - ir.bottom) * 100) / 100,
    };
  });
  record(
    'Community cards (.community-item & .create-community-btn) match Chat items 100% (6.75px pad/inset, 4.5px gap, 0px margin)',
    !!(
      commParity &&
      commParity.scrollRowGap === '4.5px' &&
      commParity.itemPad === '6.75px 6.75px 6.75px 6.75px' &&
      commParity.createPad === '6.75px 6.75px 6.75px 6.75px' &&
      commParity.createMarginTop === '0px' &&
      commParity.itemGap === commParity.createGap &&
      Math.abs(commParity.itemHeight - commParity.createHeight) <= 1 &&
      Math.abs(commParity.gapBetweenCardsPx - 4.5) <= 1
    ),
    JSON.stringify(commParity),
  );
  await page.screenshot({ path: path.join(OUT, '08_community_sidebar_parity.png') });

  record('Zero pageerror events', errors.length === 0, errors.join('; '));
} finally {
  await browser.close();
}

const passed = checks.filter((c) => c.pass).length;
console.log(`\nSummary: ${passed}/${checks.length} passed`);
if (passed !== checks.length) process.exit(1);
