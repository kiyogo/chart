// ChatW UI layout audit — drives the real Dioxus web bundle against the real backend.
//
// Usage:
//   node app/e2e/ui-audit.mjs <out_dir> [--quick]
//
// Requirements: backend running on CHATW_URL (default http://127.0.0.1:8085) with
// DEV_LOGIN=true (loopback only), and Playwright from web/frontend/node_modules.
// Uses the system Chrome binary (CHROME_PATH, default /usr/bin/google-chrome).
//
// What it does:
//   1. Creates fresh test accounts (alice_/bob_/charlie_ + run id) through dev-login.
//   2. Makes them friends via the real friends API.
//   3. Sends real E2EE messages through the UI (short, long paragraph, long URL, multi-line).
//   4. Creates a community server and a group DM.
//   5. For each viewport x theme: captures screenshots and runs DOM/bounding-box assertions:
//        - no horizontal page overflow
//        - header and composer fully inside the viewport and not covered
//        - message column keeps a readable width (or the details panel is a modal drawer)
//        - bubbles respect a readable max width; long URLs do not overflow bubbles
//   6. Writes results.json next to the screenshots.

import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve(path.dirname(new URL(import.meta.url).pathname), '../../web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const CHROME = process.env.CHROME_PATH || '/usr/bin/google-chrome';
const OUT = path.resolve(process.argv[2] || 'artifacts/screenshots/after');
const QUICK = process.argv.includes('--quick');
fs.mkdirSync(OUT, { recursive: true });

const RUN = Date.now().toString(36).slice(-5);
const NAMES = { alice: `alice_${RUN}`, bob: `bob_${RUN}`, charlie: `charlie_${RUN}` };

const VIEWPORTS = QUICK
  ? [[375, 667], [768, 1024], [900, 800], [1366, 768], [1920, 1080], [2560, 1440]]
  : [[320, 568], [360, 740], [375, 667], [390, 844], [430, 932], [768, 1024], [900, 800],
     [1024, 768], [1280, 800], [1366, 768], [1440, 900], [1920, 1080], [2560, 1440], [3840, 2160]];

const LONG_PARAGRAPH = 'Đây là một tin nhắn rất dài để kiểm tra khả năng đọc của bong bóng tin nhắn trên màn hình lớn. '
  + 'Nội dung phải được xuống dòng tự nhiên, không kéo dài hết chiều ngang màn hình và không bị cắt chữ tiếng Việt có dấu như ẩ, ặ, ỗ, ữ, ự. '
  + 'Một đoạn văn bản dài cần có độ rộng đọc hợp lý khoảng sáu mươi đến tám mươi ký tự mỗi dòng.';
const LONG_URL = 'https://github.com/kiyogo/chart/blob/main/app/src/pages/chat.rs?query=averyveryverylongparameterthatkeepsgoingandgoingwithoutanybreakopportunity_0123456789abcdefghijklmnopqrstuvwxyz';
const MULTILINE = 'Dòng một\nDòng hai\nDòng ba';

const results = { run: RUN, base: BASE, checks: [] };
function record(name, pass, detail = '') {
  results.checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? '  — ' + detail : ''}`);
}

async function newUser(browser, name, theme = 'light') {
  const ctx = await browser.newContext({ viewport: { width: 1366, height: 768 }, deviceScaleFactor: 1 });
  await ctx.addInitScript((t) => { try { localStorage.setItem('chatw_theme', t); } catch (_) {} }, theme);
  const res = await ctx.request.post(`${BASE}/api/auth/dev-login`, { data: { username: name } });
  if (!res.ok()) throw new Error(`dev-login ${name} failed: ${res.status()}`);
  const me = await (await ctx.request.get(`${BASE}/api/auth/me`)).json();
  const page = await ctx.newPage();
  page.on('pageerror', (e) => results.checks.push({ name: `pageerror ${name}`, pass: false, detail: String(e) }));
  await page.goto(BASE + '/');
  return { ctx, page, me, name };
}

async function api(user, method, url, data) {
  const r = await user.ctx.request.fetch(`${BASE}${url}`, { method, data });
  let body = null;
  try { body = await r.json(); } catch (_) {}
  return { status: r.status(), body };
}

async function waitApp(page) {
  await page.waitForSelector('.chat-item, .empty-chat, .empty-chat-inner, .workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(600);
}

async function openDm(page, partner) {
  const item = page.locator('.chat-item', { hasText: partner }).first();
  await item.waitFor({ timeout: 20000 });
  await item.click();
  await page.waitForTimeout(900);
}

async function sendText(page, text) {
  const input = page.locator('#dm-chat-input');
  await input.click();
  if (text.includes('\n')) {
    const lines = text.split('\n');
    for (let i = 0; i < lines.length; i++) {
      await input.type(lines[i]);
      if (i < lines.length - 1) await input.press('Shift+Enter');
    }
  } else {
    await input.fill(text);
  }
  await input.press('Enter');
  await page.waitForTimeout(500);
}

// --- Geometry helpers executed in the page ---
async function measure(page) {
  return page.evaluate(() => {
    const vw = window.innerWidth, vh = window.innerHeight;
    const q = (s) => document.querySelector(s);
    const box = (el) => { if (!el) return null; const r = el.getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height, r: r.right, b: r.bottom }; };
    const visible = (el) => { if (!el) return false; const cs = getComputedStyle(el); const r = el.getBoundingClientRect(); return cs.display !== 'none' && cs.visibility !== 'hidden' && r.width > 0 && r.height > 0; };
    const topAt = (el) => { if (!el) return null; const r = el.getBoundingClientRect(); const cx = r.x + r.width / 2, cy = r.y + r.height / 2; const hit = document.elementFromPoint(Math.min(Math.max(cx, 0), vw - 1), Math.min(Math.max(cy, 0), vh - 1)); return hit ? (el.contains(hit) || hit.contains(el)) : false; };
    const header = q('.chat-header');
    const composer = q('.composer, .chat-input-bar');
    const msgs = q('.chat-messages');
    const details = q('.details-panel, .partner-info-sidebar, .right-panel');
    const bubbles = [...document.querySelectorAll('.message-bubble')].filter(visible);
    const bubbleOverflow = bubbles.filter((b) => b.scrollWidth > b.clientWidth + 1).length;
    const maxBubble = bubbles.reduce((m, b) => Math.max(m, b.getBoundingClientRect().width), 0);
    return {
      vw, vh,
      docScrollW: document.documentElement.scrollWidth,
      bodyScrollW: document.body.scrollWidth,
      header: visible(header) ? box(header) : null, headerOnTop: topAt(header),
      composer: visible(composer) ? box(composer) : null, composerOnTop: topAt(composer),
      msgs: visible(msgs) ? box(msgs) : null,
      details: visible(details) ? box(details) : null,
      detailsModal: visible(q('.drawer-backdrop')),
      bubbles: bubbles.length, maxBubble, bubbleOverflow,
      msgFont: bubbles[0] ? parseFloat(getComputedStyle(bubbles[0]).fontSize) : null,
    };
  });
}

function assertLayout(tag, m, { expectChat }) {
  record(`${tag} no horizontal overflow`, m.docScrollW <= m.vw + 1 && m.bodyScrollW <= m.vw + 1, `scrollW=${m.docScrollW}/${m.vw}`);
  if (!expectChat) return;
  const inView = (b) => b && b.x >= -1 && b.y >= -1 && b.r <= m.vw + 1 && b.b <= m.vh + 1;
  record(`${tag} header in viewport`, inView(m.header), JSON.stringify(m.header));
  record(`${tag} composer in viewport`, inView(m.composer), JSON.stringify(m.composer));
  record(`${tag} composer not covered`, m.composerOnTop === true, '');
  if (m.msgs) {
    const minW = Math.min(320, m.vw - 16);
    const coveredByDetails = m.details && !m.detailsModal && m.details.x < m.msgs.r - 1 && m.details.r > m.msgs.x + 1;
    record(`${tag} message list readable width`, m.msgs.w >= minW || m.detailsModal, `w=${Math.round(m.msgs.w)}`);
    record(`${tag} details panel does not cover chat`, !coveredByDetails, m.details ? `details.x=${Math.round(m.details.x)} msgs.r=${Math.round(m.msgs.r)}` : 'closed');
  }
  if (m.bubbles) {
    record(`${tag} bubble max width <= 720px`, m.maxBubble <= 720, `max=${Math.round(m.maxBubble)}`);
    record(`${tag} no bubble content overflow`, m.bubbleOverflow === 0, `overflowing=${m.bubbleOverflow}`);
    record(`${tag} message font >= 14px`, (m.msgFont || 0) >= 14, `font=${m.msgFont}`);
  }
}

async function shot(page, name) {
  await page.screenshot({ path: path.join(OUT, `${name}.png`), fullPage: false });
}

async function main() {
  const browser = await chromium.launch({ executablePath: CHROME, headless: true, args: ['--force-color-profile=srgb'] });

  // Login page (unauthenticated)
  for (const theme of ['light', 'dark']) {
    for (const [w, h] of QUICK ? [[375, 667], [1366, 768], [2560, 1440]] : [[375, 667], [768, 1024], [1366, 768], [1920, 1080], [2560, 1440]]) {
      const ctx = await browser.newContext({ viewport: { width: w, height: h } });
      await ctx.addInitScript((t) => localStorage.setItem('chatw_theme', t), theme);
      const p = await ctx.newPage();
      await p.goto(BASE + '/');
      await p.waitForSelector('.auth-container, .auth-shell', { timeout: 30000 });
      await p.waitForTimeout(400);
      await shot(p, `login_${theme}_${w}x${h}`);
      const m = await measure(p);
      assertLayout(`login ${theme} ${w}x${h}`, m, { expectChat: false });
      const card = await p.evaluate(() => { const e = document.querySelector('.auth-card, .auth-box'); if (!e) return null; const r = e.getBoundingClientRect(); return { cx: r.x + r.width / 2, w: r.width }; });
      if (card && w >= 1024) record(`login ${theme} ${w}x${h} primary card visible`, card.w >= 320, `w=${Math.round(card.w)}`);
      await ctx.close();
    }
  }

  // Accounts
  const alice = await newUser(browser, NAMES.alice);
  const bob = await newUser(browser, NAMES.bob);
  const charlie = await newUser(browser, NAMES.charlie);
  for (const u of [alice, bob, charlie]) await waitApp(u.page);

  // Friendships through the real API
  for (const [a, b] of [[alice, bob], [alice, charlie], [bob, charlie]]) {
    const req = await api(a, 'POST', '/api/friends/requests', { username: b.name });
    const reqs = await api(b, 'GET', '/api/friends/requests');
    const pending = (reqs.body || []).find((r) => r.sender_id === a.me.id);
    if (pending) await api(b, 'POST', `/api/friends/requests/${pending.id}/accept`);
    record(`friendship ${a.name} -> ${b.name}`, !!pending, `req status ${req.status}`);
  }

  // Community server + group DM through the real API
  const srv = await api(alice, 'POST', '/api/servers', { name: `ChatW Studio ${RUN}` });
  record('create community server', srv.status < 300, `status ${srv.status}`);
  const grp = await api(alice, 'POST', '/api/groups', { name: `Nhóm dự án ${RUN}`, member_ids: [bob.me.id, charlie.me.id] });
  record('create group DM', grp.status < 300, `status ${grp.status}`);

  for (const u of [alice, bob]) { await u.page.reload(); await waitApp(u.page); }

  // Real E2EE messages through the UI
  await openDm(alice.page, NAMES.bob);
  await openDm(bob.page, NAMES.alice);
  await sendText(alice.page, 'Chào Bob 👋');
  await sendText(bob.page, 'Chào Alice, mình nhận được rồi.');
  await sendText(alice.page, LONG_PARAGRAPH);
  await sendText(bob.page, LONG_URL);
  await sendText(alice.page, MULTILINE);
  await sendText(bob.page, 'OK');
  await alice.page.waitForTimeout(800);

  const received = await alice.page.locator('.message-bubble', { hasText: 'OK' }).count();
  record('E2EE DM delivered and decrypted (Bob -> Alice)', received > 0, `found=${received}`);
  const decryptErrors = await alice.page.locator('.message-bubble', { hasText: 'Không thể giải mã' }).count();
  record('no decrypt errors in fresh conversation', decryptErrors === 0, `errors=${decryptErrors}`);

  // Viewport sweep on Alice's DM view
  for (const theme of ['light', 'dark']) {
    await alice.page.evaluate((t) => { localStorage.setItem('chatw_theme', t); document.documentElement.setAttribute('data-theme', t); }, theme);
    for (const [w, h] of VIEWPORTS) {
      await alice.page.setViewportSize({ width: w, height: h });
      await alice.page.waitForTimeout(350);
      // Ensure DM is open (mobile layouts may have navigated back)
      if (!(await alice.page.locator('#dm-chat-input').isVisible().catch(() => false))) {
        await openDm(alice.page, NAMES.bob).catch(() => {});
      }
      const tag = `dm ${theme} ${w}x${h}`;
      await shot(alice.page, `dm_${theme}_${w}x${h}`);
      assertLayout(tag, await measure(alice.page), { expectChat: true });

      // Details panel open
      const infoBtn = alice.page.locator('[data-testid="toggle-details"], button[aria-label="Thông tin tài khoản"]').first();
      if (await infoBtn.isVisible().catch(() => false)) {
        const open = await alice.page.locator('.details-panel, .partner-info-sidebar').isVisible().catch(() => false);
        if (!open) await infoBtn.click();
        await alice.page.waitForTimeout(300);
        await shot(alice.page, `dm_details_${theme}_${w}x${h}`);
        const m = await measure(alice.page);
        assertLayout(`${tag} +details`, m, { expectChat: !m.detailsModal });
        // close again so next viewport starts clean
        const closeBtn = alice.page.locator('.details-panel button[aria-label="Đóng"], .partner-info-sidebar button[aria-label="Đóng"]').first();
        if (await closeBtn.isVisible().catch(() => false)) await closeBtn.click();
        await alice.page.waitForTimeout(200);
      }
    }
  }

  // Settings, community, group views at representative sizes
  const reps = [[375, 667], [900, 800], [1366, 768], [2560, 1440]];
  for (const theme of ['light', 'dark']) {
    await alice.page.evaluate((t) => { localStorage.setItem('chatw_theme', t); document.documentElement.setAttribute('data-theme', t); }, theme);
    for (const [w, h] of reps) {
      await alice.page.setViewportSize({ width: w, height: h });
      await alice.page.waitForTimeout(300);
      // On mobile the chat view hides navigation: go back to the list first.
      const backBtn = alice.page.locator('.main-stage .back-btn').first();
      if (await backBtn.isVisible().catch(() => false)) { await backBtn.click(); await alice.page.waitForTimeout(250); }
      const accountBtn = alice.page.locator('.account-btn').first();
      if (await accountBtn.isVisible().catch(() => false)) { await accountBtn.click(); await alice.page.waitForTimeout(200); }
      const settingsBtn = alice.page.locator('.account-menu button[aria-label="Cài đặt"]').first();
      if (await settingsBtn.isVisible().catch(() => false)) {
        await settingsBtn.click();
        await alice.page.waitForTimeout(500);
        await shot(alice.page, `settings_${theme}_${w}x${h}`);
        const secTab = alice.page.locator('.settings-nav-item', { hasText: 'Bảo mật và mã hóa' }).first();
        if (await secTab.isVisible().catch(() => false)) {
          await secTab.click();
          await alice.page.waitForTimeout(300);
          await shot(alice.page, `settings_security_${theme}_${w}x${h}`);
        }
        const appTab = alice.page.locator('.settings-nav-item', { hasText: 'Giao diện' }).first();
        if (w >= 768 && await appTab.isVisible().catch(() => false)) {
          await appTab.click();
          await alice.page.waitForTimeout(300);
          await shot(alice.page, `settings_appearance_${theme}_${w}x${h}`);
        }
        assertLayout(`settings ${theme} ${w}x${h}`, await measure(alice.page), { expectChat: false });
        const clipped = await alice.page.evaluate(() => [...document.querySelectorAll('.settings-value, .settings-row, .fingerprint')].filter((e) => e.getBoundingClientRect().right > window.innerWidth + 1).length);
        record(`settings ${theme} ${w}x${h} no clipped rows`, clipped === 0, `clipped=${clipped}`);
        // Phones: first press returns to the settings list, second closes settings.
        for (let i = 0; i < 2; i++) {
          const back = alice.page.locator('button[aria-label="Đóng cài đặt"]').first();
          if (!(await back.isVisible().catch(() => false))) break;
          await back.click();
          await alice.page.waitForTimeout(300);
        }
      } else {
        record(`settings ${theme} ${w}x${h} reachable`, false, 'settings button not visible');
      }
    }
  }

  await alice.page.setViewportSize({ width: 1366, height: 768 });
  await alice.page.locator('.section-tab', { hasText: 'Cộng đồng' }).first().click();
  await alice.page.waitForTimeout(500);
  await shot(alice.page, 'communities_list_light_1366x768');
  record('no left server rail rendered', await alice.page.locator('.nav-rail').count() === 0, '');
  const srvBtn = alice.page.locator(`button[aria-label="Cộng đồng ChatW Studio ${RUN}"]`).first();
  if (await srvBtn.isVisible().catch(() => false)) {
    await srvBtn.click();
    await alice.page.waitForTimeout(900);
    await shot(alice.page, 'community_light_1366x768');
    record('community opens from sidebar list', await alice.page.locator('.channel-row').count() > 0, '');
  } else {
    record('community listed in sidebar', false, 'not visible');
  }

  // Group DM, emoji picker and mobile list views
  for (const theme of ['light', 'dark']) {
    await alice.page.evaluate((t) => { localStorage.setItem('chatw_theme', t); document.documentElement.setAttribute('data-theme', t); }, theme);
    await alice.page.setViewportSize({ width: 1366, height: 768 });
    await alice.page.locator('.section-tab', { hasText: 'Tin nhắn' }).first().click();
    await alice.page.waitForTimeout(400);
    const grpItem = alice.page.locator('.chat-item', { hasText: `Nhóm dự án ${RUN}` }).first();
    if (await grpItem.isVisible().catch(() => false)) {
      await grpItem.click();
      await alice.page.waitForTimeout(900);
      const gi = alice.page.locator('#group-chat-input');
      await gi.fill('Tin nhắn nhóm kiểm thử giao diện');
      await gi.press('Enter');
      await alice.page.waitForTimeout(700);
      await shot(alice.page, `group_${theme}_1366x768`);
      record(`group ${theme} message rendered`, await alice.page.locator('.message-bubble', { hasText: 'Tin nhắn nhóm kiểm thử giao diện' }).count() > 0, '');
      assertLayout(`group ${theme} 1366x768`, await measure(alice.page), { expectChat: true });
    } else {
      record(`group ${theme} listed in sidebar`, false, 'not visible');
    }
    await openDm(alice.page, NAMES.bob);
    await alice.page.locator('button[aria-label="Chọn emoji"]').first().click();
    await alice.page.waitForTimeout(400);
    await shot(alice.page, `emoji_picker_${theme}_1366x768`);
    await alice.page.locator('#emoji-search').fill('tim');
    await alice.page.waitForTimeout(300);
    const hits = await alice.page.locator('.picker-popover .emoji-btn').count();
    record(`emoji search ${theme} returns results`, hits > 0, `hits=${hits}`);
    await shot(alice.page, `emoji_search_${theme}_1366x768`);
    const pickerBox = await alice.page.locator('.picker-popover').boundingBox();
    record(`emoji picker ${theme} inside viewport`, !!pickerBox && pickerBox.y >= 0 && pickerBox.x >= 0 && pickerBox.x + pickerBox.width <= 1366, JSON.stringify(pickerBox));
    await alice.page.keyboard.press('Escape');
    await alice.page.locator('button[aria-label="Chọn emoji"]').first().click().catch(() => {});
    await alice.page.waitForTimeout(200);
    if (await alice.page.locator('.picker-popover').isVisible().catch(() => false)) await alice.page.locator('button[aria-label="Chọn emoji"]').first().click();

    await alice.page.setViewportSize({ width: 375, height: 667 });
    await alice.page.waitForTimeout(300);
    const back = alice.page.locator('.main-stage .back-btn').first();
    if (await back.isVisible().catch(() => false)) await back.click();
    await alice.page.waitForTimeout(300);
    await shot(alice.page, `mobile_list_${theme}_375x667`);
    assertLayout(`mobile list ${theme}`, await measure(alice.page), { expectChat: false });
  }

  fs.writeFileSync(path.join(OUT, 'results.json'), JSON.stringify(results, null, 2));
  const failed = results.checks.filter((c) => !c.pass).length;
  console.log(`\n${results.checks.length - failed}/${results.checks.length} checks passed. Screenshots: ${OUT}`);
  await browser.close();
  process.exitCode = failed ? 1 : 0;
}

main().catch((e) => { console.error(e); process.exit(2); });
