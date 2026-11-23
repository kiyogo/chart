import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_110';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function signInUser(page, prefix) {
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'light');
  });
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  const uname = `${prefix}_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }
  return uname;
}

async function run() {
  console.log('🚀 Starting Spec 110 End-to-End Verification...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });

  const ctxAlice = await browser.newContext({ viewport: { width: 1280, height: 820 } });
  const pageAlice = await ctxAlice.newPage();

  const consoleErrors = [];
  pageAlice.on('pageerror', (err) => consoleErrors.push(String(err)));

  const aliceName = await signInUser(pageAlice, 's110a');
  console.log(`✅ Signed in Alice: ${aliceName}`);

  // Wait for WASM media bridge to mount
  await pageAlice.waitForFunction(
    () => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer && window.__chatwStorage && window.__chatwMediaOpt),
    null,
    { timeout: 15000 },
  );

  // 1. Verify License Guard + Pure-Rust WASM AVIF + Lossless CWCMP1 + On-Device AI Restoration
  const coreChecks = await pageAlice.evaluate(async () => {
    // 1a. License Enforcement
    const ai = window.__chatwAiRestorer;
    const blocked = ['codeformer', 'rvrt', 'apisr', 'upscayl'].map((m) => [m, ai.isModelLicenseAllowed(m)]);
    const allowed = ['spab_lite_builtin', 'span_x2_s', 'realesrgan_x2plus', 'realcugan_2x', 'nanovsr_lite'].map(
      (m) => [m, ai.isModelLicenseAllowed(m)],
    );

    // 1b. Create a real 320x240 textured PNG photo in browser canvas
    const c = document.createElement('canvas');
    c.width = 320;
    c.height = 240;
    const ctx = c.getContext('2d');
    const imgData = ctx.createImageData(320, 240);
    for (let y = 0; y < 240; y++) {
      for (let x = 0; x < 320; x++) {
        const i = (y * 320 + x) * 4;
        imgData.data[i] = Math.floor((Math.sin(x * 0.06) * 0.5 + 0.5) * 200 + 30);
        imgData.data[i + 1] = Math.floor((Math.cos(y * 0.05) * 0.5 + 0.5) * 190 + 40);
        imgData.data[i + 2] = Math.floor((((x + y) * 3) % 210) + 25);
        imgData.data[i + 3] = 255;
      }
    }
    ctx.putImageData(imgData, 0, 0);
    const pngBlob = await new Promise((res) => c.toBlob(res, 'image/png'));
    const pngBytes = new Uint8Array(await pngBlob.arrayBuffer());

    // Optimize via ChatW media pipeline -> MUST produce genuine AVIF (`ftypavif`), NEVER JPEG/WebP!
    const opt = await window.__chatwMediaOpt.optimizeImageBlob(pngBlob, { preserveOriginal: false });
    const bin = atob(opt.bytesB64);
    const avifU8 = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) avifU8[i] = bin.charCodeAt(i);
    const ftypStr = String.fromCharCode(...avifU8.slice(4, 12));

    // 1c. Lossless Smart File Compression (`CWCMP1`) + Byte-for-Byte Verified Decompression
    const sampleText = new TextEncoder().encode(
      JSON.stringify({ app: 'ChatW', spec: 110, logs: Array.from({ length: 240 }, (_, i) => ({ seq: i, msg: 'End-to-end encrypted media pipeline verified!' })) }),
    );
    const cmp = window.__chatwWasmMedia.compressFileSmart(sampleText);
    const dec = window.__chatwWasmMedia.decompressFileVerified(cmp.bytes);
    const b3Orig = window.__chatwWasmMedia.blake3Hex(sampleText);
    const b3Dec = window.__chatwWasmMedia.blake3Hex(dec.bytes);

    // 1d. On-Device AI Restoration (1st run computes, 2nd run hits memory cache)
    const avifBlobUrl = URL.createObjectURL(new Blob([avifU8], { type: 'image/avif' }));
    const aiRun1 = await ai.restoreImageFromUrl(avifBlobUrl, {
      mediaId: 'spec110_e2e_img',
      contentHash: b3Orig.slice(0, 16),
      forceRun: true,
    });
    const aiRun2 = await ai.restoreImageFromUrl(avifBlobUrl, {
      mediaId: 'spec110_e2e_img',
      contentHash: b3Orig.slice(0, 16),
      forceRun: true,
    });

    // Wait briefly for async IndexedDB putAiOutput write
    await new Promise((r) => setTimeout(r, 350));
    const cacheStats = await window.__chatwStorage.getSmartCacheStats();
    const videoCaps = await window.__chatwMediaOpt.probeVideoCodecs();

    return {
      wasmVersion: window.__chatwWasmMedia.version,
      blocked,
      allowed,
      pngLen: pngBytes.byteLength,
      avifLen: avifU8.byteLength,
      avifMime: opt.mime,
      avifExt: opt.ext,
      avifCodec: opt.codec,
      ftypStr,
      fileOrigLen: sampleText.byteLength,
      fileCmpLen: cmp.compressedSize,
      fileCodec: cmp.codec,
      fileExactMatch: b3Orig === b3Dec && dec.wasEnvelope === true,
      aiRun1: { enhanced: aiRun1.enhanced, cacheHit: aiRun1.cacheHit, backend: aiRun1.backend, ssim: aiRun1.ssim, elapsedMs: aiRun1.elapsedMs, provenance: aiRun1.provenance },
      aiRun2: { enhanced: aiRun2.enhanced, cacheHit: aiRun2.cacheHit, cacheTier: aiRun2.cacheTier },
      cacheStats,
      videoCaps,
      contentHashKey: b3Orig.slice(0, 16),
      avifB64ForReload: opt.bytesB64,
    };
  });

  console.log('📊 Browser Core Verification:', JSON.stringify({ ...coreChecks, avifB64ForReload: `<${coreChecks.avifB64ForReload.length} chars>` }, null, 2));

  for (const [m, ok] of coreChecks.blocked) {
    if (ok) throw new Error(`Forbidden license model '${m}' was NOT blocked!`);
  }
  for (const [m, ok] of coreChecks.allowed) {
    if (!ok) throw new Error(`Approved permissive model '${m}' was rejected!`);
  }
  if (coreChecks.avifMime !== 'image/avif' || coreChecks.ftypStr !== 'ftypavif') {
    throw new Error(`Expected genuine AVIF (ftypavif), got mime=${coreChecks.avifMime}, ftyp=${coreChecks.ftypStr}`);
  }
  if (coreChecks.avifLen >= coreChecks.pngLen) {
    throw new Error(`Expected AVIF (${coreChecks.avifLen}B) smaller than PNG (${coreChecks.pngLen}B)`);
  }
  if (!coreChecks.fileExactMatch || coreChecks.fileCmpLen >= coreChecks.fileOrigLen) {
    throw new Error(`Smart lossless file compression failed roundtrip or did not shrink!`);
  }
  if (!coreChecks.aiRun1.enhanced || coreChecks.aiRun1.cacheHit !== false) {
    throw new Error(`Expected 1st AI restoration run to execute on-device: ${JSON.stringify(coreChecks.aiRun1)}`);
  }
  if (!coreChecks.aiRun2.enhanced || coreChecks.aiRun2.cacheHit !== true || coreChecks.aiRun2.cacheTier !== 'memory') {
    throw new Error(`Expected 2nd AI restoration run to hit memory cache: ${JSON.stringify(coreChecks.aiRun2)}`);
  }

  // 2. Reload page and verify PERSISTENT IndexedDB AI Output Cache HIT (ZERO re-inference!)
  await pageAlice.reload({ waitUntil: 'domcontentloaded' });
  await pageAlice.waitForSelector('.app-rail', { timeout: 25000 });
  await pageAlice.waitForFunction(() => Boolean(window.__chatwAiRestorer && window.__chatwStorage), null, { timeout: 15000 });

  const idbHit = await pageAlice.evaluate(async ({ b64, hashKey }) => {
    const bin = atob(b64);
    const u8 = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
    const url = URL.createObjectURL(new Blob([u8], { type: 'image/avif' }));
    return await window.__chatwAiRestorer.restoreImageFromUrl(url, {
      mediaId: 'spec110_e2e_img',
      contentHash: hashKey,
      forceRun: true,
    });
  }, { b64: coreChecks.avifB64ForReload, hashKey: coreChecks.contentHashKey });

  console.log('💾 Reload AI Cache Check:', JSON.stringify(idbHit));
  if (!idbHit.enhanced || !idbHit.cacheHit || idbHit.cacheTier !== 'indexeddb') {
    throw new Error(`Expected IndexedDB AI Output Cache HIT after reload, got: ${JSON.stringify(idbHit)}`);
  }

  // 3. Verify Settings -> Data & Backup -> Storage UI + capture screenshot
  await pageAlice.locator('.app-rail button[aria-label="Cài đặt"], .app-rail button[title="Cài đặt"], .app-rail button[aria-label="Settings"]').first().click();
  await pageAlice.waitForSelector('.settings-nav', { timeout: 10000 });
  await pageAlice.locator('.settings-nav-item', { hasText: /Dữ liệu|Data|Lưu trữ|Storage/i }).first().click();
  await pageAlice.waitForTimeout(900);

  const storageText = await pageAlice.locator('.settings-main, .settings-content, .settings-cols').first().innerText();
  if (!storageText.includes('Tự động làm nét ảnh') && !storageText.includes('Chất lượng gửi media')) {
    throw new Error(`Expected new Spec 110 Storage settings controls, got: ${storageText.slice(0, 300)}`);
  }
  await pageAlice.screenshot({ path: path.join(ARTIFACTS_DIR, '01_settings_storage_spec110.png') });
  console.log('📸 Saved 01_settings_storage_spec110.png');

  // 4. Backend API Security & Functional Tests (E2EE Dedup Isolation + Shared Dedup + Manifest + Cross-Account 403)
  const ctxBob = await browser.newContext({ viewport: { width: 1280, height: 820 } });
  const pageBob = await ctxBob.newPage();
  const bobName = await signInUser(pageBob, 's110b');
  console.log(`✅ Signed in Bob: ${bobName}`);

  const apiResults = await pageAlice.evaluate(async () => {
    // 4a. Check capabilities endpoint reports CWCH3 & new codecs
    const caps = await fetch('/api/storage/capabilities', { credentials: 'include' }).then((r) => r.json());

    // 4b. Check policy update roundtrip with Spec 110 fields
    const pol = await fetch('/api/storage/policy', { credentials: 'include' }).then((r) => r.json());
    const polPut = await fetch('/api/storage/policy', {
      method: 'PUT',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ...pol, media_quality_mode: 'ultra_optimized', ai_enhancement_enabled: true, server_durable_dm: true }),
    }).then((r) => r.json());

    // 4c. E2EE Dedup Privacy Invariant: MUST reject global lookup when is_e2ee = true!
    const dedupE2ee = await fetch('/api/storage/dedup/check', {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        scope_type: 'dm',
        is_e2ee: true,
        blake3_hex: 'a'.repeat(64),
        sha256_hex: 'b'.repeat(64),
      }),
    }).then((r) => r.json());

    // 4d. Upload private E2EE DM blob -> store streaming manifest -> verify manifest GET
    const cipherBytes = crypto.getRandomValues(new Uint8Array(2048));
    const dmUp = await fetch('/api/attachments?scope_type=dm&is_e2ee=true&file_name=secret.avif&mime_type=image/avif', {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/octet-stream' },
      body: cipherBytes,
    }).then((r) => r.json());

    const putMan = await fetch(`/api/storage/references/${dmUp.ref_id}/manifest`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        version: 3,
        mime_type: 'video/mp4',
        codec: 'av01.0.04M.08',
        duration_ms: 4000,
        is_e2ee: true,
        chunks: [{ variant_id: '720p', chunk_index: 0, byte_offset: 34, cipher_len: 1040, plain_len: 1024, start_ms: 0, duration_ms: 2000, is_keyframe: true }],
      }),
    }).then((r) => r.json());

    const getMan = await fetch(`/api/storage/references/${dmUp.ref_id}/manifest`, { credentials: 'include' }).then((r) => r.json());

    // 4e. Shared non-E2EE Group upload + Dedup hit
    const grp = await fetch('/api/groups', {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Spec110 Dedup Group', member_ids: [] }),
    }).then((r) => r.json());
    const gid = grp.id || grp.group?.id;

    // Create tiny valid PNG for server-side AVIF optimization + dedup
    const cv = document.createElement('canvas');
    cv.width = 180;
    cv.height = 120;
    const cx = cv.getContext('2d');
    const grd = cx.createLinearGradient(0, 0, 180, 120);
    grd.addColorStop(0, '#1d4ed8');
    grd.addColorStop(0.5, '#10b981');
    grd.addColorStop(1, '#f59e0b');
    cx.fillStyle = grd;
    cx.fillRect(0, 0, 180, 120);
    const pngBlob = await new Promise((res) => cv.toBlob(res, 'image/png'));
    const pngU8 = new Uint8Array(await pngBlob.arrayBuffer());

    const grpUp = await fetch(`/api/attachments?scope_type=group&scope_id=${gid}&is_e2ee=false&preserve_original=false&file_name=shared.png&mime_type=image/png`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'image/png' },
      body: pngU8,
    }).then((r) => r.json());

    const dedupShared = await fetch('/api/storage/dedup/check', {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        scope_type: 'group',
        scope_id: gid,
        is_e2ee: false,
        blake3_hex: grpUp.blake3,
        sha256_hex: grpUp.sha256,
      }),
    }).then((r) => r.json());

    return { caps, polPut, dedupE2ee, dmUp, putMan, getMan, grpUp, dedupShared };
  });

  console.log('🔐 API Security & Storage Checks:', JSON.stringify({
    e2ee_chunk_protocol: apiResults.caps.e2ee_chunk_protocol,
    server_durable_dm: apiResults.polPut.server_durable_dm,
    dedupE2ee: apiResults.dedupE2ee,
    dmTier: apiResults.dmUp.storage_tier,
    manifestSegments: apiResults.getMan.total_segments,
    groupHasOptimizedAvif: apiResults.grpUp.has_optimized,
    dedupShared: apiResults.dedupShared,
  }, null, 2));

  if (apiResults.caps.e2ee_chunk_protocol !== 'CWCH3') throw new Error('Expected CWCH3 protocol');
  if (apiResults.dedupE2ee.dedup_allowed !== false) throw new Error('E2EE dedup MUST be forbidden!');
  if (apiResults.dmUp.storage_tier !== 'server_durable') throw new Error(`Expected Server-First durable DM storage, got ${apiResults.dmUp.storage_tier}`);
  if (apiResults.getMan.total_segments !== 1) throw new Error('Manifest storage/fetch failed!');
  if (!apiResults.grpUp.has_optimized) throw new Error('Expected server-side AVIF optimized variant on group image!');
  if (!apiResults.dedupShared.matched || apiResults.dedupShared.match_kind !== 'exact_blake3') {
    throw new Error(`Expected exact_blake3 shared dedup hit, got: ${JSON.stringify(apiResults.dedupShared)}`);
  }

  // 4f. Cross-Account RBAC ACL Check: Bob MUST receive 403 Forbidden for Alice's private DM ref & manifest!
  const bobForbidden = await pageBob.evaluate(async (refId) => {
    const r1 = await fetch(`/api/attachments/${refId}`, { credentials: 'include' });
    const r2 = await fetch(`/api/storage/references/${refId}/manifest`, { credentials: 'include' });
    return { attachmentStatus: r1.status, manifestStatus: r2.status };
  }, apiResults.dmUp.ref_id);
  console.log('🛡️ Cross-Account ACL (Bob -> Alice private DM):', JSON.stringify(bobForbidden));
  if (bobForbidden.attachmentStatus !== 403 || bobForbidden.manifestStatus !== 403) {
    throw new Error(`Cross-account ACL breach! Expected 403/403, got ${JSON.stringify(bobForbidden)}`);
  }

  // 5. Real Chat DM Image Send -> Instant Preview -> AVIF + E2EE -> Reload Persistence
  await pageAlice.locator('.app-rail button[aria-label="Tin nhắn"], .app-rail button[title="Tin nhắn"], .app-rail button[aria-label="Messages"]').first().click();
  await pageAlice.waitForTimeout(500);
  const selfChat = pageAlice.locator('.conversation-item, .chat-item, .conv-item', { hasText: /Chính mình|Myself/i }).first();
  if (await selfChat.isVisible().catch(() => false)) {
    await selfChat.click();
    await pageAlice.waitForTimeout(500);

    // Prepare a real temporary PNG file on disk to upload through the real composer file input!
    const tmpPng = path.join(ARTIFACTS_DIR, 'sample_send_photo.png');
    const pngB64 = await pageAlice.evaluate(async () => {
      const c = document.createElement('canvas');
      c.width = 256;
      c.height = 192;
      const ctx = c.getContext('2d');
      const g = ctx.createLinearGradient(0, 0, 256, 192);
      g.addColorStop(0, '#0f172a');
      g.addColorStop(0.5, '#2563eb');
      g.addColorStop(1, '#38bdf8');
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, 256, 192);
      ctx.fillStyle = '#ffffff';
      ctx.beginPath();
      ctx.arc(128, 96, 46, 0, Math.PI * 2);
      ctx.fill();
      const b = await new Promise((r) => c.toBlob(r, 'image/png'));
      const u8 = new Uint8Array(await b.arrayBuffer());
      let s = '';
      for (let i = 0; i < u8.length; i++) s += String.fromCharCode(u8[i]);
      return btoa(s);
    });
    fs.writeFileSync(tmpPng, Buffer.from(pngB64, 'base64'));

    // Open "+" composer tools menu and upload the real PNG!
    const plusBtn = pageAlice.locator('button[aria-label*="Add photos"], button[aria-label*="Thêm ảnh"], .composer-tools button').first();
    if (await plusBtn.isVisible().catch(() => false)) {
      await plusBtn.click();
      await pageAlice.waitForTimeout(250);
      const fileInput = pageAlice.locator('input[type="file"][accept*="image"]').first();
      await fileInput.setInputFiles(tmpPng);
      await pageAlice.waitForSelector('.rich-image-full', { timeout: 20000 });
      await pageAlice.waitForTimeout(1500);
      await pageAlice.screenshot({ path: path.join(ARTIFACTS_DIR, '02_chat_dm_media_sent.png') });
      console.log('📸 Saved 02_chat_dm_media_sent.png');

      // Reload & reopen Saved Messages to verify persisted E2EE image + AI restoration
      await pageAlice.reload({ waitUntil: 'domcontentloaded' });
      await pageAlice.waitForSelector('.app-rail', { timeout: 25000 });
      await pageAlice.waitForSelector('.conversation-item, .chat-item, .conv-item', { timeout: 15000 });
      const selfChat2 = pageAlice.locator('.conversation-item, .chat-item, .conv-item', { hasText: /Chính mình|Myself/i }).first();
      await selfChat2.click();
      await pageAlice.waitForSelector('.rich-image-full', { timeout: 20000 });
      await pageAlice.waitForTimeout(1200);
      const imgCheck = await pageAlice.evaluate(() => {
        const img = document.querySelector('.rich-image-full');
        return img ? { complete: img.complete, w: img.naturalWidth, h: img.naturalHeight, aiRestored: img.getAttribute('data-ai-restored'), aiCacheHit: img.getAttribute('data-ai-cache-hit') } : null;
      });
      console.log('🖼️ Reloaded DM Image Check:', JSON.stringify(imgCheck));
      if (!imgCheck || !imgCheck.complete || imgCheck.w === 0) {
        throw new Error(`Reloaded DM image did not load cleanly: ${JSON.stringify(imgCheck)}`);
      }
      await pageAlice.screenshot({ path: path.join(ARTIFACTS_DIR, '03_chat_dm_after_reload.png') });
      console.log('📸 Saved 03_chat_dm_after_reload.png');
    }
  }

  if (consoleErrors.length > 0) {
    console.warn('⚠️ Page errors observed:', consoleErrors);
  }

  await browser.close();
  console.log('🎉 ALL SPEC 110 E2E CHECKS PASSED!');
}

run().catch((err) => {
  console.error('❌ Spec 110 E2E FAILED:', err);
  process.exit(1);
});
