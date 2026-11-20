// Live Chromium verification for Spec 026: Modern Flat Minimalist Poll Card UI
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_026';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

async function api(ctx, method, urlPath, body) {
  const res = await ctx.request.fetch(`${BASE}${urlPath}`, {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : undefined,
    data: body ? JSON.stringify(body) : undefined,
  });
  let json = null;
  try { json = await res.json(); } catch (_) {}
  return { status: res.status(), body: json };
}

async function createAccount(browser, label) {
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  const suffix = `${Date.now().toString(36).slice(-5)}${Math.floor(Math.random() * 90 + 10)}`;
  const username = `${label}_${suffix}`;
  await page.goto(`${BASE}/`);
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(username);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
  const me = (await api(ctx, 'GET', '/api/auth/me')).body;
  return { ctx, page, username, userId: me?.id };
}

const browser = await chromium.launch({
  executablePath: '/usr/bin/google-chrome',
  headless: true,
  args: ['--no-sandbox', '--disable-gpu'],
});

const errors = [];
try {
  const { ctx: ctxA, page: pageA, username: nameA, userId: idA } = await createAccount(browser, 'pA');
  const { ctx: ctxB, page: pageB, username: nameB } = await createAccount(browser, 'pB');
  pageA.on('pageerror', (e) => errors.push(`A:${e.message}`));
  pageB.on('pageerror', (e) => errors.push(`B:${e.message}`));

  // Befriend A and B
  const rq = await api(ctxA, 'POST', '/api/friends/requests', { username: nameB });
  const reqs = (await api(ctxB, 'GET', '/api/friends/requests')).body || [];
  for (const r of reqs) {
    if (r.sender_id === idA) {
      await api(ctxB, 'POST', `/api/friends/requests/${r.id}/accept`);
    }
  }
  await pageA.reload();
  await pageA.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await pageA.waitForTimeout(800);
  await pageB.reload();
  await pageB.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await pageB.waitForTimeout(800);

  // Both open DM with each other
  if (!(await pageA.locator('.chat-item', { hasText: nameB }).first().isVisible().catch(() => false))) {
    await pageA.locator('.app-rail button[aria-label="Bạn bè"]').click();
    await pageA.waitForTimeout(400);
  }
  await pageA.locator('.chat-item', { hasText: nameB }).first().click();
  await pageA.waitForSelector('.composer-box', { timeout: 10000 });
  if (!(await pageB.locator('.chat-item', { hasText: nameA }).first().isVisible().catch(() => false))) {
    await pageB.locator('.app-rail button[aria-label="Bạn bè"]').click();
    await pageB.waitForTimeout(400);
  }
  await pageB.locator('.chat-item', { hasText: nameA }).first().click();
  await pageB.waitForSelector('.composer-box', { timeout: 10000 });
  await pageA.waitForTimeout(400);

  // User A creates a 3-option poll
  await pageA.locator('.composer-tools button[aria-label*="Thêm"]').click();
  await pageA.waitForSelector('.tools-menu', { timeout: 5000 });
  await pageA.locator('.tools-menu button', { hasText: 'Bình chọn' }).click();
  await pageA.waitForSelector('.modal-dialog', { timeout: 5000 });

  await pageA.locator('.modal-dialog input').nth(0).fill('Trưa nay ăn gì?');
  await pageA.locator('.modal-dialog input').nth(1).fill('Bún chả');
  await pageA.locator('.modal-dialog input').nth(2).fill('Phở bò');
  await pageA.locator('.modal-dialog button', { hasText: 'Thêm lựa chọn' }).click();
  await pageA.locator('.modal-dialog input').nth(3).fill('Cơm tấm');
  await pageA.locator('.modal-dialog button', { hasText: 'Gửi' }).click();

  await pageA.waitForSelector('.poll-card', { timeout: 10000 });
  await pageB.waitForSelector('.poll-card', { timeout: 10000 });
  await pageA.waitForTimeout(300);
  await pageA.screenshot({ path: path.join(OUT, '01_poll_unvoted.png') });

  const unvotedFoot = await pageA.locator('.poll-card .rich-foot').last().innerText();
  record('Unvoted poll shows clean footer', unvotedFoot.trim() === 'Chưa có lượt bình chọn', unvotedFoot);

  // User A votes "Phở bò", User B votes "Bún chả" -> 50% / 50%
  await pageA.locator('.poll-card').last().locator('.poll-option', { hasText: 'Phở bò' }).click();
  await pageA.waitForSelector('.poll-option.chosen', { timeout: 10000 });
  await pageB.locator('.poll-card').last().locator('.poll-option', { hasText: 'Bún chả' }).click();
  await pageB.waitForSelector('.poll-option.chosen', { timeout: 10000 });
  await pageA.waitForTimeout(500);
  await pageA.screenshot({ path: path.join(OUT, '02_poll_voted_light.png') });
  await pageB.screenshot({ path: path.join(OUT, '03_poll_received_light.png') });

  const votedStats = await pageA.evaluate(() => {
    const cards = document.querySelectorAll('.poll-card');
    const last = cards[cards.length - 1];
    const kind = last.querySelector('.rich-kind');
    const chosen = last.querySelector('.poll-option.chosen');
    const check = chosen?.querySelector('.poll-check');
    const label = chosen?.querySelector('.poll-label');
    const stats = chosen?.querySelector('.poll-stats');
    const cr = check?.getBoundingClientRect();
    const lr = label?.getBoundingClientRect();
    const sr = stats?.getBoundingClientRect();
    return {
      kindTransform: getComputedStyle(kind).textTransform,
      chosenBoxShadow: getComputedStyle(chosen).boxShadow,
      checkSize: cr ? `${Math.round(cr.width)}x${Math.round(cr.height)}` : null,
      inlineAligned: cr && lr && sr
        ? Math.abs((cr.top + cr.height / 2) - (lr.top + lr.height / 2)) < 2 &&
          Math.abs((lr.top + lr.height / 2) - (sr.top + sr.height / 2)) < 2
        : false,
      statsText: stats?.innerText,
      footText: last.querySelector('.rich-foot')?.innerText,
    };
  });

  record('Poll header is sentence-case (no forced UPPERCASE)', votedStats.kindTransform === 'none', votedStats.kindTransform);
  record(
    'Voted option has NO thick 2px outline and aligns check/label/stats cleanly on 1 row',
    votedStats.chosenBoxShadow === 'none' &&
      votedStats.checkSize === '18x18' &&
      votedStats.inlineAligned &&
      votedStats.statsText?.includes('%') &&
      votedStats.statsText?.includes('1'),
    JSON.stringify(votedStats),
  );

  // Dark theme screenshot
  await pageA.evaluate(() => document.documentElement.setAttribute('data-theme', 'dark'));
  await pageA.waitForTimeout(250);
  await pageA.screenshot({ path: path.join(OUT, '04_poll_voted_dark.png') });

  record('No uncaught page errors', errors.length === 0, errors.join('; '));
  await ctxA.close();
  await ctxB.close();
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
