import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec144';
fs.mkdirSync(OUT_DIR, { recursive: true });

const TEST_IMAGES = [
  {
    name: 'portrait1_sailor',
    path: '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/.user_uploaded/media_1795549648014_4417d69b.jpg',
    crop: [0.30, 0.16, 0.38, 0.56], // Face + eyes + bangs + silver necklace
  },
  {
    name: 'portrait2_highdetail',
    path: '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/.user_uploaded/media_1795545206648_e1cddb86.jpg',
    crop: [0.26, 0.14, 0.44, 0.48],
  },
];

const payloads = TEST_IMAGES.map((t) => ({
  ...t,
  b64: fs.readFileSync(t.path).toString('base64'),
}));

const browser = await chromium.launch({
  headless: true,
  executablePath: '/usr/bin/google-chrome',
  args: ['--no-sandbox', '--disable-setuid-sandbox', '--use-gl=angle', '--use-angle=gl', '--enable-webgl', '--ignore-gpu-blocklist'],
});

try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const pageErrors = [];
  page.on('pageerror', (e) => pageErrors.push(String(e)));

  await page.goto('http://127.0.0.1:8085/', { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(
    () => Boolean(window.__chatwWasmMedia && window.__chatwWasmMedia.encodeAvifAdaptive && window.__chatwAiRestorer),
    { timeout: 25000 },
  );

  const report = await page.evaluate(async ({ payloads }) => {
    const b64ToU8 = (s) => Uint8Array.from(atob(s), (c) => c.charCodeAt(0));
    const wasm = window.__chatwWasmMedia;
    const restorer = window.__chatwAiRestorer;

    const evalTrueMetrics = (refU8, tstU8, W, H) => {
      let mse = 0, maeColor = 0;
      const n = W * H;
      const refL = new Float32Array(n), tstL = new Float32Array(n);
      for (let i = 0; i < n; i++) {
        const r0 = refU8[i * 4], g0 = refU8[i * 4 + 1], b0 = refU8[i * 4 + 2];
        const r1 = tstU8[i * 4], g1 = tstU8[i * 4 + 1], b1 = tstU8[i * 4 + 2];
        const dr = r0 - r1, dg = g0 - g1, db = b0 - b1;
        mse += (dr * dr + dg * dg + db * db) / 3;
        maeColor += (Math.abs(dr) + Math.abs(dg) + Math.abs(db)) / 3;
        refL[i] = 0.299 * r0 + 0.587 * g0 + 0.114 * b0;
        tstL[i] = 0.299 * r1 + 0.587 * g1 + 0.114 * b1;
      }
      mse /= n;
      const psnr = mse < 1e-9 ? 99 : Number((10 * Math.log10((255 * 255) / mse)).toFixed(2));
      const colorMae = Number((maeColor / n).toFixed(2));

      const C1 = 6.5025, C2 = 58.5225;
      let ssimSum = 0, ssimCnt = 0;
      for (let y = 0; y <= H - 8; y += 8) {
        for (let x = 0; x <= W - 8; x += 8) {
          let m0 = 0, m1 = 0;
          for (let dy = 0; dy < 8; dy++) for (let dx = 0; dx < 8; dx++) {
            const idx = (y + dy) * W + (x + dx);
            m0 += refL[idx]; m1 += tstL[idx];
          }
          m0 /= 64; m1 /= 64;
          let v0 = 0, v1 = 0, cov = 0;
          for (let dy = 0; dy < 8; dy++) for (let dx = 0; dx < 8; dx++) {
            const idx = (y + dy) * W + (x + dx);
            const a = refL[idx] - m0, b = tstL[idx] - m1;
            v0 += a * a; v1 += b * b; cov += a * b;
          }
          v0 /= 63; v1 /= 63; cov /= 63;
          ssimSum += ((2 * m0 * m1 + C1) * (2 * cov + C2)) / ((m0 * m0 + m1 * m1 + C1) * (v0 + v1 + C2));
          ssimCnt++;
        }
      }
      const trueSsim = Number((ssimSum / Math.max(1, ssimCnt)).toFixed(4));

      let gDot = 0, g0Sq = 0, g1Sq = 0;
      for (let y = 1; y < H - 1; y += 2) {
        for (let x = 1; x < W - 1; x += 2) {
          const idx = y * W + x;
          const m0 = Math.hypot(refL[idx + 1] - refL[idx - 1], refL[idx + W] - refL[idx - W]);
          const m1 = Math.hypot(tstL[idx + 1] - tstL[idx - 1], tstL[idx + W] - tstL[idx - W]);
          gDot += m0 * m1; g0Sq += m0 * m0; g1Sq += m1 * m1;
        }
      }
      const edgeCorr = Number((gDot / Math.sqrt(Math.max(1e-6, g0Sq * g1Sq))).toFixed(4));
      return { psnr, trueSsim, edgeCorr, colorMae };
    };

    const drawToCanvas = async (url, w, h) => {
      const b = await (await fetch(url)).blob();
      const bmp = await createImageBitmap(b);
      const c = document.createElement('canvas');
      c.width = w || bmp.width; c.height = h || bmp.height;
      const ctx = c.getContext('2d');
      ctx.imageSmoothingEnabled = true; ctx.imageSmoothingQuality = 'high';
      ctx.drawImage(bmp, 0, 0, c.width, c.height);
      bmp.close();
      return c;
    };

    const cropPng = (c, box) => {
      const fc = document.createElement('canvas');
      const sx = Math.round(c.width * box[0]), sy = Math.round(c.height * box[1]);
      const sw = Math.round(c.width * box[2]), sh = Math.round(c.height * box[3]);
      fc.width = Math.max(sw, 640); fc.height = Math.round(fc.width * sh / sw);
      const ctx = fc.getContext('2d');
      ctx.imageSmoothingEnabled = true; ctx.imageSmoothingQuality = 'high';
      ctx.drawImage(c, sx, sy, sw, sh, 0, 0, fc.width, fc.height);
      return fc.toDataURL('image/png').split(',')[1];
    };

    const diffHeatmapPng = (refU8, tstU8, W, H) => {
      const dc = document.createElement('canvas'); dc.width = W; dc.height = H;
      const dctx = dc.getContext('2d');
      const id = dctx.createImageData(W, H);
      for (let i = 0; i < W * H; i++) {
        const d = (Math.abs(refU8[i*4] - tstU8[i*4]) + Math.abs(refU8[i*4+1] - tstU8[i*4+1]) + Math.abs(refU8[i*4+2] - tstU8[i*4+2])) / 3;
        const v = Math.min(255, Math.round(d * 6));
        id.data[i*4] = v; id.data[i*4+1] = Math.min(255, Math.round(v * 0.4)); id.data[i*4+2] = Math.max(0, 255 - v); id.data[i*4+3] = 255;
      }
      dctx.putImageData(id, 0, 0);
      return dc.toDataURL('image/png').split(',')[1];
    };

    const results = {};
    for (const item of payloads) {
      const origBytes = b64ToU8(item.b64);
      const origUrl = URL.createObjectURL(new Blob([origBytes], { type: 'image/jpeg' }));
      const cOrig = await drawToCanvas(origUrl, 0, 0);
      const OW = cOrig.width, OH = cOrig.height;
      const origU8 = cOrig.getContext('2d').getImageData(0, 0, OW, OH).data;

      // 1. Compress via ChatW's real AVIF encoder (UltraAdaptive)
      const tEnc = performance.now();
      const avif = wasm.encodeAvifAdaptive(origBytes, 'ultra');
      const encMs = Math.round(performance.now() - tEnc);
      const avifUrl = URL.createObjectURL(new Blob([avif.bytes], { type: 'image/avif' }));
      const cAvif = await drawToCanvas(avifUrl, OW, OH);
      const avifU8 = cAvif.getContext('2d').getImageData(0, 0, OW, OH).data;
      const avifMetrics = evalTrueMetrics(origU8, avifU8, OW, OH);

      // 2. Restore via live production 2-stage pipeline (Algorithmic -> Adaptive SPAN 2x)
      let stage1Ms = null;
      const tAi = performance.now();
      const ai = await restorer.restoreImageFromUrl(avifUrl, {
        mediaId: `spec144_${item.name}_${Date.now()}`,
        origProfileJson: avif.origProfileJson || '',
        forceRun: true,
        onStage1: () => { stage1Ms = Math.round(performance.now() - tAi); },
      });
      const aiMs = Math.round(performance.now() - tAi);
      const cAiFull = await drawToCanvas(ai.url, 0, 0);
      const cAiAligned = await drawToCanvas(ai.url, OW, OH);
      const aiAlignedU8 = cAiAligned.getContext('2d').getImageData(0, 0, OW, OH).data;
      const aiMetrics = evalTrueMetrics(origU8, aiAlignedU8, OW, OH);

      results[item.name] = {
        origRes: `${OW}x${OH}`,
        origBytes: origBytes.byteLength,
        avifBytes: avif.bytes.byteLength,
        ratio: Number((origBytes.byteLength / avif.bytes.byteLength).toFixed(2)),
        encMs,
        modelId: ai.modelId,
        modelLabel: ai.modelLabel,
        backend: ai.backend,
        outRes: `${cAiFull.width}x${cAiFull.height}`,
        stage1Ms,
        aiMs,
        trueMetrics_avif_vs_orig: avifMetrics,
        trueMetrics_ai_vs_orig: aiMetrics,
        origCropPng: cropPng(cOrig, item.crop),
        avifCropPng: cropPng(cAvif, item.crop),
        aiCropPng: cropPng(cAiFull, item.crop),
        diffPng: diffHeatmapPng(origU8, aiAlignedU8, OW, OH),
      };
    }
    return results;
  }, { payloads });

  for (const [name, r] of Object.entries(report)) {
    fs.writeFileSync(path.join(OUT_DIR, `${name}_01_orig_400pct.png`), Buffer.from(r.origCropPng, 'base64'));
    fs.writeFileSync(path.join(OUT_DIR, `${name}_02_avif_400pct.png`), Buffer.from(r.avifCropPng, 'base64'));
    fs.writeFileSync(path.join(OUT_DIR, `${name}_03_ai_span2x_400pct.png`), Buffer.from(r.aiCropPng, 'base64'));
    fs.writeFileSync(path.join(OUT_DIR, `${name}_04_error_diff_6x.png`), Buffer.from(r.diffPng, 'base64'));
    delete r.origCropPng; delete r.avifCropPng; delete r.aiCropPng; delete r.diffPng;
  }
  console.log('SPEC 144 TRUE GROUND-TRUTH E2E REPORT:', JSON.stringify({ report, pageErrors }, null, 2));

  for (const [name, r] of Object.entries(report)) {
    if (r.modelId !== 'nomosuni_span_x2') throw new Error(`${name}: expected nomosuni_span_x2, got ${r.modelId}`);
    if (r.trueMetrics_ai_vs_orig.trueSsim < 0.96) throw new Error(`${name}: true SSIM too low (${r.trueMetrics_ai_vs_orig.trueSsim})`);
    if (r.trueMetrics_ai_vs_orig.edgeCorr < 0.96) throw new Error(`${name}: edgeCorr too low (${r.trueMetrics_ai_vs_orig.edgeCorr})`);
  }
  if (pageErrors.length > 0) throw new Error('Page errors: ' + pageErrors.join('; '));
  console.log('ALL SPEC 144 TRUE GROUND-TRUTH ASSERTIONS PASSED!');
} finally {
  await browser.close();
}
