// Automated Live Playwright E2E Verification for Spec 073:
// Reproduces exact user DevTools scenario (custom app wallpaper + translucent adaptive edge strips +
// avatar + E2EE chat image with micro-blur + re-renders + warm reload) and verifies:
// 1. ZERO `/uploads/*` or `/api/attachments/*` HTTP requests in Network tab
// 2. ZERO `data:image/*` entries in Network tab, DOM `<img>`, or CSS variables
// 3. 100% `blob:` URLs for wallpapers, avatars, edge strips & chat images
// 4. ZERO double-loading (exactly 1 `blob:` URL per unique asset; switching chats & re-rendering creates 0 new blobs!)
// 5. ZERO duplicate `logo.svg` / `chatw_app_bg.wasm` SW re-fetches on warm reload

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC073_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_073';
fs.mkdirSync(SHOT_DIR, { recursive: true });

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

// Valid 1x1 PNG & valid 8x8 colored PNG
const TINY_PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
  'base64'
);

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
  const ALICE = `s73_a_${tag}`;
  const BOB = `s73_b_${tag}`;

  const ctxA = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const ctxB = await browser.newContext({ viewport: { width: 1400, height: 900 } });

  const pageA = await loginFresh(ctxA, ALICE);
  const pageB = await loginFresh(ctxB, BOB);

  // Establish friendship + upload avatars + upload custom wallpaper
  const meA = await (await ctxA.request.get(`${BASE}/api/auth/me`)).json();
  const meB = await (await ctxB.request.get(`${BASE}/api/auth/me`)).json();
  const frResp = await ctxA.request.post(`${BASE}/api/friends/requests`, { data: { username: BOB } });
  const frJson = await frResp.json().catch(() => ({}));
  if (frJson?.id) {
    await ctxB.request.post(`${BASE}/api/friends/requests/${frJson.id}/accept`, { data: {} });
  }
  await ctxA.request.post(`${BASE}/api/users/avatar`, { headers: { 'Content-Type': 'image/png' }, data: TINY_PNG });
  await ctxB.request.post(`${BASE}/api/users/avatar`, { headers: { 'Content-Type': 'image/png' }, data: TINY_PNG });

  // Upload a real wallpaper image to /api/media/upload -> returns /uploads/media/...
  const wpResp = await ctxA.request.post(`${BASE}/api/media/upload?kind=wallpaper`, {
    headers: { 'Content-Type': 'image/png' },
    data: TINY_PNG,
  });
  const wpJson = await wpResp.json().catch(() => ({}));
  const wpUrl = wpJson.url || wpJson.file_url || '';

  // Configure Alice's prefs with custom /uploads/media/... wallpaper & translucent panels (triggers adaptive_surface edge strips!)
  await pageA.evaluate(({ uid, wpUrl }) => {
    const key = `chatw_prefs_${uid}`;
    let cur = {};
    try { cur = JSON.parse(localStorage.getItem(key) || '{}'); } catch (_) {}
    if (wpUrl) cur.app_wallpaper = wpUrl;
    cur.panel_opacity = 78;
    cur.panel_glass = true;
    localStorage.setItem(key, JSON.stringify(cur));
  }, { uid: meA.id, wpUrl });

  // Generate a real 320x200 colorful PNG (so the chat photo & full-viewport lightbox are visually verifiable!)
  const realPhotoB64 = await pageA.evaluate(() => {
    const c = document.createElement('canvas');
    c.width = 320;
    c.height = 200;
    const ctx = c.getContext('2d');
    const g = ctx.createLinearGradient(0, 0, 320, 200);
    g.addColorStop(0, '#3B82F6');
    g.addColorStop(0.5, '#EC4899');
    g.addColorStop(1, '#F59E0B');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, 320, 200);
    ctx.fillStyle = 'rgba(255,255,255,0.92)';
    ctx.beginPath();
    ctx.arc(160, 100, 56, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = '#1E293B';
    ctx.font = 'bold 24px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('ChatW', 160, 108);
    return c.toDataURL('image/png').split(',')[1];
  });
  const REAL_PHOTO_PNG = Buffer.from(realPhotoB64, 'base64');

  // Upload a real E2EE image attachment and send it with a micro-blur data URI embedded in message JSON!
  const imgKey = crypto.randomBytes(32);
  const imgNonce = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv('aes-256-gcm', imgKey, imgNonce);
  const encImg = Buffer.concat([cipher.update(REAL_PHOTO_PNG), cipher.final(), cipher.getAuthTag()]);
  const attResp = await ctxA.request.post(
    `${BASE}/api/attachments?scope_type=dm&scope_id=${meB.id}&is_e2ee=true&file_name=anime_test.png&mime_type=image%2Fpng`,
    { headers: { 'Content-Type': 'application/octet-stream' }, data: encImg }
  );
  const attJson = await attResp.json();
  const attUrl = attJson.download_url;
  const microBlur = `data:image/png;base64,${TINY_PNG.toString('base64')}`;

  // Reload Page A once to warm SW cache, open DM with Bob, and send the E2EE image message!
  await pageA.reload({ waitUntil: 'load' });
  await pageA.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA.waitForTimeout(900);
  await pageA.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA.waitForTimeout(400);

  // Send a rich E2EE image message with embedded blur_data_url through the real composer!
  const imgPayload = `img:${JSON.stringify({
    u: attUrl,
    k: imgKey.toString('base64url'),
    n: imgNonce.toString('base64url'),
    m: 'image/png',
    w: 240,
    h: 180,
    name: 'anime_test.png',
    blur: microBlur,
    scope: 'dm',
  })}`;
  await pageA.locator('#dm-chat-input').fill(imgPayload);
  await pageA.keyboard.press('Enter');
  await pageA.locator('.rich-image-full').first().waitFor({ timeout: 12000 });
  await pageA.waitForTimeout(800);

  // ===========================================================================
  // NOW START STRICT DEVTOOLS NETWORK & BLOB AUDIT ON WARM RELOAD!
  // ===========================================================================
  const rawUploadsReqs = [];
  const dataImageReqs = [];
  const apiFetchXhrReqs = [];
  const staticCounts = new Map();

  pageA.on('request', (req) => {
    const url = req.url();
    const rt = req.resourceType();
    if (url.startsWith('data:image/')) {
      dataImageReqs.push(url.slice(0, 60));
    }
    if (url.includes('/uploads/') || url.includes('/api/attachments/')) {
      rawUploadsReqs.push(`${rt} ${url}`);
    }
    if ((rt === 'fetch' || rt === 'xhr') && url.includes('/api/')) {
      apiFetchXhrReqs.push(`${req.method()} ${new URL(url).pathname}`);
    }
    if (url.includes('logo.svg') || url.includes('chatw_app_bg.wasm')) {
      const k = new URL(url).pathname;
      staticCounts.set(k, (staticCounts.get(k) || 0) + 1);
    }
  });

  // Track exact URL.createObjectURL calls on the page after reload
  await pageA.addInitScript(() => {
    const orig = URL.createObjectURL.bind(URL);
    window.__blobCreations = [];
    URL.createObjectURL = function trackedCreateObjectURL(obj) {
      const out = orig(obj);
      window.__blobCreations.push({
        url: out,
        type: obj && obj.type ? obj.type : '',
        size: obj && obj.size ? obj.size : 0,
      });
      return out;
    };
  });

  await pageA.reload({ waitUntil: 'load' });
  await pageA.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA.waitForTimeout(1000);

  await pageA.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA.locator('.rich-image-full').first().waitFor({ timeout: 12000 });
  await pageA.waitForTimeout(800);

  const imgSrcBefore = await pageA.locator('.rich-image-full').first().getAttribute('src');
  const poolSizeAfterOpen = await pageA.evaluate(() => window.__chatwCore.getBlobPoolSize());

  // Switch away to "Note to self" AND back to Bob + send a new text message (triggers re-render!)
  // Must NOT re-load or mint a 2nd blob: URL for the exact same chat image!
  await pageA.locator('.nav-sidebar .chat-item', { hasText: 'Note to self' }).first().click();
  await pageA.waitForTimeout(400);
  await pageA.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA.locator('.rich-image-full').first().waitFor({ timeout: 10000 });
  await pageA.locator('#dm-chat-input').fill(`Rerender check ${tag}`);
  await pageA.keyboard.press('Enter');
  await pageA.waitForTimeout(700);

  const imgSrcAfter = await pageA.locator('.rich-image-full').first().getAttribute('src');
  const domAudit = await pageA.evaluate(() => {
    const imgs = Array.from(document.querySelectorAll('img')).map((el) => ({
      cls: el.className,
      src: el.getAttribute('src') || '',
    }));
    const dataImgs = imgs.filter((i) => i.src.startsWith('data:image/'));
    const rawImgs = imgs.filter((i) => i.src.includes('/uploads/') || i.src.includes('/api/attachments/'));
    const blobImgs = imgs.filter((i) => i.src.startsWith('blob:'));
    const rootWp = document.documentElement.style.getPropertyValue('--app-wp-image') || '';
    const chatCol = document.querySelector('.chat-column');
    const topStrip = chatCol ? chatCol.style.getPropertyValue('--chat-top-fade-strip') : '';
    const botStrip = chatCol ? chatCol.style.getPropertyValue('--chat-bot-fade-strip') : '';
    return {
      totalImgs: imgs.length,
      blobImgCount: blobImgs.length,
      dataImgs,
      rawImgs,
      rootWp,
      topStrip,
      botStrip,
      blobPoolSize: window.__chatwCore.getBlobPoolSize(),
      blobCreations: window.__blobCreations || [],
    };
  });

  await pageA.screenshot({ path: path.join(SHOT_DIR, '01_custom_wallpaper_e2ee_image_blob_only.png') });

  // Spec 075: Click chat image -> verify full-viewport Lightbox (NOT trapped in .message-row) + non-overlapping controls + Escape close
  await pageA.locator('.rich-image-btn').first().click();
  await pageA.locator('.lightbox').first().waitFor({ timeout: 5000 });
  await pageA.waitForTimeout(250);
  const lbAudit = await pageA.evaluate(() => {
    const lb = document.querySelector('.lightbox');
    const dl = document.querySelector('.lightbox-actions .lightbox-download');
    const cl = document.querySelector('.lightbox-close');
    const r = lb ? lb.getBoundingClientRect() : null;
    const dr = dl ? dl.getBoundingClientRect() : null;
    const cr = cl ? cl.getBoundingClientRect() : null;
    return {
      vw: window.innerWidth,
      vh: window.innerHeight,
      rect: r ? { left: Math.round(r.left), top: Math.round(r.top), w: Math.round(r.width), h: Math.round(r.height) } : null,
      nonOverlapping: Boolean(dr && cr && dr.right <= cr.left + 1),
      dlRect: dr ? { left: Math.round(dr.left), right: Math.round(dr.right) } : null,
      clRect: cr ? { left: Math.round(cr.left), right: Math.round(cr.right) } : null,
    };
  });
  await pageA.screenshot({ path: path.join(SHOT_DIR, '02_fullscreen_lightbox.png') });
  await pageA.keyboard.press('Escape');
  await pageA.waitForTimeout(200);
  const lbClosed = (await pageA.locator('.lightbox').count()) === 0;
  record(
    '7. Spec 075: Clicking image opens full-viewport Lightbox (100vw x 100vh, not trapped in .message-row) with non-overlapping Download & Close controls and Escape closes it',
    Boolean(
      lbAudit.rect &&
      lbAudit.rect.left === 0 &&
      lbAudit.rect.top === 0 &&
      lbAudit.rect.w >= lbAudit.vw - 2 &&
      lbAudit.rect.h >= lbAudit.vh - 2 &&
      lbAudit.nonOverlapping &&
      lbClosed
    ),
    JSON.stringify({ ...lbAudit, lbClosed })
  );

  record(
    '1. ZERO /uploads/* or /api/attachments/* HTTP requests in Network tab (wallpaper + avatar + chat image all over SharedWorker)',
    rawUploadsReqs.length === 0,
    `rawUploadsReqs=${JSON.stringify(rawUploadsReqs)}`
  );
  record(
    '2. ZERO data:image/* entries in Network tab, DOM <img> tags, or CSS edge strips',
    dataImageReqs.length === 0 &&
      domAudit.dataImgs.length === 0 &&
      !domAudit.topStrip.includes('data:image/') &&
      !domAudit.botStrip.includes('data:image/'),
    `netData=${dataImageReqs.length} domData=${domAudit.dataImgs.length} topStrip="${domAudit.topStrip.slice(0, 45)}"`
  );
  record(
    '3. 100% of DOM images AND custom app wallpaper use blob: URLs (0 raw URLs)',
    domAudit.blobImgCount >= 2 &&
      domAudit.rawImgs.length === 0 &&
      (!wpUrl || domAudit.rootWp.includes('blob:')),
    `blobImgs=${domAudit.blobImgCount}/${domAudit.totalImgs} rawImgs=${domAudit.rawImgs.length} rootWp="${domAudit.rootWp.slice(0, 50)}"`
  );
  record(
    '4. ZERO double-loading of chat image: switching chats & re-rendering keeps the EXACT SAME single blob: URL',
    Boolean(imgSrcBefore && imgSrcBefore.startsWith('blob:') && imgSrcBefore === imgSrcAfter) &&
      domAudit.blobPoolSize === poolSizeAfterOpen,
    `before=${imgSrcBefore} after=${imgSrcAfter} poolBefore=${poolSizeAfterOpen} poolAfter=${domAudit.blobPoolSize}`
  );
  record(
    '5. ZERO duplicate logo.svg / chatw_app_bg.wasm SW re-fetches & ZERO /api/* Fetch/XHR',
    apiFetchXhrReqs.length === 0 &&
      (staticCounts.get('/logo.svg') || 0) <= 1 &&
      (staticCounts.get('/pkg/chatw_app_bg.wasm') || 0) <= 1,
    `apiFetchXhr=${apiFetchXhrReqs.length} staticCounts=${JSON.stringify(Object.fromEntries(staticCounts))}`
  );
  record('6. Zero uncaught page errors', pageErrors.length === 0, pageErrors.join('; ') || '0 errors');

  await ctxA.close();
  await ctxB.close();
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 073 Summary: ${checks.length - failed.length}/${checks.length} PASS`);
if (failed.length > 0) {
  console.error('FAILED:', failed);
  process.exit(1);
}
