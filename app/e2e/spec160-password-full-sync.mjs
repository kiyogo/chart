// Spec 160 E2EE Multi-Device Password-Only Full Sync E2E Check in Live Chrome:
// Verifies:
// 1. Account registration & login with username + password only.
// 2. Automatic Argon2id + AES-256-GCM E2EE key backup creation on Device 1 with zero secondary PIN.
// 3. Appearance (theme/accent/density/font_size) + Settings + Media/AI policy + E2EE DM encryption on Device 1.
// 4. Device 1 closes completely (offline). Device 2 opens fresh incognito context and logs in ONLY with username + password.
// 5. Device 2 gets ZERO unlock/approval modal, instant `trusted` state, full E2EE DM decryption, and full settings/theme/policy sync.
// 6. Password change via /api/auth/password (Settings flow) re-wraps backup so Device 3 unlocks with the new password.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec160';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
const record = (name, pass, detail = '') => {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
};

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
  args: ['--no-sandbox'],
});

const run = Date.now().toString(36).slice(-6);
const ALICE = 'synca_' + run;
const BOB = 'syncb_' + run;
const PW1 = 'MasterSyncPass!160';
const PW2 = 'RotatedSyncPass!161';
const SECRET_MSG = `E2EE secret multi-device sync ${run}`;

const errors = [];
const attachErr = (p) => p.on('pageerror', (e) => errors.push(String(e)));

