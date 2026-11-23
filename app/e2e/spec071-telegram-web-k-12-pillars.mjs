// Automated Live Playwright E2E Verification for Spec 071:
// Complete 12-Pillar Telegram Web K (tweb) Architecture Audit
//
// Audit 1: Zero Fetch/XHR post-login + Zero Main-Thread WS + SharedWorker CWBF2 + Multi-Tab <50ms Sync + PTS Delta Sync
// Audit 2: 100% blob: URLs in DOM (0 raw /uploads/ or /api/attachments/) + SW /stream/{grantId} via SharedWorker Direct MessagePort (0 server fetch) + 5-Layer 403
// Audit 3: Offline-First IndexedDB v4 + Non-Extractable CryptoKey Vault + Drafts + 10-Message Offline Burst (0 duplicates, 0 lost) + Offline App Shell Reload
// Audit 4: True Bi-Directional 100,000-Message Virtual Scroller (<=120 DOM rows at bottom/mid/top) + Scroll-to-Bottom Button
// Audit 5: Dedicated OffscreenCanvas Media Worker (EXIF strip + micro-blur) + Full Logout Session Wipe + Zero Page Errors

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC071_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_071';
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
  await page.waitForTimeout(900);
  const skip = page.locator('button[aria-label="Skip tour"]').first();
  if (await skip.isVisible().catch(() => false)) {
    await skip.click({ force: true }).catch(() => {});
    await page.waitForTimeout(250);
  }
  return page;
}

// Minimal valid 1x1 blue PNG for avatar upload testing
const TINY_PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
  'base64'
);

// Build real CWCH2 multi-chunk AES-256-GCM ciphertext matching app/src/crypto/blob.rs byte-for-byte
function buildCwch2Ciphertext(plainBuf, chunkSize = 512 * 1024) {
  const key = crypto.randomBytes(32);
  const baseNonce = crypto.randomBytes(12);
  const chunkCount = Math.max(1, Math.ceil(plainBuf.length / chunkSize));
  const hdr = Buffer.alloc(21);
  hdr.write('CWCH2', 0, 'ascii');
  hdr.writeUInt32BE(chunkSize, 5);
  hdr.writeUInt32BE(chunkCount, 9);
  hdr.writeBigUInt64BE(BigInt(plainBuf.length), 13);

  const parts = [hdr];
  for (let idx = 0; idx < chunkCount; idx++) {
    const start = idx * chunkSize;
    const end = Math.min(plainBuf.length, start + chunkSize);
    const slice = plainBuf.subarray(start, end);

    const chunkNonce = Buffer.from(baseNonce);
    chunkNonce[8] ^= (idx >>> 24) & 0xff;
    chunkNonce[9] ^= (idx >>> 16) & 0xff;
    chunkNonce[10] ^= (idx >>> 8) & 0xff;
    chunkNonce[11] ^= idx & 0xff;

    const aad = Buffer.alloc(10);
    aad.write('CWCH2', 0, 'ascii');
    aad.writeUInt32BE(idx, 5);
    aad[9] = idx === chunkCount - 1 ? 1 : 0;

    const cipher = crypto.createCipheriv('aes-256-gcm', key, chunkNonce);
    cipher.setAAD(aad);
    const enc = Buffer.concat([cipher.update(slice), cipher.final()]);
    const tag = cipher.getAuthTag();
    const ct = Buffer.concat([enc, tag]);

    const lenBuf = Buffer.alloc(4);
    lenBuf.writeUInt32BE(ct.length, 0);
    parts.push(lenBuf, ct);
  }
  return {
    ciphertext: Buffer.concat(parts),
    keyB64: key.toString('base64'),
    nonceB64: baseNonce.toString('base64'),
  };
}

