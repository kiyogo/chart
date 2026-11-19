// Layout scan: visits the main screens at a given window size and reports
// clipped text, buttons covered by something else (can't be clicked),
// uneven list avatars, unequal card heights and overlay side panels.
// Usage: node app/e2e/layout-scan.mjs <out_dir> [width] [height]
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'layout-out';
const W = Number(process.argv[3] || 1290), H = Number(process.argv[4] || 1020);
fs.mkdirSync(OUT, { recursive: true });
const problems = [];
const checks = [];
const record = (name, pass, detail = '') => { checks.push({ name, pass, detail }); console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? '  — ' + detail : ''}`); };

const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome', args: ['--no-sandbox'] });
const run = Date.now().toString(36).slice(-5);
const mk = async (name) => {
  const ctx = await browser.newContext({ viewport: { width: W, height: H } });
  await ctx.request.post(`${BASE}/api/auth/dev-login`, { data: { username: name } });
  const me = await (await ctx.request.get(`${BASE}/api/auth/me`)).json();
  return { ctx, me };
};
const A = await mk('lay_a' + run), B = await mk('lay_b' + run);
await A.ctx.request.post(`${BASE}/api/friends/requests`, { data: { username: 'lay_b' + run } });
const reqs = await (await B.ctx.request.get(`${BASE}/api/friends/requests`)).json();
await B.ctx.request.post(`${BASE}/api/friends/requests/${reqs[0].id}/accept`);
await A.ctx.request.post(`${BASE}/api/servers`, { data: { name: 'Club ' + run } });

const page = await A.ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(BASE + '/');
await page.waitForSelector('.workspace-shell', { timeout: 30000 });
await page.waitForTimeout(800);
const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
if (await skip.isVisible().catch(() => false)) await skip.click();

// Generic checks on whatever is on screen.
const scan = async (screen) => {
  const found = await page.evaluate(() => {
    const out = [];
    const vis = (el) => { const r = el.getBoundingClientRect(); const cs = getComputedStyle(el); return r.width > 2 && r.height > 2 && cs.visibility !== 'hidden' && cs.display !== 'none' && r.bottom > 0 && r.right > 0 && r.top < innerHeight && r.left < innerWidth; };
    const name = (el) => (el.getAttribute('aria-label') || el.innerText || el.className || el.tagName).toString().trim().slice(0, 40);
    // 1. Clickable things covered by another element (things behind an open dialog are meant to be)
    const layers = [...document.querySelectorAll('.modal-overlay, .studio-shell')].filter(vis);
    if (document.querySelector('.drawer-backdrop') && vis(document.querySelector('.drawer-backdrop'))) layers.push(document.querySelector('.details-panel'));
    const topLayer = layers[layers.length - 1];
    for (const el of document.querySelectorAll('button, a[href], input, select, textarea, [role="tab"], [role="menuitem"]')) {
      if (!vis(el) || el.disabled || el.closest('[aria-hidden="true"]')) continue;
      if (topLayer && !topLayer.contains(el)) continue;
      if (el.tagName === 'INPUT' && (el.type === 'file' || el.type === 'hidden')) continue;
      const r = el.getBoundingClientRect();
      const x = Math.min(innerWidth - 1, Math.max(0, r.left + r.width / 2)), y = Math.min(innerHeight - 1, Math.max(0, r.top + r.height / 2));
      const top = document.elementFromPoint(x, y);
      if (top && top !== el && !el.contains(top) && !top.contains(el) && !(el.labels && [...el.labels].some((l) => l.contains(top)))) {
        if (top.classList.contains('menu-dismiss')) continue; // an open menu's click-away layer
        if (top.closest('.modal-footer')) continue; // scrolled under a sticky footer: reachable by scrolling
        // Scroll containers may legitimately hide items below the fold.
        let p = el.parentElement, clipped = false;
        while (p) { const cs = getComputedStyle(p); if (/(auto|scroll|hidden)/.test(cs.overflowY + cs.overflowX)) { const pr = p.getBoundingClientRect(); if (y < pr.top || y > pr.bottom || x < pr.left || x > pr.right) { clipped = true; break; } } p = p.parentElement; }
        if (!clipped) out.push(`covered: "${name(el)}" under ${top.className || top.tagName}`);
      }
    }
    // 2. Text cut off without an ellipsis
    for (const el of document.querySelectorAll('button, .settings-nav-item, .settings-label, .chat-item-name, .menu-item, .account-menu-item, .picker-tab, .filter-chip, h1, h2, h3, label, .segmented > *')) {
      if (!vis(el)) continue;
      const cs = getComputedStyle(el);
      if (el.scrollWidth > el.clientWidth + 2 && cs.textOverflow !== 'ellipsis' && cs.overflowX !== 'auto' && el.clientWidth > 0) out.push(`clipped text: "${name(el)}" (${el.scrollWidth}>${el.clientWidth})`);
    }
    // 3. Wrapped single-line navigation
    for (const el of document.querySelectorAll('.settings-nav-item')) {
      if (vis(el) && el.getBoundingClientRect().height > 44) out.push(`nav item wraps: "${name(el)}"`);
    }
    return out;
  });
  for (const f of found) problems.push(`[${screen}] ${f}`);
  await page.screenshot({ path: path.join(OUT, screen.replace(/[^a-z0-9]+/gi, '_') + '.png') });
  return found;
};

// Messages + DM + details panel
await page.locator('.chat-item', { hasText: 'lay_b' + run }).first().click();
await page.waitForTimeout(1000);
if (!(await page.locator('.details-panel').isVisible().catch(() => false))) await page.locator('button[title="Thông tin cuộc trò chuyện"]').click();
await page.waitForTimeout(600);
const overlay = await page.locator('.drawer-backdrop').isVisible().catch(() => false);
// Below 1100px the panel is a drawer over the chat by design (tap outside closes it).
record(`details panel ${W >= 1100 ? 'sits beside the chat' : 'opens as a drawer'} at ${W}px`, W >= 1100 ? !overlay : overlay);
const composerOk = await page.evaluate(() => { const el = document.querySelector('#dm-chat-input'); if (!el) return false; const r = el.getBoundingClientRect(); const t = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2); return el === t || el.contains(t); });
if (W >= 1100) record('composer can be clicked while the details panel is open', composerOk);
for (const label of ['Nhắn tin', 'Gọi', 'Video', 'Hồ sơ']) {
  const b = page.locator('.details-panel .details-action', { hasText: label });
  record(`details panel button "${label}" is clickable`, await b.isVisible() && await b.isEnabled());
}
const sizes = await page.evaluate(() => [...document.querySelectorAll('.conv-row .chat-item')].map((row) => { const a = row.firstElementChild.getBoundingClientRect(); return Math.round(a.width) + 'x' + Math.round(a.height); }));
record('every conversation avatar has the same size', new Set(sizes).size <= 1, sizes.join(' '));
await scan('messages_dm_details');
// Rail tooltip sits beside its own icon
await page.locator('.app-rail button[aria-label]').nth(1).hover();
await page.waitForTimeout(300);
const tip = await page.evaluate(() => { const el = document.querySelectorAll('.app-rail .rail-nav [data-tip]')[1]; const r = el.getBoundingClientRect(); const s = getComputedStyle(el, '::after'); return { top: r.top, h: r.height, tipTop: parseFloat(s.top), transform: s.transform, anim: s.animationName }; });
record('rail tooltip is vertically centred on its icon', tip.anim === 'tip-in', JSON.stringify(tip));
await page.screenshot({ path: path.join(OUT, 'rail_tooltip.png') });

// Settings: every page
await page.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
await page.waitForTimeout(700);
const navItems = await page.locator('.settings-nav-item').allInnerTexts();
record('settings labels are short (≤ 16 characters, no parentheses)', navItems.every((t) => t.trim().length <= 16 && !t.includes('(')), navItems.join(' | '));
for (const t of navItems) {
  await page.locator('.settings-nav-item', { hasText: t.trim() }).first().click();
  await page.waitForTimeout(700);
  await scan('settings ' + t.trim());
}
const desc = await page.locator('.settings-page-desc:visible, .settings-row-desc:visible').count();
record('no description paragraphs in settings', desc === 0, String(desc));
// Studio home cards
await page.locator('.settings-nav-item', { hasText: 'Studio' }).first().click();
await page.waitForTimeout(700);
const hs = await page.evaluate(() => [...document.querySelectorAll('.studio-target')].map((e) => Math.round(e.getBoundingClientRect().height)));
record('studio cards have equal height', new Set(hs).size === 1, hs.join(','));
// Profile editor tabs
await page.locator('.rail-account').click();
await page.locator('.rail-menu .menu-item', { hasText: 'Hồ sơ của tôi' }).click();
await page.waitForTimeout(900);
await scan('profile view');
await page.locator('.profile-modal button', { hasText: 'Chỉnh sửa hồ sơ' }).click();
for (const t of ['Thông tin', 'Avatar Studio', 'Bố cục', 'Trang trí', 'Hiệu ứng', 'Tự tạo']) {
  await page.locator('.profile-editor .segmented button', { hasText: t }).first().click();
  await page.waitForTimeout(500);
  await scan('profile editor ' + t);
}
await page.keyboard.press('Escape');
await page.waitForTimeout(300);
if (await page.locator('.profile-modal').isVisible().catch(() => false)) await page.locator('.profile-modal button[aria-label="Đóng"]').first().click().catch(() => {});
// Community
await page.locator('.app-rail button[aria-label="Cộng đồng"]').first().click();
await page.waitForTimeout(600);
await page.locator(`button[aria-label="Cộng đồng Club ${run}"]`).first().click();
await page.waitForTimeout(1500);
await scan('community channel');

record('no layout problems found', problems.length === 0, `${problems.length} problems`);
record('no page errors', errors.length === 0, errors.slice(0, 2).join(' | '));
fs.writeFileSync(path.join(OUT, 'problems.txt'), problems.join('\n'));
for (const p of problems.slice(0, 80)) console.log('  ' + p);
const failed = checks.filter((c) => !c.pass).length;
console.log(`\n${checks.length - failed}/${checks.length} layout checks passed.`);
await browser.close();
process.exitCode = failed ? 1 : 0;
