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

// Build a valid RGBA PNG (rounded colored square) without extra dependencies.
function makePng(w, h) {
  const zlib = require('node:zlib');
  const crcTable = Array.from({ length: 256 }, (_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
  const crc = (buf) => { let c = 0xffffffff; for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
  const chunk = (type, data) => { const len = Buffer.alloc(4); len.writeUInt32BE(data.length); const td = Buffer.concat([Buffer.from(type), data]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([len, td, c]); };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) { raw[y * (w * 4 + 1)] = 0; for (let x = 0; x < w; x++) { const o = y * (w * 4 + 1) + 1 + x * 4; const inside = Math.hypot(x - w / 2, y - h / 2) < w / 2 - 2; raw[o] = 39; raw[o + 1] = 100; raw[o + 2] = 231; raw[o + 3] = inside ? 255 : 0; } }
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}
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
  // First run shows the welcome tour; record it once and dismiss it.
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) {
    if (!results.tourSeen) { results.tourSeen = true; record('welcome tour shown to a new account', true, ''); }
    await skip.click();
    await page.waitForTimeout(300);
  }
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


  // ---------------- Feature checks (spec 005) ----------------
  await alice.page.setViewportSize({ width: 1366, height: 768 });
  await bob.page.setViewportSize({ width: 1366, height: 768 });
  const A = alice.page, B = bob.page;

  // DM reaction from Alice on Bob's "OK", seen by Bob in realtime
  const okRow = A.locator('.message-row.received', { hasText: 'OK' }).last();
  await okRow.hover();
  await okRow.locator('.react-trigger').hover();
  await A.waitForTimeout(250);
  record('reaction tray expands on hover', await okRow.locator('.react-option').first().isVisible(), '');
  await okRow.locator('button[aria-label="Thả biểu cảm 👍"]').click();
  await B.waitForTimeout(1200);
  record('DM reaction syncs to the other side', await B.locator('.message-row.sent .reaction-pill', { hasText: '👍' }).count() > 0, '');

  // Built-in sticker inside the E2EE envelope
  await A.locator('button[aria-label="Chọn sticker"]').first().click();
  await A.waitForTimeout(800);
  await shot(A, 'sticker_picker_light_1366x768');
  await A.locator('.picker-popover .sticker-cell').first().click();
  await A.waitForTimeout(1200);
  record('built-in sticker renders for sender', await A.locator('.message-row.sent .sticker-img').count() > 0, '');
  record('sticker decrypts and renders for receiver', await B.locator('.message-row.received .sticker-img').count() > 0, '');

  // Profile popover from a message avatar
  await A.locator('.message-row.received .message-avatar-slot button').last().click();
  await A.waitForTimeout(800);
  record('profile popover opens from avatar', await A.locator('.profile-popover .profile-card').count() > 0, '');
  await shot(A, 'profile_popover_light_1366x768');
  await A.keyboard.press('Escape');
  await A.waitForTimeout(300);
  record('profile popover closes with Escape', await A.locator('.profile-popover').count() === 0, '');

  // Quick switcher Ctrl+K
  await A.keyboard.press('Control+k');
  await A.waitForTimeout(400);
  record('quick switcher opens with Ctrl+K', await A.locator('.quick-switcher').isVisible(), '');
  await A.locator('#quick-switch-input').fill(NAMES.charlie);
  await shot(A, 'quick_switcher_light_1366x768');
  await A.keyboard.press('Enter');
  await A.waitForTimeout(800);
  record('quick switcher opens the picked chat', await A.locator('.chat-header-title', { hasText: NAMES.charlie }).count() > 0, '');

  // Profile editor: decoration, effect and theme persisted on the server
  await A.locator('.rail-account').click();
  await A.locator('.rail-menu .menu-item', { hasText: 'Hồ sơ của tôi' }).click();
  await A.waitForTimeout(800);
  await A.locator('.profile-modal button', { hasText: 'Chỉnh sửa hồ sơ' }).click();
  await A.waitForTimeout(300);
  await A.locator('.profile-editor input[placeholder="Bạn đang làm gì?"]').fill('Đang thử giao diện mới');
  await A.locator('.profile-editor textarea').fill('Xin chào, mình là Alice. Hồ sơ này dùng màu chủ đề và hiệu ứng tuyết rơi.');
  await A.locator('.profile-editor .segmented button', { hasText: 'Trang trí' }).click();
  await A.locator('.theme-swatch[aria-label="Chủ đề Thiên hà"]').click();
  await A.locator('.option-tile', { hasText: 'Vương miện' }).click();
  await A.locator('.option-tile', { hasText: 'Tuyết rơi' }).click();
  await A.waitForTimeout(500);
  await shot(A, 'profile_editor_light_1366x768');
  await A.locator('.profile-editor button', { hasText: 'Lưu thay đổi' }).click();
  await A.waitForTimeout(1000);
  const prof = await api(alice, 'GET', `/api/users/${alice.me.id}/profile`);
  record('profile customization saved on server', prof.body && prof.body.avatar_decoration === 'crown' && prof.body.profile_effect === 'snow' && prof.body.profile_color === '#6D28D9' && !!prof.body.member_since, JSON.stringify({ d: prof.body && prof.body.avatar_decoration, e: prof.body && prof.body.profile_effect, c: prof.body && prof.body.profile_color }));
  await shot(A, 'profile_view_light_1366x768');
  await A.locator('.profile-modal button[aria-label="Đóng hồ sơ"]').click().catch(() => {});
  await A.keyboard.press('Escape');

  // Security: client cannot award itself badges or set an external banner
  const selfBadge = await api(alice, 'POST', '/api/users/me/profile', { badges: ['staff'] });
  const after = await api(alice, 'GET', `/api/users/${alice.me.id}/profile`);
  record('client cannot self-assign badges', (after.body.badges || []).length === 0, `status ${selfBadge.status}`);
  const extBanner = await api(alice, 'POST', '/api/users/me/profile', { custom_banner_url: 'https://evil.example/x.png' });
  record('external banner URL rejected', extBanner.status === 400, `status ${extBanner.status}`);

  // Settings › Tùy chỉnh: pin communities + groups to the rail
  await A.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await A.locator('.settings-nav-item', { hasText: 'Tùy chỉnh' }).click();
  await A.waitForTimeout(300);
  await A.locator('label.toggle-row', { hasText: 'Ghim cộng đồng' }).locator('input').check();
  await A.locator('label.toggle-row', { hasText: 'Ghim nhóm trò chuyện' }).locator('input').check();
  await A.getByRole('radio', { name: 'Bong bóng' }).first().click();
  await A.waitForTimeout(400);
  await shot(A, 'settings_customize_light_1366x768');
  record('pinned communities appear in rail', await A.locator('.rail-pin.community').count() > 0, '');
  record('pinned groups appear in rail', await A.locator('.rail-pin.group').count() > 0, '');

  // Settings › Sticker: create a pack and upload a real PNG
  await A.locator('.settings-nav-item', { hasText: 'Thư viện' }).click();
  await A.waitForTimeout(600);
  await A.locator('input[aria-label="Tên bộ sticker mới"]').fill('Bộ thử nghiệm');
  await A.locator('.inline-form button[type="submit"]').click();
  await A.waitForTimeout(800);
  const png = makePng(96, 96);
  await A.locator('input[aria-label="Tên sticker"]').fill('Ô vuông');
  await A.locator('#sticker-file-input').setInputFiles({ name: 'square.png', mimeType: 'image/png', buffer: png });
  await A.waitForTimeout(2000);
  await shot(A, 'settings_stickers_light_1366x768');
  record('uploaded sticker appears in my pack', await A.locator('.sticker-pack-card .sticker-cell.manage img').count() > 0, '');
  const badUpload = await alice.ctx.request.fetch(`${BASE}/api/stickers/upload`, { method: 'POST', headers: { 'content-type': 'image/png' }, data: Buffer.from('<svg onload=alert(1)></svg>') });
  record('non-image sticker upload rejected', badUpload.status() === 400, `status ${badUpload.status()}`);
  for (let i = 0; i < 2; i++) {
    const back = A.locator('button[aria-label="Đóng cài đặt"]').first();
    if (!(await back.isVisible().catch(() => false))) break;
    await back.click(); await A.waitForTimeout(300);
  }
  // ---------------- Feature checks (spec 007) ----------------
  const safe = async (name, fn) => { try { await fn(); } catch (e) { record(name, false, String(e).split('\n')[0].slice(0, 160)); } };
  await safe('self chat flow', async () => {
    const selfRow = A.locator('[data-conv="self"]');
    record('"Chính mình" is listed in Messages', await selfRow.count() > 0, '');
    await selfRow.click(); await A.waitForTimeout(1200);
    record('self chat shows this device', await A.locator('.device-chip.current').count() > 0, (await A.locator('.device-chip').allInnerTexts()).join('|'));
    await sendText(A, 'Ghi chú cho chính mình');
    await A.waitForTimeout(1200);
    record('self note is encrypted, sent and shown', await A.locator('.message-row', { hasText: 'Ghi chú cho chính mình' }).count() > 0, '');
    await shot(A, 'self_chat_light_1366x768');
  });
  await safe('composer suggestions', async () => {
    await openDm(A, NAMES.bob);
    const dmInput = A.locator('#dm-chat-input');
    await dmInput.fill('chào @' + NAMES.bob.slice(0, 4));
    await A.waitForTimeout(400);
    record('@ mention suggestions appear', await A.locator('.suggest-list .suggest-item').count() > 0, '');
    await dmInput.press('Tab');
    record('Tab inserts the mention', (await dmInput.inputValue()).includes('@' + NAMES.bob + ' '), await dmInput.inputValue());
    await dmInput.fill('cảm ơn');
    await A.waitForTimeout(900);
    record('sticker suggestions match the message', await A.locator('.sticker-suggest .sticker-suggest-cell').count() > 0, '');
    await dmInput.fill('');
  });
  await safe('openmoji stickers', async () => {
    await openDm(A, NAMES.bob);
    await A.locator('button[aria-label="Chọn sticker"]').first().click();
    await A.waitForTimeout(800);
    await A.locator('.picker-popover .picker-tab', { hasText: 'OpenMoji' }).click();
    await A.waitForTimeout(1200);
    record('OpenMoji tab lists many stickers', await A.locator('.openmoji-grid .sticker-cell').count() > 100, String(await A.locator('.openmoji-grid .sticker-cell').count()));
    record('OpenMoji categories are shown', await A.locator('.picker-subtabs .picker-subtab').count() >= 9, '');
    record('OpenMoji credit and license are shown', /OpenMoji/.test(await A.locator('.picker-credit').innerText()) && /CC BY-SA 4\.0/.test(await A.locator('.picker-credit').innerText()), '');
    await A.locator('.picker-subtab[aria-label="Food & Drink"]').click();
    await A.waitForTimeout(900);
    const pizza = A.locator('.openmoji-grid img[alt="Pizza"]');
    record('OpenMoji category switch shows food', await pizza.count() === 1, '');
    const loaded = await pizza.evaluate(async (img) => { img.scrollIntoView(); for (let i = 0; i < 30 && !img.complete; i++) await new Promise((r) => setTimeout(r, 100)); return img.naturalWidth > 0; });
    record('OpenMoji SVG image loads', loaded, '');
    await shot(A, 'openmoji_picker_light_1366x768');
    await A.locator('.picker-popover input[aria-label="Search stickers"]').fill('mèo');
    await A.waitForTimeout(600);
    record('Vietnamese search finds OpenMoji cats', await A.locator('.picker-popover .sticker-cell img[alt="Cat"]').count() > 0, '');
    await A.locator('.picker-popover .sticker-cell img[alt="Cat"]').first().click();
    await A.waitForTimeout(1500);
    record('OpenMoji sticker sends and renders', await A.locator('.message-row.sent .sticker-img[src*="/stickers/openmoji/"]').count() > 0, '');
    record('OpenMoji sticker renders for receiver', await B.locator('.message-row.received .sticker-img[src*="/stickers/openmoji/"]').count() > 0, '');
    const dmInput = A.locator('#dm-chat-input');
    await dmInput.fill('pizza');
    await A.waitForTimeout(900);
    record('typing "pizza" suggests the OpenMoji sticker', await A.locator('.sticker-suggest img[src*="/stickers/openmoji/1F355.svg"]').count() > 0, '');
    await dmInput.fill('');
  });
  await safe('spreadsheet paste', async () => {
    await A.evaluate(() => {
      const el = document.getElementById('dm-chat-input');
      const dt = new DataTransfer();
      dt.setData('text/plain', 'Tên\tSố lượng\nTáo\t3\nCam\t5');
      el.dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true }));
    });
    await A.waitForTimeout(600);
    const asked = await A.locator('.paste-choice').count() > 0;
    record('pasting spreadsheet cells asks image / table / text', asked, '');
    await shot(A, 'paste_choice_light_1366x768');
    await A.locator('.paste-choice button', { hasText: 'Dạng bảng' }).click();
    await A.waitForTimeout(2000);
    record('table message renders for the receiver (E2EE)', await B.locator('.table-card').count() > 0, '');
  });
  await safe('poll', async () => {
    await A.locator('.composer-tools > button').click();
    await A.locator('.tools-menu .menu-item', { hasText: 'Bình chọn' }).click();
    await A.locator('input[placeholder="Đi ăn trưa ở đâu?"]').fill('Tối nay ăn gì?');
    await A.locator('input[placeholder="Lựa chọn 1"]').fill('Phở');
    await A.locator('input[placeholder="Lựa chọn 2"]').fill('Bún chả');
    await A.locator('.modal-overlay button.btn-primary', { hasText: 'Gửi' }).click();
    await A.waitForTimeout(2000);
    const poll = B.locator('.poll-option', { hasText: 'Phở' }).first();
    record('poll arrives for the friend', await poll.count() > 0, '');
    await poll.click(); await B.waitForTimeout(2000);
    record('vote is counted on both sides', (await A.locator('.poll-option', { hasText: 'Phở' }).first().innerText()).includes('1'), '');
  });
  await safe('message pin + seen', async () => {
    await sendText(A, 'Tin nhắn quan trọng');
    await A.waitForTimeout(800);
    const row = A.locator('.message-row', { hasText: 'Tin nhắn quan trọng' }).last();
    await row.hover();
    await row.locator('button[aria-label="Ghim tin nhắn"]').click();
    await A.waitForTimeout(900);
    record('pinned message bar appears', await A.locator('.pinned-bar').count() > 0, '');
    await openDm(B, NAMES.alice);
    await A.waitForTimeout(1500);
    record('"Đã xem" shows after the friend reads', await A.locator('.seen-indicator').count() > 0, '');
    await shot(A, 'dm_pins_seen_light_1366x768');
  });
  await safe('conversation menu', async () => {
    const bobRow = A.locator('.conv-row', { hasText: NAMES.bob }).first();
    await bobRow.hover(); await bobRow.locator('.conv-more').click();
    await A.locator('.conv-menu .account-menu-item', { hasText: 'Ghim lên đầu' }).click();
    await A.waitForTimeout(800);
    record('pinned conversation moves to the pinned section', await A.locator('.sidebar-section-header', { hasText: 'Đã ghim' }).count() > 0, '');
    await A.locator('.chat-header button[aria-label="Tùy chọn khác"]').click();
    await A.locator('.chat-more-menu .account-menu-item', { hasText: 'Chặn người này' }).click();
    await A.waitForTimeout(900);
    record('blocking shows a banner and disables sending', await A.locator('.chat-banner.danger').count() > 0 && await A.locator('.composer-disabled').count() > 0, '');
    await shot(A, 'blocked_light_1366x768');
    await A.locator('.chat-banner.danger button', { hasText: 'Bỏ chặn' }).click();
    await A.waitForTimeout(900);
    record('unblocking restores the composer', await A.locator('#dm-chat-input').count() > 0, '');
  });
  await safe('group management', async () => {
    await A.locator('.conv-row', { hasText: `Nhóm dự án ${RUN}` }).first().locator('.chat-item').click();
    await A.waitForTimeout(1500);
    await A.locator('.chat-header button[aria-label="Tùy chọn khác"]').click();
    await A.locator('.chat-more-menu .account-menu-item', { hasText: 'Quản lý nhóm' }).click();
    await A.waitForTimeout(900);
    record('group management dialog opens', await A.locator('.manage-dialog').count() > 0, '');
    await shot(A, 'group_manage_light_1366x768');
    await A.locator('.manage-dialog .filter-chip', { hasText: 'Tham gia' }).click();
    await A.waitForTimeout(900);
    record('group invite is a link', (await A.locator('.invite-link-text').first().innerText()).includes('/invite/g/'), await A.locator('.invite-link-text').first().innerText());
    await A.keyboard.press('Escape');
    await A.waitForTimeout(300);
  });
  await safe('search page', async () => {
    await A.locator('.app-rail button[aria-label="Tìm kiếm"]').click();
    await A.waitForTimeout(500);
    await A.locator('#global-search-input').fill(NAMES.charlie.slice(0, 9));
    await A.waitForTimeout(1200);
    record('full-page search finds friends', await A.locator('.search-page .result-card', { hasText: NAMES.charlie }).count() > 0, '');
    await shot(A, 'search_light_1366x768');
  });
  await safe('language', async () => {
    await A.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
    await A.locator('.settings-nav-item', { hasText: 'Ngôn ngữ' }).click();
    await A.locator('.lang-tile', { hasText: 'English' }).click();
    await A.waitForTimeout(500);
    record('language switch translates the rail', await A.locator('.app-rail button[aria-label="Messages"]').count() > 0, '');
    await A.locator('.lang-tile', { hasText: 'العربية' }).click();
    await A.waitForTimeout(400);
    record('right-to-left languages flip the layout', (await A.evaluate(() => document.documentElement.dir)) === 'rtl', '');
    await shot(A, 'settings_language_ar_1366x768');
    await A.locator('.lang-tile', { hasText: 'Tiếng Việt' }).click();
    await A.waitForTimeout(400);
  });
  await safe('pin lock', async () => {
    await A.locator('.settings-nav-item', { hasText: 'Khóa ứng dụng' }).click();
    await A.waitForTimeout(400);
    const pins = A.locator('input.pin-input');
    await pins.nth(0).fill('2468'); await pins.nth(1).fill('2468');
    await A.locator('button.btn-primary', { hasText: 'Đặt mã PIN' }).click();
    await A.waitForTimeout(2000);
    await A.locator('button', { hasText: 'Khóa ngay' }).click();
    await A.waitForTimeout(600);
    record('lock screen appears', await A.locator('.lock-screen').isVisible(), '');
    await shot(A, 'pin_lock_light_1366x768');
    await A.keyboard.type('1111'); await A.keyboard.press('Enter'); await A.waitForTimeout(1500);
    record('wrong PIN is rejected', await A.locator('.lock-screen .status-badge.error').count() > 0, '');
    await A.keyboard.type('2468'); await A.keyboard.press('Enter'); await A.waitForTimeout(2000);
    record('correct PIN unlocks', await A.locator('.lock-screen').count() === 0, '');
  });
  await safe('settings pages render', async () => {
    for (const tab of ['Trò chuyện', 'Thông báo và âm thanh', 'Quyền riêng tư và chặn', 'Trả lời tự động', 'Kinh doanh', 'Bot']) {
      await A.locator('.settings-nav-item', { hasText: tab }).click();
      await A.waitForTimeout(700);
      const ok = await A.locator('.settings-content .settings-group').count() > 0;
      record(`settings page "${tab}" renders`, ok, '');
      await shot(A, `settings_${tab.split(' ')[0].normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/đ/g, 'd').toLowerCase()}_light_1366x768`);
    }
  });
  for (let i = 0; i < 2; i++) {
    const back = A.locator('button[aria-label="Đóng cài đặt"]').first();
    if (!(await back.isVisible().catch(() => false))) break;
    await back.click(); await A.waitForTimeout(300);
  }

  // reset wallpaper/pins for the layout sweep that follows
  await A.evaluate((id) => localStorage.removeItem('chatw_prefs_' + id), alice.me.id);
  await A.reload(); await waitApp(A);
  await openDm(A, NAMES.bob);

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
      let settingsBtn = alice.page.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').first();
      if (!(await settingsBtn.isVisible().catch(() => false))) {
        await alice.page.locator('.rail-account').first().click().catch(() => {});
        await alice.page.waitForTimeout(200);
        settingsBtn = alice.page.locator('.rail-menu button[aria-label="Mở cài đặt"]').first();
      }
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
  await alice.page.locator('.app-rail button[aria-label="Cộng đồng"]').first().click();
  await alice.page.waitForTimeout(500);
  await shot(alice.page, 'communities_list_light_1366x768');
  record('rail shows navigation, no servers by default', await alice.page.locator('.app-rail .rail-item').count() >= 4 && await alice.page.locator('.rail-pin').count() === 0, '');
  const railText = await alice.page.evaluate(() => (() => { const r = document.querySelector('.app-rail').cloneNode(true); r.querySelectorAll('.user-avatar').forEach((e) => e.remove()); r.style.position = 'fixed'; document.body.appendChild(r); const t = r.innerText; r.remove(); return t; })().replace(/[0-9+\s]/g, ''));
  record('rail is icon-only (no text labels)', railText === '', `text="${railText}"`);
  record('all rail icons use the ChatW Duo set', await alice.page.evaluate(() => [...document.querySelectorAll('.app-rail svg')].every((s) => s.classList.contains('cw-icon'))), '');
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
    await alice.page.locator('.app-rail button[aria-label="Tin nhắn"]').first().click();
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
