import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec135';
fs.mkdirSync(SHOT_DIR, { recursive: true });

async function run() {
  const browser = await chromium.launch({
    headless: true,
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-gpu'],
  });

  const page = await browser.newPage({ viewport: { width: 1280, height: 860 } });

  console.log(`Navigating to ${BASE}...`);
  await page.goto(BASE, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(1500);

  // 1. Audit Chat image loading stage with refined laser sweep
  console.log('\n--- AUDIT 1: Chat Image Loading with Refined Slender Laser Sweep ---');
  await page.evaluate(() => {
    const stage = document.createElement('div');
    stage.id = 'spec135-chat-stage';
    stage.style.cssText = 'position: fixed; top: 40px; left: 40px; width: 280px; height: 210px; z-index: 99999; border-radius: 12px; overflow: hidden; box-shadow: 0 10px 25px rgba(0,0,0,0.5);';
    stage.innerHTML = `
      <div style="position: absolute; inset: 0; background: linear-gradient(135deg, #1e293b, #0f172a);"></div>
      <div class="ai-gen-overlay" style="position: absolute; inset: 0; border-radius: 12px; overflow: hidden;">
        <div class="ai-gen-frost"></div>
        <div class="ai-gen-aurora"></div>
        <div class="ai-gen-dots-base"></div>
        <div class="ai-gen-dots-wave"></div>
        <span class="ai-gen-pct">42%</span>
      </div>
    `;
    document.body.appendChild(stage);
  });

  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(SHOT_DIR, '01_chat_loading_refined_sweep.png') });
  console.log('Captured 01_chat_loading_refined_sweep.png');

  // 2. Audit Lightbox Wide Landscape with refined laser sweep
  console.log('\n--- AUDIT 2: Lightbox Wide Landscape with Refined Slender Laser Sweep ---');
  await page.evaluate(() => {
    const old = document.getElementById('spec135-chat-stage');
    if (old) old.remove();

    const box = document.createElement('div');
    box.id = 'spec135-lb-stage';
    box.style.cssText = 'position: fixed; inset: 0; z-index: 99999; background: rgba(5,7,12,0.92); display: flex; align-items: center; justify-content: center;';
    box.innerHTML = `
      <div class="lightbox-img-frame" style="width: 860px; height: 480px; position: relative; overflow: hidden; border-radius: 8px;">
        <div style="position: absolute; inset: 0; background: linear-gradient(160deg, #0f172a 0%, #0369a1 25%, #67e8f9 50%, #7c3aed 75%, #0f172a 100%);"></div>
        <div class="ai-gen-overlay lightbox-ai-gen" style="position: absolute; inset: 0; overflow: hidden;">
          <div class="ai-gen-frost"></div>
          <div class="ai-gen-aurora"></div>
          <div class="ai-gen-dots-base"></div>
          <div class="ai-gen-dots-wave"></div>
          <span class="ai-gen-pct">16%</span>
        </div>
      </div>
    `;
    document.body.appendChild(box);
  });

  // Verify CSS properties of .ai-gen-dots-wave and ::after
  const cssAudit = await page.evaluate(() => {
    const wave = document.querySelector('#spec135-lb-stage .ai-gen-dots-wave');
    const after = window.getComputedStyle(wave, '::after');
    const waveCs = window.getComputedStyle(wave);
    return {
      waveOverflow: waveCs.overflow,
      waveMixBlendMode: waveCs.mixBlendMode,
      waveBackground: waveCs.backgroundImage,
      afterWidth: after.width,
      afterBg: after.backgroundImage,
      afterAnimation: after.animationName,
    };
  });
  console.log('CSS Audit:', cssAudit);

  if (cssAudit.waveMixBlendMode !== 'screen') {
    throw new Error('Expected wave mix-blend-mode to be screen! Got: ' + cssAudit.waveMixBlendMode);
  }
  if (!cssAudit.afterAnimation.includes('chatwAiLaserScan')) {
    throw new Error('Expected after animation to be chatwAiLaserScan! Got: ' + cssAudit.afterAnimation);
  }

  await page.waitForTimeout(800);
  await page.screenshot({ path: path.join(SHOT_DIR, '02_lightbox_landscape_refined_sweep.png') });
  console.log('Captured 02_lightbox_landscape_refined_sweep.png');

  // 3. Audit Lightbox Portrait (Tall)
  console.log('\n--- AUDIT 3: Lightbox Portrait (Tall) with Refined Slender Laser Sweep ---');
  await page.evaluate(() => {
    const frame = document.querySelector('#spec135-lb-stage .lightbox-img-frame');
    frame.style.width = '420px';
    frame.style.height = '620px';
  });

  await page.waitForTimeout(800);
  await page.screenshot({ path: path.join(SHOT_DIR, '03_lightbox_portrait_refined_sweep.png') });
  console.log('Captured 03_lightbox_portrait_refined_sweep.png');

  await browser.close();
  console.log('\n🎉 ALL SPEC 135 VERIFICATIONS PASSED SUCCESSFULLY!');
}

run().catch((err) => {
  console.error('❌ Test failed:', err);
  process.exit(1);
});
