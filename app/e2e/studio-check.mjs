// ChatW Studio end-to-end check — real Dioxus web bundle against the real backend.
//
// Usage: node app/e2e/studio-check.mjs <out_dir>
// Same requirements as ui-audit.mjs (DEV_LOGIN on loopback, Playwright, system Chrome).
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'studio-out';
fs.mkdirSync(OUT, { recursive: true });
const checks = [];
const record = (name, pass, detail = '') => { checks.push({ name, pass, detail }); console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? '  — ' + detail : ''}`); };
const shot = (page, n) => page.screenshot({ path: path.join(OUT, n + '.png') });

const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome', args: ['--no-sandbox'] });
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, acceptDownloads: true });
const run = Date.now().toString(36).slice(-5);
await ctx.request.post(`${BASE}/api/auth/dev-login`, { data: { username: 'studio_' + run } });
const me = await (await ctx.request.get(`${BASE}/api/auth/me`)).json();
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
await page.goto(BASE + '/');
await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
await page.waitForTimeout(800);
const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
if (await skip.isVisible().catch(() => false)) await skip.click();

const api = async (method, url, data) => { const r = await ctx.request.fetch(`${BASE}${url}`, { method, data }); let b = null; try { b = await r.json(); } catch (_) {} return { status: r.status(), body: b }; };
const state = () => page.evaluate(() => window.ChatwStudio && window.ChatwStudio.state());
const drag = async (from, to, steps = 12) => {
  const box = await page.locator('#studio-overlay').boundingBox();
  const s = await state();
  const px = (p) => ({ x: box.x + (p[0] / s.w) * box.width, y: box.y + (p[1] / s.h) * box.height });
  const a = px(from), b = px(to);
  await page.mouse.move(a.x, a.y); await page.mouse.down(); await page.mouse.move(b.x, b.y, { steps }); await page.mouse.up();
  await page.waitForTimeout(80);
};
const pixel = (x, y, frame) => page.evaluate(([x, y, f]) => {
  const im = new Image(); return new Promise((ok) => { im.onload = () => { const c = document.createElement('canvas'); c.width = im.width; c.height = im.height; const g = c.getContext('2d'); g.drawImage(im, 0, 0); ok(Array.from(g.getImageData(x, y, 1, 1).data)); }; im.src = window.ChatwStudio.png(f); });
}, [x, y, frame]);

try {
  await page.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await page.locator('.settings-nav-item', { hasText: 'ChatW Studio' }).click();
  await page.waitForTimeout(600);
  record('Studio home lists 8 targets', await page.locator('.studio-target').count() === 8);
  await page.locator('.studio-target', { hasText: 'Sticker' }).click();
  await page.waitForTimeout(900);
  let s = await state();
  record('studio opens a 320×320 sticker canvas', !!s && s.w === 320 && s.h === 320, JSON.stringify(s && [s.w, s.h]));

  // Brush stroke in red
  await page.locator('.studio-palette button[aria-label="Màu #EF4444"]').click();
  await page.locator('input[aria-label="Cỡ nét"]').fill('20');
  await drag([60, 160], [260, 160]);
  let p = await pixel(160, 160, 0);
  record('brush paints on the canvas', p[0] > 200 && p[1] < 90 && p[3] > 200, JSON.stringify(p));
  // Undo / redo
  await page.locator('.studio-top button', { hasText: 'Hoàn tác' }).click();
  p = await pixel(160, 160, 0);
  record('undo removes the stroke', p[3] === 0, JSON.stringify(p));
  await page.locator('.studio-top button', { hasText: 'Làm lại' }).click();
  p = await pixel(160, 160, 0);
  record('redo brings it back', p[3] > 200, JSON.stringify(p));
  // Shapes, fill, eraser, text
  await page.locator('button[aria-label="Rectangle (R)"]').click();
  await page.locator('.studio-palette button[aria-label="Màu #2764E7"]').click();
  await drag([40, 40], [120, 120]);
  p = await pixel(40, 80, 0);
  record('rectangle tool draws an outline', p[2] > 180 && p[3] > 200, JSON.stringify(p));
  await page.locator('button[aria-label="Fill (G)"]').click();
  await page.locator('.studio-palette button[aria-label="Màu #22C55E"]').click();
  await drag([80, 80], [80, 80], 1);
  p = await pixel(80, 80, 0);
  record('fill bucket fills the closed shape', p[1] > 150 && p[0] < 100 && p[3] > 200, JSON.stringify(p));
  await page.locator('button[aria-label="Eraser (E)"]').click();
  await drag([150, 150], [170, 170]);
  p = await pixel(160, 160, 0);
  record('eraser clears pixels', p[3] < 40, JSON.stringify(p));
  await page.locator('button[aria-label="Text (T)"]').click();
  await page.locator('textarea[aria-label="Chữ sẽ đặt lên hình"]').fill('Hi');
  await drag([200, 220], [200, 220], 1);
  s = await state();
  record('text and history are tracked', s.undo >= 5, `undo=${s.undo}`);
  // Layers
  await page.locator('.studio-panel button', { hasText: '+ Lớp' }).click();
  await page.locator('button[aria-label="Ellipse (O)"]').click();
  await page.locator('.studio-palette button[aria-label="Màu #F59E0B"]').click();
  await drag([200, 40], [300, 120]);
  s = await state();
  record('second layer added and selected', s.layers.length === 2 && s.layer === 1, JSON.stringify(s.layers.map((l) => l.name)));
  await page.locator('.studio-layer.active .studio-eye').click();
  s = await state();
  record('layer can be hidden', s.layers[1].visible === false);
  await page.locator('.studio-layer.active .studio-eye').click();
  await page.locator('select[aria-label="Chế độ hòa trộn"]').selectOption('multiply');
  s = await state();
  record('blend mode set on the layer', s.layers[1].blend === 'multiply');
  await shot(page, 'studio_drawing');

  // Animation: tween preset on layer 2 → 8 frames
  await page.locator('.studio-timeline button', { hasText: 'Chuyển động' }).click();
  await page.locator('.studio-chip', { hasText: 'Spin' }).click();
  await page.locator('input[aria-label="Số khung cho chuyển động"]').fill('8');
  await page.locator('.studio-tween button', { hasText: 'Tạo chuyển động' }).click();
  await page.waitForTimeout(500);
  s = await state();
  record('motion preset creates 8 frames', s.frames === 8, `frames=${s.frames}`);
  record('timeline shows frame thumbnails', await page.locator('.studio-frame').count() === 8);
  const bgKept = await pixel(80, 80, 5);
  record('other layers stay on generated frames', bgKept[1] > 150 && bgKept[3] > 200, JSON.stringify(bgKept));
  await page.locator('.studio-timeline button', { hasText: '+ Khung' }).click();
  s = await state();
  record('add frame', s.frames === 9 && s.frame === 9 - 1 + 0 || s.frames === 9, `frames=${s.frames}`);
  await page.locator('.studio-timeline button', { hasText: 'Xóa khung' }).click();
  await page.locator('input[aria-label="Số khung hình mỗi giây"]').fill('12');
  await page.locator('input[aria-label="Số khung hình mỗi giây"]').press('Tab');
  await page.locator('.studio-timeline button', { hasText: 'Phát' }).click();
  await page.waitForTimeout(700);
  const playing = await state();
  await page.locator('.studio-timeline button', { hasText: 'Dừng' }).click();
  record('playback runs at the set FPS', playing.playing === true && playing.fps === 12, JSON.stringify([playing.playing, playing.fps]));
  await shot(page, 'studio_timeline');

  // Export GIF (download) and check bytes
  await page.locator('.studio-top button', { hasText: 'Xuất' }).click();
  const [dl] = await Promise.all([page.waitForEvent('download', { timeout: 20000 }), page.locator('.studio-menu .account-menu-item', { hasText: 'GIF động' }).click()]);
  const gifPath = path.join(OUT, 'export.gif');
  await dl.saveAs(gifPath);
  const gif = fs.readFileSync(gifPath);
  record('GIF export downloads an animated GIF89a', gif.slice(0, 6).toString() === 'GIF89a' && gif.length > 500, `${gif.length} bytes`);
  record('GIF has 8 frames', (gif.toString('latin1').match(/\x21\xF9\x04/g) || []).length === 8);

  // Save project, reopen with layers and frames intact
  await page.locator('input[aria-label="Tên dự án"]').fill('Spin test');
  await page.locator('.studio-top button', { hasText: 'Lưu dự án' }).click();
  await page.waitForTimeout(1500);
  const list = await api('GET', '/api/drawings');
  const proj = (list.body || [])[0];
  record('project saved on the server', !!proj && proj.name === 'Spin test' && proj.frames === 8 && proj.layers === 2, JSON.stringify(proj && [proj.name, proj.frames, proj.layers]));
  const thumb = proj ? await ctx.request.get(`${BASE}/api/drawings/${proj.id}/thumb`) : null;
  record('project thumbnail is a private PNG', !!thumb && thumb.status() === 200 && thumb.headers()['content-type'] === 'image/png');

  // Use as sticker (animated) and as avatar
  await page.locator('.studio-top button', { hasText: 'Dùng làm' }).click();
  await page.locator('.studio-menu .account-menu-item', { hasText: 'Sticker' }).click();
  await page.waitForTimeout(2500);
  const packs = await api('GET', '/api/stickers/packs');
  const mine = (packs.body || []).find((pk) => pk.name === 'My Drawings');
  record('animated sticker added to "My Drawings"', !!mine && mine.stickers.length === 1 && mine.stickers[0].is_animated === true, JSON.stringify(mine && mine.stickers.map((x) => x.file_url)));
  await page.locator('.studio-top button', { hasText: 'Dùng làm' }).click();
  await page.locator('.studio-menu .account-menu-item', { hasText: 'Ảnh đại diện' }).click();
  await page.waitForTimeout(2000);
  const prof = await api('GET', `/api/users/${me.id}/profile`);
  record('drawing set as avatar', !!(prof.body && prof.body.avatar_url && prof.body.avatar_url.startsWith('/uploads/avatars/')));
  await page.locator('.studio-top button', { hasText: 'Dùng làm' }).click();
  await page.locator('.studio-menu .account-menu-item', { hasText: 'Ảnh bìa' }).click();
  await page.waitForTimeout(2500);
  const prof2 = await api('GET', `/api/users/${me.id}/profile`);
  record('animated drawing set as cover (GIF kept)', !!(prof2.body && /\.gif\?/.test(prof2.body.custom_banner_url || '')), String(prof2.body && prof2.body.custom_banner_url));
  await page.locator('.studio-top button', { hasText: 'Dùng làm' }).click();
  await page.locator('.studio-menu .account-menu-item', { hasText: 'Emoji' }).click();
  await page.waitForTimeout(2000);
  const emojis = await api('GET', '/api/media/emojis');
  record('drawing added as custom emoji', (emojis.body || []).some((e) => e.name === 'spin_test'), JSON.stringify((emojis.body || []).map((e) => e.name)));
  await shot(page, 'studio_used');

  // Close and reopen from the project list
  await page.locator('button[aria-label="Đóng ChatW Studio"]').click();
  await page.waitForTimeout(800);
  record('project listed with thumbnail', await page.locator('.studio-project img').count() >= 1);
  await page.locator('.studio-project-open').first().click();
  await page.waitForTimeout(2000);
  s = await state();
  record('reopened project keeps layers and frames', s.frames === 8 && s.layers.length === 2 && s.layers[1].blend === 'multiply', JSON.stringify([s.frames, s.layers.length, s.layers[1] && s.layers[1].blend]));
  await shot(page, 'studio_reopened');
  await page.locator('button[aria-label="Đóng ChatW Studio"]').click();

  // Server-side validation
  const bad = await api('POST', '/api/drawings', { name: 'x', data: JSON.stringify({ v: 1, w: 64, h: 64, layers: [{ name: 'a' }], frames: [{ cells: ['data:text/html;base64,PHNjcmlwdD4='] }] }) });
  record('server rejects non-PNG cells', bad.status === 400, `status ${bad.status}`);
  const other = await browser.newContext();
  await other.request.post(`${BASE}/api/auth/dev-login`, { data: { username: 'studio_x' + run } });
  const peek = proj ? await other.request.get(`${BASE}/api/drawings/${proj.id}`) : null;
  record('another user cannot open my project', !!peek && peek.status() === 404, `status ${peek && peek.status()}`);
  await other.close();
} catch (e) {
  record('studio flow', false, String(e).split('\n').slice(0, 3).join(' '));
  await shot(page, 'studio_fail').catch(() => {});
}
record('no page errors', errors.filter((e) => !/favicon|ERR_ABORTED/.test(e)).length === 0, errors.slice(0, 3).join(' | '));
fs.writeFileSync(path.join(OUT, 'results.json'), JSON.stringify(checks, null, 2));
const failed = checks.filter((c) => !c.pass).length;
console.log(`\n${checks.length - failed}/${checks.length} studio checks passed.`);
await browser.close();
process.exitCode = failed ? 1 : 0;
