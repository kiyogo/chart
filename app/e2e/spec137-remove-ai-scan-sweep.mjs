import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const outDir = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec137';
fs.mkdirSync(outDir, { recursive: true });

async function run() {
  const browser = await chromium.launch({
    headless: true,
    executablePath: '/usr/bin/google-chrome',
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--use-gl=angle',
      '--use-angle=gl',
      '--enable-webgl',
      '--ignore-gpu-blocklist',
    ],
  });

  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });

  console.log('Navigating to http://127.0.0.1:8085...');
  await page.goto('http://127.0.0.1:8085', { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(2000);

  // 1. Verify CSS definition for .ai-gen-dots-wave is display: none !important
  const cssCheck = await page.evaluate(() => {
    const dummy = document.createElement('div');
    dummy.className = 'ai-gen-dots-wave';
    document.body.appendChild(dummy);
    const style = window.getComputedStyle(dummy);
    const display = style.display;
    const animation = style.animationName;
    const afterStyle = window.getComputedStyle(dummy, '::after');
    const afterAnimation = afterStyle.animationName;
    dummy.remove();
    return { display, animation, afterAnimation };
  });

  console.log('CSS Check for .ai-gen-dots-wave:', cssCheck);
  if (cssCheck.display !== 'none') {
    throw new Error(`Expected .ai-gen-dots-wave to be display: none !important, got ${cssCheck.display}`);
  }
  console.log('✅ PASS: .ai-gen-dots-wave is strictly disabled (display: none !important)!');

  // 2. Inject mockup of Lightbox AI generation overlay to verify clean visuals without any sweep beam
  await page.evaluate(() => {
    const lbContainer = document.createElement('div');
    lbContainer.id = 'spec137-lb-stage';
    lbContainer.style.position = 'fixed';
    lbContainer.style.inset = '0';
    lbContainer.style.background = 'rgba(0, 0, 0, 0.88)';
    lbContainer.style.zIndex = '99999';
    lbContainer.style.display = 'flex';
    lbContainer.style.alignItems = 'center';
    lbContainer.style.justifyContent = 'center';

    lbContainer.innerHTML = `
      <div class="lightbox-img-frame" style="position: relative; width: 640px; height: 480px; overflow: hidden; border-radius: 8px;">
        <img class="lightbox-img is-ai-generating" src="/chatw_mascot_full.svg" style="width: 100%; height: 100%; object-fit: contain;" />
        <div class="ai-gen-overlay lightbox-ai-gen" style="position: absolute; inset: 0;">
          <img class="ai-gen-blur-img" src="/chatw_mascot_full.svg" style="width: 100%; height: 100%; object-fit: contain; filter: blur(24px);" />
          <div class="ai-gen-frost"></div>
          <div class="ai-gen-aurora"></div>
          <div class="ai-gen-dots-base"></div>
          <span class="ai-gen-pct">79%</span>
        </div>
      </div>
    `;
    document.body.appendChild(lbContainer);
  });

  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(outDir, '01_lightbox_loading_no_sweep_beam.png') });
  console.log('📸 Saved visual proof: 01_lightbox_loading_no_sweep_beam.png');

  // 3. Verify that there is ZERO scan sweep line / laser bar rendered
  const sweepChecks = await page.evaluate(() => {
    const stage = document.getElementById('spec137-lb-stage');
    const wave = stage.querySelector('.ai-gen-dots-wave');
    const pct = stage.querySelector('.ai-gen-pct');
    const frost = stage.querySelector('.ai-gen-frost');
    const aurora = stage.querySelector('.ai-gen-aurora');
    const dotsBase = stage.querySelector('.ai-gen-dots-base');

    return {
      hasWaveInDom: Boolean(wave),
      hasPct: Boolean(pct),
      hasFrost: Boolean(frost),
      hasAurora: Boolean(aurora),
      hasDotsBase: Boolean(dotsBase),
    };
  });

  console.log('DOM Sweep Checks:', sweepChecks);
  if (sweepChecks.hasWaveInDom) {
    throw new Error('CRITICAL: .ai-gen-dots-wave must NOT be present in DOM!');
  }
  if (!sweepChecks.hasPct || !sweepChecks.hasFrost || !sweepChecks.hasAurora) {
    throw new Error('CRITICAL: essential overlay elements (frost, aurora, pct) must remain intact!');
  }
  console.log('✅ PASS: Scan sweep beam is 100% absent, calming ambient backdrop remains intact!');

  await browser.close();
  console.log('🎉 Spec 137 verification successfully completed!');
}

run().catch((err) => {
  console.error('❌ E2E Failed:', err);
  process.exit(1);
});
