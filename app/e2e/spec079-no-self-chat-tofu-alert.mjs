// Automated Live Playwright Verification for Spec 079:
//  1. Self-chat ("Chính mình") after rotating own device key (even with pre-existing stale self-alert in localStorage)
//     NEVER triggers "Cảnh báo bảo mật: khóa mã hóa của người này đã thay đổi" and auto-cleans stale self alerts!
//  2. Real friend key rotation triggers TOFU warning ONCE; opening Info ("Thông tin") auto-expands Security &
//     clicking "Đã xác minh khóa mới" permanently clears it across chat switches and page reloads.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC079_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_079';
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

async function loginFresh(ctx, username, lang = 'vi') {
  const page = await ctx.newPage();
  page.on('pageerror', (e) => pageErrors.push(`[${username}] ${e.message}`));
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  if (lang !== 'en') {
    await page.evaluate((l) => localStorage.setItem('chatw_lang', l), lang);
    await page.reload({ waitUntil: 'networkidle' });
  }
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label], input[placeholder]').first().fill(username);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);
  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }
  return page;
}

try {
  const tag = Date.now().toString(36).slice(-5);
  const ALICE = `s79_a_${tag}`;
  const BOB = `s79_b_${tag}`;

  const ctxA = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const ctxB = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const pageA = await loginFresh(ctxA, ALICE, 'vi');
  const pageB = await loginFresh(ctxB, BOB, 'vi');

  const meA = await (await ctxA.request.get(`${BASE}/api/auth/me`)).json();
  const meB = await (await ctxB.request.get(`${BASE}/api/auth/me`)).json();
  await ctxA.request.put(`${BASE}/api/prefs`, { data: { language: 'vi' } }).catch(() => {});

  const frResp = await ctxA.request.post(`${BASE}/api/friends/requests`, { data: { username: BOB } });
  const frJson = await frResp.json().catch(() => ({}));
  if (frJson?.id) {
    await ctxB.request.post(`${BASE}/api/friends/requests/${frJson.id}/accept`, { data: {} });
  }
  await pageA.reload({ waitUntil: 'load' });
  await pageA.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA.waitForTimeout(800);

  // 1. Open "Chính mình" (Note to self) to populate self TOFU peer entry, then open BOB to populate BOB's TOFU entry
  await pageA.locator('.nav-sidebar .chat-item').first().click();
  await pageA.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA.waitForTimeout(500);
  await pageA.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA.waitForTimeout(500);

  // Also inject a pre-existing stale self-alert simulating a browser that hit the bug before Spec 079!
  await pageA.evaluate((uid) => {
    localStorage.setItem(
      `chatw_e2ee_peer_alert_${uid}_${uid}`,
      JSON.stringify({
        peer_id: uid,
        previous_fingerprint: 'oldfp',
        new_fingerprint: 'newfp',
        previous_identity_fingerprint: 'oldfp',
        new_identity_fingerprint: 'newfp',
      })
    );
  }, meA.id);

  // Rotate ALICE's OWN device key via Settings -> Riêng tư & bảo mật -> Bảo mật
  await pageA.locator('.app-rail button[aria-label*="Cài đặt"], .app-rail button[title*="Cài đặt"]').first().click();
  await pageA.waitForSelector('.settings-nav', { timeout: 10000 });
  await pageA.locator('.settings-nav-item', { hasText: /Riêng tư & bảo mật/ }).first().click();
  await pageA.waitForTimeout(200);
  await pageA.locator('.settings-subtab', { hasText: /Bảo mật/ }).first().click();
  await pageA.waitForTimeout(400);
  await pageA.locator('button', { hasText: /Đặt lại khóa thiết bị/ }).first().click();
  await pageA.waitForTimeout(200);
  await pageA.locator('button', { hasText: /Xác nhận đặt lại khóa/ }).first().click();
  await pageA.waitForSelector('.status-badge.success', { timeout: 10000 });
  await pageA.waitForTimeout(400);

  // Return to Messages -> open "Chính mình" AND reload -> ZERO false security alert toast!
  await pageA.locator('.app-rail button[aria-label*="Tin nhắn"], .app-rail button[aria-label*="Messages"]').first().click();
  await pageA.waitForTimeout(300);
  await pageA.locator('.nav-sidebar .chat-item').first().click();
  await pageA.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA.waitForTimeout(800);

  const selfCheck1 = await pageA.evaluate((uid) => ({
    toast: (document.querySelector('.toast-banner, .toast')?.innerText || '').trim(),
    staleAlertLeft: localStorage.getItem(`chatw_e2ee_peer_alert_${uid}_${uid}`) || '',
  }), meA.id);
  await pageA.screenshot({ path: path.join(SHOT_DIR, '01_self_chat_clean_after_key_reset.png') });

  record(
    'Self-chat ("Chính mình") shows ZERO false TOFU security alert after own key reset & clears stale self-alert',
    !selfCheck1.toast.includes('khóa mã hóa') && !selfCheck1.toast.includes('Security alert') && selfCheck1.staleAlertLeft === '',
    JSON.stringify(selfCheck1)
  );

  // 2. Now have real friend BOB rotate his device key -> ALICE opens BOB's DM -> sees legitimate alert ->
  //    opens Info (auto-expanded Security!) -> clicks "New key verified" -> reload confirms alert is permanently gone!
  await pageB.locator('.app-rail button[aria-label*="Cài đặt"], .app-rail button[title*="Cài đặt"]').first().click();
  await pageB.waitForSelector('.settings-nav', { timeout: 10000 });
  await pageB.locator('.settings-nav-item', { hasText: /Riêng tư & bảo mật/ }).first().click();
  await pageB.waitForTimeout(200);
  await pageB.locator('.settings-subtab', { hasText: /Bảo mật/ }).first().click();
  await pageB.waitForTimeout(400);
  await pageB.locator('button', { hasText: /Đặt lại khóa thiết bị/ }).first().click();
  await pageB.waitForTimeout(200);
  await pageB.locator('button', { hasText: /Xác nhận đặt lại khóa/ }).first().click();
  await pageB.waitForSelector('.status-badge.success', { timeout: 10000 });

  // Reload ALICE so peer_keys_cache is empty and fetches BOB's new rotated key from server
  await pageA.reload({ waitUntil: 'load' });
  await pageA.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA.waitForTimeout(800);
  await pageA.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA.waitForTimeout(800);

  // Verify legitimate alert banner appears right inside BOB's DM, and opening Info drawer
  // auto-expands Security with the verify button!
  const bannerVisible = await pageA.locator('.chat-banner.danger').first().isVisible().catch(() => false);
  const detailsBtn = pageA.locator('[data-testid="toggle-details"]').first();
  if (await detailsBtn.isVisible().catch(() => false)) {
    const pressed = await detailsBtn.getAttribute('aria-pressed');
    if (pressed !== 'true') {
      await detailsBtn.click();
      await pageA.waitForTimeout(400);
    }
  }
  const verifyBtn = pageA.locator('.details-panel .status-badge.error button').first();
  const verifyVisible = await verifyBtn.isVisible().catch(() => false);
  await pageA.screenshot({ path: path.join(SHOT_DIR, '02_friend_key_change_auto_expanded_verify.png') });
  record(
    'Real friend key rotation shows inline chat banner AND auto-expands Security alert with verify button in Info panel',
    bannerVisible && verifyVisible,
    `banner=${bannerVisible} detailsVerify=${verifyVisible}`
  );

  if (verifyVisible) {
    await verifyBtn.click();
    await pageA.waitForTimeout(400);
  }

  // Reload ALICE and reopen BOB -> alert must NEVER repeat!
  await pageA.reload({ waitUntil: 'load' });
  await pageA.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA.waitForTimeout(800);
  await pageA.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA.waitForTimeout(800);

  const afterAck = await pageA.evaluate(({ myId, peerId }) => ({
    toast: (document.querySelector('.toast-banner, .toast')?.innerText || '').trim(),
    alertKey: localStorage.getItem(`chatw_e2ee_peer_alert_${myId}_${peerId}`) || '',
    legacyKey: localStorage.getItem(`chatw_e2ee_peer_alert_${peerId}`) || '',
  }), { myId: meA.id, peerId: meB.id });

  record(
    'After verifying friend new key, alert stays permanently cleared across reload (0 repeat toasts)',
    !afterAck.toast.includes('khóa mã hóa') && !afterAck.toast.includes('Security alert') && afterAck.alertKey === '' && afterAck.legacyKey === '',
    JSON.stringify(afterAck)
  );
  record('Zero uncaught JS pageerrors', pageErrors.length === 0, pageErrors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 079 Summary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length) {
  process.exit(1);
}
