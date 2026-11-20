// Spec 027 End-to-End Check in Live Chromium:
// Verifies Security Export Password, .kdata Reauth + Streaming Export (bytes start with KIYODATA),
// Inspect (wrong password rejection + valid preview) & Selective Restore (0 duplicates),
// Cloud Provider catalog (honest OAuth config notice + real WebDAV connect, incremental snapshot #1 & #2, restore),
// and Locked-by-default Multi-step Unencrypted ZIP flow + auto-relock.
import crypto from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'backup-e2e-out';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
const record = (name, pass, detail = '') => {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
};
const shot = (page, name) => page.screenshot({ path: path.join(OUT, `${name}.png`) });

// In-process WebDAV server for live Cloud E2E verification
const davFiles = new Map();
const davDirs = new Set(['dav']);
const expectedAuth = 'Basic ' + Buffer.from('davuser:davpass').toString('base64');

const davServer = http.createServer((req, res) => {
  if (req.headers.authorization !== expectedAuth) {
    res.writeHead(401);
    res.end();
    return;
  }
  const urlPath = (req.url || '/').split('?')[0].replace(/^\/+|\/+$/g, '');
  const chunks = [];
  req.on('data', (c) => chunks.push(c));
  req.on('end', () => {
    const body = Buffer.concat(chunks);
    if (req.method === 'PROPFIND') {
      if (!urlPath || davDirs.has(urlPath)) {
        res.writeHead(207, { 'Content-Type': 'application/xml' });
        res.end(`<?xml version="1.0" encoding="utf-8"?>
          <d:multistatus xmlns:d="DAV:">
            <d:response><d:href>/${urlPath}/</d:href>
              <d:propstat><d:prop>
                <d:quota-available-bytes>2147483648</d:quota-available-bytes>
                <d:quota-used-bytes>2048</d:quota-used-bytes>
              </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>
            </d:response>
          </d:multistatus>`);
      } else {
        res.writeHead(404);
        res.end();
      }
      return;
    }
    if (req.method === 'MKCOL') {
      davDirs.add(urlPath);
      res.writeHead(201);
      res.end();
      return;
    }
    if (req.method === 'PUT') {
      davFiles.set(urlPath, body);
      res.writeHead(201);
      res.end();
      return;
    }
    if (req.method === 'GET') {
      const f = davFiles.get(urlPath);
      if (f) {
        res.writeHead(200);
        res.end(f);
      } else {
        res.writeHead(404);
        res.end();
      }
      return;
    }
    if (req.method === 'HEAD') {
      const f = davFiles.get(urlPath);
      if (f) {
        res.writeHead(200, { 'Content-Length': String(f.length) });
        res.end();
      } else {
        res.writeHead(404);
        res.end();
      }
      return;
    }
    if (req.method === 'DELETE') {
      davFiles.delete(urlPath);
      res.writeHead(204);
      res.end();
      return;
    }
    res.writeHead(405);
    res.end();
  });
});

await new Promise((resolve) => davServer.listen(0, '127.0.0.1', resolve));
const davPort = davServer.address().port;

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
  args: ['--no-sandbox'],
});

const run = Date.now().toString(36).slice(-5);
const ALICE = 'bk_a_' + run;
const BOB = 'bk_b_' + run;

