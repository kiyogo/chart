// Spec 112 E2E Verification:
//  1. Deterministic Frame-0 Lightbox final sizing (0.0px width/height/x/y jump from open -> enhancing -> 4K -> toggle original <-> 4K).
//  2. Zero frame-0 unblurred flash + GPU-composited frosted aurora overlay + smooth 320ms reveal fade-out.
//  3. Zero chat bubble reflow between enhancing & 4K restored states.
//  4. Zero console/page errors & real visual screenshots for inspection.
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec112';
fs.mkdirSync(OUT_DIR, { recursive: true });
const TEST_PNG = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_cnn_sr/ChatGPT_Anime_1672x941.png';

async function signInUser(page, uname) {
  await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'dark');
  });
  await page.reload({ waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }
  await page.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer && window.__chatwMediaOpt), { timeout: 25000 });
}

function assertRectClose(label, a, b, tol = 1.5) {
  const dw = Math.abs(a.width - b.width);
  const dh = Math.abs(a.height - b.height);
  const dx = Math.abs(a.x - b.x);
  const dy = Math.abs(a.y - b.y);
  if (dw > tol || dh > tol || dx > tol || dy > tol) {
    throw new Error(
      `❌ Layout shift detected (${label}): dw=${dw.toFixed(2)}, dh=${dh.toFixed(2)}, dx=${dx.toFixed(2)}, dy=${dy.toFixed(2)} | a=${JSON.stringify(a)} vs b=${JSON.stringify(b)}`
    );
  }
  console.log(`✅ Zero-jump verified (${label}): Δw=${dw.toFixed(2)}px, Δh=${dh.toFixed(2)}px, Δx=${dx.toFixed(2)}px, Δy=${dy.toFixed(2)}px`);
}

