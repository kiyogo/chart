// Automated Live Playwright E2E Verification for Spec 069:
// 1. Dedicated Web Worker (/network_worker.js) + ECDH P-256 Encrypted Binary WebSocket (/api/ws?proto=bin, CWBF1)
// 2. Service Worker Media Proxy (/sw.js) + O(1) Random-Access E2EE CWCH2 HTTP 206 Streaming + Strict 403 Anti-Direct-Access
// 3. Offline-First IndexedDB v3 Instant Hydration + Optimistic UI (ClockIcon pending) + Auto-Flushing Offline Outbox
// 4. Progressive Blur-Up Media Preview (blurDataUrl + aspect-ratio) & 100k Virtual Scroll CSS (content-visibility: auto)

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC069_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_069';
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

// Build a real CWCH2 multi-chunk AES-256-GCM ciphertext matching app/src/crypto/blob.rs byte-for-byte
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
  const ALICE = `s69_a_${tag}`;
  const BOB = `s69_b_${tag}`;

  const ctxA = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const ctxB = await browser.newContext({ viewport: { width: 1400, height: 900 } });

  const pageA = await loginFresh(ctxA, ALICE);
  const pageB = await loginFresh(ctxB, BOB);

  // Wait for Dedicated Web Worker ECDH handshake & Service Worker activation
  await pageA.waitForTimeout(1200);
  await pageB.waitForTimeout(1200);

  // ---------------------------------------------------------------------------
  // PILLAR 1: Dedicated Web Worker + ECDH Encrypted Binary WebSocket (CWBF1)
  // ---------------------------------------------------------------------------
  const statsA = await pageA.evaluate(() => window.__chatwNetBridge && window.__chatwNetBridge.getStats());
  const statsB = await pageB.evaluate(() => window.__chatwNetBridge && window.__chatwNetBridge.getStats());
  record(
    'Dedicated Web Worker active with ECDH binary WS handshake (zero plaintext WS frames)',
    Boolean(
      statsA?.workerActive && statsA?.connected && statsA?.handshakeCompleted &&
      statsA?.binaryFramesReceived >= 1 && statsA?.plaintextFramesReceived === 0 &&
      statsB?.workerActive && statsB?.connected && statsB?.handshakeCompleted &&
      statsB?.plaintextFramesReceived === 0
    ),
    `A=${JSON.stringify(statsA)} B=${JSON.stringify(statsB)}`
  );

  // Make Alice & Bob friends via API so we test real-time E2EE DM delivery over CWBF1 binary WS!
  const meA = await (await ctxA.request.get(`${BASE}/api/auth/me`)).json();
  const meB = await (await ctxB.request.get(`${BASE}/api/auth/me`)).json();
  const frResp = await ctxA.request.post(`${BASE}/api/friends/requests`, { data: { username: BOB } });
  const frJson = await frResp.json().catch(() => ({}));
  if (frJson?.id) {
    await ctxB.request.post(`${BASE}/api/friends/requests/${frJson.id}/accept`, { data: {} });
  }
  await pageA.reload({ waitUntil: 'load' });
  await pageA.waitForSelector('.app-rail', { timeout: 25000 });
  await pageB.reload({ waitUntil: 'load' });
  await pageB.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA.waitForTimeout(1000);
  await pageB.waitForTimeout(1000);

  // Open Alice <-> Bob DM on both sides
  await pageA.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageB.locator('.nav-sidebar .chat-item', { hasText: ALICE }).first().click();
  await pageB.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA.waitForTimeout(600);
  await pageB.waitForTimeout(600);

  const realtimeMsg = `CWBF1 binary realtime check ${tag}`;
  await pageA.locator('#dm-chat-input').fill(realtimeMsg);
  await pageA.keyboard.press('Enter');
  await pageB.locator('.message-bubble', { hasText: realtimeMsg }).first().waitFor({ timeout: 12000 });
  const statsBAfter = await pageB.evaluate(() => window.__chatwNetBridge.getStats());
  record(
    'Realtime E2EE DM delivered to Bob via Worker-decrypted CWBF1 binary frame',
    statsBAfter.binaryEventsDecrypted >= 1 && statsBAfter.plaintextFramesReceived === 0,
    `binaryEventsDecrypted=${statsBAfter.binaryEventsDecrypted} plaintextFrames=${statsBAfter.plaintextFramesReceived}`
  );

  // ---------------------------------------------------------------------------
  // PILLAR 2: Service Worker Media Proxy, CWCH2 HTTP 206 Seeking & Strict 403
  // ---------------------------------------------------------------------------
  const swStatus = await pageA.evaluate(() => window.__chatwMediaStream.getStatus());
  record(
    'Service Worker (/sw.js) registered and controlling root scope',
    Boolean(swStatus?.ok && swStatus?.controlling && String(swStatus?.version || '').startsWith('chatw-sw-')),
    JSON.stringify(swStatus)
  );

  // Upload 1.2 MiB multi-chunk E2EE CWCH2 media payload (3 x 512 KiB chunks)
  const plainSize = 1_258_291; // ~1.2 MiB across 3 chunks
  const plainMedia = crypto.randomBytes(plainSize);
  const { ciphertext, keyB64, nonceB64 } = buildCwch2Ciphertext(plainMedia, 512 * 1024);
  const upResp = await ctxA.request.post(
    `${BASE}/api/attachments?scope_type=dm&scope_id=${meB.id}&is_e2ee=true&file_name=demo_stream.mp4&mime_type=video%2Fmp4`,
    {
      headers: { 'Content-Type': 'application/octet-stream' },
      data: ciphertext,
    }
  );
  const upJson = await upResp.json();
  const attUrl = upJson.download_url;
  record('Uploaded 1.2 MiB multi-chunk CWCH2 E2EE video to /api/attachments', upResp.status() === 200 && Boolean(attUrl), `status=${upResp.status()} url=${attUrl}`);

  // Register tab-bound SW stream grant inside Page A
  const grantRes = await pageA.evaluate(async ({ url, keyB64, nonceB64 }) => {
    return await window.__chatwMediaStream.registerStream({
      url,
      keyB64,
      nonceB64,
      mime: 'video/mp4',
      fileName: 'demo_stream.mp4',
    });
  }, { url: attUrl, keyB64, nonceB64 });
  record('Registered tab-bound SW stream grant (/stream/{grantId})', Boolean(grantRes?.ok && grantRes?.streamUrl?.startsWith('/stream/')), JSON.stringify(grantRes));

  // Range 1: Head bytes [0..65535] (Chunk 0)
  const range1 = await pageA.evaluate(async (streamUrl) => {
    const r = await fetch(streamUrl, { headers: { Range: 'bytes=0-65535' } });
    const buf = new Uint8Array(await r.arrayBuffer());
    return {
      status: r.status,
      mode: r.headers.get('x-chatw-stream-mode'),
      contentRange: r.headers.get('content-range'),
      acceptRanges: r.headers.get('accept-ranges'),
      len: buf.byteLength,
      headHex: Array.from(buf.slice(0, 16), (b) => b.toString(16).padStart(2, '0')).join(''),
      tailHex: Array.from(buf.slice(-16), (b) => b.toString(16).padStart(2, '0')).join(''),
    };
  }, grantRes.streamUrl);
  const expHead1 = plainMedia.subarray(0, 16).toString('hex');
  const expTail1 = plainMedia.subarray(65536 - 16, 65536).toString('hex');
  record(
    'SW HTTP 206 Partial Content decrypts CWCH2 chunk 0 byte-for-byte',
    range1.status === 206 && range1.mode === 'sw-cwch2-206' && range1.contentRange === `bytes 0-65535/${plainSize}` && range1.headHex === expHead1 && range1.tailHex === expTail1,
    `status=${range1.status} mode=${range1.mode} range=${range1.contentRange}`
  );

  // Range 2: Random-access SEEK directly into Chunk 1 & Chunk 2 boundary [520000..700000]
  const seekStart = 520000;
  const seekEnd = 700000;
  const range2 = await pageA.evaluate(async ({ streamUrl, s, e }) => {
    const r = await fetch(streamUrl, { headers: { Range: `bytes=${s}-${e}` } });
    const buf = new Uint8Array(await r.arrayBuffer());
    return {
      status: r.status,
      mode: r.headers.get('x-chatw-stream-mode'),
      contentRange: r.headers.get('content-range'),
      len: buf.byteLength,
      headHex: Array.from(buf.slice(0, 16), (b) => b.toString(16).padStart(2, '0')).join(''),
      tailHex: Array.from(buf.slice(-16), (b) => b.toString(16).padStart(2, '0')).join(''),
    };
  }, { streamUrl: grantRes.streamUrl, s: seekStart, e: seekEnd });
  const expHead2 = plainMedia.subarray(seekStart, seekStart + 16).toString('hex');
  const expTail2 = plainMedia.subarray(seekEnd + 1 - 16, seekEnd + 1).toString('hex');
  record(
    'SW HTTP 206 random-access seek across CWCH2 chunk boundary [520000..700000] matches byte-for-byte',
    range2.status === 206 && range2.mode === 'sw-cwch2-206' && range2.len === (seekEnd - seekStart + 1) && range2.headHex === expHead2 && range2.tailHex === expTail2,
    `status=${range2.status} len=${range2.len} range=${range2.contentRange}`
  );

  // Strict 403 Test A: Opening /stream/media/{grantId} directly in a NEW TAB (navigate) -> 403 Forbidden!
  const tab2 = await ctxA.newPage();
  const navStreamResp = await tab2.goto(`${BASE}${grantRes.streamUrl}`, { waitUntil: 'domcontentloaded' });
  record(
    'Opening /stream/media/{grantId} directly in a new tab returns 403 Forbidden',
    navStreamResp?.status() === 403,
    `status=${navStreamResp?.status()} reason=${navStreamResp?.headers()['x-chatw-deny-reason'] || 'backend_403'}`
  );

  // Strict 403 Test B: Subresource fetch() to /stream/media/{grantId} from a DIFFERENT tab (foreign clientId) -> 403 Forbidden!
  await tab2.goto(BASE + '/', { waitUntil: 'load' });
  const foreignFetch = await tab2.evaluate(async (streamUrl) => {
    const r = await fetch(streamUrl, { headers: { Range: 'bytes=0-1023' } });
    return { status: r.status, reason: r.headers.get('x-chatw-deny-reason') };
  }, grantRes.streamUrl);
  record(
    'Fetching /stream/media/{grantId} from another tab (foreign clientId) returns 403 Forbidden',
    foreignFetch.status === 403,
    JSON.stringify(foreignFetch)
  );

  // Strict 403 Test C: Direct HTTP request bypassing Service Worker (curl-equivalent) -> Backend returns 403 Forbidden!
  const directSwBypass = await ctxA.request.get(`${BASE}${grantRes.streamUrl}`);
  record(
    'Bypassing Service Worker to /stream/media/{grantId} returns backend 403 Forbidden',
    directSwBypass.status() === 403,
    `status=${directSwBypass.status()}`
  );

  // Strict 403 Test D: Opening raw /api/attachments/{id} directly in a new browser tab (Sec-Fetch-Mode: navigate) -> 403 Forbidden!
  const navAttResp = await tab2.goto(`${BASE}${attUrl}`, { waitUntil: 'domcontentloaded' });
  record(
    'Opening raw /api/attachments/{id} directly in a new tab returns 403 Forbidden',
    navAttResp?.status() === 403,
    `status=${navAttResp?.status()}`
  );
  await tab2.close();

  // ---------------------------------------------------------------------------
  // PILLAR 3: Offline-First IndexedDB v3, Optimistic UI (ClockIcon) & Outbox
  // ---------------------------------------------------------------------------
  const idbCheck = await pageA.evaluate(async ({ myId, peerId, expectedText }) => {
    const db = await new Promise((resolve, reject) => {
      const r = indexedDB.open('chatw_storage_v1');
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
    const stores = Array.from(db.objectStoreNames);
    const ver = db.version;
    db.close();
    const cached = await window.__chatwStorage.getChatMessages(`dm:${myId}:${peerId}`);
    const hasExpected = cached.found && cached.items.some((m) => String(m.text || '').includes(expectedText));
    return { ver, stores, found: cached.found, count: cached.items.length, hasExpected };
  }, { myId: meA.id, peerId: meB.id, expectedText: realtimeMsg });
  record(
    'IndexedDB v3 has chat_messages & chat_outbox and cached DM history automatically',
    idbCheck.ver >= 3 && idbCheck.stores.includes('chat_messages') && idbCheck.stores.includes('chat_outbox') && idbCheck.hasExpected,
    JSON.stringify(idbCheck)
  );

  // Simulate offline send via SharedWorker offline disconnect -> Optimistic UI renders pending ClockIcon + queues in chat_outbox!
  const offlineMsg = `Offline queued optimistic message ${tag}`;
  await pageA.evaluate(() => window.__chatwCore.setSimulatedOffline(true));
  await pageA.locator('#dm-chat-input').fill(offlineMsg);
  await pageA.keyboard.press('Enter');
  await pageA.waitForTimeout(600);

  const pendingRow = pageA.locator('.message-row.pending', { hasText: offlineMsg }).first();
  const hasPendingRow = await pendingRow.isVisible().catch(() => false);
  const hasClockIcon = await pendingRow.locator('.pending-clock svg').isVisible().catch(() => false);
  const outboxWhileOffline = await pageA.evaluate(() => window.__chatwStorage.listOutbox());
  record(
    'Offline send shows instant Optimistic UI pending bubble with ClockIcon & queues in IndexedDB chat_outbox',
    hasPendingRow && hasClockIcon && outboxWhileOffline.some((o) => o.text === offlineMsg),
    `pendingRow=${hasPendingRow} clockIcon=${hasClockIcon} outboxLen=${outboxWhileOffline.length}`
  );
  await pageA.screenshot({ path: path.join(SHOT_DIR, '01_optimistic_pending_clock_offline.png') });

  // Restore connectivity -> SharedWorker reconnects & outbox auto-flushes, confirming bubble Sent!
  await pageA.evaluate(() => window.__chatwCore.setSimulatedOffline(false));
  await pageB.locator('.message-bubble', { hasText: offlineMsg }).first().waitFor({ timeout: 12000 });
  await pageA.waitForTimeout(700);
  const outboxAfterOnline = await pageA.evaluate(() => window.__chatwStorage.listOutbox());
  const stillPending = await pageA.locator('.message-row.pending', { hasText: offlineMsg }).count();
  record(
    'Reconnecting auto-flushes IndexedDB chat_outbox, confirms bubble Sent, and delivers to Bob',
    outboxAfterOnline.length === 0 && stillPending === 0,
    `outboxAfter=${outboxAfterOnline.length} stillPending=${stillPending}`
  );
  await pageA.screenshot({ path: path.join(SHOT_DIR, '02_outbox_flushed_confirmed.png') });

  // Instant 0ms IndexedDB hydration on reload even when server GET /api/messages is delayed by 1800ms!
  let delayedGetTriggered = false;
  await pageA.route('**/api/messages?*', async (route) => {
    delayedGetTriggered = true;
    await new Promise((r) => setTimeout(r, 1800));
    await route.continue();
  });
  await pageA.reload({ waitUntil: 'load' });
  await pageA.waitForSelector('.nav-sidebar .chat-item', { timeout: 25000 });
  await pageA.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  const t0 = Date.now();
  await pageA.locator('.message-bubble', { hasText: offlineMsg }).first().waitFor({ timeout: 900 });
  const hydrateMs = Date.now() - t0;
  record(
    'Instant IndexedDB hydration renders cached DM history before delayed 1800ms network response',
    hydrateMs < 900,
    `hydrateMs=${hydrateMs}ms delayedGet=${delayedGetTriggered}`
  );

  // ---------------------------------------------------------------------------
  // PILLAR 4: Progressive Blur-Up Media Preview & 100k Virtual CSS
  // ---------------------------------------------------------------------------
  const blurOptCheck = await pageA.evaluate(async () => {
    const c = document.createElement('canvas');
    c.width = 320;
    c.height = 200;
    const ctx = c.getContext('2d');
    const g = ctx.createLinearGradient(0, 0, 320, 200);
    g.addColorStop(0, '#2563EB');
    g.addColorStop(1, '#EC4899');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, 320, 200);
    const blob = await new Promise((r) => c.toBlob(r, 'image/png'));
    const res = await window.__chatwMediaOpt.optimizeImageBlob(blob, {});
    return {
      width: res.width,
      height: res.height,
      hasBlur: typeof res.blurDataUrl === 'string' && res.blurDataUrl.startsWith('data:image/'),
      blurLen: res.blurDataUrl ? res.blurDataUrl.length : 0,
    };
  });
  record(
    'media_optimizer.js generates lightweight micro-blur preview (< 1200B) & exact aspect dimensions',
    blurOptCheck.width === 320 && blurOptCheck.height === 200 && blurOptCheck.hasBlur && blurOptCheck.blurLen < 1200,
    JSON.stringify(blurOptCheck)
  );

  const cvCss = await pageA.evaluate(() => {
    const row = document.querySelector('.message-row');
    return row ? getComputedStyle(row).contentVisibility : null;
  });
  record(
    'Large-scale chat CSS applies content-visibility: auto on .message-row',
    cvCss === 'auto',
    `contentVisibility=${cvCss}`
  );
  await pageA.screenshot({ path: path.join(SHOT_DIR, '03_hydrated_chat_and_virtual_css.png') });

  record('Zero uncaught JS/WASM page errors during Spec 069 E2E', pageErrors.length === 0, pageErrors.join(' | '));

  await ctxA.close();
  await ctxB.close();
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 069 Summary: ${checks.length - failed.length}/${checks.length} PASS`);
if (failed.length > 0) process.exit(1);
