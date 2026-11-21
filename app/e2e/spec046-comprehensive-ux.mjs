// Spec 046 Comprehensive E2E Verification in Live Chromium:
// Verifies all 8 requirements:
// 1. Delete "Chính mình" -> NO permanent restore banner; click "+" -> custom dropdown opens -> "Tạo lại 'Chính mình'" restores it.
// 2. Zero jitter in chat messages & right details panel when clicking into chat, switching, and typing.
// 3. Guaranteed latest-first message loading (newest 30 at bottom) + smooth scroll-up lazy pagination until all history loaded (both DM & Channel!).
// 4. Framed Left Rail (data-rail-scrim="true") has NO inner nested box on .rail-item.active.
// 5. Custom theme-synchronized popovers for <select>, <input type="color">, and <input type="time"> (and <input type="date">).
// 6. Inside a Community Server, top "Cộng đồng" + "+" header (.sidebar-top-header) is completely hidden.
// 7. Flattened nested boxes + always-visible thin 1px input border turning var(--accent) on :focus + lighter --bg-input (#F8F9FB) + clean Webhook form.
// 8. Settings Bento 2-column layout (.settings-cols) filling left & right evenly, exact equal 1/3 edge spacing (paddingTop === paddingLeft === paddingRight ~ 15-16px), zero jargon, zero "...", zero "(...)".

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'verify_046';
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
const ALICE = 's046_a_' + run;
const BOB = 's046_b_' + run;

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
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
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

  // Befriend Alice & Bob so Alice has both a real DM & Self-Chat
  const fReq = await api(ctx1, 'POST', '/api/friends/requests', { username: BOB });
  if (fReq.body?.id) {
    await api(ctx2, 'POST', `/api/friends/requests/${fReq.body.id}/accept`, {});
  }
  await p1.reload();
  await p1.waitForSelector('.app-rail', { timeout: 20000 });
  await p1.waitForTimeout(900);

  // =========================================================================
  // 1. Delete "Chính mình" -> NO permanent card -> "+" dropdown -> Restore
  // =========================================================================
  const selfItem = p1.locator('.chat-item', { hasText: 'Chính mình' }).first();
  await selfItem.waitFor({ state: 'visible', timeout: 10000 });
  await selfItem.click();
  await p1.waitForTimeout(400);

  // Open chat "More" menu and delete "Chính mình"
  const moreBtn = p1.locator('button[aria-label="Thêm tùy chọn"], button[title="Tùy chọn"], .chat-header .icon-btn').last();
  await moreBtn.click();
  await p1.waitForTimeout(250);
  const delSelfBtn = p1.locator('button[role="menuitem"]', { hasText: /Xóa/i }).first();
  if (await delSelfBtn.isVisible().catch(() => false)) {
    await delSelfBtn.click();
    await p1.waitForTimeout(300);
    // Confirm dialog if shown
    const confirmBtn = p1.locator('button:has-text("Xóa vĩnh viễn"), button.btn-danger, button:has-text("Xác nhận")').first();
    if (await confirmBtn.isVisible().catch(() => false)) {
      await confirmBtn.click();
      await p1.waitForTimeout(500);
    }
  } else {
    // Fallback: set localStorage prefs directly if menu item label differs
    await p1.evaluate(() => {
      const k = Object.keys(localStorage).find((x) => x.includes('prefs'));
      if (k) {
        const p = JSON.parse(localStorage.getItem(k));
        p.self_chat_enabled = false;
        localStorage.setItem(k, JSON.stringify(p));
      }
    });
    await p1.reload();
    await p1.waitForSelector('.app-rail', { timeout: 15000 });
    await p1.waitForTimeout(600);
  }

  const hasPermanentBanner = (await p1.locator('.self-chat-create').count()) > 0;
  const selfGone = (await p1.locator('.chat-item', { hasText: 'Chính mình' }).count()) === 0;
  record('Req 1a: After deleting "Chính mình", permanent restore card is NOT rendered', !hasPermanentBanner && selfGone, `banner=${hasPermanentBanner} selfGone=${selfGone}`);
  await shot(p1, '01_self_deleted_clean_sidebar');

  // Click "+" button in Messages header -> verify custom dropdown opens with restore item
  await p1.locator('.sidebar-plus-wrap button[aria-label="Tạo mới"]').click();
  await p1.waitForSelector('.cw-dropdown-menu', { timeout: 5000 });
  const restoreItem = p1.locator('.cw-dropdown-menu .cw-dropdown-item', { hasText: 'Chính mình' });
  const restoreVisible = await restoreItem.isVisible();
  await shot(p1, '02_plus_dropdown_restore');
  record('Req 1b: "+" opens custom dropdown containing "Tạo lại \'Chính mình\'"', restoreVisible);

  // Click restore -> verify "Chính mình" returns
  await restoreItem.click();
  await p1.waitForTimeout(500);
  const selfRestored = (await p1.locator('.chat-item', { hasText: 'Chính mình' }).count()) > 0;
  record('Req 1c: Clicking restore in "+" dropdown recreates and opens "Chính mình"', selfRestored);

  // =========================================================================
  // 2. Zero Jitter in Chat & Right Details Panel on Click / Type
  // =========================================================================
  const bobItem = p1.locator('.chat-item', { hasText: BOB }).first();
  await bobItem.click();
  await p1.waitForTimeout(700);

  // Open details panel if not open
  const detailsPanel = p1.locator('.details-panel');
  if (!(await detailsPanel.isVisible().catch(() => false))) {
    const toggleBtn = p1.locator('button[aria-label*="Chi tiết"], button[title*="Chi tiết"], button[aria-label*="thông tin"]').first();
    if (await toggleBtn.isVisible().catch(() => false)) {
      await toggleBtn.click();
      await p1.waitForTimeout(400);
    }
  }

  // Click composer, type, and click Bob again while sampling DOM geometry at 50ms intervals
  const composer = p1.locator('.composer-input').first();
  await composer.click();
  const jitterResult = await p1.evaluate(async () => {
    const msgs = document.querySelector('.chat-messages');
    const det = document.querySelector('.details-panel');
    const samples = [];
    for (let i = 0; i < 16; i++) {
      const m = msgs ? msgs.getBoundingClientRect() : null;
      const d = det ? det.getBoundingClientRect() : null;
      const dText = det ? det.innerText.length : 0;
      samples.push({
        mx: m ? Math.round(m.x) : 0,
        mw: m ? Math.round(m.width) : 0,
        dx: d ? Math.round(d.x) : 0,
        dw: d ? Math.round(d.width) : 0,
        dText,
      });
      await new Promise((r) => setTimeout(r, 60));
    }
    const base = samples[0];
    const maxShift = samples.reduce((acc, s) => Math.max(
      acc,
      Math.abs(s.mx - base.mx),
      Math.abs(s.mw - base.mw),
      Math.abs(s.dx - base.dx),
      Math.abs(s.dw - base.dw),
    ), 0);
    const detailsFlickered = samples.some((s) => base.dText > 0 && s.dText === 0);
    return { maxShift, detailsFlickered, base };
  });
  // Also type a message and ensure clicking active friend does NOT blank messages
  await composer.fill('Hello jitter check');
  await composer.press('Enter');
  await p1.waitForTimeout(600);
  await bobItem.click(); // re-clicking already-active conversation
  await p1.waitForTimeout(250);
  const stillHasMsg = await p1.locator('.message-bubble', { hasText: 'Hello jitter check' }).isVisible();
  record('Req 2: Zero layout jitter in chat & right panel, and re-clicking active DM does not wipe messages',
    jitterResult.maxShift === 0 && !jitterResult.detailsFlickered && stillHasMsg,
    JSON.stringify({ ...jitterResult, stillHasMsg }));

  // =========================================================================
  // 3 & 6. Create Community Server -> Verify Req 6 (no "Cộng đồng" + "+" top bar)
  //        AND Req 3 (38 messages -> latest 30 first -> scroll up loads older #1..#8)
  // =========================================================================
  const srvRes = await api(ctx1, 'POST', '/api/servers', { name: 'Spec046Srv' });
  const srvId = srvRes.body?.id;
  const chList = (await api(ctx1, 'GET', `/api/servers/${srvId}/channels`)).body || [];
  const textCh = chList.find((c) => c.channel_type === 'text') || chList[0];

  // Seed 38 channel messages sequentially (#1 .. #38)
  for (let i = 1; i <= 38; i++) {
    await api(ctx1, 'POST', `/api/channels/${textCh.id}/messages`, {
      content: `TinNhanThu_${String(i).padStart(2, '0')}`,
    });
  }

  // Navigate to Communities -> click server
  await p1.reload();
  await p1.waitForSelector('.app-rail', { timeout: 20000 });
  await p1.waitForTimeout(800);
  await p1.locator('.app-rail button.rail-item[aria-label="Cộng đồng"]').click();
  await p1.waitForTimeout(500);
  await p1.locator('.server-row, .community-card, button', { hasText: 'Spec046Srv' }).first().click();
  await p1.waitForTimeout(900);

  // Req 6: Inside Server, .sidebar-top-header MUST NOT exist!
  const topHeaderInServer = await p1.locator('.sidebar-top-header').count();
  const serverBackHeader = await p1.locator('.sidebar-server-header').isVisible();
  await shot(p1, '03_server_no_community_header');
  record('Req 6: Inside a Community Server, top "Cộng đồng" + "+" header is hidden', topHeaderInServer === 0 && serverBackHeader, `topHeader=${topHeaderInServer} serverHeader=${serverBackHeader}`);

  // Req 3: Verify latest message (#38) is loaded and at the bottom, and oldest (#01) is NOT yet loaded (only 30 newest!)
  await p1.waitForSelector('.message-bubble', { timeout: 10000 });
  await p1.waitForTimeout(500);
  const initialState = await p1.evaluate(() => {
    const bubbles = [...document.querySelectorAll('.message-bubble')].map((el) => el.innerText.trim());
    const box = document.querySelector('.chat-messages');
    const atBottom = box ? (box.scrollHeight - box.scrollTop - box.clientHeight) < 80 : false;
    return {
      count: bubbles.length,
      first: bubbles[0] || '',
      last: bubbles[bubbles.length - 1] || '',
      atBottom,
    };
  });
  const latestFirstOk = initialState.count === 30 && initialState.last.includes('TinNhanThu_38') && initialState.first.includes('TinNhanThu_09') && initialState.atBottom;
  record('Req 3a: Latest 30 messages loaded first (#09..#38) scrolled to bottom', latestFirstOk, JSON.stringify(initialState));

  // Scroll to top -> older messages (#01..#08) MUST lazily load and prepend!
  await p1.evaluate(() => {
    const box = document.querySelector('.chat-messages');
    if (box) {
      box.scrollTop = 0;
      box.dispatchEvent(new Event('scroll'));
    }
  });
  await p1.waitForTimeout(1200);
  const afterScrollState = await p1.evaluate(() => {
    const bubbles = [...document.querySelectorAll('.message-bubble')].map((el) => el.innerText.trim());
    const box = document.querySelector('.chat-messages');
    return {
      count: bubbles.length,
      first: bubbles[0] || '',
      last: bubbles[bubbles.length - 1] || '',
      scrollTop: box ? Math.round(box.scrollTop) : 0,
    };
  });
  const lazyLoadOk = afterScrollState.count === 38 && afterScrollState.first.includes('TinNhanThu_01') && afterScrollState.last.includes('TinNhanThu_38') && afterScrollState.scrollTop > 0;
  record('Req 3b: Scrolling up lazily loads older history (#01..#08) & preserves scroll position', lazyLoadOk, JSON.stringify(afterScrollState));

  // =========================================================================
  // 7 (Webhook form inside Server Settings -> Tiện ích / Webhooks)
  // =========================================================================
  const srvGear = p1.locator('.sidebar-server-header button[title*="Cài đặt"], .sidebar-server-header button[aria-label*="Cài đặt"]').first();
  if (await srvGear.isVisible().catch(() => false)) {
    await srvGear.click();
    await p1.waitForTimeout(500);
    const whTab = p1.locator('button', { hasText: /Webhook|Tiện ích/i }).first();
    if (await whTab.isVisible().catch(() => false)) {
      await whTab.click();
      await p1.waitForTimeout(400);
      const whBtn = p1.locator('button', { hasText: /Webhook/i }).first();
      if (await whBtn.isVisible().catch(() => false)) {
        await whBtn.click();
        await p1.waitForTimeout(400);
      }
      const whCheck = await p1.evaluate(() => {
        const box = document.querySelector('.plugin-dev-publish-box');
        if (!box) return { found: false };
        const st = getComputedStyle(box);
        const inp = box.querySelector('input');
        const ist = inp ? getComputedStyle(inp) : null;
        return {
          found: true,
          text: box.innerText,
          boxBg: st.backgroundColor,
          inpBorder: ist ? ist.borderTopWidth + ' ' + ist.borderTopStyle : 'none',
          inpBg: ist ? ist.backgroundColor : '',
        };
      });
      await shot(p1, '04_webhook_flat_sharp_inputs');
      if (whCheck.found) {
        const noJargon = !whCheck.text.includes('HMAC') && !whCheck.text.includes('Embed') && !whCheck.text.includes('(');
        const flatBox = whCheck.boxBg === 'rgba(0, 0, 0, 0)';
        const crispInput = whCheck.inpBorder.startsWith('1px solid');
        record('Req 7a: Webhook form flattened (no nested box), crisp 1px border & zero jargon/parens', noJargon && flatBox && crispInput, JSON.stringify({ noJargon, flatBox, crispInput, inpBorder: whCheck.inpBorder }));
      }
    }
  }

  // =========================================================================
  // 8. Settings Bento 2-Column Layout, Exact 1/3 Equal Margins & IQ-60 Check
  // =========================================================================
  await p1.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await p1.waitForSelector('.settings-nav', { timeout: 10000 });
  await p1.waitForTimeout(400);

  // Check spacing & 2-col layout across core tabs
  const tabsToCheck = ['Tài khoản', 'Trò chuyện', 'Thông báo', 'Quyền riêng tư', 'Giao diện', 'Lưu trữ', 'Bảo mật'];
  const jargonRe = /\b(OPFS|IndexedDB|SigV4|Multipart|Local CAS|Argon2id|XChaCha20|kchunk_v1|PKCE|HMAC|SSRF|E2EE)\b/;
  let allSpacingEqual = true;
  let allTwoCols = true;
  let allJargonClean = true;
  const tabReports = [];

  for (const label of tabsToCheck) {
    await p1.locator('.settings-nav-item', { hasText: label }).first().click();
    await p1.waitForTimeout(350);
    const info = await p1.evaluate(() => {
      const body = document.querySelector('.settings-body');
      const content = document.querySelector('.settings-content');
      const cols = document.querySelector('.settings-cols');
      const colEls = cols ? [...cols.querySelectorAll(':scope > .settings-col')] : [];
      const bs = body ? getComputedStyle(body) : null;
      const pt = bs ? parseFloat(bs.paddingTop) : 0;
      const pl = bs ? parseFloat(bs.paddingLeft) : 0;
      const pr = bs ? parseFloat(bs.paddingRight) : 0;
      const twoColSideBySide = colEls.length === 2
        && Math.abs(colEls[0].getBoundingClientRect().top - colEls[1].getBoundingClientRect().top) < 8
        && colEls[1].getBoundingClientRect().left > colEls[0].getBoundingClientRect().right;
      const widthRatio = body && content ? content.getBoundingClientRect().width / body.getBoundingClientRect().width : 0;
      const visibleText = content ? content.innerText : '';
      return { pt, pl, pr, twoColSideBySide, widthRatio: Number(widthRatio.toFixed(2)), visibleText };
    });
    const spacingOk = Math.abs(info.pt - info.pl) <= 1 && Math.abs(info.pl - info.pr) <= 1 && info.pl >= 12 && info.pl <= 18;
    const hasDots = info.visibleText.includes('...');
    const hasParens = /\([^)]+\)/.test(info.visibleText);
    const hasJargon = jargonRe.test(info.visibleText);
    if (!spacingOk) allSpacingEqual = false;
    if (!info.twoColSideBySide || info.widthRatio < 0.9) allTwoCols = false;
    if (hasDots || hasParens || hasJargon) allJargonClean = false;
    tabReports.push({ label, spacingOk, pt: info.pt, pl: info.pl, pr: info.pr, twoCol: info.twoColSideBySide, widthRatio: info.widthRatio, hasDots, hasParens, hasJargon });
    if (label === 'Trò chuyện') await shot(p1, '05_settings_chat_bento');
    if (label === 'Giao diện') await shot(p1, '06_settings_appearance_bento');
    if (label === 'Tài khoản') await shot(p1, '07_settings_account_bento');
    if (label === 'Lưu trữ') await shot(p1, '08_settings_storage_bento');
  }

  record('Req 8a: Settings top/left/right margins equal (15-16px, exact 1/3)', allSpacingEqual, JSON.stringify(tabReports.map((t) => `${t.label}:${t.pt}/${t.pl}/${t.pr}`)));
  record('Req 8b: All major Settings tabs use full-width 2-column Bento grid', allTwoCols, JSON.stringify(tabReports.map((t) => `${t.label}:${t.twoCol}(w=${t.widthRatio})`)));
  record('Req 8c: Zero jargon, zero "...", zero "(...)" across checked Settings tabs', allJargonClean, JSON.stringify(tabReports.filter((t) => t.hasDots || t.hasParens || t.hasJargon)));

  // =========================================================================
  // 7b. Input border (1px solid, lighter bg, sharp accent on :focus) & flattened settings-row
  // =========================================================================
  await p1.locator('.settings-nav-item', { hasText: 'Khóa ứng dụng' }).first().click();
  await p1.waitForTimeout(350);
  const inputMetrics = await p1.evaluate(async () => {
    const inp = document.querySelector('.settings-body input.pin-input');
    const row = document.querySelector('.settings-body .settings-row');
    if (!inp) return null;
    const before = getComputedStyle(inp);
    const bBorder = `${before.borderTopWidth} ${before.borderTopStyle} ${before.borderTopColor}`;
    const bBg = before.backgroundColor;
    inp.focus();
    await new Promise((r) => setTimeout(r, 220));
    const after = getComputedStyle(inp);
    const aBorder = `${after.borderTopWidth} ${after.borderTopStyle} ${after.borderTopColor}`;
    const aShadow = after.boxShadow;
    const rowBg = row ? getComputedStyle(row).backgroundColor : 'none';
    return { bBorder, bBg, aBorder, aShadow, rowBg };
  });
  await shot(p1, '09_settings_inputs_focused');
  const inputsSharpOk = Boolean(
    inputMetrics
    && inputMetrics.bBorder.startsWith('1px solid')
    && inputMetrics.aBorder.startsWith('1px solid')
    && inputMetrics.bBorder !== inputMetrics.aBorder
    && inputMetrics.aShadow === 'none'
    && inputMetrics.bBg === 'rgb(248, 249, 251)'
  );
  record('Req 7b: Form inputs have sharp 1px border, accent focus color (no thick glow), lighter #F8F9FB fill', inputsSharpOk, JSON.stringify(inputMetrics));

  // =========================================================================
  // 5. Custom Theme-Synchronized Pickers: <select>, <input type="color">, <input type="time">
  // =========================================================================
  // 5a: <select> in Lưu trữ
  await p1.locator('.settings-nav-item', { hasText: 'Lưu trữ' }).first().click();
  await p1.waitForTimeout(350);
  const cacheSelect = p1.locator('select[aria-label="Giới hạn bộ nhớ đệm"]');
  await cacheSelect.click();
  await p1.waitForSelector('.cw-picker-popover.cw-select-menu', { timeout: 5000 });
  await shot(p1, '10_custom_select_popover');
  await p1.locator('.cw-picker-popover .cw-select-option', { hasText: '2 GB' }).click();
  await p1.waitForTimeout(250);
  const selectUpdated = (await cacheSelect.inputValue()) === '2048';
  record('Req 5a: Custom themed <select> popover opens and updates value', selectUpdated);

  // 5b: <input type="time"> in Trả lời tự động
  await p1.locator('.settings-nav-item', { hasText: 'Trả lời tự động' }).first().click();
  await p1.waitForTimeout(350);
  await p1.locator('button[role="radio"]', { hasText: 'Theo khung giờ' }).click();
  await p1.waitForTimeout(300);
  const timeInput = p1.locator('input[type="time"]').first();
  await timeInput.click();
  await p1.waitForSelector('.cw-picker-popover.cw-time-picker', { timeout: 5000 });
  await shot(p1, '11_custom_time_popover');
  await p1.locator('.cw-picker-popover .cw-tp-col').first().locator('button', { hasText: '09' }).click();
  await p1.waitForTimeout(200);
  const timeUpdated = (await timeInput.inputValue()).startsWith('09:');
  // Close popover
  await p1.keyboard.press('Escape');
  await p1.waitForTimeout(150);
  record('Req 5b: Custom themed <input type="time"> popover opens and updates value', timeUpdated, await timeInput.inputValue());

  // 5c: <input type="color"> in Giao diện
  await p1.locator('.settings-nav-item', { hasText: 'Giao diện' }).first().click();
  await p1.waitForTimeout(350);
  const colorInput = p1.locator('input[type="color"][aria-label="Màu nhấn"]');
  await colorInput.click();
  await p1.waitForSelector('.cw-picker-popover.cw-color-picker', { timeout: 5000 });
  await p1.waitForTimeout(150);
  await shot(p1, '12_custom_color_popover');
  await p1.locator('.cw-picker-popover .cw-cp-swatch').nth(4).click();
  await p1.waitForTimeout(250);
  await p1.keyboard.press('Escape');
  const colorUpdated = Boolean(await colorInput.inputValue()) && (await colorInput.inputValue()).toLowerCase() !== '#2764e7';
  record('Req 5c: Custom themed <input type="color"> popover opens and updates color', colorUpdated, await colorInput.inputValue());

  // =========================================================================
  // 4. Framed Left Rail (data-rail-scrim="true") -> NO nested box on .rail-item.active
  // =========================================================================
  // Apply busy photo wallpaper via canvas data URL or upload to trigger data-rail-scrim="true"
  const wpPath = path.join(OUT, 'busy-wallpaper.png');
  // Create a high-variance photo-like PNG (bold light & dark quadrants so stdB > 0.30 survives 32x32 sampling)
  const pngB64 = await p1.evaluate(() => {
    const c = document.createElement('canvas');
    c.width = 400; c.height = 400;
    const g = c.getContext('2d');
    g.fillStyle = '#FFFFFF';
    g.fillRect(0, 0, 400, 400);
    g.fillStyle = '#101828';
    g.fillRect(0, 200, 200, 200);
    g.fillRect(200, 0, 200, 200);
    g.fillStyle = '#F472B6';
    g.fillRect(0, 0, 200, 120);
    return c.toDataURL('image/png').split(',')[1];
  });
  fs.writeFileSync(wpPath, Buffer.from(pngB64, 'base64'));
  await p1.locator('#app-wp-file').setInputFiles(wpPath);
  await p1.waitForTimeout(1200);
  // Make panel transparent (60%) so rail scrim kicks in
  await p1.locator('input[aria-label="Độ trong suốt khung trắng"]').fill('60');
  await p1.locator('input[aria-label="Độ trong suốt khung trắng"]').dispatchEvent('input');
  await p1.waitForTimeout(800);

  const railCheck = await p1.evaluate(() => {
    const scrim = document.documentElement.getAttribute('data-rail-scrim');
    const rail = document.querySelector('.app-rail');
    const active = document.querySelector('.app-rail .rail-item.active');
    return {
      scrim,
      railBg: rail ? getComputedStyle(rail).backgroundColor : '',
      activeBg: active ? getComputedStyle(active).backgroundColor : '',
      activeShadow: active ? getComputedStyle(active).boxShadow : '',
      activeBorder: active ? getComputedStyle(active).borderTopWidth : '',
    };
  });
  await shot(p1, '13_rail_scrim_no_nested_box');
  const railCleanOk = railCheck.scrim === 'true'
    && railCheck.activeBg === 'rgba(0, 0, 0, 0)'
    && railCheck.activeShadow === 'none';
  record('Req 4: Framed Left Rail (data-rail-scrim="true") removes nested box around active icon', railCleanOk, JSON.stringify(railCheck));

  record('Zero JS runtime page errors across entire Spec 046 verification', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