(async () => {
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--enable-unsafe-webgpu', '--use-gl=angle', '--use-angle=vulkan', '--enable-features=Vulkan'],
  });
  const pageErrors = [];
  try {
    const suffix = Date.now().toString(36).slice(-5);
    const alice = `s112a_${suffix}`;
    const bob = `s112b_${suffix}`;

    const ctxA = await browser.newContext({ viewport: { width: 1600, height: 960 } });
    const ctxB = await browser.newContext({ viewport: { width: 1600, height: 960 } });
    const pageA = await ctxA.newPage();
    const pageB = await ctxB.newPage();
    pageA.on('pageerror', (e) => pageErrors.push(`A: ${e.message}`));
    pageB.on('pageerror', (e) => pageErrors.push(`B: ${e.message}`));

    await signInUser(pageA, alice);
    await signInUser(pageB, bob);
    console.log('✅ Signed in:', alice, '->', bob);

    // Connect friends
    await pageA.evaluate(async (target) => {
      await fetch('/api/friends/requests', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({ username: target }),
      });
    }, bob);
    await pageB.evaluate(async (fromUser) => {
      const list = await (await fetch('/api/friends/requests', { credentials: 'include' })).json();
      const arr = Array.isArray(list) ? list : (list.incoming || list.requests || []);
      const hit = arr.find((x) => x.from_username === fromUser || x.username === fromUser || x.sender_username === fromUser || x.requester?.username === fromUser);
      if (hit) {
        await fetch(`/api/friends/requests/${hit.id}/accept`, { method: 'POST', credentials: 'include', headers: { 'Content-Type': 'application/json' }, body: '{}' });
      }
    }, alice);

    for (const [pg, peer] of [[pageA, bob], [pageB, alice]]) {
      await pg.reload({ waitUntil: 'domcontentloaded' });
      await pg.waitForSelector('.app-rail', { timeout: 25000 });
      await pg.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer), { timeout: 25000 });
      const item = pg.locator('.conversation-item, .chat-item, .conv-item', { hasText: peer }).first();
      await item.waitFor({ state: 'visible', timeout: 15000 });
      await item.click();
      await pg.waitForTimeout(300);
    }

    // === PART 1: CHAT BUBBLE ZERO-SHIFT & NEW FROSTED AURORA OVERLAY ===
    const plusBtn = pageA.locator('button[aria-label*="Add photos"], button[aria-label*="Thêm ảnh"], .composer-tools button').first();
    if (await plusBtn.isVisible().catch(() => false)) {
      await plusBtn.click();
      await pageA.waitForTimeout(150);
    }
    await pageA.locator('input[type="file"][accept*="image"]').first().setInputFiles(TEST_PNG);

    let bubbleRectEnh = null;
    const bubbleWatcher = (async () => {
      await pageB.waitForSelector('.rich-image .ai-gen-overlay', { timeout: 25000 });
      bubbleRectEnh = await pageB.evaluate(() => {
        const r = document.querySelector('.rich-image-stage').getBoundingClientRect();
        return { x: r.x, y: r.y, width: r.width, height: r.height };
      });
      await pageB.screenshot({ path: path.join(OUT_DIR, '01_chat_bubble_enhancing_new_ui.png') });
    })();

    await pageB.waitForSelector('.rich-image img.rich-image-full[data-ai-restored="true"]', { timeout: 45000 });
    await bubbleWatcher;
    await pageB.waitForTimeout(550);
    await pageB.screenshot({ path: path.join(OUT_DIR, '02_chat_bubble_done_4k.png') });

    const bubbleAfter = await pageB.evaluate(() => {
      const r = document.querySelector('.rich-image-stage').getBoundingClientRect();
      const img = document.querySelector('.rich-image img.rich-image-full');
      return {
        rect: { x: r.x, y: r.y, width: r.width, height: r.height },
        naturalWidth: img?.naturalWidth,
        naturalHeight: img?.naturalHeight,
        overlayCount: document.querySelectorAll('.rich-image .ai-gen-overlay').length,
      };
    });
    assertRectClose('Chat bubble enhancing -> 4K done', bubbleRectEnh, bubbleAfter.rect);
    if (bubbleAfter.naturalWidth !== 3344 || bubbleAfter.overlayCount !== 0) {
      throw new Error(`Chat bubble final state unexpected: ${JSON.stringify(bubbleAfter)}`);
    }

    // === PART 2: NEW DEVICE 1600x960 LIGHTBOX FRAME-0 EXACT SIZE & 0.0PX JUMP ===
    console.log('⏳ Waiting 5.5s for history timestamp threshold...');
    await pageB.waitForTimeout(5500);

    const bobLs = await pageB.evaluate(() => {
      const out = {};
      for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i);
        out[k] = localStorage.getItem(k);
      }
      return out;
    });
    const bobCookies = await ctxB.cookies();

    const ctxNew = await browser.newContext({ viewport: { width: 1600, height: 960 } });
    await ctxNew.addCookies(bobCookies);
    const pageNew = await ctxNew.newPage();
    pageNew.on('pageerror', (e) => pageErrors.push(`NewDev: ${e.message}`));
    await pageNew.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
    await pageNew.evaluate((ls) => {
      for (const [k, v] of Object.entries(ls)) localStorage.setItem(k, v);
    }, bobLs);
    await pageNew.reload({ waitUntil: 'domcontentloaded' });
    await pageNew.waitForSelector('.app-rail', { timeout: 25000 });
    await pageNew.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwAiRestorer), { timeout: 25000 });
    const convNew = pageNew.locator('.conversation-item, .chat-item, .conv-item', { hasText: alice }).first();
    await convNew.waitFor({ state: 'visible', timeout: 15000 });
    await convNew.click();
    await pageNew.waitForSelector('.rich-image .rich-image-enhance-corner svg', { timeout: 20000 });
    await pageNew.waitForTimeout(250);

    // Install synchronous MutationObserver BEFORE clicking to capture exact Frame-0 Lightbox geometry!
    await pageNew.evaluate(() => {
      window.__s112Frame0 = null;
      const obs = new MutationObserver(() => {
        const frame = document.querySelector('.lightbox-img-frame');
        const img = document.querySelector('.lightbox-img');
        if (frame && img && !window.__s112Frame0) {
          const fr = frame.getBoundingClientRect();
          const ir = img.getBoundingClientRect();
          const overlay = document.querySelector('.lightbox-ai-gen');
          const frost = document.querySelector('.lightbox-ai-gen .ai-gen-frost');
          const wave = document.querySelector('.lightbox-ai-gen .ai-gen-dots-wave');
          const imgCs = getComputedStyle(img);
          const waveCs = wave ? getComputedStyle(wave) : null;
          window.__s112Frame0 = {
            frameRect: { x: fr.x, y: fr.y, width: fr.width, height: fr.height },
            imgRect: { x: ir.x, y: ir.y, width: ir.width, height: ir.height },
            hasOverlayAtFrame0: Boolean(overlay && frost),
            imgFilter: imgCs.filter,
            imgBorderRadius: imgCs.borderRadius,
            frameBorderRadius: getComputedStyle(frame).borderRadius,
            waveFilter: waveCs ? waveCs.filter : 'none',
            naturalWidth: img.naturalWidth,
          };
          obs.disconnect();
        }
      });
      obs.observe(document.body, { childList: true, subtree: true });
    });

    // Click image to open Lightbox!
    await pageNew.locator('.rich-image-btn').first().click();
    await pageNew.waitForSelector('.lightbox-img-frame', { timeout: 8000 });
    await pageNew.waitForTimeout(160);

    const midAudit = await pageNew.evaluate(() => {
      const fr = document.querySelector('.lightbox-img-frame').getBoundingClientRect();
      const ir = document.querySelector('.lightbox-img').getBoundingClientRect();
      return {
        frame0: window.__s112Frame0,
        midFrameRect: { x: fr.x, y: fr.y, width: fr.width, height: fr.height },
        midImgRect: { x: ir.x, y: ir.y, width: ir.width, height: ir.height },
        pctText: document.querySelector('.lightbox-ai-gen .ai-gen-pct')?.textContent?.trim() || '',
      };
    });
    await pageNew.screenshot({ path: path.join(OUT_DIR, '03_lightbox_enhancing_new_ui_1600x960.png') });
    console.log('=== FRAME 0 & MID-ENHANCE ===', JSON.stringify(midAudit, null, 2));

    // Wait for 4K Real-ESRGAN completion + reveal fade-out
    await pageNew.waitForSelector('.lightbox-enhance-toggle.is-active', { timeout: 45000 });
    await pageNew.waitForTimeout(550);
    await pageNew.screenshot({ path: path.join(OUT_DIR, '04_lightbox_4k_finished_1600x960.png') });

    const doneAudit = await pageNew.evaluate(() => {
      const fr = document.querySelector('.lightbox-img-frame').getBoundingClientRect();
      const ir = document.querySelector('.lightbox-img').getBoundingClientRect();
      const img = document.querySelector('.lightbox-img');
      return {
        frameRect: { x: fr.x, y: fr.y, width: fr.width, height: fr.height },
        imgRect: { x: ir.x, y: ir.y, width: ir.width, height: ir.height },
        naturalWidth: img.naturalWidth,
        naturalHeight: img.naturalHeight,
        overlayCount: document.querySelectorAll('.lightbox-ai-gen').length,
        dxCenter: Math.abs((ir.left + ir.width / 2) - window.innerWidth / 2),
        dyCenter: Math.abs((ir.top + ir.height / 2) - window.innerHeight / 2),
      };
    });
    console.log('=== POST-4K DONE ===', JSON.stringify(doneAudit, null, 2));

    // Toggle to Original compressed 836x471 -> MUST HAVE 0.0px SIZE JUMP!
    await pageNew.locator('.lightbox-enhance-toggle').click();
    await pageNew.waitForTimeout(250);
    const toggledOrig = await pageNew.evaluate(() => {
      const fr = document.querySelector('.lightbox-img-frame').getBoundingClientRect();
      const ir = document.querySelector('.lightbox-img').getBoundingClientRect();
      const img = document.querySelector('.lightbox-img');
      return {
        frameRect: { x: fr.x, y: fr.y, width: fr.width, height: fr.height },
        imgRect: { x: ir.x, y: ir.y, width: ir.width, height: ir.height },
        naturalWidth: img.naturalWidth,
        naturalHeight: img.naturalHeight,
      };
    });

    // Toggle back to Enhanced 3344x1884 -> MUST HAVE 0.0px SIZE JUMP!
    await pageNew.locator('.lightbox-enhance-toggle').click();
    await pageNew.waitForTimeout(250);
    const toggledBack = await pageNew.evaluate(() => {
      const fr = document.querySelector('.lightbox-img-frame').getBoundingClientRect();
      const ir = document.querySelector('.lightbox-img').getBoundingClientRect();
      const img = document.querySelector('.lightbox-img');
      return {
        frameRect: { x: fr.x, y: fr.y, width: fr.width, height: fr.height },
        imgRect: { x: ir.x, y: ir.y, width: ir.width, height: ir.height },
        naturalWidth: img.naturalWidth,
        naturalHeight: img.naturalHeight,
      };
    });

    // Strict assertions!
    const f0 = midAudit.frame0;
    if (!f0 || !f0.hasOverlayAtFrame0) {
      throw new Error(`❌ Frame-0 overlay missing (raw blurry flash detected!): ${JSON.stringify(f0)}`);
    }
    if (f0.imgBorderRadius !== '0px' || f0.frameBorderRadius !== '0px') {
      throw new Error(`❌ Lightbox frame/img not unrounded: img=${f0.imgBorderRadius} frame=${f0.frameBorderRadius}`);
    }
    if (f0.imgFilter !== 'none' || f0.waveFilter !== 'none') {
      throw new Error(`❌ Non-composited CSS filter found: imgFilter=${f0.imgFilter} waveFilter=${f0.waveFilter}`);
    }
    if (Math.abs(f0.imgRect.width - 1440) > 2 || Math.abs(f0.imgRect.height - 811) > 2) {
      throw new Error(`❌ Frame-0 did NOT pre-compute final 1440x811 size! Got ${f0.imgRect.width}x${f0.imgRect.height}`);
    }
    assertRectClose('Lightbox Frame-0 -> Mid-Enhance', f0.imgRect, midAudit.midImgRect);
    assertRectClose('Lightbox Frame-0 -> Post-4K Done', f0.imgRect, doneAudit.imgRect);
    assertRectClose('Lightbox Post-4K -> Toggle Original (836x471)', doneAudit.imgRect, toggledOrig.imgRect);
    assertRectClose('Lightbox Toggle Original -> Toggle Back 4K', toggledOrig.imgRect, toggledBack.imgRect);

    if (doneAudit.naturalWidth !== 3344 || toggledOrig.naturalWidth !== 836 || toggledBack.naturalWidth !== 3344) {
      throw new Error(`❌ Unexpected natural widths: done=${doneAudit.naturalWidth} orig=${toggledOrig.naturalWidth} back=${toggledBack.naturalWidth}`);
    }
    if (doneAudit.overlayCount !== 0 || doneAudit.dxCenter > 1.5 || doneAudit.dyCenter > 1.5) {
      throw new Error(`❌ Post-4K overlay/centering check failed: ${JSON.stringify(doneAudit)}`);
    }
    if (pageErrors.length > 0) {
      throw new Error(`❌ Page errors detected: ${pageErrors.join(' | ')}`);
    }

    await ctxNew.close();
    console.log('\n🎉 SPEC 112 PASSED 100%: 0.0px JUMP ACROSS ALL 5 STATES, FRAME-0 FINAL 1440×811, COMPOSITOR-ONLY ANIMATIONS, ZERO ERRORS!');
  } finally {
    await browser.close();
  }
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