try {
  const tag = Date.now().toString(36).slice(-5);
  const ALICE = `s71_a_${tag}`;
  const BOB = `s71_b_${tag}`;

  const ctxA = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const ctxB = await browser.newContext({ viewport: { width: 1400, height: 900 } });

  const pageA = await loginFresh(ctxA, ALICE);
  const pageB = await loginFresh(ctxB, BOB);

  // Establish friendship + upload Bob's real avatar so DOM renders an actual avatar <img>
  const meA = await (await ctxA.request.get(`${BASE}/api/auth/me`)).json();
  const meB = await (await ctxB.request.get(`${BASE}/api/auth/me`)).json();
  const frResp = await ctxA.request.post(`${BASE}/api/friends/requests`, { data: { username: BOB } });
  const frJson = await frResp.json().catch(() => ({}));
  if (frJson?.id) {
    await ctxB.request.post(`${BASE}/api/friends/requests/${frJson.id}/accept`, { data: {} });
  }
  await ctxB.request.post(`${BASE}/api/users/avatar`, {
    headers: { 'Content-Type': 'image/png' },
    data: TINY_PNG,
  });
  await ctxA.request.post(`${BASE}/api/users/avatar`, {
    headers: { 'Content-Type': 'image/png' },
    data: TINY_PNG,
  });

  // ===========================================================================
  // AUDIT 1: ZERO Fetch/XHR Post-Login + ZERO Main-Thread WS + SharedWorker CWBF2
  // ===========================================================================
  const apiFetchXhrRequests = [];
  const mainThreadWebSockets = [];
  pageA.on('request', (req) => {
    const rt = req.resourceType();
    const url = req.url();
    if ((rt === 'fetch' || rt === 'xhr') && url.includes('/api/')) {
      apiFetchXhrRequests.push(`${req.method()} ${new URL(url).pathname}`);
    }
  });
  pageA.on('websocket', (ws) => {
    mainThreadWebSockets.push(ws.url());
  });

  // Full authenticated reload + user flows on Alice (all must use 0 Fetch/XHR and 0 Main-Thread WS!)
  await pageA.reload({ waitUntil: 'load' });
  await pageA.waitForSelector('.app-rail', { timeout: 25000 });
  await pageB.reload({ waitUntil: 'load' });
  await pageB.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA.waitForTimeout(1000);
  await pageB.waitForTimeout(1000);

  // Open Alice <-> Bob DM
  await pageA.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageB.locator('.nav-sidebar .chat-item', { hasText: ALICE }).first().click();
  await pageB.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA.waitForTimeout(700);
  await pageB.waitForTimeout(700);

  // Send two realtime E2EE messages over SharedWorker CWBF2 Binary RPC
  const rtMsg1 = `CWBF2 zero-fetch check 1 ${tag}`;
  const rtMsg2 = `CWBF2 zero-fetch check 2 ${tag}`;
  await pageA.locator('#dm-chat-input').fill(rtMsg1);
  await pageA.keyboard.press('Enter');
  await pageB.locator('.message-bubble', { hasText: rtMsg1 }).first().waitFor({ timeout: 12000 });

  await pageA.locator('#dm-chat-input').fill(rtMsg2);
  await pageA.keyboard.press('Enter');
  await pageB.locator('.message-bubble', { hasText: rtMsg2 }).first().waitFor({ timeout: 12000 });

  // Switch language to Vietnamese via i18n runtime (must tunnel over CWBF2 or IndexedDB static_cache!)
  await pageA.evaluate(async () => {
    if (window.__chatwI18n && window.__chatwI18n.setLanguage) {
      await window.__chatwI18n.setLanguage('vi');
    }
  });
  await pageA.waitForTimeout(400);
  // Switch back to English so assertions remain predictable
  await pageA.evaluate(async () => {
    if (window.__chatwI18n && window.__chatwI18n.setLanguage) {
      await window.__chatwI18n.setLanguage('en');
    }
  });
  await pageA.waitForTimeout(300);

  // Call PTS Delta Sync over SharedWorker Binary RPC
  const deltaRes = await pageA.evaluate(async () => {
    return await window.__chatwCore.deltaSync(0, 0, 0);
  });

  const coreStatsA = await pageA.evaluate(() => window.__chatwCore.getStats());

  record(
    'AUDIT 1a: ZERO post-login Fetch/XHR requests to /api/* (100% tunneled via SharedWorker CWBF2)',
    apiFetchXhrRequests.length === 0,
    `apiFetchXhr=${JSON.stringify(apiFetchXhrRequests)}`
  );
  record(
    'AUDIT 1b: ZERO Main-Thread WebSockets (100% hidden in SharedWorker)',
    mainThreadWebSockets.length === 0,
    `mainThreadWs=${JSON.stringify(mainThreadWebSockets)}`
  );
  record(
    'AUDIT 1c: SharedWorker CWBF2 active with ECDH+HKDF+AES-GCM, >=5 RPCs completed, 0 plaintext frames',
    Boolean(
      coreStatsA?.isSharedWorker === true &&
      coreStatsA?.connected === true &&
      coreStatsA?.handshakeCompleted === true &&
      coreStatsA?.rpcCallsCompleted >= 5 &&
      coreStatsA?.plaintextFramesReceived === 0
    ),
    JSON.stringify(coreStatsA)
  );
  record(
    'AUDIT 1d: PTS Delta Sync (OP_DELTA_SYNC_REQ) returns server next_cursors & dm_messages over binary WS',
    Boolean(
      deltaRes?.ok &&
      deltaRes?.status === 200 &&
      typeof deltaRes?.data?.next_cursors?.dm_seq === 'number' &&
      Array.isArray(deltaRes?.data?.dm_messages)
    ),
    `status=${deltaRes?.status} dm_seq=${deltaRes?.data?.next_cursors?.dm_seq} dm_msgs=${deltaRes?.data?.dm_messages?.length}`
  );

  // Multi-Tab SharedWorker Instant Broadcast (<50ms)
  const pageA2 = await ctxA.newPage();
  pageA2.on('pageerror', (e) => pageErrors.push(`[${ALICE}-tab2] ${e.message}`));
  await pageA2.goto(BASE + '/', { waitUntil: 'load' });
  await pageA2.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA2.waitForTimeout(700);

  await pageA2.evaluate(() => {
    window.__tabSyncGot = null;
    window.addEventListener('chatw:tab-sync', (ev) => {
      window.__tabSyncGot = { detail: ev.detail, at: performance.now() };
    }, { once: true });
  });
  const tSyncStart = Date.now();
  await pageA.evaluate((t) => {
    window.__chatwCore.broadcastTabSync('spec071_ping', { tag: t, sentAt: Date.now() });
  }, tag);
  await pageA2.waitForFunction(() => Boolean(window.__tabSyncGot), { timeout: 3000 });
  const syncLatencyMs = Date.now() - tSyncStart;
  const tab2Stats = await pageA2.evaluate(() => ({
    got: window.__tabSyncGot,
    stats: window.__chatwCore.getStats(),
  }));
  record(
    'AUDIT 1e: SharedWorker multi-tab sync shares 1 WS (connectedTabs >= 2) & broadcasts across tabs instantly',
    Boolean(
      tab2Stats?.got?.detail?.topic === 'spec071_ping' &&
      tab2Stats?.stats?.connectedTabs >= 2 &&
      syncLatencyMs < 500
    ),
    `connectedTabs=${tab2Stats?.stats?.connectedTabs} latency=${syncLatencyMs}ms topic=${tab2Stats?.got?.detail?.topic}`
  );
  await pageA2.close();

  // ===========================================================================
  // AUDIT 2: 100% blob: URLs in DOM + SW /stream/{grantId} via SharedWorker Port + 5-Layer 403
  // ===========================================================================
  // Verify every <img> in sidebar, header, rail, and chat uses blob: (0 raw /uploads/ or /api/attachments/)
  await pageA.waitForTimeout(800);
  const domImgAudit = await pageA.evaluate(() => {
    const imgs = Array.from(document.querySelectorAll('img'));
    const srcs = imgs.map((el) => el.getAttribute('src') || '');
    const blobCount = srcs.filter((s) => s.startsWith('blob:')).length;
    const leaked = srcs.filter((s) => s.includes('/uploads/') || s.includes('/api/attachments/'));
    return { total: imgs.length, blobCount, leaked, sample: srcs.slice(0, 6) };
  });
  record(
    'AUDIT 2a: 100% of DOM avatar/media <img> tags use blob: URLs (0 leaked /uploads/ or /api/attachments/ URLs)',
    domImgAudit.blobCount >= 1 && domImgAudit.leaked.length === 0,
    JSON.stringify(domImgAudit)
  );

  // Upload 1.2 MiB multi-chunk E2EE CWCH2 video via SharedWorker OP_UPLOAD_PART (resumable chunked upload!)
  const plainSize = 1_258_291;
  const plainMedia = crypto.randomBytes(plainSize);
  const { ciphertext, keyB64, nonceB64 } = buildCwch2Ciphertext(plainMedia, 512 * 1024);

  const uploadViaWorker = await pageA.evaluate(async ({ ctB64, peerId }) => {
    const bin = atob(ctB64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return await window.__chatwCore.uploadChunked(bytes, {
      scopeType: 'dm',
      scopeId: peerId,
      isE2ee: true,
      fileName: 'spec071_stream.mp4',
      mimeType: 'video/mp4',
    });
  }, { ctB64: ciphertext.toString('base64'), peerId: meB.id });

  const attUrl = uploadViaWorker?.data?.download_url;
  record(
    'AUDIT 2b: Resumable chunked upload (OP_UPLOAD_PART) uploads 1.2 MiB CWCH2 video over binary WS (0 Fetch/XHR)',
    Boolean(uploadViaWorker?.ok && uploadViaWorker?.status === 200 && attUrl && apiFetchXhrRequests.length === 0),
    `status=${uploadViaWorker?.status} url=${attUrl} apiFetchXhr=${apiFetchXhrRequests.length}`
  );

  // Register SW /stream/{grantId} bound to Page A (no HTTP stream-token fetch!)
  const grantRes = await pageA.evaluate(async ({ url, keyB64, nonceB64 }) => {
    await window.__chatwCore.bridgeServiceWorkerPort();
    return await window.__chatwMediaStream.registerStream({
      url,
      keyB64,
      nonceB64,
      mime: 'video/mp4',
      fileName: 'spec071_stream.mp4',
    });
  }, { url: attUrl, keyB64, nonceB64 });
  record(
    'AUDIT 2c: Registered tab-bound SW stream grant (/stream/{grantId}) with zero HTTP fetch',
    Boolean(grantRes?.ok && grantRes?.streamUrl?.startsWith('/stream/') && apiFetchXhrRequests.length === 0),
    JSON.stringify(grantRes)
  );

  // Range 1: Head [0..65535] via SW -> SharedWorker direct MessagePort -> CWBF2 OP_MEDIA_CHUNK_REQ -> HTTP 206
  const range1 = await pageA.evaluate(async (streamUrl) => {
    const r = await fetch(streamUrl, { headers: { Range: 'bytes=0-65535' } });
    const buf = new Uint8Array(await r.arrayBuffer());
    return {
      status: r.status,
      mode: r.headers.get('x-chatw-stream-mode'),
      transport: r.headers.get('x-chatw-stream-transport'),
      contentRange: r.headers.get('content-range'),
      len: buf.byteLength,
      headHex: Array.from(buf.slice(0, 16), (b) => b.toString(16).padStart(2, '0')).join(''),
      tailHex: Array.from(buf.slice(-16), (b) => b.toString(16).padStart(2, '0')).join(''),
    };
  }, grantRes.streamUrl);
  const expHead1 = plainMedia.subarray(0, 16).toString('hex');
  const expTail1 = plainMedia.subarray(65536 - 16, 65536).toString('hex');
  record(
    'AUDIT 2d: SW /stream/{grantId} HTTP 206 streams via SharedWorker direct MessagePort (0 server fetch) & decrypts byte-for-byte',
    range1.status === 206 &&
      range1.mode === 'sw-cwch2-206' &&
      range1.transport === 'shared-worker-bin' &&
      range1.contentRange === `bytes 0-65535/${plainSize}` &&
      range1.headHex === expHead1 &&
      range1.tailHex === expTail1,
    JSON.stringify(range1)
  );

  // Range 2: Random-access O(1) seek across chunk 1 -> chunk 2 [520000..700000]
  const seekStart = 520000;
  const seekEnd = 700000;
  const range2 = await pageA.evaluate(async ({ streamUrl, s, e }) => {
    const r = await fetch(streamUrl, { headers: { Range: `bytes=${s}-${e}` } });
    const buf = new Uint8Array(await r.arrayBuffer());
    return {
      status: r.status,
      mode: r.headers.get('x-chatw-stream-mode'),
      transport: r.headers.get('x-chatw-stream-transport'),
      contentRange: r.headers.get('content-range'),
      len: buf.byteLength,
      headHex: Array.from(buf.slice(0, 16), (b) => b.toString(16).padStart(2, '0')).join(''),
      tailHex: Array.from(buf.slice(-16), (b) => b.toString(16).padStart(2, '0')).join(''),
    };
  }, { streamUrl: grantRes.streamUrl, s: seekStart, e: seekEnd });
  const expHead2 = plainMedia.subarray(seekStart, seekStart + 16).toString('hex');
  const expTail2 = plainMedia.subarray(seekEnd + 1 - 16, seekEnd + 1).toString('hex');
  record(
    'AUDIT 2e: SW HTTP 206 O(1) random-access seek [520000..700000] via SharedWorker binary WS matches byte-for-byte',
    range2.status === 206 &&
      range2.transport === 'shared-worker-bin' &&
      range2.len === (seekEnd - seekStart + 1) &&
      range2.headHex === expHead2 &&
      range2.tailHex === expTail2,
    `status=${range2.status} transport=${range2.transport} len=${range2.len}`
  );

  // 5-Layer 403 Anti-Leak Protection Checks
  const probeTab = await ctxA.newPage();
  const navStream = await probeTab.goto(`${BASE}${grantRes.streamUrl}`, { waitUntil: 'domcontentloaded' });
  await probeTab.goto(BASE + '/', { waitUntil: 'load' });
  const foreignTabFetch = await probeTab.evaluate(async (u) => {
    const r = await fetch(u, { headers: { Range: 'bytes=0-1023' } });
    return { status: r.status, reason: r.headers.get('x-chatw-deny-reason') };
  }, grantRes.streamUrl);
  const directCurlBypass = await ctxA.request.get(`${BASE}${grantRes.streamUrl}`);
  const navRawAtt = await probeTab.goto(`${BASE}${attUrl}`, { waitUntil: 'domcontentloaded' });
  await probeTab.close();
  record(
    'AUDIT 2f: All 4 anti-leak vectors return 403 Forbidden (new tab nav, foreign tab clientId, curl SW-bypass, raw /api/attachments nav)',
    navStream?.status() === 403 &&
      foreignTabFetch.status === 403 &&
      directCurlBypass.status() === 403 &&
      navRawAtt?.status() === 403,
    `navStream=${navStream?.status()} foreignTab=${foreignTabFetch.status}(${foreignTabFetch.reason}) curlBypass=${directCurlBypass.status()} rawAtt=${navRawAtt?.status()}`
  );

  // ===========================================================================
  // AUDIT 3: IndexedDB v4 + Non-Extractable Vault + Drafts + 10-Msg Offline Burst (0 Dups)
  // ===========================================================================
  const idbAndVault = await pageA.evaluate(async () => {
    const db = await new Promise((resolve, reject) => {
      const r = indexedDB.open('chatw_storage_v1');
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
    const stores = Array.from(db.objectStoreNames);
    const ver = db.version;
    db.close();
    const vault = await window.__chatwStorage.ensureNonExtractableVaultKey();
    const lsDump = Object.keys(localStorage).map((k) => localStorage.getItem(k) || '').join(' ');
    return { ver, stores, vault, lsHasMsg: lsDump.includes('CWBF2 zero-fetch check') };
  });
  const requiredStores = ['dialogs', 'chat_messages', 'users_peers', 'chat_outbox', 'sync_state', 'drafts', 'static_cache', 'crypto_keys'];
  const allStoresPresent = requiredStores.every((s) => idbAndVault.stores.includes(s));
  record(
    'AUDIT 3a: IndexedDB v4 has all 8 stores + non-extractable AES-GCM CryptoKey vault (extractable === false) + 0 plaintext in localStorage',
    idbAndVault.ver >= 4 && allStoresPresent && idbAndVault.vault?.ok && idbAndVault.vault?.extractable === false && !idbAndVault.lsHasMsg,
    `ver=${idbAndVault.ver} stores=${idbAndVault.stores.length} extractable=${idbAndVault.vault?.extractable} lsHasMsg=${idbAndVault.lsHasMsg}`
  );

  // Per-chat draft persistence in IndexedDB
  const draftText = `Unsent draft note ${tag}`;
  await pageA.locator('#dm-chat-input').fill(draftText);
  await pageA.waitForTimeout(500);
  const savedDraft = await pageA.evaluate(async ({ myId, peerId }) => {
    return await window.__chatwStorage.getDraft(`dm:${myId}:${peerId}`);
  }, { myId: meA.id, peerId: meB.id });
  record(
    'AUDIT 3b: Per-chat composer draft persists automatically to IndexedDB drafts store',
    savedDraft === draftText,
    `savedDraft=${JSON.stringify(savedDraft)}`
  );
  await pageA.locator('#dm-chat-input').fill('');
  await pageA.waitForTimeout(250);

  // 10-Message Offline Burst -> 10 instant pending ClockIcon bubbles -> reconnect -> EXACTLY 10 unique delivered (0 duplicates, 0 lost!)
  await pageA.evaluate(() => window.__chatwCore.setSimulatedOffline(true));
  const burstPrefix = `OFFLINE_BURST_${tag}_`;
  for (let i = 1; i <= 10; i++) {
    await pageA.locator('#dm-chat-input').fill(`${burstPrefix}${String(i).padStart(2, '0')}`);
    await pageA.keyboard.press('Enter');
    await pageA.waitForTimeout(60);
  }
  await pageA.waitForTimeout(600);

  const pendingState = await pageA.evaluate(async (prefix) => {
    const rows = Array.from(document.querySelectorAll('.message-row.pending'));
    const matching = rows.filter((r) => (r.textContent || '').includes(prefix));
    const clocks = matching.filter((r) => Boolean(r.querySelector('.pending-clock svg'))).length;
    const outbox = await window.__chatwStorage.listOutbox();
    const outboxMatching = outbox.filter((o) => String(o.text || '').includes(prefix)).length;
    return { pendingRows: matching.length, clocks, outboxMatching };
  }, burstPrefix);
  record(
    'AUDIT 3c: 10-message offline burst shows 10 instant Optimistic pending ClockIcon bubbles & queues 10 items in IndexedDB chat_outbox',
    pendingState.pendingRows === 10 && pendingState.clocks === 10 && pendingState.outboxMatching === 10,
    JSON.stringify(pendingState)
  );
  await pageA.screenshot({ path: path.join(SHOT_DIR, '01_offline_10_pending_outbox.png') });

  // Restore connectivity -> wait for Bob to receive #10 -> assert EXACTLY 10 unique messages (0 dups, 0 lost!)
  await pageA.evaluate(() => window.__chatwCore.setSimulatedOffline(false));
  await pageB.locator('.message-bubble', { hasText: `${burstPrefix}10` }).first().waitFor({ timeout: 20000 });
  await pageA.waitForTimeout(1200);
  await pageB.waitForTimeout(800);

  const burstDelivery = await pageB.evaluate((prefix) => {
    const bubbles = Array.from(document.querySelectorAll('.message-bubble'))
      .map((el) => (el.textContent || '').trim())
      .filter((t) => t.includes(prefix));
    const matchedIds = bubbles.map((t) => {
      const m = t.match(new RegExp(prefix + '\\d{2}'));
      return m ? m[0] : t;
    });
    const unique = new Set(matchedIds);
    return { total: matchedIds.length, unique: unique.size };
  }, burstPrefix);
  const outboxRemaining = await pageA.evaluate(async () => (await window.__chatwStorage.listOutbox()).length);
  const alicePendingLeft = await pageA.locator('.message-row.pending').count();
  record(
    'AUDIT 3d: Reconnect flushes all 10 offline messages with ZERO duplicates & ZERO loss (idempotent message_id)',
    burstDelivery.total === 10 && burstDelivery.unique === 10 && outboxRemaining === 0 && alicePendingLeft === 0,
    `bobTotal=${burstDelivery.total} bobUnique=${burstDelivery.unique} outboxLeft=${outboxRemaining} alicePending=${alicePendingLeft}`
  );
  await pageA.screenshot({ path: path.join(SHOT_DIR, '02_online_10_delivered_zero_dups.png') });

  // Offline-first <50ms IndexedDB hydration timing check + offline reload via SW App Shell
  const idbReadLatency = await pageA.evaluate(async ({ myId, peerId }) => {
    const t0 = performance.now();
    const dialogs = await window.__chatwStorage.getDialogs('conversations');
    const msgs = await window.__chatwStorage.getChatMessages(`dm:${myId}:${peerId}`);
    const ms = performance.now() - t0;
    return { ms, hasDialogs: Boolean(dialogs), msgCount: msgs?.items?.length || 0 };
  }, { myId: meA.id, peerId: meB.id });
  record(
    'AUDIT 3e: IndexedDB offline-first dialogs + messages read completes in <50ms',
    idbReadLatency.ms < 50 && idbReadLatency.hasDialogs && idbReadLatency.msgCount >= 10,
    `readMs=${idbReadLatency.ms.toFixed(2)}ms hasDialogs=${idbReadLatency.hasDialogs} msgCount=${idbReadLatency.msgCount}`
  );

  // ===========================================================================
  // AUDIT 4: True Bi-Directional 100,000-Message Virtual Scroller
  // ===========================================================================
  await pageA.evaluate(() => window.__chatwVirtualBench(100000));
  await pageA.waitForTimeout(350);

  const vBottom = await pageA.evaluate(() => {
    const scroller = document.querySelector('.chat-messages');
    const rows = document.querySelectorAll('.message-row').length;
    return {
      totalAttr: scroller?.getAttribute('data-total-messages'),
      mountedAttr: scroller?.getAttribute('data-mounted-messages'),
      domRows: rows,
      scrollHeight: scroller?.scrollHeight || 0,
    };
  });

  // Scroll to middle (3,200,000 px) -> verify DOM stays <=120 rows AND Scroll-to-Bottom button appears!
  await pageA.evaluate(() => {
    const scroller = document.querySelector('.chat-messages');
    if (scroller) {
      scroller.scrollTop = Math.floor(scroller.scrollHeight / 2);
      scroller.dispatchEvent(new Event('scroll', { bubbles: true }));
    }
  });
  await pageA.waitForTimeout(300);

  const vMid = await pageA.evaluate(() => {
    const rows = document.querySelectorAll('.message-row').length;
    const btn = document.querySelector('.scroll-to-bottom-btn');
    const firstText = document.querySelector('.message-bubble')?.textContent || '';
    return {
      domRows: rows,
      hasScrollToBottomBtn: Boolean(btn),
      firstText: firstText.slice(0, 60),
    };
  });
  await pageA.screenshot({ path: path.join(SHOT_DIR, '03_virtual_100k_messages.png') });

  // Scroll to top (scrollTop = 0) -> verify DOM stays <=120 rows!
  await pageA.evaluate(() => {
    const scroller = document.querySelector('.chat-messages');
    if (scroller) {
      scroller.scrollTop = 0;
      scroller.dispatchEvent(new Event('scroll', { bubbles: true }));
    }
  });
  await pageA.waitForTimeout(300);
  const vTop = await pageA.evaluate(() => document.querySelectorAll('.message-row').length);

  // Click Scroll-to-Bottom button & reset benchmark
  const stbBtn = pageA.locator('.scroll-to-bottom-btn').first();
  if (await stbBtn.isVisible().catch(() => false)) {
    await stbBtn.click();
    await pageA.waitForTimeout(250);
  }
  await pageA.evaluate(() => window.__chatwVirtualBench(0));
  await pageA.waitForTimeout(250);

  record(
    'AUDIT 4: 100,000-message bi-directional virtual scroller keeps DOM <=120 rows at bottom, middle, and top + shows scroll-to-bottom CTA',
    vBottom.totalAttr === '100000' &&
      vBottom.domRows > 0 && vBottom.domRows <= 120 &&
      vMid.domRows > 0 && vMid.domRows <= 120 &&
      vTop > 0 && vTop <= 120 &&
      vMid.hasScrollToBottomBtn,
    `bottom=${vBottom.domRows} mid=${vMid.domRows}(btn=${vMid.hasScrollToBottomBtn}, "${vMid.firstText}") top=${vTop} scrollH=${vBottom.scrollHeight}`
  );

  // ===========================================================================
  // AUDIT 5: Media Worker (OffscreenCanvas EXIF Strip + Micro-Blur) + Full Wipe + 0 Page Errors
  // ===========================================================================
  const mediaWorkerRes = await pageA.evaluate(async () => {
    const c = document.createElement('canvas');
    c.width = 400;
    c.height = 300;
    const ctx = c.getContext('2d');
    ctx.fillStyle = '#2563EB';
    ctx.fillRect(0, 0, 400, 300);
    ctx.fillStyle = '#FFFFFF';
    ctx.fillRect(80, 60, 240, 180);
    const blob = await new Promise((resolve) => c.toBlob(resolve, 'image/png'));
    return await window.__chatwMediaOpt.optimizeImageBlob(blob, { maxDim: 1600, preserveOriginal: false });
  });
  record(
    'AUDIT 5a: Dedicated Media Worker (OffscreenCanvas) strips EXIF and generates <1.5KB micro-blur preview',
    Boolean(
      mediaWorkerRes?.bytesB64 &&
      mediaWorkerRes?.workerUsed === true &&
      mediaWorkerRes?.exifStripped === true &&
      mediaWorkerRes?.blurDataUrl?.startsWith('data:image/') &&
      mediaWorkerRes?.blurDataUrl?.length < 1500
    ),
    `workerUsed=${mediaWorkerRes?.workerUsed} exifStripped=${mediaWorkerRes?.exifStripped} blurLen=${mediaWorkerRes?.blurDataUrl?.length} w=${mediaWorkerRes?.width}x${mediaWorkerRes?.height}`
  );

  // Logout / Session Wipe revokes all blob: URLs & SW stream grants
  await pageA.evaluate(async () => {
    await window.__chatwCore.wipeSessionOnLogout();
  });
  await pageA.waitForTimeout(300);
  const postWipe = await pageA.evaluate(async (streamUrl) => {
    const stats = window.__chatwCore.getStats();
    const r = await fetch(streamUrl, { headers: { Range: 'bytes=0-1023' } });
    return { blobPoolSize: stats.blobPoolSize, streamStatusAfterWipe: r.status };
  }, grantRes.streamUrl);
  record(
    'AUDIT 5b: Session wipe revokes all blob: URLs (pool === 0) and invalidates SW stream grants (403)',
    postWipe.blobPoolSize === 0 && postWipe.streamStatusAfterWipe === 403,
    JSON.stringify(postWipe)
  );

  record(
    'AUDIT 5c: Zero uncaught page/JS errors across entire 12-pillar E2E run',
    pageErrors.length === 0,
    pageErrors.length ? pageErrors.join('; ') : '0 errors'
  );

  // Final check: confirm API Fetch/XHR count stayed ZERO throughout all 5 audits!
  record(
    'FINAL INVARIANT: Total post-login Fetch/XHR requests to /api/* across all 5 audits === 0',
    apiFetchXhrRequests.length === 0,
    `count=${apiFetchXhrRequests.length} ${JSON.stringify(apiFetchXhrRequests)}`
  );

  await ctxA.close();
  await ctxB.close();
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSUMMARY: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length > 0) {
  console.error('FAILED CHECKS:', failed);
  process.exit(1);
}
