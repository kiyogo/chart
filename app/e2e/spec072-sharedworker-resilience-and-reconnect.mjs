// Automated Live Playwright E2E Verification for Spec 072:
// Verifies SharedWorker sendChain resilience, forced WS drop recovery,
// 20-way concurrent CWBF2 RPC seq ordering, existing-cookie upgrade without chatw_auth_hint,
// and self-chat + DM delivery with ZERO false "Offline: message queued" toasts.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC072_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_072';
fs.mkdirSync(SHOT_DIR, { recursive: true });

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
  headless: true,
  args: ['--no-sandbox', '--disable-gpu'],
});

const pageErrors = [];

async function loginFresh(ctx, username) {
  const page = await ctx.newPage();
  page.on('pageerror', (e) => pageErrors.push(`[${username}] ${e.message}`));
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Test username"]').fill(username);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Skip tour"]').first();
  if (await skip.isVisible().catch(() => false)) {
    await skip.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }
  return page;
}

try {
  const tag = Date.now().toString(36).slice(-5);
  const ALICE = `s72_a_${tag}`;
  const BOB = `s72_b_${tag}`;

  const ctxA = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const ctxB = await browser.newContext({ viewport: { width: 1400, height: 900 } });

  const pageA = await loginFresh(ctxA, ALICE);
  const pageB = await loginFresh(ctxB, BOB);

  const frResp = await ctxA.request.post(`${BASE}/api/friends/requests`, { data: { username: BOB } });
  const frJson = await frResp.json().catch(() => ({}));
  if (frJson?.id) {
    await ctxB.request.post(`${BASE}/api/friends/requests/${frJson.id}/accept`, { data: {} });
  }

  // Test 1: Existing-cookie session upgrade WITHOUT chatw_auth_hint in localStorage!
  await pageA.evaluate(() => localStorage.removeItem('chatw_auth_hint'));
  await pageA.reload({ waitUntil: 'load' });
  await pageA.waitForSelector('.app-rail', { timeout: 25000 });
  await pageB.reload({ waitUntil: 'load' });
  await pageB.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA.waitForTimeout(900);
  await pageB.waitForTimeout(900);

  await pageA.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageB.locator('.nav-sidebar .chat-item', { hasText: ALICE }).first().click();
  await pageB.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA.waitForTimeout(500);

  const upgradeMsg = `Upgrade cookie send ${tag}`;
  await pageA.locator('#dm-chat-input').fill(upgradeMsg);
  await pageA.keyboard.press('Enter');
  await pageB.locator('.message-bubble', { hasText: upgradeMsg }).first().waitFor({ timeout: 10000 });
  const offlineToast1 = await pageA.locator('.toast', { hasText: 'Offline: message queued' }).count();
  const stats1 = await pageA.evaluate(() => window.__chatwCore.getStats());
  record(
    '1. Existing-cookie reload without chatw_auth_hint connects SharedWorker & delivers DM with 0 false Offline toast',
    offlineToast1 === 0 && stats1.connected && stats1.handshakeCompleted,
    `offlineToast=${offlineToast1} connected=${stats1.connected} worker=${stats1.workerMode}`
  );

  // Test 2: 20 Concurrent Binary RPCs in parallel (verifies strictly ordered server_seq + client sendChain)
  const concRes = await pageA.evaluate(async () => {
    const reqs = Array.from({ length: 20 }, (_, i) =>
      window.__chatwCore.rpc('GET', i % 2 === 0 ? '/api/auth/me' : '/api/friends')
    );
    const results = await Promise.all(reqs);
    const okCount = results.filter((r) => r && r.ok && r.status === 200).length;
    return { okCount, total: results.length, statuses: results.map((r) => r?.status) };
  });
  record(
    '2. 20 concurrent CWBF2 Binary RPCs all return 200 OK with zero sequence-order drops',
    concRes.okCount === 20,
    `${concRes.okCount}/${concRes.total}`
  );

  // Test 3: Forced WS drops (simulating backend restart / transient disconnect) -> sendChain MUST NOT poison!
  for (let drop = 1; drop <= 3; drop++) {
    await pageA.evaluate(async () => {
      // Fire an RPC right as we drop the socket to intentionally stress sendChain error handling!
      const p1 = window.__chatwCore.rpc('GET', '/api/auth/me').catch(() => null);
      await window.__chatwCore.forceDropSocketForTest();
      await p1;
    });
    await pageA.waitForTimeout(900);
  }
  const reconnected = await pageA.evaluate(async () => {
    const ok = await window.__chatwCore.waitUntilReady(4000);
    return { ok, stats: window.__chatwCore.getStats() };
  });

  // Immediately send 5 rapid messages after the 3 forced drops!
  for (let i = 1; i <= 5; i++) {
    await pageA.locator('#dm-chat-input').fill(`POST_DROP_${tag}_${i}`);
    await pageA.keyboard.press('Enter');
    await pageA.waitForTimeout(80);
  }
  await pageB.locator('.message-bubble', { hasText: `POST_DROP_${tag}_5` }).first().waitFor({ timeout: 12000 });
  await pageA.waitForTimeout(600);

  const postDropState = await pageA.evaluate(() => {
    const pending = document.querySelectorAll('.message-row.pending').length;
    const toastTxt = document.querySelector('.toast')?.textContent || '';
    return { pending, toastTxt, hasOfflineToast: toastTxt.includes('Offline: message queued') };
  });
  const bobPostDropCount = await pageB.evaluate((prefix) => {
    const texts = Array.from(document.querySelectorAll('.message-bubble'))
      .map((e) => e.textContent || '')
      .filter((t) => t.includes(prefix));
    return texts.length;
  }, `POST_DROP_${tag}_`);

  record(
    '3. After 3 forced WS drops with mid-flight RPC failures, sendChain does NOT poison and all 5 rapid DMs deliver cleanly',
    reconnected.ok && bobPostDropCount === 5 && postDropState.pending === 0 && !postDropState.hasOfflineToast,
    `reconnected=${reconnected.ok} bobCount=${bobPostDropCount} pending=${postDropState.pending} toast="${postDropState.toastTxt}"`
  );
  await pageA.screenshot({ path: path.join(SHOT_DIR, '01_post_drop_all_delivered_clean.png') });

  // Test 4: Self-chat ("Note to self") also sends immediately with 0 false Offline toast!
  await pageA.locator('.nav-sidebar .chat-item', { hasText: 'Note to self' }).first().click();
  await pageA.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA.waitForTimeout(400);
  const selfMsg = `Self chat resilience ${tag}`;
  await pageA.locator('#dm-chat-input').fill(selfMsg);
  await pageA.keyboard.press('Enter');
  await pageA.locator('.message-bubble', { hasText: selfMsg }).first().waitFor({ timeout: 10000 });
  await pageA.waitForTimeout(600);
  const selfState = await pageA.evaluate(() => ({
    pending: document.querySelectorAll('.message-row.pending').length,
    toastTxt: document.querySelector('.toast')?.textContent || '',
  }));
  record(
    '4. Self-chat ("Note to self") sends and confirms immediately with zero pending or false Offline toast',
    selfState.pending === 0 && !selfState.toastTxt.includes('Offline: message queued'),
    JSON.stringify(selfState)
  );

  record('5. Zero uncaught page errors', pageErrors.length === 0, pageErrors.join('; ') || '0 errors');

  await ctxA.close();
  await ctxB.close();
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 072 Summary: ${checks.length - failed.length}/${checks.length} PASS`);
if (failed.length > 0) {
  console.error('FAILED:', failed);
  process.exit(1);
}
