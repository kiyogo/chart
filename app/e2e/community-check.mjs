// Spec 013 end-to-end check: welcome screen + rules, community profile,
// community emoji, forum threads, events with RSVP, moderation audit log.
// Usage: node app/e2e/community-check.mjs <out_dir>   (same requirements as ui-audit.mjs)
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'community-out';
fs.mkdirSync(OUT, { recursive: true });
const checks = [];
const record = (name, pass, detail = '') => { checks.push({ name, pass, detail }); console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? '  — ' + detail : ''}`); };
const shot = (page, n) => page.screenshot({ path: path.join(OUT, n + '.png') });
const png = (r, g, b) => {
  const crc = (buf) => { let c, t = []; for (let n = 0; n < 256; n++) { c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } let x = 0xffffffff; for (const v of buf) x = t[(x ^ v) & 0xff] ^ (x >>> 8); return (x ^ 0xffffffff) >>> 0; };
  const chunk = (type, data) => { const len = Buffer.alloc(4); len.writeUInt32BE(data.length); const td = Buffer.concat([Buffer.from(type), data]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([len, td, c]); };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(16, 0); ihdr.writeUInt32BE(16, 4); ihdr[8] = 8; ihdr[9] = 2;
  const raw = Buffer.alloc(16 * 49); for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) { const i = y * 49 + 1 + x * 3; raw[i] = r; raw[i + 1] = g; raw[i + 2] = b; }
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
};

const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome', args: ['--no-sandbox'] });
const run = Date.now().toString(36).slice(-5);
const mk = async (name, open = true) => {
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  await ctx.request.post(`${BASE}/api/auth/dev-login`, { data: { username: name } });
  const me = await (await ctx.request.get(`${BASE}/api/auth/me`)).json();
  let page = null;
  const errors = [];
  if (open) {
    page = await ctx.newPage();
    page.on('pageerror', (e) => errors.push(String(e)));
    await page.goto(BASE + '/');
    await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
    await page.waitForTimeout(700);
    const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
    if (await skip.isVisible().catch(() => false)) await skip.click();
  }
  return { ctx, page, me, errors };
};
const api = async (u, method, url, data, raw) => {
  const opts = raw ? { method, headers: { 'content-type': raw.type }, data: raw.body } : { method, data };
  const r = await u.ctx.request.fetch(`${BASE}${url}`, opts);
  let b = null; try { b = await r.json(); } catch (_) {} return { status: r.status(), body: b };
};

const A = await mk('com_a' + run, false), B = await mk('com_b' + run, false);
try {
  // --- Owner sets the community up through the API
  const srv = (await api(A, 'POST', '/api/servers', { name: `Club ${run}` })).body;
  const forum = (await api(A, 'POST', `/api/servers/${srv.id}/channels`, { name: 'hoi-dap', channel_type: 'forum' })).body;
  const evch = (await api(A, 'POST', `/api/servers/${srv.id}/channels`, { name: 'su-kien', channel_type: 'event' })).body;
  record('forum and event channels can be created', forum && forum.channel_type === 'forum' && evch && evch.channel_type === 'event');
  const w = await api(A, 'PUT', `/api/servers/${srv.id}/welcome`, { welcome_message: 'Chào mừng bạn đến Club!', rules: '1. Tôn trọng mọi người\n2. Không spam', require_rules: true });
  record('welcome screen and rules saved', w.status === 200 && w.body.require_rules === true);
  const up = await api(A, 'POST', '/api/media/upload', null, { type: 'image/png', body: png(240, 160, 30) });
  const emo = await api(A, 'POST', `/api/servers/${srv.id}/expressions`, { kind: 'emoji', name: 'club_star', file_url: up.body.url });
  const stk = await api(A, 'POST', `/api/servers/${srv.id}/expressions`, { kind: 'sticker', name: 'club_wave', file_url: up.body.url });
  record('community emoji and sticker added', emo.status === 200 && stk.status === 200, `${emo.status}/${stk.status}`);
  const dup = await api(A, 'POST', `/api/servers/${srv.id}/expressions`, { kind: 'emoji', name: 'club_star', file_url: up.body.url });
  record('duplicate emoji name refused', dup.status === 409);
  const startsAt = new Date(Date.now() + 86400000).toISOString();
  const ev = await api(A, 'POST', `/api/servers/${srv.id}/events`, { title: 'Buổi vẽ chung', description: 'Vẽ sticker cùng nhau', location: '#su-kien', starts_at: startsAt });
  record('event created', ev.status === 200);
  const inv = (await api(A, 'POST', `/api/servers/${srv.id}/invites`, {})).body;
  const code = inv.code || inv.invite_code || inv;
  await api(B, 'POST', `/api/servers/join/${code}`);
  // --- Member: rules gate
  const chans = (await api(B, 'GET', `/api/servers/${srv.id}/channels`)).body;
  const general = chans.find((c) => c.channel_type === 'text');
  const blocked = await api(B, 'POST', `/api/channels/${general.id}/messages`, { content: 'xin chào' });
  record('posting is blocked until the rules are accepted', blocked.status === 403 && blocked.body.error === 'rules_not_accepted', String(blocked.status));
  const noEmoji = await api(B, 'POST', `/api/servers/${srv.id}/expressions`, { kind: 'emoji', name: 'nope', file_url: up.body.url });
  record('members without Manage Expressions cannot add emoji', noEmoji.status === 403);
  const noAudit = await api(B, 'GET', `/api/servers/${srv.id}/audit-log`);
  record('members cannot read the audit log', noAudit.status === 403);

  // --- Member UI
  const page = await B.ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(BASE + '/');
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
  await page.locator('.app-rail button[aria-label="Cộng đồng"]').first().click();
  await page.waitForTimeout(600);
  await page.locator(`button[aria-label="Cộng đồng Club ${run}"]`).first().click();
  await page.waitForTimeout(1500);
  record('welcome screen opens with the rules', await page.locator('.welcome-dialog').isVisible() && (await page.locator('.welcome-rules').innerText()).includes('Không spam'));
  await shot(page, 'welcome');
  record('cannot enter before agreeing', await page.locator('.welcome-dialog button', { hasText: 'Đồng ý và vào cộng đồng' }).isDisabled());
  await page.locator('.welcome-dialog input[type="checkbox"]').check();
  await page.locator('.welcome-dialog button', { hasText: 'Đồng ý và vào cộng đồng' }).click();
  await page.waitForTimeout(800);
  record('accepting closes the welcome screen', await page.locator('.welcome-dialog').count() === 0);
  // Community profile
  await page.locator('.channel-row', { hasText: general.name }).first().click().catch(() => {});
  await page.waitForTimeout(800);
  await page.locator('.chat-header-actions button', { hasText: 'Hồ sơ ở đây' }).click();
  await page.locator('.modal-content input[placeholder="com_b' + run + '"]').fill('Bình Club');
  await page.locator('.modal-content button', { hasText: 'Lưu' }).click();
  await page.waitForTimeout(700);
  await page.keyboard.press('Escape');
  await page.locator('#channel-chat-input').fill('Chào cả nhà');
  await page.locator('#channel-chat-input').press('Enter');
  await page.waitForTimeout(1200);
  const msgs = (await api(A, 'GET', `/api/channels/${general.id}/messages`)).body;
  record('message shows the community nickname', (msgs || []).some((m) => m.sender_name === 'Bình Club'), JSON.stringify((msgs || []).map((m) => m.sender_name)));
  const global = (await api(A, 'GET', `/api/users/${B.me.id}/profile`)).body;
  record('global profile name unchanged', global.username === 'com_b' + run && global.display_name == null);
  // Community emoji in the picker
  await page.locator('button[aria-label="Chọn emoji"]').first().click();
  await page.waitForTimeout(600);
  record('community emoji appear in the emoji picker', await page.locator('.emoji-btn.custom[aria-label=":club_star:"]').count() === 1);
  await page.keyboard.press('Escape');
  await page.locator('button[aria-label="Chọn sticker"]').first().click().catch(() => {});
  await page.waitForTimeout(800);
  record('community stickers appear in the sticker picker', await page.locator('.picker-tab', { hasText: 'Community' }).count() === 1);
  await page.keyboard.press('Escape');
  // Forum
  await page.locator('.channel-row', { hasText: 'hoi-dap' }).first().click();
  await page.waitForTimeout(900);
  await page.locator('input[aria-label="Tiêu đề chủ đề"]').fill('Làm sao để vẽ sticker động?');
  await page.locator('textarea[aria-label="Nội dung chủ đề"]').fill('Mình mới dùng ChatW Studio.');
  await page.locator('.forum-new button', { hasText: 'Đăng chủ đề' }).click();
  await page.waitForTimeout(900);
  record('forum topic created', await page.locator('.forum-row', { hasText: 'Làm sao để vẽ sticker động?' }).count() === 1);
  await page.locator('.forum-row').first().click();
  await page.waitForTimeout(700);
  await page.locator('input[aria-label="Trả lời chủ đề"]').fill('Dùng Motion Presets nhé');
  await page.locator('.forum-thread button[type="submit"]').click();
  await page.waitForTimeout(800);
  record('reply posted in the topic', await page.locator('.forum-post').count() === 2);
  await shot(page, 'forum');
  const threads = (await api(A, 'GET', `/api/channels/${forum.id}/threads`)).body;
  await api(A, 'PUT', `/api/channels/${forum.id}/threads/${threads[0].id}`, { locked: true });
  const lockedReply = await api(B, 'POST', `/api/channels/${forum.id}/threads/${threads[0].id}/messages`, { content: 'còn ai không' });
  record('locked topic refuses replies', lockedReply.status === 403);
  const mainList = (await api(A, 'GET', `/api/channels/${forum.id}/messages`)).body || [];
  record('forum replies stay out of the channel feed', mainList.length === 0, String(mainList.length));
  // Events
  await page.locator('.channel-row', { hasText: 'su-kien' }).first().click();
  await page.waitForTimeout(900);
  record('event channel lists the event', await page.locator('.event-card', { hasText: 'Buổi vẽ chung' }).count() === 1);
  await page.locator('.event-card button', { hasText: 'Tham gia' }).click();
  await page.waitForTimeout(800);
  const evs = (await api(A, 'GET', `/api/servers/${srv.id}/events`)).body;
  record('RSVP counted', evs[0].going === 1, JSON.stringify(evs.map((e) => [e.title, e.going])));
  await shot(page, 'events');
  // Audit log
  const log = (await api(A, 'GET', `/api/servers/${srv.id}/audit-log`)).body.map((x) => x.action);
  record('audit log records moderation actions', ['channel_created', 'welcome_updated', 'emoji_added', 'sticker_added', 'event_created', 'thread_locked'].every((k) => log.includes(k)), log.join(','));
  record('no page errors', errors.length === 0, errors.slice(0, 2).join(' | '));
} catch (e) {
  record('community flow', false, String(e).split('\n').slice(0, 3).join(' '));
}
fs.writeFileSync(path.join(OUT, 'results.json'), JSON.stringify(checks, null, 2));
const failed = checks.filter((c) => !c.pass).length;
console.log(`\n${checks.length - failed}/${checks.length} community checks passed.`);
await browser.close();
process.exitCode = failed ? 1 : 0;
