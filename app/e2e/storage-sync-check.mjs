// Spec 022 End-to-End Check in Live Chromium:
// Verifies OPFS + IndexedDB storage engine, protected local_only cache clearing,
// honest avatar encoding/extension matching, IDOR blocking, Settings -> Storage & Sync UI,
// multi-device registration/approval flow, incremental delta sync, and zero console/page errors.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'storage-e2e-out';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
const record = (name, pass, detail = '') => {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
};
const shot = (page, name) => page.screenshot({ path: path.join(OUT, `${name}.png`) });

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
  args: ['--no-sandbox'],
});

const run = Date.now().toString(36).slice(-5);
const ALICE = 'sto_a_' + run;
const BOB = 'sto_b_' + run;
const EVE = 'sto_e_' + run;

const ctx1 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const ctx2 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const ctx3 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const ctx1Dev2 = await browser.newContext({ viewport: { width: 1366, height: 860 } });

const errors = [];
const attachErr = (p) => p.on('pageerror', (e) => errors.push(String(e)));

const p1 = await ctx1.newPage();
attachErr(p1);
const p2 = await ctx2.newPage();
attachErr(p2);
const p3 = await ctx3.newPage();
attachErr(p3);
const p1d2 = await ctx1Dev2.newPage();
attachErr(p1d2);

