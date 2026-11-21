// Spec 048 Comprehensive Live Chromium Verification:
// 1. Left Rail bottom avatar has clean >= 6px breathing room from bottom edge of .app-rail.
// 2. Opening an uncached DM shows centered .chat-loading-initial .spinner (ZERO .conversation-intro flash).
// 3. Switching away and returning to an already-opened DM renders cached messages INSTANTLY (zero reload/spinner/blanking).
// 4. Custom <select>, <input type="color">, <input type="time"> popovers NEVER flash at (0, 0) on insertion.
// 5. Server sidebar header (< 123) has balanced top clearance (~50px height matching Messages header) and compact gap to first category.
// 6. Unified Translucent Hover (soft alpha grey) & Active (alpha theme accent) across .chat-item, .channel-row, .settings-nav-item,
//    .rail-item, .segmented button, .conv-more — ZERO opaque white boxes, ZERO text/icon color change, ZERO icon fill shift, ZERO bolding!
// 7. Selective essential Settings titles visible ("Xem trước", "Sao lưu chìa khóa bảo mật", "Nhật ký bảo mật", "Danh sách chặn")
//    while redundant titles ("Khi trò chuyện", "Ai có thể liên hệ với bạn", "Tài khoản", page/nav titles) stay gone.
// 8. Equal symmetrical left & right edge insets (~8px) across Messages, Server channels, App Settings nav, and Server Settings nav,
//    plus wide breathing room between Column 1 and Column 2 in .settings-cols.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'verify_048';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
const record = (name, pass, detail = '') => {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
};
const shot = (page, name) => page.screenshot({ path: path.join(OUT, `${name}.png`) });

const parseAlpha = (colorStr) => {
  if (!colorStr || colorStr === 'transparent') return 0;
  const rgba = colorStr.match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\s*\)/i);
  if (rgba) return rgba[4] === undefined ? 1 : parseFloat(rgba[4]);
  const srgb = colorStr.match(/color\(\s*srgb\s+[\d.]+\s+[\d.]+\s+[\d.]+(?:\s*\/\s*([\d.]+))?\s*\)/i);
  if (srgb) return srgb[1] === undefined ? 1 : parseFloat(srgb[1]);
  return -1;
};

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
  args: ['--no-sandbox'],
});

const run = Date.now().toString(36).slice(-5);
const ALICE = 's048_a_' + run;
const BOB = 's048_b_' + run;

const ctx1 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const ctx2 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const errors = [];
const p1 = await ctx1.newPage();
p1.on('pageerror', (e) => errors.push(String(e)));
const p2 = await ctx2.newPage();
p2.on('pageerror', (e) => errors.push(String(e)));

const devLogin = async (page, username) => {
  await page.goto(BASE + '/');
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(username);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 30000 });
  await page.waitForTimeout(900);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
};

const api = async (ctx, method, url, data) => {
  const opts = { method };
  if (data !== undefined) opts.data = data;
  const r = await ctx.request.fetch(`${BASE}${url}`, opts);
  let body = null;
  try { body = await r.json(); } catch (_) {}
  return { status: r.status(), body };
};

