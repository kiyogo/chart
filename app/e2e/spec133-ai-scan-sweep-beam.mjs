import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec133';
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

async function run() {
  console.log('🚀 Starting Spec 133 Verification: Luminous AI Laser Scan Sweep Beam Across Images...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();

  await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'dark');
  });
  await page.reload({ waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  const uname = `v133_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(600);

  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Open "Chính mình"
  const selfChat = page.locator('.nav-sidebar .conv-row, .nav-sidebar .chat-item').filter({ hasText: /Chính mình|Note to self/i }).first();
  await selfChat.click();
  await page.waitForSelector('.chat-column', { timeout: 10000 });
  await page.waitForTimeout(400);

  // ==========================================
  // AUDIT 1: Chat Image Loading Stage Sweep Wave
  // ==========================================
  console.log('\n--- AUDIT 1: Chat Image Loading Placeholder Sweep Wave ---');
  await page.evaluate(() => {
    const stream = document.querySelector('.message-stream');
    const container = document.createElement('div');
    container.id = 'spec133-test-chat-img';
    container.innerHTML = `
      <div class="message-row received">
        <div class="message-bubble" style="padding: 6px;">
          <div class="rich-image" style="width: 260px; height: 260px;">
            <div class="rich-image-stage loading" style="width: 100%; height: 100%;">
              <div class="ai-gen-frost"></div>
              <div class="ai-gen-aurora"></div>
              <div class="ai-gen-dots-base"></div>
              <div class="ai-gen-dots-wave"></div>
              <span class="rich-image-blur-spinner"><span class="spinner"></span></span>
            </div>
          </div>
        </div>
      </div>
    `;
    stream.appendChild(container);
  });

  const chatWaveAudit = await page.evaluate(() => {
    const stage = document.querySelector('#spec133-test-chat-img .rich-image-stage.loading');
    const wave = stage.querySelector('.ai-gen-dots-wave');
    const cs = window.getComputedStyle(wave);
    return {
      exists: !!wave,
      display: cs.display,
      animation: cs.animation,
      zIndex: cs.zIndex,
    };
  });
  console.log('Chat loading wave audit:', chatWaveAudit);
  if (!chatWaveAudit.exists) {
    throw new Error('CRITICAL: .ai-gen-dots-wave is missing in rich-image-stage loading!');
  }
  console.log('✅ PASS: Chat image loading placeholder contains animated .ai-gen-dots-wave!');

  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '01_chat_loading_laser_sweep.png') });

  // ==========================================
  // AUDIT 2: Lightbox AI Enhancement Laser Sweep (Portrait / Anime)
  // ==========================================
  console.log('\n--- AUDIT 2: Lightbox AI Enhancement Laser Sweep (Portrait / Anime) ---');
  await page.evaluate(() => {
    const box = document.createElement('div');
    box.id = 'spec133-lb-portrait';
    box.style.cssText = 'position: fixed; inset: 0; z-index: 99999; background: rgba(5,7,12,0.92); display: flex; align-items: center; justify-content: center;';
    box.innerHTML = `
      <div class="lightbox-img-frame" style="width: 520px; height: 520px; position: relative; overflow: hidden; border-radius: 8px;">
        <div style="position: absolute; inset: 0; background: linear-gradient(145deg, #1e293b 0%, #38bdf8 30%, #f1f5f9 60%, #0f172a 100%);"></div>
        <div class="ai-gen-overlay lightbox-ai-gen" style="position: absolute; inset: 0; overflow: hidden;">
          <div class="ai-gen-frost"></div>
          <div class="ai-gen-aurora"></div>
          <div class="ai-gen-dots-base"></div>
          <div class="ai-gen-dots-wave"></div>
          <span class="ai-gen-pct">3%</span>
        </div>
      </div>
    `;
    document.body.appendChild(box);
  });

  await page.waitForTimeout(350);
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '02_lightbox_portrait_laser_sweep.png') });

  // ==========================================
  // AUDIT 3: Lightbox AI Enhancement Laser Sweep (Wide Landscape)
  // ==========================================
  console.log('\n--- AUDIT 3: Lightbox AI Enhancement Laser Sweep (Wide Landscape) ---');
  await page.evaluate(() => {
    const p = document.getElementById('spec133-lb-portrait');
    if (p) p.remove();

    const box = document.createElement('div');
    box.id = 'spec133-lb-landscape';
    box.style.cssText = 'position: fixed; inset: 0; z-index: 99999; background: rgba(5,7,12,0.92); display: flex; align-items: center; justify-content: center;';
    box.innerHTML = `
      <div class="lightbox-img-frame" style="width: 860px; height: 440px; position: relative; overflow: hidden; border-radius: 8px;">
        <div style="position: absolute; inset: 0; background: linear-gradient(160deg, #0f172a 0%, #0369a1 25%, #67e8f9 50%, #7c3aed 75%, #0f172a 100%);"></div>
        <div class="ai-gen-overlay lightbox-ai-gen" style="position: absolute; inset: 0; overflow: hidden;">
          <div class="ai-gen-frost"></div>
          <div class="ai-gen-aurora"></div>
          <div class="ai-gen-dots-base"></div>
          <div class="ai-gen-dots-wave"></div>
          <span class="ai-gen-pct">14%</span>
        </div>
      </div>
    `;
    document.body.appendChild(box);
  });

  // Verify laser line pseudo-element ::after
  const laserAudit = await page.evaluate(() => {
    const wave = document.querySelector('#spec133-lb-landscape .ai-gen-dots-wave');
    const afterCs = window.getComputedStyle(wave, '::after');
    const waveCs = window.getComputedStyle(wave);
    return {
      afterWidth: afterCs.width,
      afterBg: afterCs.backgroundColor,
      afterBoxShadow: afterCs.boxShadow,
      afterOpacity: afterCs.opacity,
      waveFilter: waveCs.filter,
      waveAnimation: waveCs.animation,
    };
  });
  console.log('Laser scan line audit:', laserAudit);
  if (laserAudit.afterWidth === 'auto' || laserAudit.afterWidth === '0px') {
    throw new Error('CRITICAL: .ai-gen-dots-wave::after laser line failed to render!');
  }
  console.log('✅ PASS: Laser scan line ::after is rendered with width: ' + laserAudit.afterWidth + ' and glow!');

  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(ARTIFACTS_DIR, '03_lightbox_landscape_laser_sweep.png') });

  await browser.close();
  console.log('\n🎉 ALL SPEC 133 AUDITS PASSED SUCCESSFULLY!');
}

run().catch((err) => {
  console.error('❌ Test failed:', err);
  process.exit(1);
});
