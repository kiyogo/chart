import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const outDir = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec134';
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

  // Evaluate model selection inside browser context
  const res = await page.evaluate(async () => {
    const wasm = window.__chatwWasmMedia;
    if (!wasm || typeof wasm.selectRealEsrganModel !== 'function') {
      return { ok: false, error: 'wasm not loaded' };
    }

    // Create a 64x64 RGBA buffer representing a human portrait with skin tones
    const w = 64;
    const h = 64;
    const rgba = new Uint8Array(w * h * 4);
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        const i = (y * w + x) * 4;
        if (y < 16 || x < 6 || x > 57) {
          rgba[i] = 18;
          rgba[i + 1] = 16;
          rgba[i + 2] = 20;
          rgba[i + 3] = 255;
        } else {
          // Warm skin tone
          rgba[i] = 235;
          rgba[i + 1] = 188;
          rgba[i + 2] = 165;
          rgba[i + 3] = 255;
        }
      }
    }

    // 1. Without QP
    const selNoQp = wasm.selectRealEsrganModel(rgba, w, h, null, '');

    // 2. With Portrait QP
    const qpJson = JSON.stringify({
      cls: 'portrait',
      w,
      h,
      sharp: 12.5,
      edge_p95: 18.0,
      acut: 10.0,
      luma_m: 170.0,
      luma_s: 66.0,
      r_m: 200.0,
      g_m: 160.0,
      b_m: 150.0,
      sat_m: 0.25,
      outline_r: 0.0037,
      loss_ratio: 1.0,
    });
    const selQp = wasm.selectRealEsrganModel(rgba, w, h, qpJson, '');

    return {
      ok: true,
      pipelineVersion: window.__chatwAiRestorer ? 'v10_ready' : 'unknown',
      selNoQp,
      selQp,
    };
  });

  console.log('Browser Evaluation Result:', JSON.stringify(res, null, 2));

  if (!res.ok) {
    throw new Error('E2E evaluation failed: ' + res.error);
  }

  if (res.selNoQp.modelId !== 'realesr_general_x4v3' || res.selNoQp.isAnime !== false) {
    throw new Error('selNoQp must select GeneralX4V3! Got: ' + JSON.stringify(res.selNoQp));
  }

  if (res.selQp.modelId !== 'realesr_general_x4v3' || res.selQp.isAnime !== false) {
    throw new Error('selQp must select GeneralX4V3! Got: ' + JSON.stringify(res.selQp));
  }

  console.log('PASS: Both No-QP and Portrait QP select GeneralX4V3 with isAnime = false!');

  await page.screenshot({ path: path.join(outDir, '01_e2e_general_x4v3_selected.png') });
  await browser.close();
}

run().catch((err) => {
  console.error(err);
  process.exit(1);
});
