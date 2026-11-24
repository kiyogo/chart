// Spec 113 E2E Verification:
//  1. Strictly unrounded Lightbox image corners (border-radius: 0px across frame, img, overlay).
//  2. True-physics mass-spring-damper return:
//     - Fast flick carries authentic forward momentum arc (overshoots release X before smoothly reversing!)
//       then glides back and settles cleanly at exact (0px, 0px).
//     - Multi-frame continuous physical motion (smooth 60Hz+ trajectory sampled via rAF).
//     - Mid-flight grab catches smoothly right at live intermediate coordinates without teleporting.
//     - Zero console / page errors.
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec113';
fs.mkdirSync(OUT_DIR, { recursive: true });
const TEST_PNG = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_cnn_sr/ChatGPT_Anime_1672x941.png';

(async () => {
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--enable-unsafe-webgpu', '--use-gl=angle', '--use-angle=vulkan', '--enable-features=Vulkan'],
  });
  const pageErrors = [];
  try {
    const ctx = await browser.newContext({ viewport: { width: 1600, height: 960 } });
    const page = await ctx.newPage();
    page.on('pageerror', (e) => pageErrors.push(e.message));

    const uname = `s113_${Date.now().toString(36).slice(-5)}`;
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
    await page.waitForFunction(() => Boolean(window.__chatwWasmMedia && window.__chatwMediaOpt), { timeout: 25000 });

    // Open "Chính mình" ("Note to self") and send high-res photo
    const selfConv = page.locator('.conversation-item, .chat-item, .conv-item', { hasText: /Chính mình|Saved|Note/i }).first();
    await selfConv.waitFor({ state: 'visible', timeout: 15000 });
    await selfConv.click();
    await page.waitForTimeout(300);

    const plusBtn = page.locator('button[aria-label*="Add photos"], button[aria-label*="Thêm ảnh"], .composer-tools button').first();
    if (await plusBtn.isVisible().catch(() => false)) {
      await plusBtn.click();
      await page.waitForTimeout(150);
    }
    await page.locator('input[type="file"][accept*="image"]').first().setInputFiles(TEST_PNG);
    await page.waitForSelector('.rich-image-btn', { timeout: 20000 });
    await page.waitForTimeout(400);

    // Open Lightbox
    await page.locator('.rich-image-btn').first().click();
    await page.waitForSelector('.lightbox .lightbox-img', { timeout: 8000 });
    await page.waitForTimeout(250);

    // 1. Assert STRICT 0px unrounded corners on Lightbox frame & img!
    const radiusAudit = await page.evaluate(() => {
      const frame = document.querySelector('.lightbox-img-frame');
      const img = document.querySelector('.lightbox-img');
      const fCs = getComputedStyle(frame);
      const iCs = getComputedStyle(img);
      return {
        frameRadius: fCs.borderRadius,
        imgRadius: iCs.borderRadius,
        imgTransition: iCs.transitionProperty,
      };
    });
    console.log('=== RADIUS & PHYSICS DRIVER AUDIT ===', JSON.stringify(radiusAudit));
    if (radiusAudit.frameRadius !== '0px' || radiusAudit.imgRadius !== '0px') {
      throw new Error(`❌ Lightbox corners are still rounded! ${JSON.stringify(radiusAudit)}`);
    }
    await page.screenshot({ path: path.join(OUT_DIR, '01_unrounded_lightbox_crisp_0px.png') });

    // 1b. Drag & capture unrounded screenshot while dragging
    const box = await page.locator('.lightbox-img').boundingBox();
    const cx = box.x + box.width / 2;
    const cy = box.y + box.height / 2;
    await page.mouse.move(cx, cy);
    await page.mouse.down();
    await page.mouse.move(cx + 280, cy + 90, { steps: 4 });
    await page.screenshot({ path: path.join(OUT_DIR, '02_dragging_unrounded_0px.png') });
    await page.mouse.up();
    await page.waitForTimeout(550);

    // 2. Fast physical flick +280px (ZERO screenshot pause between move & mouseup!) -> sample rAF trajectory!
    await page.evaluate(() => {
      window.__s113Samples = [];
      window.__s113Recording = false;
      const parseTx = (tf) => {
        const m = /translate3d\(([-\d.]+)px,\s*([-\d.]+)px/.exec(tf || '');
        return m ? { x: parseFloat(m[1]), y: parseFloat(m[2]) } : null;
      };
      window.__s113StartRec = () => {
        window.__s113Samples = [];
        window.__s113Recording = true;
        const t0 = performance.now();
        const tick = () => {
          const img = document.querySelector('.lightbox-img');
          if (img) {
            const p = parseTx(img.style.transform);
            if (p) window.__s113Samples.push({ t: Math.round(performance.now() - t0), x: p.x, y: p.y });
          }
          if (window.__s113Recording && performance.now() - t0 < 750) {
            requestAnimationFrame(tick);
          }
        };
        requestAnimationFrame(tick);
      };
    });

    await page.mouse.move(cx, cy);
    await page.mouse.down();
    await page.evaluate(() => window.__s113StartRec());
    await page.mouse.move(cx + 120, cy + 40, { steps: 2 });
    await page.mouse.move(cx + 280, cy + 90, { steps: 2 });
    await page.mouse.up();
    await page.waitForTimeout(760);

    const flickTrajectory = await page.evaluate(() => {
      window.__s113Recording = false;
      const s = window.__s113Samples || [];
      const maxX = s.reduce((m, p) => Math.max(m, p.x), -Infinity);
      const uniqueXs = new Set(s.map((p) => p.x.toFixed(1))).size;
      const last = s[s.length - 1] || null;
      return {
        sampleCount: s.length,
        uniquePositions: uniqueXs,
        releaseX: s[0]?.x ?? null,
        peakMomentumX: maxX,
        finalPos: last,
        finalTransform: document.querySelector('.lightbox-img')?.style.transform || '',
      };
    });
    console.log('=== FAST FLICK SPRING TRAJECTORY ===', JSON.stringify(flickTrajectory));
    if (flickTrajectory.uniquePositions < 10) {
      throw new Error(`❌ Spring did not produce smooth multi-frame motion! uniquePositions=${flickTrajectory.uniquePositions}`);
    }
    if (flickTrajectory.peakMomentumX <= 282) {
      throw new Error(`❌ Flick momentum carry missing! peakX=${flickTrajectory.peakMomentumX} (expected > 282px past 280px release)`);
    }
    if (!flickTrajectory.finalTransform.includes('translate3d(0px, 0px')) {
      throw new Error(`❌ Spring did not settle at exact (0,0): ${flickTrajectory.finalTransform}`);
    }

    // 3. Mid-flight grab test: drag +320px, release, wait 90ms, grab mid-air -> must NOT teleport to 0!
    await page.mouse.move(cx, cy);
    await page.mouse.down();
    await page.mouse.move(cx + 320, cy + 80, { steps: 4 });
    await page.waitForTimeout(80);
    await page.mouse.up();
    await page.waitForTimeout(80);
    await page.mouse.move(cx + 150, cy + 40);
    await page.mouse.down();
    await page.waitForTimeout(120);
    const caught = await page.evaluate(() => {
      const tf = document.querySelector('.lightbox-img')?.style.transform || '';
      const m = /translate3d\(([-\d.]+)px/.exec(tf);
      return { tf, x: m ? parseFloat(m[1]) : 0 };
    });
    await page.mouse.up();
    await page.waitForTimeout(480);
    const afterCaughtRelease = await page.evaluate(() => document.querySelector('.lightbox-img')?.style.transform || '');
    console.log('=== MID-FLIGHT CATCH ===', JSON.stringify({ caught, afterCaughtRelease }));
    if (caught.x < 30 || caught.x > 315) {
      throw new Error(`❌ Mid-flight grab did not freeze cleanly mid-air: ${JSON.stringify(caught)}`);
    }
    if (!afterCaughtRelease.includes('translate3d(0px, 0px')) {
      throw new Error(`❌ After mid-flight catch & release, failed to settle at (0,0): ${afterCaughtRelease}`);
    }

    if (pageErrors.length > 0) {
      throw new Error(`❌ Page errors: ${pageErrors.join(' | ')}`);
    }

    console.log('\n🎉 SPEC 113 PASSED 100%: STRICT 0px CORNERS, TRUE-PHYSICS MOMENTUM + SPRING RETURN, MID-FLIGHT CATCH, ZERO ERRORS!');
  } finally {
    await browser.close();
  }
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