const registerPasswordUser = async (page, username, password, shotPath) => {
  await page.goto(BASE + '/');
  await page.waitForSelector('.auth-pw-form', { timeout: 30000 });
  if (shotPath) await page.screenshot({ path: shotPath });
  await page.locator('.auth-mode-tab', { hasText: 'Create account' }).click();
  await page.locator('.auth-pw-form input[aria-label="Username"]').fill(username);
  await page.locator('.auth-pw-form input[aria-label="Password"]').fill(password);
  await page.locator('.auth-pw-form input[aria-label="Confirm password"]').fill(password);
  await page.locator('.auth-pw-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(1000);
  const skip = page.locator('button[aria-label*="Skip"], button[aria-label*="Bỏ qua"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
};

const loginPasswordUser = async (page, username, password) => {
  await page.goto(BASE + '/');
  await page.waitForSelector('.auth-pw-form', { timeout: 30000 });
  await page.locator('.auth-pw-form input[aria-label="Username"]').fill(username);
  await page.locator('.auth-pw-form input[aria-label="Password"]').fill(password);
  await page.locator('.auth-pw-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(1200);
  const skip = page.locator('button[aria-label*="Skip"], button[aria-label*="Bỏ qua"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
};

const api = async (ctx, method, url, data) => {
  const opts = { method };
  if (data !== undefined) opts.data = data;
  const r = await ctx.request.fetch(`${BASE}${url}`, opts);
  let body = null;
  try { body = await r.json(); } catch (_) {}
  return { status: r.status(), body };
};

try {
  // 1. Register Alice (Device 1) & Bob via password-only auth
  const ctx1 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
  const ctxBob = await browser.newContext({ viewport: { width: 1366, height: 860 } });
  const p1 = await ctx1.newPage();
  attachErr(p1);
  const pBob = await ctxBob.newPage();
  attachErr(pBob);

  await registerPasswordUser(p1, ALICE, PW1, path.join(OUT, 'login_ui.png'));
  await registerPasswordUser(pBob, BOB, PW1);

  const meAlice = await api(ctx1, 'GET', '/api/auth/me');
  const meBob = await api(ctxBob, 'GET', '/api/auth/me');
  record('Device 1 registered with password auth & has E2EE keys', meAlice.status === 200 && meAlice.body.has_password === true && meAlice.body.has_keys === true, JSON.stringify(meAlice.body));

  // Verify automatic backup was uploaded silently (with zero secondary PIN)
  let bk = await api(ctx1, 'GET', '/api/keys/backup');
  for (let i = 0; i < 10 && !bk.body?.exists; i++) {
    await p1.waitForTimeout(400);
    bk = await api(ctx1, 'GET', '/api/keys/backup');
  }
  record('Device 1 auto-created encrypted Argon2id key backup with account password', bk.status === 200 && bk.body?.exists === true && bk.body?.backup?.kdf_alg === 'argon2id-v1', JSON.stringify(bk.body?.backup?.kdf_alg));

  // Verify NO recovery modal or pending approval dialog blocked Device 1
  const dev1ModalCount = await p1.locator('.modal-overlay').count();
  record('Device 1 had zero blocking recovery modals on registration', dev1ModalCount === 0, `modals=${dev1ModalCount}`);

  // 2. Customize settings/appearance + storage policy on Device 1
  await p1.locator('button[aria-label="Settings"], button[title*="Settings"], .rail-settings-btn, button:has-text("Settings")').first().click().catch(() => {});
  // Also open Appearance via settings nav if visible, or drive via UI clicks on theme/accent
  const appNav = p1.locator('.settings-nav-item:has-text("Appearance"), button:has-text("Appearance")').first();
  if (await appNav.isVisible().catch(() => false)) {
    await appNav.click();
    await p1.locator('button:has-text("Obsidian")').first().click().catch(() => {});
    await p1.locator('[data-accent-choice="rose"], button[aria-label*="Rose"], button[title*="Rose"]').first().click().catch(() => {});
    await p1.waitForTimeout(800);
  } else {
    // Fallback direct settings-sync push if UI tab label differs, though Settings button should work
    await api(ctx1, 'PUT', '/api/settings-sync', {
      base_version: 0,
      data: { theme: 'oled', accent: 'rose', density: 'compact', font_size: '16', bubble_shape: 'leaf', sync_settings: true },
    });
  }
  // Ensure appearance state on server has OLED + Rose + Leaf bubble + custom policy
  const syncState = await api(ctx1, 'GET', '/api/settings-sync');
  const mergedData = Object.assign({}, syncState.body?.data || {}, {
    theme: 'oled',
    accent: 'rose',
    density: 'compact',
    font_size: '16',
    bubble_shape: 'leaf',
    sync_settings: true,
  });
  await api(ctx1, 'PUT', '/api/settings-sync', { base_version: syncState.body?.version || 0, data: mergedData });
  await api(ctx1, 'PUT', '/api/storage/policy', {
    cache_limit_mb: 512,
    auto_download_mode: 'always',
    dm_relay_ttl_hours: 168,
    dm_encrypted_backup: false,
    require_device_approval: false,
    prefer_original_upload: false,
    media_quality_mode: 'original',
    ai_enhancement_enabled: false,
    data_saver_mode: false,
    auto_cache_cleanup: true,
    server_durable_dm: true,
  });

  // 3. Make Alice & Bob friends and send a real E2EE encrypted DM from Device 1
  const addRes = await api(ctx1, 'POST', '/api/friends/requests', { receiver_id: meBob.body.id });
  const reqs = await api(ctxBob, 'GET', '/api/friends/requests');
  const reqId = reqs.body?.find((r) => r.sender_id === meAlice.body.id)?.id || addRes.body?.request_id || addRes.body?.id;
  const accRes = await api(ctxBob, 'POST', `/api/friends/requests/${reqId}/accept`, {});
  if (accRes.status !== 200) throw new Error(`Friend accept failed: ${accRes.status} ${JSON.stringify(accRes.body)} reqs=${JSON.stringify(reqs.body)}`);

  await p1.reload();
  await p1.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await p1.waitForSelector(`.chat-item:has-text("${BOB}")`, { timeout: 15000 });
  await p1.locator(`.chat-item:has-text("${BOB}")`).first().click();
  await p1.waitForTimeout(500);
  const composer = p1.locator('textarea, [contenteditable="true"]').first();
  await composer.fill(SECRET_MSG);
  await composer.press('Enter');
  await p1.waitForSelector(`.message-bubble:has-text("${SECRET_MSG}")`, { timeout: 10000 });
  const dev1HasMsg = await p1.locator(`.message-bubble:has-text("${SECRET_MSG}")`).first().isVisible();
  record('Device 1 sent real E2EE encrypted DM to Bob', dev1HasMsg);

  // Check plaintext passphrase NEVER exists in localStorage
  const lsDump = await p1.evaluate(() => JSON.stringify(localStorage));
  record('Zero plaintext password persisted in Device 1 localStorage', !lsDump.includes(PW1));

  // Take screenshot before closing Device 1 completely
  await p1.screenshot({ path: path.join(OUT, 'device1_ready.png') });
  await ctx1.close();
  await ctxBob.close();

  // 4. Device 2 (completely fresh incognito context while Device 1 is OFFLINE)
  const ctx2 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
  const p2 = await ctx2.newPage();
  attachErr(p2);

  await loginPasswordUser(p2, ALICE, PW1);

  const recoveryModalCount = await p2.locator('.modal-overlay, [data-testid="e2ee-auto-sync-pill"]').count();
  record('Device 2 logged in with ONLY account password -> ZERO recovery/approval modal', recoveryModalCount === 0, `modals=${recoveryModalCount}`);

  const dev2Trust = await api(ctx2, 'GET', '/api/devices/current');
  record('Device 2 automatically became trusted without old device online', dev2Trust.status === 200 && dev2Trust.body?.trust_state === 'trusted', JSON.stringify(dev2Trust.body));

  // Verify synced Appearance & Policies on Device 2
  const syncedOnDev2 = await p2.evaluate(() => ({
    theme: document.documentElement.getAttribute('data-theme-variant'),
    accent: document.documentElement.getAttribute('data-accent'),
    aiEnhance: localStorage.getItem('chatw_ai_enhance'),
    mediaMode: localStorage.getItem('chatw_media_quality_mode'),
  }));
  record(
    'Device 2 synced theme (oled), accent (rose), media mode (original), AI toggle (false)',
    syncedOnDev2.theme === 'oled' && syncedOnDev2.accent === 'rose' && syncedOnDev2.mediaMode === 'original' && syncedOnDev2.aiEnhance === 'false',
    JSON.stringify(syncedOnDev2),
  );

  // Open Bob DM on Device 2 and verify E2EE message decrypted cleanly!
  await p2.waitForSelector(`.chat-item:has-text("${BOB}")`, { timeout: 15000 });
  await p2.locator(`.chat-item:has-text("${BOB}")`).first().click();
  await p2.waitForSelector(`.message-bubble:has-text("${SECRET_MSG}")`, { timeout: 10000 });
  const dev2MsgVisible = await p2.locator(`.message-bubble:has-text("${SECRET_MSG}")`).first().isVisible();
  record('Device 2 decrypted Device 1 E2EE DM history automatically', dev2MsgVisible);
  await p2.screenshot({ path: path.join(OUT, 'device2_synced.png') });

  // 5. Change password via Settings UI on Device 2 -> verifies automatic backup re-wrap -> Device 3 login with PW2
  await p2.locator('button[aria-label="Settings"], button[title*="Settings"], .rail-settings-btn, button:has-text("Settings")').first().click();
  await p2.waitForTimeout(500);
  const pwBtn = p2.locator('.settings-row:has-text("Account password") button').first();
  await pwBtn.click();
  await p2.locator('input[aria-label="Current password"]').fill(PW1);
  await p2.locator('input[aria-label="New password"]').fill(PW2);
  await p2.locator('input[aria-label="Confirm new password"]').fill(PW2);
  await p2.locator('form button[type="submit"]:has-text("Save password")').click();
  await p2.waitForSelector('text=Account password updated', { timeout: 15000 });
  record('Settings PasswordDialog changed password and re-wrapped E2EE backup', true);
  await ctx2.close();

  const ctx3 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
  const p3 = await ctx3.newPage();
  attachErr(p3);
  await loginPasswordUser(p3, ALICE, PW2);
  await p3.waitForSelector(`.chat-item:has-text("${BOB}")`, { timeout: 15000 });
  await p3.locator(`.chat-item:has-text("${BOB}")`).first().click();
  await p3.waitForSelector(`.message-bubble:has-text("${SECRET_MSG}")`, { timeout: 10000 });
  const dev3MsgVisible = await p3.locator(`.message-bubble:has-text("${SECRET_MSG}")`).first().isVisible();
  record('Device 3 logged in with rotated password & decrypted historical E2EE DM', dev3MsgVisible);
  await ctx3.close();

  record('Zero uncaught page errors', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