try {
  await devLogin(p1, ALICE);
  await devLogin(p2, BOB);

  // Befriend Alice & Bob and send a message from Bob to Alice so Bob's DM has real messages
  const fReq = await api(ctx1, 'POST', '/api/friends/requests', { username: BOB });
  if (fReq.body?.id) {
    await api(ctx2, 'POST', `/api/friends/requests/${fReq.body.id}/accept`, {});
  }
  await p2.reload();
  await p2.waitForSelector('.app-rail', { timeout: 20000 });
  await p2.waitForTimeout(900);
  const aliceRowOnP2 = p2.locator('.chat-item', { hasText: ALICE }).first();
  if (await aliceRowOnP2.isVisible().catch(() => false)) {
    await aliceRowOnP2.click();
    await p2.waitForTimeout(400);
    const ta = p2.locator('.composer-input, textarea').first();
    await ta.fill('Tin nhắn kiểm thử Spec 048 từ Bob');
    await ta.press('Enter');
    await p2.waitForTimeout(700);
  }

  // Create a Community Server as Alice so we can verify Server Header + Channel Rows + Server Settings
  const srvRes = await api(ctx1, 'POST', '/api/servers', { name: '123' });
  const srvId = srvRes.body?.id || srvRes.body?.server?.id;
  if (srvId) {
    await api(ctx1, 'POST', `/api/servers/${srvId}/channels`, { name: 'chung', kind: 'text' });
  }

  // Reload p1 fresh so Bob's DM is guaranteed UNCACHED
  await p1.reload();
  await p1.waitForSelector('.app-rail', { timeout: 20000 });
  await p1.waitForTimeout(900);

  // Set soft pink translucent wallpaper (24% transparency) like the user's screenshots
  await p1.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await p1.waitForSelector('.settings-nav', { timeout: 10000 });
  await p1.waitForTimeout(350);
  await p1.locator('.settings-nav-item', { hasText: 'Giao diện' }).first().click();
  await p1.waitForTimeout(350);
  const wpPath = path.join(OUT, 'pink-wallpaper.png');
  const pngB64 = await p1.evaluate(() => {
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
  await p1.locator('#app-wp-file').setInputFiles(wpPath);
  await p1.waitForTimeout(900);
  await p1.locator('input[aria-label="Độ trong suốt khung trắng"]').fill('24');
  await p1.locator('input[aria-label="Độ trong suốt khung trắng"]').dispatchEvent('input');
  await p1.waitForTimeout(500);

  // =========================================================================
  // Req 1: Bottom avatar clearance inside .app-rail
  // =========================================================================
  const railPadding = await p1.evaluate(() => {
    const rail = document.querySelector('.app-rail');
    const lastAv = document.querySelector('.rail-account .user-avatar, .rail-account button, .rail-bottom .user-avatar');
    if (!rail || !lastAv) return null;
    const rr = rail.getBoundingClientRect();
    const ar = lastAv.getBoundingClientRect();
    return {
      railBottom: Math.round(rr.bottom * 10) / 10,
      avatarBottom: Math.round(ar.bottom * 10) / 10,
      bottomGap: Math.round((rr.bottom - ar.bottom) * 10) / 10,
      padBottom: getComputedStyle(rail).paddingBottom,
    };
  });
  record(
    'Req 1: Left rail bottom user avatar has >= 6px breathing room (never touches bottom edge)',
    Boolean(railPadding && railPadding.bottomGap >= 6 && railPadding.bottomGap <= 24),
    JSON.stringify(railPadding),
  );

  // =========================================================================
  // Req 2 & 3: Initial chat loading spinner (no intro flash) + instant cached switch
  // =========================================================================
  await p1.locator('.app-rail button.rail-item[aria-label="Tin nhắn"]').click();
  await p1.waitForSelector('.nav-sidebar', { timeout: 10000 });
  await p1.waitForTimeout(400);

  // Delay DM history fetch by 550ms to observe loading state cleanly
  let dmFetchCount = 0;
  let routeActive = true;
  await p1.route('**/api/messages/**', async (route) => {
    if (routeActive && route.request().method() === 'GET') {
      dmFetchCount += 1;
      await new Promise((r) => setTimeout(r, 550));
    }
    try { await route.continue(); } catch (_) {}
  });

  const bobItem = p1.locator('.chat-item', { hasText: BOB }).first();
  await bobItem.waitFor({ state: 'visible', timeout: 10000 });
  await bobItem.click();
  await p1.waitForTimeout(120);

  const loadingState = await p1.evaluate(() => ({
    hasSpinner: Boolean(document.querySelector('.chat-loading-initial .spinner')),
    hasIntro: Boolean(document.querySelector('.conversation-intro')),
    hasE2eeNotice: Boolean(document.querySelector('.e2ee-notice')),
  }));
  await shot(p1, '01_chat_loading_spinner');
  record(
    'Req 2: Clicking uncached DM shows centered spinner and does NOT flash conversation intro',
    loadingState.hasSpinner && !loadingState.hasIntro && !loadingState.hasE2eeNotice,
    JSON.stringify(loadingState),
  );

  // Wait for Bob's DM messages to finish loading
  await p1.waitForSelector('.message-bubble', { timeout: 10000 });
  await p1.waitForTimeout(250);
  const loadedCount = await p1.locator('.message-bubble').count();

  // Switch away to "Chính mình", then click BACK to Bob's DM and check IMMEDIATELY (< 60ms while route delay is 550ms!)
  const selfItem = p1.locator('.chat-item', { hasText: 'Chính mình' }).first();
  await selfItem.click();
  await p1.waitForTimeout(750);

  await bobItem.click();
  await p1.waitForTimeout(60);
  const cachedState = await p1.evaluate(() => ({
    bubbleCount: document.querySelectorAll('.message-bubble').length,
    hasSpinner: Boolean(document.querySelector('.chat-loading-initial .spinner')),
    firstBubbleText: document.querySelector('.message-bubble')?.innerText?.trim() || '',
  }));
  record(
    'Req 3: Re-opening previously visited DM renders cached messages INSTANTLY with zero spinner & zero blank reload',
    loadedCount >= 1 && cachedState.bubbleCount >= loadedCount && !cachedState.hasSpinner,
    `loaded=${loadedCount} instant=${JSON.stringify(cachedState)} dmFetches=${dmFetchCount}`,
  );
  routeActive = false;
  await p1.waitForTimeout(650);
  await p1.unrouteAll({ behavior: 'ignoreErrors' });
  await p1.waitForTimeout(150);
  await shot(p1, '02_messages_translucent_active');

  // =========================================================================
  // Req 6 (Messages): Translucent Hover, Translucent Accent Active, .conv-more NOT white, ZERO text color/weight shift
  // =========================================================================
  const msgStyleCheck = await p1.evaluate(() => {
    const selected = document.querySelector('.chat-item.selected');
    const idle = [...document.querySelectorAll('.chat-item')].find((el) => !el.classList.contains('selected'));
    if (!selected || !idle) return null;
    const sn = selected.querySelector('.chat-item-name');
    const iname = idle.querySelector('.chat-item-name');
    const more = selected.closest('.conv-row')?.querySelector('.conv-more');
    return {
      selectedBg: getComputedStyle(selected).backgroundColor,
      selectedNameColor: sn ? getComputedStyle(sn).color : null,
      idleNameColor: iname ? getComputedStyle(iname).color : null,
      selectedNameWeight: sn ? getComputedStyle(sn).fontWeight : null,
      idleNameWeight: iname ? getComputedStyle(iname).fontWeight : null,
      convMoreBg: more ? getComputedStyle(more).backgroundColor : null,
    };
  });
  await selfItem.hover();
  await p1.waitForTimeout(150);
  const selfHoverCheck = await selfItem.evaluate((el) => {
    const nm = el.querySelector('.chat-item-name');
    return {
      bg: getComputedStyle(el).backgroundColor,
      nameColor: nm ? getComputedStyle(nm).color : null,
      nameWeight: nm ? getComputedStyle(nm).fontWeight : null,
    };
  });
  const selAlpha = parseAlpha(msgStyleCheck?.selectedBg);
  const hovAlpha = parseAlpha(selfHoverCheck.bg);
  const convMoreAlpha = parseAlpha(msgStyleCheck?.convMoreBg);
  record(
    'Req 6a (Messages): Hover is translucent soft grey, Active is translucent accent, .conv-more is transparent, ZERO text color/weight shift',
    Boolean(
      msgStyleCheck &&
      selAlpha > 0.05 && selAlpha < 0.45 &&
      hovAlpha > 0.02 && hovAlpha < 0.25 &&
      convMoreAlpha === 0 &&
      msgStyleCheck.selectedNameColor === msgStyleCheck.idleNameColor &&
      selfHoverCheck.nameColor === msgStyleCheck.idleNameColor &&
      msgStyleCheck.selectedNameWeight === msgStyleCheck.idleNameWeight &&
      selfHoverCheck.nameWeight === msgStyleCheck.idleNameWeight
    ),
    JSON.stringify({ msgStyleCheck, selfHoverCheck, selAlpha, hovAlpha, convMoreAlpha }),
  );

  // Measure left/right symmetry in Messages sidebar
  const msgInset = await p1.evaluate(() => {
    const sb = document.querySelector('.nav-sidebar');
    const item = document.querySelector('.chat-item');
    if (!sb || !item) return null;
    const sr = sb.getBoundingClientRect();
    const ir = item.getBoundingClientRect();
    return {
      leftGap: Math.round((ir.left - sr.left) * 10) / 10,
      rightGap: Math.round((sr.right - ir.right) * 10) / 10,
    };
  });

  // =========================================================================
  // Req 5 & 6b: Community Server header spacing + channel row hover/active + symmetry
  // =========================================================================
  await p1.locator('.app-rail button.rail-item[aria-label="Cộng đồng"]').click();
  await p1.waitForTimeout(500);
  const srvTile = p1.locator('.chat-item', { hasText: '123' }).first();
  await srvTile.waitFor({ state: 'visible', timeout: 10000 });
  await srvTile.click();
  await p1.waitForSelector('.sidebar-server-header', { timeout: 10000 });
  await p1.waitForTimeout(500);

  const serverLayout = await p1.evaluate(() => {
    const sb = document.querySelector('.nav-sidebar');
    const hdr = document.querySelector('.sidebar-server-header');
    const nameEl = document.querySelector('.sidebar-server-name');
    const firstChild = document.querySelector('.sidebar-scrollable > :first-child');
    const ch = document.querySelector('.channel-row');
    if (!sb || !hdr || !nameEl || !ch) return null;
    const sr = sb.getBoundingClientRect();
    const hr = hdr.getBoundingClientRect();
    const nr = nameEl.getBoundingClientRect();
    const cr = firstChild ? firstChild.getBoundingClientRect() : null;
    const chr = ch.getBoundingClientRect();
    return {
      headerHeight: Math.round(hr.height),
      titleTopGap: Math.round((nr.top - sr.top) * 10) / 10,
      firstItemGapBelowHeader: cr ? Math.round((cr.top - hr.bottom) * 10) / 10 : null,
      chLeftGap: Math.round((chr.left - sr.left) * 10) / 10,
      chRightGap: Math.round((sr.right - chr.right) * 10) / 10,
    };
  });
  await shot(p1, '03_server_sidebar_header_and_channels');
  record(
    'Req 5: Server header (< 123) has balanced vertical height (~47-52px, >= 8px top gap) and compact spacing to first item (<= 14px)',
    Boolean(
      serverLayout &&
      serverLayout.headerHeight >= 46 &&
      serverLayout.titleTopGap >= 8 &&
      (serverLayout.firstItemGapBelowHeader === null || serverLayout.firstItemGapBelowHeader <= 14)
    ),
    JSON.stringify(serverLayout),
  );

  // Check channel row normal vs hover vs selected color/svg/weight
  const chRows = p1.locator('.channel-row');
  const chCount = await chRows.count();
  let chStyleOk = false;
  let chDetail = {};
  if (chCount >= 2) {
    const firstCh = chRows.nth(0);
    const secondCh = chRows.nth(1);
    await firstCh.click();
    await p1.waitForTimeout(250);
    const idleInfo = await secondCh.evaluate((el) => ({
      color: getComputedStyle(el).color,
      svgColor: el.querySelector('svg') ? getComputedStyle(el.querySelector('svg')).color : null,
      weight: getComputedStyle(el).fontWeight,
    }));
    await secondCh.hover();
    await p1.waitForTimeout(150);
    const hoverInfo = await secondCh.evaluate((el) => ({
      bg: getComputedStyle(el).backgroundColor,
      color: getComputedStyle(el).color,
      svgColor: el.querySelector('svg') ? getComputedStyle(el.querySelector('svg')).color : null,
      weight: getComputedStyle(el).fontWeight,
    }));
    const selInfo = await firstCh.evaluate((el) => ({
      bg: getComputedStyle(el).backgroundColor,
      color: getComputedStyle(el).color,
      svgColor: el.querySelector('svg') ? getComputedStyle(el.querySelector('svg')).color : null,
      weight: getComputedStyle(el).fontWeight,
    }));
    const hA = parseAlpha(hoverInfo.bg);
    const sA = parseAlpha(selInfo.bg);
    chStyleOk =
      hA > 0.02 && hA < 0.25 &&
      sA > 0.05 && sA < 0.45 &&
      hoverInfo.color === idleInfo.color &&
      selInfo.color === idleInfo.color &&
      hoverInfo.svgColor === idleInfo.svgColor &&
      selInfo.svgColor === idleInfo.svgColor &&
      hoverInfo.weight === idleInfo.weight &&
      selInfo.weight === idleInfo.weight;
    chDetail = { idleInfo, hoverInfo, selInfo, hA, sA };
  }
  record(
    'Req 6b (Channels): Hover is translucent grey, Selected is translucent accent, ZERO text/icon color change & ZERO bolding',
    chStyleOk,
    JSON.stringify(chDetail),
  );

  // Check Server Settings nav left/right symmetry
  let srvSetInset = null;
  const srvSetBtn = p1.locator('.sidebar-server-header button[aria-label*="Cài đặt"], .sidebar-server-header button[title*="Cài đặt"]').first();
  if (await srvSetBtn.isVisible().catch(() => false)) {
    await srvSetBtn.click();
    await p1.waitForSelector('.server-settings-stage .settings-nav', { timeout: 8000 }).catch(() => {});
    await p1.waitForTimeout(400);
    srvSetInset = await p1.evaluate(() => {
      const nav = document.querySelector('.server-settings-stage .settings-nav');
      const item = nav ? [...nav.querySelectorAll('.settings-nav-item')].find((el) => el.getBoundingClientRect().width > 0) : null;
      if (!nav || !item) return null;
      const nr = nav.getBoundingClientRect();
      const ir = item.getBoundingClientRect();
      return {
        leftGap: Math.round((ir.left - nr.left) * 10) / 10,
        rightGap: Math.round((nr.right - ir.right) * 10) / 10,
      };
    });
    await shot(p1, '04_server_settings_nav_symmetry');
  }

  // =========================================================================
  // Req 4, 6c, 7, 8: Settings — Selective Titles, Zero-Flash Pickers, Unified Hover/Active, Column Gap & Symmetry
  // =========================================================================
  await p1.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await p1.waitForSelector('.settings-nav', { timeout: 10000 });
  await p1.waitForTimeout(400);

  // Check Settings nav symmetry & active/hover zero color/weight shift
  await p1.locator('.settings-nav-item', { hasText: 'Trò chuyện' }).first().click();
  await p1.waitForTimeout(350);
  await shot(p1, '05_settings_chat_cols_gap');

  const navIdle = p1.locator('.settings-nav-item', { hasText: 'Thông báo' }).first();
  const navActive = p1.locator('.settings-nav-item.active').first();
  const navIdleInfo = await navIdle.evaluate((el) => ({
    color: getComputedStyle(el).color,
    svgColor: el.querySelector('svg') ? getComputedStyle(el.querySelector('svg')).color : null,
    weight: getComputedStyle(el).fontWeight,
  }));
  await navIdle.hover();
  await p1.waitForTimeout(150);
  const navHoverInfo = await navIdle.evaluate((el) => ({
    bg: getComputedStyle(el).backgroundColor,
    color: getComputedStyle(el).color,
    svgColor: el.querySelector('svg') ? getComputedStyle(el.querySelector('svg')).color : null,
    weight: getComputedStyle(el).fontWeight,
  }));
  const navActiveInfo = await navActive.evaluate((el) => ({
    bg: getComputedStyle(el).backgroundColor,
    color: getComputedStyle(el).color,
    svgColor: el.querySelector('svg') ? getComputedStyle(el.querySelector('svg')).color : null,
    fillOpacity: el.querySelector('[fill-opacity]') ? getComputedStyle(el.querySelector('[fill-opacity]')).fillOpacity : '0',
    weight: getComputedStyle(el).fontWeight,
  }));
  const nhA = parseAlpha(navHoverInfo.bg);
  const naA = parseAlpha(navActiveInfo.bg);
  record(
    'Req 6c (Settings Nav): Hover translucent grey, Active translucent accent, ZERO text/icon color change, fill-opacity 0, ZERO bolding',
    nhA > 0.02 && nhA < 0.25 &&
    naA > 0.05 && naA < 0.45 &&
    navHoverInfo.color === navIdleInfo.color &&
    navActiveInfo.color === navIdleInfo.color &&
    navHoverInfo.svgColor === navIdleInfo.svgColor &&
    navActiveInfo.svgColor === navIdleInfo.svgColor &&
    parseFloat(navActiveInfo.fillOpacity) === 0 &&
    navHoverInfo.weight === navIdleInfo.weight &&
    navActiveInfo.weight === navIdleInfo.weight,
    JSON.stringify({ navIdleInfo, navHoverInfo, navActiveInfo, nhA, naA }),
  );

  // Check segmented button active/idle zero bolding & zero color change on Chat tab
  const segCheck = await p1.evaluate(() => {
    const active = document.querySelector('.segmented button.active');
    const idle = [...document.querySelectorAll('.segmented button')].find((b) => !b.classList.contains('active'));
    if (!active || !idle) return null;
    return {
      activeBg: getComputedStyle(active).backgroundColor,
      activeColor: getComputedStyle(active).color,
      idleColor: getComputedStyle(idle).color,
      activeWeight: getComputedStyle(active).fontWeight,
      idleWeight: getComputedStyle(idle).fontWeight,
    };
  });
  const segA = parseAlpha(segCheck?.activeBg);
  record(
    'Req 6d (Segmented): Active button uses translucent accent tint, identical text color & identical non-bold weight',
    Boolean(
      segCheck &&
      segA > 0.05 && segA < 0.45 &&
      segCheck.activeColor === segCheck.idleColor &&
      segCheck.activeWeight === segCheck.idleWeight
    ),
    JSON.stringify({ segCheck, segA }),
  );

  // Check Rail active icon: translucent accent bg, ZERO icon color shift, fill-opacity 0
  const railCheck = await p1.evaluate(() => {
    const active = document.querySelector('.app-rail .rail-item.active:not(.has-logo)');
    const idle = [...document.querySelectorAll('.app-rail .rail-item:not(.has-logo)')].find((b) => !b.classList.contains('active'));
    if (!active || !idle) return null;
    return {
      activeBg: getComputedStyle(active).backgroundColor,
      activeColor: getComputedStyle(active).color,
      idleColor: getComputedStyle(idle).color,
      fillOpacity: active.querySelector('[fill-opacity]') ? getComputedStyle(active.querySelector('[fill-opacity]')).fillOpacity : '0',
    };
  });
  const railA = parseAlpha(railCheck?.activeBg);
  record(
    'Req 6e (Left Rail): Active rail icon uses translucent accent bg, identical icon color & fill-opacity 0',
    Boolean(
      railCheck &&
      railA > 0.05 && railA < 0.45 &&
      railCheck.activeColor === railCheck.idleColor &&
      parseFloat(railCheck.fillOpacity) === 0
    ),
    JSON.stringify({ railCheck, railA }),
  );

  // Check Settings Nav symmetry & .settings-cols column gap
  const settingsLayout = await p1.evaluate(() => {
    const nav = document.querySelector('.settings-nav');
    const item = nav?.querySelector('.settings-nav-item');
    const cols = document.querySelectorAll('.settings-cols > .settings-col');
    if (!nav || !item || cols.length < 2) return null;
    const nr = nav.getBoundingClientRect();
    const ir = item.getBoundingClientRect();
    const c1 = cols[0].getBoundingClientRect();
    const c2 = cols[1].getBoundingClientRect();
    return {
      navLeftGap: Math.round((ir.left - nr.left) * 10) / 10,
      navRightGap: Math.round((nr.right - ir.right) * 10) / 10,
      colGap: Math.round((c2.left - c1.right) * 10) / 10,
    };
  });
  const allInsetsSym =
    msgInset && Math.abs(msgInset.leftGap - msgInset.rightGap) <= 1.5 &&
    serverLayout && Math.abs(serverLayout.chLeftGap - serverLayout.chRightGap) <= 1.5 &&
    settingsLayout && Math.abs(settingsLayout.navLeftGap - settingsLayout.navRightGap) <= 1.5 &&
    (!srvSetInset || Math.abs(srvSetInset.leftGap - srvSetInset.rightGap) <= 1.5);
  record(
    'Req 8: Equal symmetrical left/right insets across Messages, Server, App Settings & Server Settings + wide .settings-cols gap (>= 32px)',
    Boolean(allInsetsSym && settingsLayout && settingsLayout.colGap >= 32),
    JSON.stringify({ msgInset, serverLayout, settingsLayout, srvSetInset }),
  );

  // Req 7: Selective essential Settings titles
  const titleAudit = {};
  const checkTabTitles = async (tabLabel, shotName) => {
    await p1.locator('.settings-nav-item', { hasText: tabLabel }).first().click();
    await p1.waitForTimeout(350);
    if (shotName) await shot(p1, shotName);
    return p1.evaluate(() => {
      const visText = (sel) => [...document.querySelectorAll(sel)]
        .filter((el) => {
          const s = getComputedStyle(el);
          const r = el.getBoundingClientRect();
          return s.display !== 'none' && s.visibility !== 'hidden' && r.height > 0;
        })
        .map((el) => el.innerText.trim());
      return {
        pageTitles: visText('.settings-page .settings-page-title'),
        navTitles: visText('.settings-page .settings-nav-title'),
        groupTitles: visText('.settings-page .settings-group-title'),
      };
    });
  };

  titleAudit.chat = await checkTabTitles('Trò chuyện', null);
  titleAudit.privacy = await checkTabTitles('Quyền riêng tư', '06_settings_privacy');
  titleAudit.security = await checkTabTitles('Bảo mật', '07_settings_security');
  titleAudit.appearance = await checkTabTitles('Giao diện', '08_settings_appearance');
  titleAudit.account = await checkTabTitles('Tài khoản', null);

  const noPageOrNavTitles = Object.values(titleAudit).every((t) => t.pageTitles.length === 0 && t.navTitles.length === 0);
  const chatClean = titleAudit.chat.groupTitles.length === 0;
  const accountClean = titleAudit.account.groupTitles.length === 0;
  const privacySelective =
    titleAudit.privacy.groupTitles.includes('Danh sách chặn') &&
    !titleAudit.privacy.groupTitles.includes('Ai có thể liên hệ với bạn');
  const securitySelective =
    titleAudit.security.groupTitles.includes('Sao lưu chìa khóa bảo mật') &&
    titleAudit.security.groupTitles.includes('Nhật ký bảo mật') &&
    !titleAudit.security.groupTitles.includes('Tài khoản');
  const appearanceSelective =
    titleAudit.appearance.groupTitles.includes('Xem trước') &&
    titleAudit.appearance.groupTitles.includes('Chủ đề') &&
    !titleAudit.appearance.groupTitles.includes('Chữ và mật độ');

  record(
    'Req 7: Selective essential titles visible ("Xem trước", "Sao lưu chìa khóa bảo mật", "Nhật ký bảo mật", "Danh sách chặn") while redundant/page/nav titles are gone',
    noPageOrNavTitles && chatClean && accountClean && privacySelective && securitySelective && appearanceSelective,
    JSON.stringify(titleAudit),
  );

  // =========================================================================
  // Req 4: Zero (0, 0) flash on Custom Pickers (<select>, <input type="color">, <input type="time">)
  // =========================================================================
  await p1.locator('.settings-nav-item', { hasText: 'Giao diện' }).first().click();
  await p1.waitForTimeout(350);

  await p1.evaluate(() => {
    window.__pickerInsertions = [];
    const obs = new MutationObserver((muts) => {
      for (const m of muts) {
        for (const node of m.addedNodes) {
          if (node instanceof HTMLElement && node.classList.contains('cw-picker-popover')) {
            const r = node.getBoundingClientRect();
            const cs = getComputedStyle(node);
            window.__pickerInsertions.push({
              top: Math.round(r.top),
              left: Math.round(r.left),
              visibility: cs.visibility,
              opacity: cs.opacity,
              visibleAtOrigin: cs.visibility !== 'hidden' && parseFloat(cs.opacity) > 0 && r.top <= 4 && r.left <= 4,
            });
          }
        }
      }
    });
    obs.observe(document.body, { childList: true });
  });

  // 4a. Test <input type="color"> in Appearance
  const colorTrigger = p1.locator('input[type="color"]').first();
  await colorTrigger.scrollIntoViewIfNeeded();
  const colorTriggerRect = await colorTrigger.boundingBox();
  await colorTrigger.click();
  await p1.waitForTimeout(180);
  const colorPopRect = await p1.locator('.cw-picker-popover').first().boundingBox();
  await shot(p1, '09_color_picker_position');
  await p1.keyboard.press('Escape');
  await p1.waitForTimeout(120);

  // 4b. Test <select> in Security (Backup export range select)
  await p1.locator('.settings-nav-item', { hasText: 'Bảo mật' }).first().click();
  await p1.waitForTimeout(350);
  const selectTrigger = p1.locator('.settings-body select').first();
  await selectTrigger.scrollIntoViewIfNeeded();
  const selectTriggerRect = await selectTrigger.boundingBox();
  await selectTrigger.click();
  await p1.waitForTimeout(180);
  const selectPopRect = await p1.locator('.cw-picker-popover').first().boundingBox();
  await shot(p1, '10_select_picker_position');
  await p1.keyboard.press('Escape');
  await p1.waitForTimeout(120);

  // 4c. Test <input type="time"> in Auto-Reply
  await p1.locator('.settings-nav-item', { hasText: 'Trả lời tự động' }).first().click();
  await p1.waitForTimeout(350);
  const toggleBtn = p1.locator('.settings-body button[role="switch"]').first();
  if (await toggleBtn.isVisible().catch(() => false)) {
    const aria = await toggleBtn.getAttribute('aria-checked');
    if (aria !== 'true') {
      await toggleBtn.click();
      await p1.waitForTimeout(300);
    }
  }
  let timePopRect = null;
  let timeTriggerRect = null;
  const timeTrigger = p1.locator('input[type="time"]').first();
  if (await timeTrigger.isVisible().catch(() => false)) {
    await timeTrigger.scrollIntoViewIfNeeded();
    timeTriggerRect = await timeTrigger.boundingBox();
    await timeTrigger.click();
    await p1.waitForTimeout(180);
    timePopRect = await p1.locator('.cw-picker-popover').first().boundingBox();
    await p1.keyboard.press('Escape');
  }

  const insertions = await p1.evaluate(() => window.__pickerInsertions || []);
  const anyOriginFlash = insertions.some((x) => x.visibleAtOrigin);
  const colorNear = colorPopRect && colorTriggerRect && Math.abs(colorPopRect.x - colorTriggerRect.x) < 260 && colorPopRect.y > 40;
  const selectNear = selectPopRect && selectTriggerRect && Math.abs(selectPopRect.x - selectTriggerRect.x) < 200 && selectPopRect.y > 40;
  const timeOk = !timeTriggerRect || (timePopRect && Math.abs(timePopRect.x - timeTriggerRect.x) < 220 && timePopRect.y > 40);

  record(
    'Req 4: Custom pickers (color / select / time) NEVER flash at (0, 0) on insertion and open directly at trigger position',
    insertions.length >= 2 && !anyOriginFlash && Boolean(colorNear) && Boolean(selectNear) && Boolean(timeOk),
    JSON.stringify({ insertions, colorPopRect, selectPopRect, timePopRect }),
  );

  // =========================================================================
  // Req 6f: Switching Theme Accent (Green) updates active translucent tint dynamically while text stays unchanged
  // =========================================================================
  await p1.locator('.settings-nav-item', { hasText: 'Giao diện' }).first().click();
  await p1.waitForTimeout(350);
  const greenSwatch = p1.locator('.accent-swatch').nth(2); // blue=0, violet=1, green=2
  await greenSwatch.click();
  await p1.waitForTimeout(300);
  const greenActive = await p1.evaluate(() => {
    const active = document.querySelector('.settings-nav-item.active');
    const idle = [...document.querySelectorAll('.settings-nav-item')].find((x) => !x.classList.contains('active'));
    return {
      accentAttr: document.documentElement.getAttribute('data-accent'),
      activeBg: active ? getComputedStyle(active).backgroundColor : null,
      activeColor: active ? getComputedStyle(active).color : null,
      idleColor: idle ? getComputedStyle(idle).color : null,
    };
  });
  await shot(p1, '11_green_accent_translucent_sync');
  const greenA = parseAlpha(greenActive.activeBg);
  record(
    'Req 6f (Dynamic Accent): Changing theme accent to Green updates translucent active tint automatically with ZERO text color change',
    greenActive.accentAttr === 'green' && greenA > 0.05 && greenA < 0.45 && greenActive.activeColor === greenActive.idleColor,
    JSON.stringify({ greenActive, greenA }),
  );

  record('Zero uncaught page errors throughout Spec 048 E2E', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length) process.exit(1);
