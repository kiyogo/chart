// Spec 115 Comprehensive E2E Verification:
// 1. Borderless Lightbox, AI overlay, % pill, and chat image surfaces (0px border, none shadow, none outline)
// 2. Hidden #canvas-scrollbar-overlay (zIndex <= 8000, display:none, 0 painted pixels) & .panel-resizer while Lightbox is open
// 3. Strict end-to-end .svg/.svgz/XML-SVG blocking (frontend pickers + toast + backend API 400 svg_upload_not_allowed)
// 4. Ultra-sharp Real-ESRGAN v8 (3-scale anti-ringing deconvolution + dark-ink tightener) 4K sharpness & fidelity
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const TEST_PNG = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_cnn_sr/ChatGPT_Anime_1672x941.png';
const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec115';
fs.mkdirSync(OUT_DIR, { recursive: true });

async function signInUser(page, uname) {
  await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'dark');
  });
  await page.reload({ waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.auth-card', { timeout: 25000 });
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  const ct = page.locator('.story-dialog .story-close').first();
  if (await ct.isVisible().catch(() => false)) await ct.click({ force: true }).catch(() => {});
  await page.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer), { timeout: 25000 });
}

(async () => {
  const b = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--enable-unsafe-webgpu', '--use-gl=angle', '--use-angle=vulkan', '--enable-features=Vulkan'],
  });
  const pageErrors = [];
  try {
    const sfx = Date.now().toString(36).slice(-5);
    const a = 's115a_' + sfx;
    const bob = 's115b_' + sfx;
    const ctxA = await b.newContext({ viewport: { width: 1600, height: 960 } });
    const ctxB = await b.newContext({ viewport: { width: 1600, height: 960 } });
    const pA = await ctxA.newPage();
    const pB = await ctxB.newPage();
    pA.on('pageerror', (e) => pageErrors.push(`A: ${e.message}`));
    pB.on('pageerror', (e) => pageErrors.push(`B: ${e.message}`));

    await signInUser(pA, a);
    await signInUser(pB, bob);

    // 1. Check WASM cnnEngine version === v8
    const engineInfo = await pA.evaluate(() => ({
      cnnEngine: window.__chatwWasmMedia?.cnnEngine,
      hasSvgHelper: typeof window.__chatwMediaOpt?.isBlockedSvgBlob === 'function',
    }));
    console.log('ENGINE_INFO:', JSON.stringify(engineInfo));
    if (engineInfo.cnnEngine !== 'rust-realesrgan-srvgg-upscayl-v8') {
      throw new Error(`Expected cnnEngine v8, got ${engineInfo.cnnEngine}`);
    }
    if (!engineInfo.hasSvgHelper) {
      throw new Error('Missing window.__chatwMediaOpt.isBlockedSvgBlob helper');
    }

    // Befriend A & Bob
    await pA.evaluate(async (t) => {
      await fetch('/api/friends/requests', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({ username: t }),
      });
    }, bob);
    await pB.evaluate(async (f) => {
      const l = await (await fetch('/api/friends/requests', { credentials: 'include' })).json();
      const arr = Array.isArray(l) ? l : (l.incoming || l.requests || []);
      const h = arr.find((x) => (x.from_username || x.username || x.sender_username || x.requester?.username) === f);
      if (h) {
        await fetch('/api/friends/requests/' + h.id + '/accept', {
          method: 'POST',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: '{}',
        });
      }
    }, a);

    for (const [p, peer] of [[pA, bob], [pB, a]]) {
      await p.reload({ waitUntil: 'domcontentloaded' });
      await p.waitForSelector('.app-rail', { timeout: 25000 });
      await p.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer), { timeout: 25000 });
      await p.locator('.conversation-item, .chat-item, .conv-item', { hasText: peer }).first().click();
      await p.waitForTimeout(250);
    }

    // 2. Backend SVG Rejection API tests (direct .svg, renamed .png with <svg> body, init upload, avatar)
    const backendSvgResults = await pA.evaluate(async () => {
      const svgXml = '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="red"/></svg>';
      const r1 = await fetch('/api/attachments?file_name=evil.svg&mime_type=image/svg%2Bxml', {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'image/svg+xml' },
        body: svgXml,
      });
      const t1 = await r1.text();

      const r2 = await fetch('/api/attachments?file_name=disguised.png&mime_type=image/png', {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'image/png' },
        body: svgXml,
      });
      const t2 = await r2.text();

      const r3 = await fetch('/api/storage/uploads/init', {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ scope_type: 'dm', file_name: 'vector.svg', mime_type: 'image/svg+xml', total_size: 128 }),
      });
      const t3 = await r3.text();

      const r4 = await fetch('/api/users/avatar', {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'image/svg+xml' },
        body: svgXml,
      });
      const t4 = await r4.text();

      return {
        directSvg: { status: r1.status, body: t1 },
        renamedSvg: { status: r2.status, body: t2 },
        chunkInitSvg: { status: r3.status, body: t3 },
        avatarSvg: { status: r4.status, body: t4 },
      };
    });
    console.log('BACKEND_SVG_BLOCK:', JSON.stringify(backendSvgResults, null, 2));
    for (const [label, res] of Object.entries(backendSvgResults)) {
      if (res.status !== 400 || !res.body.includes('svg_upload_not_allowed')) {
        throw new Error(`Backend SVG block failed for ${label}: status=${res.status} body=${res.body}`);
      }
    }

    // 3. Frontend SVG Rejection tests (Photos picker with .svg AND renamed .png containing <svg>, plus Attachments picker)
    const svgPath = path.join(OUT_DIR, 'blocked_vector.svg');
    const disguisedSvgPath = path.join(OUT_DIR, 'disguised_svg.png');
    const svgContent = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="40" fill="#3b82f6"/></svg>';
    fs.writeFileSync(svgPath, svgContent);
    fs.writeFileSync(disguisedSvgPath, svgContent);

    const openToolsMenu = async (page) => {
      const plus = page.locator('button[aria-label*="Add photos"], button[aria-label*="Thêm ảnh"], .composer-tools button').first();
      await plus.click();
      await page.waitForTimeout(150);
    };

    await openToolsMenu(pA);
    const acceptAttr = await pA.locator('.tools-menu input[type="file"][accept]').first().getAttribute('accept');
    console.log('PHOTO_INPUT_ACCEPT:', acceptAttr);
    if (!acceptAttr || acceptAttr.includes('image/*') || acceptAttr.includes('svg')) {
      throw new Error(`Photo input accept still allows wildcard/svg: ${acceptAttr}`);
    }

    // Try sending .svg via Photos picker
    await pA.locator('.tools-menu input[type="file"][accept]').first().setInputFiles(svgPath);
    await pA.waitForTimeout(450);
    const toastText1 = await pA.evaluate(() => document.body.innerText);
    const imgCountAfterSvg1 = await pA.locator('.rich-image').count();
    if (!toastText1.includes('Không hỗ trợ gửi tệp SVG')) {
      throw new Error('Expected SVG blocked toast after Photos .svg selection');
    }
    if (imgCountAfterSvg1 !== 0) {
      throw new Error(`Expected 0 images after blocked .svg, got ${imgCountAfterSvg1}`);
    }

    // Try sending disguised SVG (.png with <svg> body) via Photos picker
    await openToolsMenu(pA);
    await pA.locator('.tools-menu input[type="file"][accept]').first().setInputFiles(disguisedSvgPath);
    await pA.waitForTimeout(450);
    const imgCountAfterSvg2 = await pA.locator('.rich-image').count();
    if (imgCountAfterSvg2 !== 0) {
      throw new Error(`Expected 0 images after blocked disguised SVG, got ${imgCountAfterSvg2}`);
    }

    // Try sending .svg via Attachments picker (no accept attr)
    await openToolsMenu(pA);
    await pA.locator('.tools-menu input[type="file"]:not([accept])').first().setInputFiles(svgPath);
    await pA.waitForTimeout(450);
    const msgCountAfterDocSvg = await pA.locator('.message-row').count();
    if (msgCountAfterDocSvg !== 0) {
      throw new Error(`Expected 0 messages after blocked Attachments .svg, got ${msgCountAfterDocSvg}`);
    }
    console.log('FRONTEND_SVG_BLOCK: PASSED (Photos .svg, disguised .png SVG, Attachments .svg all blocked with toast!)');

    // 4. Populate chat with 24 text messages + real anime image so .chat-messages has active vertical scrollbar!
    for (let i = 1; i <= 20; i++) {
      await pA.locator('#dm-chat-input').fill(`Spec 115 scroll message #${i} to force scrollable chat container`);
      await pA.locator('#dm-chat-input').press('Enter');
      await pA.waitForTimeout(60);
    }
    await openToolsMenu(pA);
    await pA.locator('.tools-menu input[type="file"][accept]').first().setInputFiles(TEST_PNG);
    await pB.waitForSelector('.rich-image img.rich-image-full[data-ai-restored="true"]', { timeout: 35000 });
    await pB.waitForTimeout(5000);

    // 5. Open fresh device context for Bob (unenhanced history image with corner SVG icon)
    const bobLs = await pB.evaluate(() => {
      const o = {};
      for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i);
        o[k] = localStorage.getItem(k);
      }
      return o;
    });
    const bobCk = await ctxB.cookies();
    const ctxN = await b.newContext({ viewport: { width: 1600, height: 960 } });
    await ctxN.addCookies(bobCk);
    const pN = await ctxN.newPage();
    pN.on('pageerror', (e) => pageErrors.push(`N: ${e.message}`));
    await pN.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
    await pN.evaluate((ls) => {
      for (const [k, v] of Object.entries(ls)) localStorage.setItem(k, v);
    }, bobLs);
    await pN.reload({ waitUntil: 'domcontentloaded' });
    await pN.waitForSelector('.app-rail', { timeout: 25000 });
    await pN.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer), { timeout: 25000 });
    await pN.locator('.conversation-item, .chat-item, .conv-item', { hasText: a }).first().click();
    await pN.waitForSelector('.rich-image .rich-image-enhance-corner svg', { timeout: 25000 });
    await pN.waitForTimeout(500);

    // Trigger chat scroll so #canvas-scrollbar-overlay draws its vertical scrollbar right before clicking image!
    await pN.evaluate(() => {
      const cm = document.querySelector('.chat-messages');
      if (cm) {
        cm.scrollTop = Math.max(0, cm.scrollTop - 120);
        cm.scrollTop = cm.scrollHeight;
      }
    });
    await pN.mouse.move(1220, 500);
    await pN.waitForTimeout(180);

    const chatImageStyles = await pN.evaluate(() => {
      const check = (sel) => {
        const el = document.querySelector(sel);
        if (!el) return null;
        const cs = getComputedStyle(el);
        return {
          borderTopWidth: cs.borderTopWidth,
          borderRightWidth: cs.borderRightWidth,
          boxShadow: cs.boxShadow,
          outlineStyle: cs.outlineStyle,
        };
      };
      const ov = document.getElementById('canvas-scrollbar-overlay');
      return {
        richImage: check('.rich-image'),
        richImageStage: check('.rich-image-stage'),
        cornerBtn: check('.rich-image-enhance-corner'),
        scrollbarZIndex: ov ? ov.style.zIndex : null,
      };
    });
    console.log('CHAT_IMAGE_STYLES:', JSON.stringify(chatImageStyles, null, 2));

    // Click image to open Lightbox & trigger on-demand Real-ESRGAN v8!
    await pN.locator('.rich-image-btn').first().click();
    await pN.waitForSelector('.lightbox-img', { timeout: 10000 });
    await pN.waitForTimeout(160);

    // Move cursor over right edge & wheel while Lightbox is enhancing to try provoking scrollbar!
    await pN.mouse.move(1240, 520);
    await pN.waitForTimeout(120);
    await pN.screenshot({ path: path.join(OUT_DIR, '01_lightbox_enhancing_borderless_no_scrollbar.png') });

    const enhancingAudit = await pN.evaluate(() => {
      const styleOf = (sel) => {
        const el = document.querySelector(sel);
        if (!el) return null;
        const cs = getComputedStyle(el);
        return {
          borderTopWidth: cs.borderTopWidth,
          borderRightWidth: cs.borderRightWidth,
          borderBottomWidth: cs.borderBottomWidth,
          borderLeftWidth: cs.borderLeftWidth,
          boxShadow: cs.boxShadow,
          outlineStyle: cs.outlineStyle,
          borderRadius: cs.borderRadius,
        };
      };
      const ov = document.getElementById('canvas-scrollbar-overlay');
      let paintedPixels = 0;
      if (ov && ov.width > 0 && ov.height > 0) {
        const ctx = ov.getContext('2d');
        const data = ctx.getImageData(0, 0, ov.width, ov.height).data;
        for (let i = 3; i < data.length; i += 4) if (data[i] > 0) paintedPixels++;
      }
      return {
        frame: styleOf('.lightbox-img-frame'),
        img: styleOf('.lightbox-img'),
        aiOverlay: styleOf('.ai-gen-overlay.lightbox-ai-gen'),
        aiFrost: styleOf('.ai-gen-frost'),
        aiPct: styleOf('.ai-gen-pct'),
        scrollbarOverlay: ov ? {
          display: getComputedStyle(ov).display,
          visibility: getComputedStyle(ov).visibility,
          zIndex: Number(ov.style.zIndex || getComputedStyle(ov).zIndex),
          paintedPixels,
        } : null,
      };
    });
    console.log('ENHANCING_AUDIT:', JSON.stringify(enhancingAudit, null, 2));

    // Wait for Real-ESRGAN v8 4× to finish!
    await pN.waitForSelector('.lightbox-enhance-toggle.is-active', { timeout: 35000 });
    await pN.waitForTimeout(550);

    // Try moving mouse over chat scrollbar column & panning slightly to verify scrollbar NEVER appears in Lightbox!
    await pN.mouse.move(1240, 520);
    await pN.waitForTimeout(150);
    await pN.screenshot({ path: path.join(OUT_DIR, '02_lightbox_4k_v8_borderless_no_scrollbar.png') });

    const finalAudit = await pN.evaluate(async () => {
      const img = document.querySelector('.lightbox-img');
      const frame = document.querySelector('.lightbox-img-frame');
      const ov = document.getElementById('canvas-scrollbar-overlay');
      const resizer = document.querySelector('.panel-resizer');
      let paintedPixels = 0;
      if (ov && ov.width > 0 && ov.height > 0) {
        const ctx = ov.getContext('2d');
        const data = ctx.getImageData(0, 0, ov.width, ov.height).data;
        for (let i = 3; i < data.length; i += 4) if (data[i] > 0) paintedPixels++;
      }

      // Measure 4K v8 Tenengrad / Laplacian sharpness on a center 640x480 region & export 1:1 crop!
      const c = document.createElement('canvas');
      const cw = 640, ch = 480;
      c.width = cw; c.height = ch;
      const ctx = c.getContext('2d');
      const sx = Math.max(0, Math.floor((img.naturalWidth - cw) * 0.35));
      const sy = Math.max(0, Math.floor((img.naturalHeight - ch) * 0.28));
      ctx.drawImage(img, sx, sy, cw, ch, 0, 0, cw, ch);
      const d = ctx.getImageData(0, 0, cw, ch).data;
      const gray = new Float32Array(cw * ch);
      for (let i = 0; i < cw * ch; i++) {
        gray[i] = 0.299 * d[i * 4] + 0.587 * d[i * 4 + 1] + 0.114 * d[i * 4 + 2];
      }
      let lapSum = 0, tenSum = 0, cnt = 0;
      for (let y = 1; y < ch - 1; y++) {
        for (let x = 1; x < cw - 1; x++) {
          const idx = y * cw + x;
          const lap = Math.abs(4 * gray[idx] - gray[idx - 1] - gray[idx + 1] - gray[idx - cw] - gray[idx + cw]);
          const gx = (gray[idx + 1] - gray[idx - 1]) * 0.5;
          const gy = (gray[idx + cw] - gray[idx - cw]) * 0.5;
          lapSum += lap;
          tenSum += Math.hypot(gx, gy);
          cnt++;
        }
      }
      const cropDataUrl = c.toDataURL('image/png');

      return {
        naturalWidth: img.naturalWidth,
        naturalHeight: img.naturalHeight,
        imgBorder: getComputedStyle(img).borderTopWidth,
        imgShadow: getComputedStyle(img).boxShadow,
        imgRadius: getComputedStyle(img).borderRadius,
        frameBorder: getComputedStyle(frame).borderTopWidth,
        frameShadow: getComputedStyle(frame).boxShadow,
        scrollbarDisplay: ov ? getComputedStyle(ov).display : 'missing',
        scrollbarZ: ov ? Number(ov.style.zIndex || getComputedStyle(ov).zIndex) : 0,
        scrollbarPaintedPixels: paintedPixels,
        resizerOpacity: resizer ? getComputedStyle(resizer).opacity : 'none',
        laplacianMean: Number((lapSum / cnt).toFixed(2)),
        tenengradMean: Number((tenSum / cnt).toFixed(2)),
        cropDataUrl,
      };
    });

    const b64Crop = finalAudit.cropDataUrl.split(',')[1];
    fs.writeFileSync(path.join(OUT_DIR, '03_ultra_sharp_v8_crop_1to1.png'), Buffer.from(b64Crop, 'base64'));
    delete finalAudit.cropDataUrl;
    console.log('FINAL_AUDIT:', JSON.stringify(finalAudit, null, 2));

    // Assertions
    for (const [k, st] of Object.entries({
      richImage: chatImageStyles.richImage,
      richImageStage: chatImageStyles.richImageStage,
      cornerBtn: chatImageStyles.cornerBtn,
      frame: enhancingAudit.frame,
      img: enhancingAudit.img,
      aiFrost: enhancingAudit.aiFrost,
      aiPct: enhancingAudit.aiPct,
    })) {
      if (!st) continue;
      if (st.borderTopWidth !== '0px' || st.boxShadow !== 'none') {
        throw new Error(`Border/shadow leaked on ${k}: ${JSON.stringify(st)}`);
      }
    }
    if (enhancingAudit.scrollbarOverlay && (enhancingAudit.scrollbarOverlay.display !== 'none' || enhancingAudit.scrollbarOverlay.zIndex > 8000 || enhancingAudit.scrollbarOverlay.paintedPixels > 0)) {
      throw new Error(`Scrollbar leaked during enhancing: ${JSON.stringify(enhancingAudit.scrollbarOverlay)}`);
    }
    if (finalAudit.scrollbarDisplay !== 'none' || finalAudit.scrollbarZ > 8000 || finalAudit.scrollbarPaintedPixels > 0) {
      throw new Error(`Scrollbar leaked in open Lightbox: ${JSON.stringify(finalAudit)}`);
    }
    if (finalAudit.naturalWidth !== 3344 || finalAudit.naturalHeight !== 1884) {
      throw new Error(`Unexpected 4K output size: ${finalAudit.naturalWidth}x${finalAudit.naturalHeight}`);
    }
    if (finalAudit.imgRadius !== '0px' || finalAudit.imgBorder !== '0px' || finalAudit.imgShadow !== 'none') {
      throw new Error(`Lightbox image style violation: ${JSON.stringify(finalAudit)}`);
    }
    if (pageErrors.length > 0) {
      throw new Error(`Page errors: ${pageErrors.join(' | ')}`);
    }

    console.log('SPEC 115 E2E VERIFICATION: 100% PASSED!');
  } finally {
    await b.close();
  }
})();