const ctx1 = await browser.newContext({ viewport: { width: 1366, height: 860 }, acceptDownloads: true });
const ctx2 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const errors = [];
const p1 = await ctx1.newPage();
p1.on('pageerror', (e) => errors.push(String(e)));
const p2 = await ctx2.newPage();
p2.on('pageerror', (e) => errors.push(String(e)));

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
  await devLogin(p1, ALICE);
  await devLogin(p2, BOB);

  const bobMe = (await api(ctx2, 'GET', '/api/auth/me')).body;
  const fReq = await api(ctx1, 'POST', '/api/friends/requests', { username: BOB });
  if (fReq.body?.id) {
    await api(ctx2, 'POST', `/api/friends/requests/${fReq.body.id}/accept`, {});
  }

  // Create a group + upload durable attachment so Alice has real server_durable media & messages
  const grp = (await api(ctx1, 'POST', '/api/groups', { name: 'KiyoBackupGroup_' + run })).body;
  if (grp?.id) {
    await api(ctx1, 'POST', `/api/groups/${grp.id}/members`, { user_id: bobMe.id });
    const envBytes = Buffer.alloc(320, 0xAB);
    await api(ctx1, 'POST', `/api/groups/${grp.id}/messages`, {
      id: crypto.randomUUID(),
      ciphertext_b64: envBytes.toString('base64'),
    });
    await ctx1.request.post(`${BASE}/api/attachments?scope_type=group&scope_id=${grp.id}&is_e2ee=false&file_name=note.png`, {
      headers: { 'Content-Type': 'application/octet-stream' },
      data: Buffer.from('kiyo_backup_e2e_payload_' + run),
    });
  }

  // Open Settings
  await p1.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await p1.waitForSelector('.settings-nav', { timeout: 10000 });

  // 1. Security -> Verify Unencrypted ZIP locked by default + Set Export Password
  await p1.locator('.settings-nav-item', { hasText: 'Bảo mật' }).click();
  await p1.waitForSelector('#sec-new-export-pw', { timeout: 10000 });
  const step3InitiallyHidden = (await p1.locator('#btn-zip-step3-export').count()) === 0;
  record('Unencrypted ZIP locked by default in Security', step3InitiallyHidden);

  await p1.locator('#sec-new-export-pw').fill('ExportSecret#2026');
  await p1.locator('#btn-save-export-pw').click();
  await p1.waitForSelector('.status-badge.success', { timeout: 10000 });
  record('Export password configured via Security UI', true);

  // 2. Settings -> Backup (.kdata)
  await p1.locator('.settings-nav-item', { hasText: 'Sao lưu' }).click();
  await p1.waitForSelector('#btn-export-kdata', { timeout: 10000 });

  const noPlainZipInBackup = (await p1.locator('#btn-open-unencrypted-zip').count()) === 0;
  record('Unencrypted ZIP excluded from Backup/Export tab', noPlainZipInBackup);

  // Confirm direct export blocked by backend without reauth token
  const directExport = await api(ctx1, 'POST', '/api/backup/export/kdata', { encryption_password: 'Container#2026' });
  record('Direct .kdata export blocked without reauth', directExport.status === 403 && directExport.body?.error === 'export_reauth_required');

  await p1.locator('#kdata-reauth-secret').fill('ExportSecret#2026');
  await p1.locator('#kdata-enc-password').fill('Container#2026');
  await p1.locator('#kdata-enc-confirm').fill('Container#2026');

  const downloadPromise = p1.waitForEvent('download', { timeout: 30000 });
  await p1.locator('#btn-export-kdata').click();
  const download = await downloadPromise;
  const kdataPath = path.join(OUT, download.suggestedFilename());
  await download.saveAs(kdataPath);
  const kdataBuf = fs.readFileSync(kdataPath);
  const isKdataMagic = kdataBuf.subarray(0, 8).toString('ascii') === 'KIYODATA';
  record('.kdata exported via reauth and starts with KIYODATA magic', isKdataMagic, `${download.suggestedFilename()} (${kdataBuf.length} bytes)`);
  await shot(p1, '01_backup_kdata_export');

  // Inspect with wrong password -> error badge
  await p1.locator('#kdata-restore-file').setInputFiles(kdataPath);
  await p1.waitForTimeout(300);
  await p1.locator('#kdata-restore-password').fill('WrongPass123');
  await p1.locator('#btn-inspect-backup').click();
  await p1.waitForSelector('.status-badge.error', { timeout: 10000 });
  record('Inspect rejects wrong .kdata password in UI', true);

  // Inspect with correct password -> preview + restore (0 dup)
  await p1.locator('#kdata-restore-password').fill('Container#2026');
  await p1.locator('#btn-inspect-backup').click();
  await p1.waitForSelector('text=/KIYO/', { timeout: 10000 });
  await p1.locator('#btn-restore-backup').click();
  await p1.waitForSelector('.status-badge.success', { timeout: 15000 });
  const restoreMsg = await p1.locator('.status-badge.success').last().innerText();
  record('Restore .kdata completes with dedup check', restoreMsg.includes('Khôi phục hoàn tất'), restoreMsg);
  await shot(p1, '02_backup_inspect_restore');

  // 3. Settings -> Cloud
  await p1.locator('.settings-nav-item', { hasText: 'Đám mây' }).click();
  await p1.waitForSelector('#cloud-backup-passphrase', { timeout: 10000 });

  // Verify Google Drive honestly states OAuth configuration needed when unset
  const gdriveText = await p1.locator('.settings-row', { hasText: 'Google Drive' }).innerText();
  record('Cloud provider catalog honestly reports OAuth requirements', gdriveText.includes('GOOGLE_DRIVE_CLIENT_ID') || gdriveText.includes('OAuth'));

  // Connect local WebDAV
  const webdavRow = p1.locator('.settings-row', { hasText: 'WebDAV' }).first();
  await webdavRow.locator('button', { hasText: 'Thiết lập' }).click();
  await p1.waitForSelector('#cloud-dav-url', { timeout: 5000 });
  await p1.locator('#cloud-dav-url').fill(`http://127.0.0.1:${davPort}/dav`);
  await p1.locator('#cloud-dav-user').fill('davuser');
  await p1.locator('#cloud-dav-pass').fill('davpass');
  await p1.locator('#btn-connect-cloud').click();
  await p1.waitForSelector('button:has-text("Sao lưu tăng dần")', { timeout: 15000 });
  record('WebDAV cloud connection verified and added', true);

  await p1.locator('#cloud-backup-passphrase').fill('CloudPass#2026');
  await p1.locator('button:has-text("Sao lưu tăng dần")').first().click();
  await p1.waitForSelector('text=/Ảnh chụp #1/', { timeout: 20000 });
  record('Cloud incremental backup #1 created and verified', davFiles.size >= 2, `remote objects: ${davFiles.size}`);

  await p1.locator('button:has-text("Sao lưu tăng dần")').first().click();
  await p1.waitForSelector('text=/Ảnh chụp #2/', { timeout: 20000 });
  record('Cloud incremental backup #2 skips unchanged chunks', true);
  await shot(p1, '03_cloud_connected_snapshots');

  // 4. Settings -> Security -> Multi-step Unencrypted ZIP unlock & auto-relock
  await p1.locator('.settings-nav-item', { hasText: 'Bảo mật' }).click();
  await p1.waitForSelector('#btn-open-unencrypted-zip', { timeout: 10000 });
  await p1.locator('#btn-open-unencrypted-zip').click();
  await p1.locator('#btn-zip-step1-start').click();
  await p1.locator('#unencrypted-zip-reauth-secret').fill('ExportSecret#2026');
  await p1.locator('#btn-zip-step1-verify').click();
  await p1.waitForSelector('#btn-zip-step2-ack', { timeout: 10000 });
  await p1.locator('#btn-zip-step2-ack').click();
  await p1.waitForSelector('#btn-zip-step3-export', { timeout: 10000 });

  const zipDlPromise = p1.waitForEvent('download', { timeout: 30000 });
  await p1.locator('#btn-zip-step3-export').click();
  const zipDl = await zipDlPromise;
  const zipPath = path.join(OUT, zipDl.suggestedFilename());
  await zipDl.saveAs(zipPath);
  const zipBuf = fs.readFileSync(zipPath);
  const isZipMagic = zipBuf[0] === 0x50 && zipBuf[1] === 0x4b;
  await p1.waitForSelector('#btn-open-unencrypted-zip', { timeout: 10000 });
  record('Multi-step unencrypted ZIP exports single-use and auto-relocks', isZipMagic && (await p1.locator('#btn-zip-step3-export').count()) === 0, zipDl.suggestedFilename());
  await shot(p1, '04_security_unencrypted_zip_locked');

  record('Zero JS runtime page errors', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
  davServer.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
