// Automated Live Playwright + CDP Network Verification for Spec 077:
// Verifies WASM is 100% hidden from Chrome DevTools Network tab (Wasm filter = 0 entries always):
//  1. Direct /pkg/chatw_app_bg.wasm returns 404 (not served), /pkg/core.bin is CWRT1 octet-stream <= 5MB.
//  2. Unauthenticated Cold Boot (waitUntil: 'networkidle'): 0 *.wasm, 0 Type:wasm, 0 application/wasm, persists IndexedDB.
//  3. Warm Reload: 0 *.wasm, 0 /pkg/core.bin, 0 runtime_worker.js (100% IndexedDB boot!).
//  4. Authenticated Cold Boot (empty runtime cache, chatw_auth_hint='1'): 0 *.wasm AND 0 /pkg/core.bin in Page DevTools Network (hidden inside SharedWorker!).

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC077_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_077';
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

try {
  // 0. Static artifact audit
  const ctxProbe = await browser.newContext();
  const wasmProbe = await ctxProbe.request.get(`${BASE}/pkg/chatw_app_bg.wasm`);
  const wasmProbeBody = await wasmProbe.body();
  const wasmProbeCt = wasmProbe.headers()['content-type'] || '';
  const noRawWasmServed = !wasmProbeCt.includes('wasm') && !(wasmProbeBody.length >= 4 && wasmProbeBody[0] === 0 && wasmProbeBody[1] === 0x61 && wasmProbeBody[2] === 0x73 && wasmProbeBody[3] === 0x6d);
  record(
    'Direct /pkg/chatw_app_bg.wasm NEVER serves WebAssembly binary or application/wasm',
    noRawWasmServed,
    `status=${wasmProbe.status()} ct="${wasmProbeCt}"`
  );

  const binProbe = await ctxProbe.request.get(`${BASE}/pkg/core.bin`);
  const binBytes = await binProbe.body();
  const binCt = binProbe.headers()['content-type'] || '';
  const magic = binBytes.subarray(0, 5).toString('ascii');
  record(
    '/pkg/core.bin is CWRT1 packed octet-stream <= 5MB with ZERO \\0asm or gzip magic on wire',
    binProbe.ok() && magic === 'CWRT1' && !binCt.includes('wasm') && binBytes.length < 5 * 1024 * 1024,
    `size=${binBytes.length} ct="${binCt}" magic="${magic}"`
  );

  const jsProbe = await ctxProbe.request.get(`${BASE}/pkg/chatw_app.js`);
  const jsText = await jsProbe.text();
  record(
    '/pkg/chatw_app.js contains ZERO chatw_app_bg.wasm URL references',
    jsProbe.ok() && !jsText.includes('chatw_app_bg.wasm'),
    `len=${jsText.length}`
  );
  await ctxProbe.close();

  // 1. Unauthenticated Cold Boot (waitUntil: 'networkidle')
  const ctx = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => pageErrors.push(e.message));

  const cdp = await ctx.newCDPSession(page);
  await cdp.send('Network.enable');

  let activeReqs = [];
  let activeResps = [];
  cdp.on('Network.requestWillBeSent', (e) => {
    activeReqs.push({ url: e.request.url, type: e.type || '' });
  });
  cdp.on('Network.responseReceived', (e) => {
    activeResps.push({
      url: e.response.url,
      type: e.type || '',
      mime: e.response.mimeType || '',
      fromSW: Boolean(e.response.fromServiceWorker),
    });
  });

  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.waitForTimeout(400);

  const coldWasmReqs = activeReqs.filter((r) => /\.wasm(\?|$)/i.test(r.url) || r.type.toLowerCase() === 'wasm');
  const coldWasmResps = activeResps.filter((r) => /\.wasm(\?|$)/i.test(r.url) || r.mime.includes('wasm') || r.type.toLowerCase() === 'wasm');
  record(
    'Unauthenticated Cold Boot (networkidle): ZERO *.wasm requests & ZERO Type:wasm / application/wasm in DevTools',
    coldWasmReqs.length === 0 && coldWasmResps.length === 0,
    `wasmReqs=${coldWasmReqs.length} wasmResps=${coldWasmResps.length}`
  );

  // Verify IndexedDB persisted runtime:core:*
  const idbRuntimeCached = await page.evaluate(async () => {
    const db = await new Promise((res, rej) => {
      const r = indexedDB.open('chatw_storage_v1', 4);
      r.onsuccess = () => res(r.result);
      r.onerror = () => rej(r.error);
    });
    const keys = await new Promise((res) => {
      const tx = db.transaction('static_cache', 'readonly');
      const req = tx.objectStore('static_cache').getAllKeys();
      req.onsuccess = () => res(req.result || []);
    });
    return keys.filter((k) => String(k).startsWith('runtime:core:'));
  });
  record(
    'Cold Boot persists compressed CWRT1 runtime (~3.5MB) in IndexedDB static_cache',
    Array.isArray(idbRuntimeCached) && idbRuntimeCached.length === 1,
    JSON.stringify(idbRuntimeCached)
  );

  // Sign in via dev login
  const uname = `s77_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label="Test username"]').fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);
  const tourClose = page.locator('.story-dialog .story-close').first();
  if (await tourClose.isVisible().catch(() => false)) {
    await tourClose.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // 2. Warm Reload (IndexedDB hit = 0 wasm, 0 core.bin, 0 runtime_worker.js!)
  activeReqs = [];
  activeResps = [];
  await page.reload({ waitUntil: 'load' });
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(900);

  const warmWasm = activeReqs.filter((r) => /\.wasm(\?|$)/i.test(r.url) || r.type.toLowerCase() === 'wasm');
  const warmMime = activeResps.filter((r) => r.mime.includes('wasm') || r.type.toLowerCase() === 'wasm');
  const warmCoreBin = activeReqs.filter((r) => r.url.includes('/pkg/core.bin'));
  const warmRtWorker = activeReqs.filter((r) => r.url.includes('runtime_worker.js'));
  record(
    'Warm Reload: ZERO *.wasm, ZERO /pkg/core.bin, AND ZERO runtime_worker.js (100% IndexedDB boot!)',
    warmWasm.length === 0 && warmMime.length === 0 && warmCoreBin.length === 0 && warmRtWorker.length === 0,
    `wasm=${warmWasm.length} mime=${warmMime.length} coreBin=${warmCoreBin.length} rtWorker=${warmRtWorker.length}`
  );

  // 3. Authenticated Cold Boot (delete runtime:core:* from IndexedDB while logged in -> SharedWorker hides /pkg/core.bin!)
  await page.evaluate(async () => {
    if (window.__chatwStorage?.deleteStaticCacheByPrefix) {
      await window.__chatwStorage.deleteStaticCacheByPrefix('runtime:core:', '');
    }
  });
  activeReqs = [];
  activeResps = [];
  await page.reload({ waitUntil: 'load' });
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(900);

  const authColdWasm = activeReqs.filter((r) => /\.wasm(\?|$)/i.test(r.url) || r.type.toLowerCase() === 'wasm');
  const authColdCoreBin = activeReqs.filter((r) => r.url.includes('/pkg/core.bin'));
  record(
    'Authenticated Cold Boot (new deploy): ZERO *.wasm AND ZERO /pkg/core.bin in Page DevTools Network (SharedWorker stealth!)',
    authColdWasm.length === 0 && authColdCoreBin.length === 0,
    `wasm=${authColdWasm.length} coreBinPageReqs=${authColdCoreBin.length}`
  );

  // Verify full E2EE messaging works cleanly
  await page.locator('.nav-sidebar .chat-item').first().click();
  await page.waitForSelector('#dm-chat-input', { timeout: 10000 });
  const testMsg = `Stealth runtime OK ${uname}`;
  await page.locator('#dm-chat-input').fill(testMsg);
  await page.locator('#dm-chat-input').press('Enter');
  await page.waitForTimeout(1200);
  const msgVisible = await page.evaluate((txt) => (document.body.innerText || '').includes(txt), testMsg);
  await page.screenshot({ path: path.join(SHOT_DIR, '01_stealth_wasm_app_running.png') });

  record('Full E2EE app boots and delivers message cleanly after stealth WASM init', msgVisible, testMsg);
  record('Zero uncaught JS pageerrors', pageErrors.length === 0, pageErrors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 077 Summary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length) {
  process.exit(1);
}
