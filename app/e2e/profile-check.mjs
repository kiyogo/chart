// Spec 012 end-to-end check: profile layout editor, templates, presets,
// showcase, guestbook, achievements, "preview as" and the inventory.
// Usage: node app/e2e/profile-check.mjs <out_dir>   (same requirements as ui-audit.mjs)
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'profile-out';
fs.mkdirSync(OUT, { recursive: true });
const checks = [];
const record = (name, pass, detail = '') => { checks.push({ name, pass, detail }); console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? '  — ' + detail : ''}`); };
const shot = (page, n) => page.screenshot({ path: path.join(OUT, n + '.png') });
const png = () => {
  const crc = (b) => { let c, t = []; for (let n = 0; n < 256; n++) { c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } let x = 0xffffffff; for (const v of b) x = t[(x ^ v) & 0xff] ^ (x >>> 8); return (x ^ 0xffffffff) >>> 0; };
  const chunk = (type, data) => { const len = Buffer.alloc(4); len.writeUInt32BE(data.length); const td = Buffer.concat([Buffer.from(type), data]); const c = Buffer.alloc(4); c.writeUInt32BE(crc(td)); return Buffer.concat([len, td, c]); };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(8, 0); ihdr.writeUInt32BE(8, 4); ihdr[8] = 8; ihdr[9] = 2;
  const raw = Buffer.alloc(8 * 25); for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) { const i = y * 25 + 1 + x * 3; raw[i] = 220; raw[i + 1] = 40; raw[i + 2] = 120; }
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
};

const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome', args: ['--no-sandbox'] });
const run = Date.now().toString(36).slice(-5);
const mk = async (name) => {
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  await ctx.request.post(`${BASE}/api/auth/dev-login`, { data: { username: name } });
  const me = await (await ctx.request.get(`${BASE}/api/auth/me`)).json();
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(BASE + '/');
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
  return { ctx, page, me, errors };
};
const api = async (u, method, url, data) => { const r = await u.ctx.request.fetch(`${BASE}${url}`, { method, data }); let b = null; try { b = await r.json(); } catch (_) {} return { status: r.status(), body: b }; };
const A = await mk('prof_a' + run), B = await mk('prof_b' + run), C = await mk('prof_c' + run);

try {
  // Friends A <-> B (C stays a stranger)
  await api(A, 'POST', '/api/friends/requests', { username: 'prof_b' + run });
  const reqs = await api(B, 'GET', '/api/friends/requests');
  await api(B, 'POST', `/api/friends/requests/${reqs.body[0].id}/accept`);
  await api(A, 'PUT', '/api/profile-ext/me', { location: 'Đà Nẵng', occupation: 'Illustrator', detail_visibility: { location: 'friends' } });
  await api(A, 'PUT', '/api/users/me/profile', { bio: 'Mình vẽ minh họa và làm sticker.' });

  const a = A.page;
  await a.locator('.rail-account').click();
  await a.locator('.rail-menu .menu-item', { hasText: 'Hồ sơ của tôi' }).click();
  await a.waitForTimeout(900);
  await a.locator('.profile-modal button', { hasText: 'Chỉnh sửa hồ sơ' }).click();
  await a.locator('.profile-editor .segmented button', { hasText: 'Bố cục' }).click();
  await a.waitForTimeout(500);
  record('21 profile templates offered', await a.locator('.template-tile').count() === 21);
  await a.locator('.template-tile', { hasText: 'Neon Profile' }).click();
  // Move the guestbook to the top with the keyboard-friendly buttons
  for (let i = 0; i < 8; i++) {
    const up = a.locator('.section-row', { hasText: 'Sổ lưu bút' }).locator('button[aria-label^="Đưa Sổ lưu bút lên"]');
    if (await up.isDisabled()) break;
    await up.click();
    await a.waitForTimeout(120);
  }
  await a.locator('.section-row', { hasText: 'Cộng đồng chung' }).locator('input[type="checkbox"]').uncheck();
  await a.locator('select[aria-label="Kích thước Khu trưng bày"]').selectOption('wide');
  await a.locator('select[aria-label="Ai được viết sổ lưu bút"]').selectOption('everyone');
  record('preview uses the template and new order', await a.locator('.profile-editor-preview .profile-card.pc-tpl-neon').count() === 1 && (await a.locator('.profile-editor-preview .pc-section').first().getAttribute('data-section')) === 'guestbook');
  // Drag and drop: move "Huy hiệu" onto the first row
  // Real mouse drag by the handle (native HTML5 drag and drop).
  const src = await a.locator('.section-row', { hasText: 'Thành tích' }).locator('.drag-handle').boundingBox();
  const dst = await a.locator('.section-row').first().locator('.drag-handle').boundingBox();
  await a.mouse.move(src.x + 4, src.y + 4); await a.mouse.down(); await a.mouse.move(dst.x + 4, dst.y + 6, { steps: 12 }); await a.mouse.up();
  await a.waitForTimeout(400);
  const order = await a.locator('.section-row .grow').allInnerTexts();
  record('drag and drop reorders sections', order[0] === 'Thành tích', order.slice(0, 3).join(' > '));
  // Preset
  await a.locator('input[aria-label="Tên bản bố cục"]').fill('Neon showcase');
  await a.locator('.layout-editor button', { hasText: 'Lưu bản hiện tại' }).click();
  await a.waitForTimeout(800);
  record('layout preset saved', await a.locator('.preset-row', { hasText: 'Neon showcase' }).count() === 1);
  // Showcase: a link and an uploaded image
  await a.locator('input[aria-label="Tiêu đề mục trưng bày"]').fill('Portfolio');
  await a.locator('input[aria-label="Địa chỉ liên kết"]').fill('https://example.com/portfolio');
  await a.locator('.layout-editor .inline-form button', { hasText: 'Thêm' }).click();
  await a.waitForTimeout(800);
  await a.locator('select[aria-label="Loại mục trưng bày"]').selectOption('image');
  await a.locator('input[aria-label="Tiêu đề mục trưng bày"]').fill('Pink');
  await a.locator('#showcase-file').setInputFiles({ name: 'pink.png', mimeType: 'image/png', buffer: png() });
  await a.waitForTimeout(1500);
  record('showcase has a link and an image', await a.locator('.showcase-manage .preset-row').count() === 2);
  await a.locator('.showcase-manage .preset-row', { hasText: 'Pink' }).locator('button', { hasText: 'Ghim' }).click();
  await a.waitForTimeout(700);
  // Preview as a stranger: friends-only location disappears
  await a.locator('select[aria-label="Xem hồ sơ như"]').selectOption('stranger');
  await a.waitForTimeout(300);
  const strangerText = await a.locator('.profile-editor-preview').innerText();
  await a.locator('select[aria-label="Xem hồ sơ như"]').selectOption('friend');
  await a.waitForTimeout(300);
  const friendText = await a.locator('.profile-editor-preview').innerText();
  record('"preview as" applies visibility rules', !strangerText.includes('Đà Nẵng') && friendText.includes('Đà Nẵng'));
  await shot(a, 'layout_editor');
  await a.locator('.profile-editor button', { hasText: 'Lưu thay đổi' }).click();
  await a.waitForTimeout(1500);
  const saved = await api(A, 'GET', `/api/profile-plus/${A.me.id}`);
  const ids = (saved.body.sections || []).map((s) => s.id);
  record('layout saved on the server', saved.body.template === 'neon' && ids[0] === 'achievements' && saved.body.sections.find((s) => s.id === 'communities').visible === false && saved.body.guestbook_mode === 'everyone', ids.slice(0, 3).join(','));
  record('showcase pinned item first', saved.body.showcase.length === 2 && saved.body.showcase[0].pinned === true, JSON.stringify(saved.body.showcase.map((s) => s.title)));
  record('achievements computed from real activity', saved.body.achievements.find((x) => x.id === 'first_friend').unlocked === true && saved.body.achievements.find((x) => x.id === 'social').progress === 1);
  await shot(a, 'profile_view_neon');
  await a.locator('.profile-modal button[aria-label="Đóng hồ sơ"]').click().catch(() => {});
  await a.keyboard.press('Escape');
  await a.waitForTimeout(300);

  // B (friend) and C (stranger) see what they are allowed to
  const bView = await api(B, 'GET', `/api/profile-plus/${A.me.id}`);
  const cView = await api(C, 'GET', `/api/profile-plus/${A.me.id}`);
  record('hidden section not sent to others', bView.body.shared_communities.length === 0 && !JSON.stringify(cView.body).includes('"communities":['));
  record('others only see unlocked achievements', bView.body.achievements.every((x) => x.unlocked));
  record('guestbook open to everyone', bView.body.can_sign_guestbook === true && cView.body.can_sign_guestbook === true);
  // B signs the guestbook through the UI (profile modal)
  const b = B.page;
  await b.locator('.chat-item', { hasText: 'prof_a' + run }).first().click();
  await b.waitForTimeout(900);
  if (!(await b.locator('.details-panel').isVisible().catch(() => false))) await b.locator('button[title="Thông tin cuộc trò chuyện"]').click();
  await b.waitForTimeout(600);
  await b.locator('.details-panel button.details-action', { hasText: 'Hồ sơ' }).click();
  await b.waitForTimeout(1200);
  record('friend sees the neon template', await b.locator('.profile-modal .profile-card.pc-tpl-neon').count() === 1);
  await b.locator('input[aria-label="Viết vào sổ lưu bút"]').fill('Tranh đẹp quá!');
  await b.locator('.guestbook-form button[type="submit"]').click();
  await b.waitForTimeout(1200);
  record('guestbook entry posted', await b.locator('.guestbook-entry', { hasText: 'Tranh đẹp quá!' }).count() === 1);
  await shot(b, 'friend_view_guestbook');
  // Spam / injection protections
  const longBody = await api(C, 'POST', `/api/profile-plus/${A.me.id}/guestbook`, { body: 'x'.repeat(301) });
  record('over-long guestbook entry rejected', longBody.status === 400);
  const badShow = await api(A, 'POST', '/api/profile-plus/me/showcase', { kind: 'image', url: 'https://evil.example/x.png' });
  record('external image refs refused in showcase', badShow.status === 400);
  const badTpl = await api(A, 'PUT', '/api/profile-plus/me/layout', { template: 'evil', sections: [] });
  record('unknown template refused', badTpl.status === 400);
  // A removes the entry
  const entry = (await api(A, 'GET', `/api/profile-plus/${A.me.id}`)).body.guestbook[0];
  const cDel = await api(C, 'DELETE', `/api/profile-plus/guestbook/${entry.id}`);
  record("others can't delete someone else's guestbook entry", cDel.status === 404);
  const aDel = await api(A, 'DELETE', `/api/profile-plus/guestbook/${entry.id}`);
  record('owner can delete entries', aDel.status === 200);

  // Inventory
  await a.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await a.locator('.settings-nav-item', { hasText: 'Kho đồ' }).click();
  await a.waitForTimeout(1500);
  record('inventory lists equipped items', await a.locator('.inv-card', { hasText: 'Profile Template' }).filter({ hasText: 'Neon Profile' }).count() === 1);
  record('inventory lists achievements with progress', await a.locator('.inventory .achievement').count() === 15);
  await a.locator('.studio-chip', { hasText: 'Profile Templates' }).click();
  await a.locator('.inv-card', { hasText: 'Neon showcase' }).locator('button', { hasText: 'Áp dụng' }).click();
  await a.waitForTimeout(800);
  record('apply a preset from the inventory', await a.locator('.status-badge.success', { hasText: 'Đã áp dụng bố cục' }).count() === 1);
  await a.locator('.studio-chip', { hasText: 'Tất cả' }).click();
  await shot(a, 'inventory');
} catch (e) {
  record('profile flow', false, String(e).split('\n').slice(0, 3).join(' '));
  await shot(A.page, 'profile_fail').catch(() => {});
}
record('no page errors', [...A.errors, ...B.errors].length === 0, [...A.errors, ...B.errors].slice(0, 2).join(' | '));
fs.writeFileSync(path.join(OUT, 'results.json'), JSON.stringify(checks, null, 2));
const failed = checks.filter((c) => !c.pass).length;
console.log(`\n${checks.length - failed}/${checks.length} profile checks passed.`);
await browser.close();
process.exitCode = failed ? 1 : 0;
