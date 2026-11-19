// Spec 011 end-to-end check: two-step verification, backup codes, several
// accounts on one browser, new-device alerts and the security log.
// Usage: node app/e2e/security-check.mjs <out_dir>   (same requirements as ui-audit.mjs)
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'security-out';
fs.mkdirSync(OUT, { recursive: true });
const checks = [];
const record = (name, pass, detail = '') => { checks.push({ name, pass, detail }); console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? '  — ' + detail : ''}`); };
const shot = (page, n) => page.screenshot({ path: path.join(OUT, n + '.png') });

// RFC 6238 TOTP (SHA1, 6 digits) for the test authenticator.
const b32 = (s) => { const A = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'; let bits = '', out = []; for (const c of s) bits += A.indexOf(c).toString(2).padStart(5, '0'); for (let i = 0; i + 8 <= bits.length; i += 8) out.push(parseInt(bits.slice(i, i + 8), 2)); return Buffer.from(out); };
const totp = (secret, offset = 0) => {
  const step = Math.floor(Date.now() / 1000 / 30) + offset, msg = Buffer.alloc(8);
  msg.writeBigUInt64BE(BigInt(step));
  const h = crypto.createHmac('sha1', b32(secret)).update(msg).digest(), o = h[19] & 15;
  return String((((h[o] & 0x7f) << 24) | (h[o + 1] << 16) | (h[o + 2] << 8) | h[o + 3]) % 1000000).padStart(6, '0');
};
const nextStep = async () => { const s = 30 - (Math.floor(Date.now() / 1000) % 30); await new Promise((r) => setTimeout(r, (s + 1) * 1000)); };

const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome', args: ['--no-sandbox'] });
const run = Date.now().toString(36).slice(-5);
const A = 'sec_a' + run, Bn = 'sec_b' + run;
const ctx = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
const api = async (method, url, data, c = ctx) => { const r = await c.request.fetch(`${BASE}${url}`, { method, data }); let b = null; try { b = await r.json(); } catch (_) {} return { status: r.status(), body: b }; };
const devLogin = async (name) => {
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(name);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForTimeout(1200);
};
const appReady = async () => {
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
};
const openSecurity = async () => {
  await page.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await page.locator('.settings-nav-item', { hasText: 'Bảo mật' }).click();
  await page.waitForTimeout(800);
};

try {
  await page.goto(BASE + '/');
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await devLogin(A);
  await appReady();
  const me = await (await ctx.request.get(`${BASE}/api/auth/me`)).json();
  // --- Set up TOTP from the settings page
  await openSecurity();
  await page.locator('.security-settings button', { hasText: 'Thiết lập' }).click();
  await page.waitForTimeout(600);
  const secret = (await page.locator('.totp-secret').innerText()).trim();
  record('setup shows a QR code and a base32 secret', await page.locator('.totp-qr svg').count() === 1 && /^[A-Z2-7]{32}$/.test(secret), secret.length + ' chars');
  await shot(page, 'totp_setup');
  await page.locator('input[aria-label="Mã 6 số để bật xác minh hai bước"]').fill('000000');
  await page.locator('.totp-setup button', { hasText: 'Bật' }).click();
  await page.waitForTimeout(600);
  record('wrong code does not enable 2FA', (await api('GET', '/api/security/status')).body.totp_enabled === false);
  await page.locator('input[aria-label="Mã 6 số để bật xác minh hai bước"]').fill(totp(secret));
  await page.locator('.totp-setup button', { hasText: 'Bật' }).click();
  await page.waitForTimeout(900);
  const codes = await page.locator('.backup-grid code').allInnerTexts();
  record('2FA enabled and 10 backup codes shown once', (await api('GET', '/api/security/status')).body.totp_enabled === true && codes.length === 10, codes.length + ' codes');
  await shot(page, 'backup_codes');
  // --- Sensitive action needs the second step
  const noCode = await api('DELETE', '/api/auth/sessions');
  record('signing out other devices requires the second step', noCode.status === 403 && noCode.body.error === 'second_factor_required', String(noCode.status));
  // --- Sign out, sign in again: second step required
  await page.locator('.rail-account').click();
  await page.locator('.rail-menu .menu-item', { hasText: /^Đăng xuất$/ }).click();
  await page.waitForSelector('.auth-card', { timeout: 20000 });
  await devLogin(A);
  record('sign-in asks for the second step', await page.locator('#twofa-code').isVisible());
  await shot(page, 'twofa_prompt');
  await page.locator('#twofa-code').fill('123456');
  await page.locator('.twofa-card button[type="submit"]').click();
  await page.waitForTimeout(700);
  record('a wrong code is refused', await page.locator('.twofa-card .auth-error').isVisible());
  await nextStep(); // the enabling code's step was used: codes can't be replayed
  await page.locator('#twofa-code').fill(totp(secret));
  await page.locator('.twofa-card button[type="submit"]').click();
  await appReady();
  record('the right code signs in', (await api('GET', '/api/auth/me')).status === 200);
  // --- Backup code sign-in (and one-time use)
  await page.locator('.rail-account').click();
  await page.locator('.rail-menu .menu-item', { hasText: /^Đăng xuất$/ }).click();
  await page.waitForSelector('.auth-card', { timeout: 20000 });
  await devLogin(A);
  await page.locator('#twofa-code').fill(codes[0]);
  await page.locator('.twofa-card button[type="submit"]').click();
  await appReady();
  record('a backup code signs in', (await api('GET', '/api/auth/me')).status === 200);
  record('used backup code is gone', (await api('GET', '/api/security/status')).body.backup_codes_left === 9);
  // --- Several accounts in one browser
  await page.locator('.rail-account').click();
  await page.locator('.rail-menu .menu-item', { hasText: 'Thêm tài khoản' }).click();
  await page.waitForSelector('.auth-card', { timeout: 20000 });
  await page.waitForTimeout(600);
  record('sign-in page lists the parked account', await page.locator('.signed-in-accounts .account-row', { hasText: A }).count() === 1);
  await shot(page, 'login_with_accounts');
  await devLogin(Bn);
  await appReady();
  const meB = await (await ctx.request.get(`${BASE}/api/auth/me`)).json();
  record('second account signed in', meB.username === Bn, meB.username);
  await page.locator('.rail-account').click();
  await page.waitForTimeout(600);
  record('account switcher lists the other account', await page.locator('.rail-menu .account-switch', { hasText: A }).count() === 1);
  await shot(page, 'account_switcher');
  await page.locator('.rail-menu .account-switch', { hasText: A }).click();
  await appReady();
  const back = await (await ctx.request.get(`${BASE}/api/auth/me`)).json();
  record('switching needs no new sign-in (or 2FA) while the session is valid', back.id === me.id, back.username);
  const accts = await api('GET', '/api/auth/accounts');
  record('both accounts stay signed in', (accts.body || []).length === 2 && accts.body.filter((a) => a.active).length === 1, JSON.stringify((accts.body || []).map((a) => [a.username, a.active])));
  // --- New-device alert (another browser signs in as A) and security log
  const other = await browser.newContext({ userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Firefox/130.0' });
  const o = await other.request.post(`${BASE}/api/auth/dev-login`, { data: { username: A } });
  const ob = await o.json().catch(() => ({}));
  record('a new device also gets the 2FA challenge', ob.requires_2fa === true);
  await nextStep();
  const ov = await other.request.post(`${BASE}/api/auth/2fa/verify`, { data: { code: totp(secret) } });
  record('new device verifies with TOTP', ov.status() === 200, String(ov.status()));
  await page.waitForTimeout(1500);
  record('the open app shows a new-device alert', await page.locator('.app-toast, .toast', { hasText: 'Firefox trên Windows' }).count() > 0);
  await shot(page, 'new_device_alert');
  await other.close();
  await openSecurity();
  const log = await page.locator('.security-event').allInnerTexts();
  record('security log lists the events', ['Bật xác minh hai bước', 'Nhập sai mã xác minh', 'Dùng một mã dự phòng', 'Đăng nhập từ thiết bị mới', 'Chuyển sang tài khoản này'].every((k) => log.some((l) => l.includes(k))), log.length + ' events');
  await shot(page, 'security_log');
  // --- Disable with a code
  await page.locator('input[aria-label="Mã hiện tại hoặc mã dự phòng"]').fill(codes[1]);
  await page.locator('.security-settings button', { hasText: 'Tắt xác minh hai bước' }).click();
  await page.waitForTimeout(800);
  record('2FA can be turned off with a backup code', (await api('GET', '/api/security/status')).body.totp_enabled === false);
  // --- Sign out every account
  await page.locator('.rail-account').click();
  await page.locator('.rail-menu .menu-item', { hasText: 'Đăng xuất tất cả tài khoản' }).click();
  await page.waitForSelector('.auth-card', { timeout: 20000 });
  await page.waitForTimeout(600);
  record('"sign out all" ends every account', (await api('GET', '/api/auth/accounts')).body.length === 0 && await page.locator('.signed-in-accounts').count() === 0);
} catch (e) {
  record('security flow', false, String(e).split('\n').slice(0, 3).join(' '));
  await shot(page, 'security_fail').catch(() => {});
}
record('no page errors', errors.length === 0, errors.slice(0, 2).join(' | '));
fs.writeFileSync(path.join(OUT, 'results.json'), JSON.stringify(checks, null, 2));
const failed = checks.filter((c) => !c.pass).length;
console.log(`\n${checks.length - failed}/${checks.length} security checks passed.`);
await browser.close();
process.exitCode = failed ? 1 : 0;