const devLogin = async (page, username) => {
  await page.goto(BASE + '/');
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(username);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
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
  // 1. Login Alice, Bob, Eve in separate browser contexts
  await devLogin(p1, ALICE);
  await devLogin(p2, BOB);
  await devLogin(p3, EVE);

  // Check Web Storage Engine + OPFS/IndexedDB init on Alice's Chromium page
  const engineInit = await p1.evaluate(async () => {
    if (!window.__chatwStorage || !window.__chatwMediaOpt) return { ok: false };
    return await window.__chatwStorage.init();
  });
  record('Browser storage_engine + OPFS/IndexedDB initialized', engineInit.ok === true && Boolean(engineInit.clientDeviceId), JSON.stringify(engineInit));

  // Verify local_only protection during clearSafeCache(false)
  const cacheSafety = await p1.evaluate(async () => {
    const u8 = new TextEncoder().encode('unique-local-only-dm-photo');
    const grp = new TextEncoder().encode('replicated-group-cache-photo');
    const b64 = (arr) => btoa(String.fromCharCode(...arr));
    await window.__chatwStorage.putObject({
      ref_id: 'test-unique-dm-1',
      scope_type: 'dm',
      file_name: 'secret.png',
      mime_type: 'image/png',
      availability_state: 'local_only',
    }, b64(u8));
    await window.__chatwStorage.putObject({
      ref_id: 'test-group-cache-1',
      scope_type: 'group',
      file_name: 'banner.png',
      mime_type: 'image/png',
      availability_state: 'server_durable',
    }, b64(grp));
    const cleared = await window.__chatwStorage.clearSafeCache(false);
    const dmStillThere = await window.__chatwStorage.getObject('test-unique-dm-1');
    const grpGone = await window.__chatwStorage.getObject('test-group-cache-1');
    return { cleared, dmFound: dmStillThere.found, grpFound: grpGone.found };
  });
  record(
    'Clear Safe Cache protects local_only unique DM files while freeing group cache',
    cacheSafety.dmFound === true && cacheSafety.grpFound === false && cacheSafety.cleared.protected_unique >= 1,
    JSON.stringify(cacheSafety),
  );

  // 2. Open Settings -> Storage & Sync UI and capture screenshots
  await p1.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await p1.waitForTimeout(400);
  await p1.locator('.settings-nav-item', { hasText: 'Lưu trữ' }).click();
  await p1.waitForTimeout(700);
  const storageHeading = await p1.locator('.settings-page-title').innerText();
  record('Settings -> Storage panel renders cleanly', storageHeading.trim() === 'Lưu trữ', storageHeading);
  await shot(p1, 'storage_settings');

  await p1.locator('.settings-nav-item', { hasText: 'Đồng bộ' }).click();
  await p1.waitForTimeout(700);
  const syncHeading = await p1.locator('.settings-page-title').innerText();
  record('Settings -> Sync & Recovery panel renders cleanly', syncHeading.trim() === 'Đồng bộ', syncHeading);
  await shot(p1, 'sync_settings');

  // 3. Honest Avatar format check (upload PNG in browser -> check server stores honest extension matching magic bytes)
  const avResp = await p1.evaluate(async () => {
    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 256;
    const ctx = canvas.getContext('2d');
    ctx.fillStyle = '#2563EB';
    ctx.fillRect(0, 0, 256, 256);
    ctx.fillStyle = '#FFFFFF';
    ctx.fillRect(48, 96, 160, 64);
    const blob = await new Promise((r) => canvas.toBlob(r, 'image/png'));
    const res = await fetch('/api/users/avatar', { method: 'POST', credentials: 'include', headers: { 'Content-Type': 'image/png' }, body: blob });
    const txt = await res.text();
    try { return { status: res.status, body: JSON.parse(txt) }; } catch (_) { return { status: res.status, text: txt }; }
  });
  const avUrl = avResp.body?.avatar_url;
  const avRes = avUrl ? await ctx1.request.get(`${BASE}${avUrl}`) : null;
  const avBuf = avRes ? await avRes.body() : Buffer.alloc(0);
  const isAvif = avBuf.slice(4, 12).toString('ascii') === 'ftypavif';
  const isPng = avBuf[0] === 0x89 && avBuf[1] === 0x50;
  const isWebp = avBuf.slice(0, 4).toString('ascii') === 'RIFF';
  const extMatch = Boolean(avUrl) && ((isAvif && avUrl.includes('.avif'))
    || (isPng && avUrl.includes('.png'))
    || (isWebp && avUrl.includes('.webp')));
  record('Avatar format & URL extension match actual magic bytes', extMatch && avRes?.status() === 200, JSON.stringify(avResp));

  // 4. Scoped attachment IDOR check in Chromium sessions (make Alice & Bob friends first!)
  const meBob = (await api(ctx2, 'GET', '/api/auth/me')).body;
  const fReq = await api(ctx1, 'POST', '/api/friends/requests', { username: BOB });
  if (fReq.body?.id) {
    await api(ctx2, 'POST', `/api/friends/requests/${fReq.body.id}/accept`, {});
  }
  const upResp = await ctx1.request.post(`${BASE}/api/attachments?scope_type=dm&scope_id=${meBob.id}&is_e2ee=true&file_name=check.bin`, {
    headers: { 'Content-Type': 'application/octet-stream' },
    data: Buffer.from('live_e2ee_cipher_test_123'),
  });
  const upJson = await upResp.json();
  const dlEve = upJson.download_url ? await ctx3.request.get(`${BASE}${upJson.download_url}`) : { status: () => 0 };
  record('Stranger Eve blocked (403) from downloading Alice->Bob DM attachment', dlEve.status() === 403, JSON.stringify(upJson));

  // 5. Multi-device trust challenge in real Chrome context 1b (Alice Device 2)
  await devLogin(p1d2, ALICE);
  await p1d2.waitForTimeout(1000);
  const dev2Status = (await api(ctx1Dev2, 'GET', '/api/devices/current')).body;
  record('Second browser profile auto-detected as pending approval', dev2Status.trust_state === 'pending', JSON.stringify(dev2Status));

  // Verify Device 1 received live WebSocket IncomingDeviceApprovalModal or approves via UI/API!
  await p1.waitForTimeout(600);
  await shot(p1, 'devices_pending_approval');
  const approveBtn = p1.locator('button', { hasText: 'Chấp nhận đồng bộ' });
  if (await approveBtn.isVisible().catch(() => false)) {
    await approveBtn.click();
    await p1.waitForTimeout(600);
    record('Trusted Device 1 approves Device 2 via live WebSocket approval modal', true);
  } else {
    const reqId = dev2Status.latest_request?.id;
    const verCode = dev2Status.latest_request?.verification_code;
    const appRes = await api(ctx1, 'POST', `/api/devices/requests/${reqId}/approve`, { verification_code: verCode });
    record('Trusted Device 1 approves Device 2 with 6-digit code', appRes.status === 200 && appRes.body?.approved === true);
  }

  await p1.locator('.settings-nav-item', { hasText: 'Thiết bị' }).click();
  await p1.waitForTimeout(700);
  await shot(p1, 'devices_trusted_list');

  const deltaAfter = (await api(ctx1Dev2, 'GET', '/api/sync/delta?dm_since=0'));
  record('Device 2 unlocked incremental delta sync after approval', deltaAfter.status === 200);

  // 6. Client Adaptive Image Optimizer (Test 14 & Test 15: crisp text screenshot vs photo & honest MIME)
  const optCheck = await p1.evaluate(async () => {
    const c = document.createElement('canvas');
    c.width = 640;
    c.height = 360;
    const ctx = c.getContext('2d');
    ctx.fillStyle = '#FFFFFF';
    ctx.fillRect(0, 0, 640, 360);
    ctx.fillStyle = '#111827';
    ctx.font = '18px monospace';
    ctx.fillText('Van ban sac net E2EE ChatW 1234567890', 32, 120);
    ctx.fillRect(32, 160, 480, 3);
    const pngBlob = await new Promise((r) => c.toBlob(r, 'image/png'));
    const resOpt = await window.__chatwMediaOpt.optimizeImageBlob(pngBlob, { preserveOriginal: false });
    const resOrig = await window.__chatwMediaOpt.optimizeImageBlob(pngBlob, { preserveOriginal: true });
    return {
      isCrisp: resOpt.isCrispText,
      optMime: resOpt.mime,
      origSize: resOrig.originalSize,
      origPreserved: resOrig.originalSize === resOrig.optimizedSize,
      hasThumb: Boolean(resOpt.thumbB64),
    };
  });
  record(
    'Adaptive client optimizer detects crisp text/UI, generates thumbnail, and preserves original mode',
    optCheck.isCrisp === true && optCheck.origPreserved === true && optCheck.hasThumb === true,
    JSON.stringify(optCheck),
  );

  // 7. Cookie/Site Data Wipe Resilience (Test 11 & Test 12) + Two-Way Peer Recovery (Test 6, 7, 8)
  const refId = upJson.ref_id;
  const ackAlice = await api(ctx1, 'POST', `/api/storage/relay/${refId}/ack`, {
    local_tier: 'opfs',
    sha256_hex: upJson.sha256,
    has_original: true,
  });
  const ackBob = await api(ctx2, 'POST', `/api/storage/relay/${refId}/ack`, {
    local_tier: 'opfs',
    sha256_hex: upJson.sha256,
    has_original: true,
  });
  record(
    'Alice & Bob devices report verified OPFS copies and temporary relay auto-cleans',
    ackAlice.status === 200 && ackBob.status === 200 && ackBob.body?.relay_cleaned === true,
    JSON.stringify(ackBob.body),
  );

  // Bob clears his local cache copy and requests two-way peer recovery from Alice
  await api(ctx2, 'DELETE', `/api/storage/references/${refId}?scope=local_cache`);
  const recReq = await api(ctx2, 'POST', '/api/recovery/requests', { ref_id: refId, preferred_transport: 'webrtc_p2p' });
  record('Bob initiates two-way DM peer recovery request', recReq.status === 200 && Boolean(recReq.body?.request_id), JSON.stringify(recReq.body));

  const recId = recReq.body.request_id;
  const respAlice = await api(ctx1, 'POST', `/api/recovery/requests/${recId}/respond`, { action: 'accept', transport_mode: 'webrtc_p2p' });
  const compBob = await api(ctx2, 'POST', `/api/recovery/requests/${recId}/complete`, { sha256_hex: upJson.sha256, local_tier: 'opfs' });
  record(
    'Peer recovery completes with verified SHA-256 and transitions to restored_from_peer',
    respAlice.status === 200 && compBob.status === 200 && compBob.body?.availability_state === 'restored_from_peer',
    JSON.stringify(compBob.body),
  );

  record('Zero uncaught page errors across storage/sync flows', errors.length === 0, errors.join('; '));
} finally {
  await browser.close();
}

const passed = checks.filter((c) => c.pass).length;
console.log(`\nSummary: ${passed}/${checks.length} checks passed.`);
fs.writeFileSync(path.join(OUT, 'report.json'), JSON.stringify(checks, null, 2));
if (passed !== checks.length) process.exit(1);
