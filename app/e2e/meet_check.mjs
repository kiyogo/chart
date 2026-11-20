// Live Chromium verification for Google Meet / Zoom-style Group Call ("Họp nhóm") & Meet Links
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_meet';
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
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    permissions: ['camera', 'microphone'],
  });
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
  args: [
    '--no-sandbox',
    '--disable-gpu',
    '--use-fake-ui-for-media-stream',
    '--use-fake-device-for-media-stream',
  ],
});

const errors = [];
try {
  const { ctx: ctxA, page: pageA, username: nameA } = await createAccount(browser, 'mA');
  const { ctx: ctxB, page: pageB, username: nameB } = await createAccount(browser, 'mB');
  pageA.on('pageerror', (e) => errors.push(`A:${e.message}`));
  pageB.on('pageerror', (e) => errors.push(`B:${e.message}`));

  // 1. User A clicks "Họp nhóm" button on the left navigation rail
  const meetRailBtnA = pageA.locator('.app-rail button[aria-label="Họp nhóm"]');
  await meetRailBtnA.waitFor({ state: 'visible', timeout: 10000 });
  record('Left rail contains "Họp nhóm" button', await meetRailBtnA.isVisible());

  await meetRailBtnA.click();
  await pageA.waitForSelector('.meet-hero-card', { timeout: 10000 });
  await pageA.waitForTimeout(300);
  await pageA.screenshot({ path: path.join(OUT, '01_meet_hub.png') });

  // 2. User A creates a link for a meeting
  await pageA.locator('input[aria-label="Tên cuộc họp"]').fill('Họp nhóm Sprint 1');
  await pageA.locator('.meet-card button:has-text("Tạo link phòng họp")').click();
  await pageA.waitForSelector('.meet-created-box', { timeout: 10000 });
  await pageA.waitForTimeout(300);

  const meetLink = await pageA.locator('input[aria-label="Đường dẫn phòng họp"]').inputValue();
  const codePill = await pageA.locator('.meet-created-box .meet-code-pill').innerText();
  record('Created meeting generates valid link & code', meetLink.includes('?meet=') && codePill.includes('Mã:'), `${meetLink} (${codePill})`);
  await pageA.screenshot({ path: path.join(OUT, '02_meet_link_created.png') });

  // 3. User A enters the meeting room
  await pageA.locator('.meet-created-box button:has-text("Vào phòng ngay")').click();
  await pageA.waitForSelector('.active-call-overlay', { timeout: 10000 });
  await pageA.waitForTimeout(400);
  const hasCopyInCall = await pageA.locator('.active-call-overlay .call-copy-link-btn').isVisible();
  record('Active call overlay displays meeting code & copy invite link button', hasCopyInCall);
  await pageA.screenshot({ path: path.join(OUT, '03_meet_room_userA.png') });

  // 4. User B navigates to "Họp nhóm", pastes full link, and joins
  await pageB.locator('.app-rail button[aria-label="Họp nhóm"]').click();
  await pageB.waitForSelector('.meet-hero-card', { timeout: 10000 });
  await pageB.locator('input[aria-label="Nhập mã hoặc link phòng họp"]').fill(meetLink);
  await pageB.locator('.meet-card button:has-text("Tham gia ngay")').click();
  await pageB.waitForSelector('.active-call-overlay', { timeout: 10000 });
  await pageB.waitForTimeout(1000);

  // 5. Verify both users see 2 participants in real-time
  const userAPeersText = await pageA.locator('.active-call-overlay').innerText();
  const userBPeersText = await pageB.locator('.active-call-overlay').innerText();
  record(
    'User A sees User B joined live via WebSocket',
    userAPeersText.includes(nameB) && userAPeersText.includes('2 người tham gia'),
    userAPeersText.replace(/\s+/g, ' ').slice(0, 140),
  );
  record(
    'User B sees User A in meeting room',
    userBPeersText.includes(nameA) && userBPeersText.includes('2 người tham gia'),
    userBPeersText.replace(/\s+/g, ' ').slice(0, 140),
  );
  await pageA.screenshot({ path: path.join(OUT, '04_meet_both_connected_A.png') });
  await pageB.screenshot({ path: path.join(OUT, '05_meet_both_connected_B.png') });

  // 6. Verify leaving room cleans up properly
  await pageB.locator('.active-call-overlay button[aria-label="Rời cuộc gọi"]').click();
  await pageB.waitForTimeout(600);
  record('User B leaves call cleanly', !(await pageB.locator('.active-call-overlay').isVisible()));

  await pageA.locator('.active-call-overlay button[aria-label="Rời cuộc gọi"]').click();
  await pageA.waitForTimeout(500);
  record('User A leaves call cleanly', !(await pageA.locator('.active-call-overlay').isVisible()));

  // 7. Verify deep-link direct join (?meet=code)
  await pageB.goto(meetLink);
  await pageB.waitForSelector('.active-call-overlay', { timeout: 30000 });
  await pageB.waitForTimeout(400);
  record('Opening shared meet URL directly auto-joins the meeting room', await pageB.locator('.active-call-overlay').isVisible());
  await pageB.screenshot({ path: path.join(OUT, '06_meet_deeplink_joined.png') });
  await pageB.locator('.active-call-overlay button[aria-label="Rời cuộc gọi"]').click();

  record('No uncaught page errors', errors.length === 0, errors.join('; '));
  await ctxA.close();
  await ctxB.close();
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
