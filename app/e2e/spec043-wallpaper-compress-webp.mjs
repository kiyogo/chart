// Live Chromium verification for Spec 043:
// - Clean label: "Làm mờ kính khung" without "(Frosted glass)"
// - Automatic compression of wallpapers > 1MB into WebP format (< 2MB)
// - Final wallpaper uploaded format is .webp
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_043';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

const browser = await chromium.launch({
  executablePath: '/usr/bin/google-chrome',
  headless: true,
  args: ['--no-sandbox', '--disable-gpu'],
});

const errors = [];
try {
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(e.message));

  const suffix = `${Date.now().toString(36).slice(-5)}${Math.floor(Math.random() * 90 + 10)}`;
  await page.goto(`${BASE}/`);
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`s043_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Open Settings > Giao diện
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  await page.waitForTimeout(300);

  // Scroll to wallpaper & panel settings
  await page.evaluate(() => {
    const el = document.querySelector('.settings-body');
    if (el) el.scrollTop = el.scrollHeight;
  });
  await page.waitForTimeout(200);

  // 1. Verify clean label without "(Frosted glass)"
  const bodyText = await page.locator('.settings-body').innerText();
  const hasPureLabel = bodyText.includes('Làm mờ kính khung');
  const hasNoFrostedEnglish = !bodyText.toLowerCase().includes('frosted glass') && !bodyText.toLowerCase().includes('(fro');
  record(
    'Toggle row label is purely "Làm mờ kính khung" and "(Frosted glass)" is completely removed',
    hasPureLabel && hasNoFrostedEnglish,
    `hasPureLabel=${hasPureLabel}, hasNoFrostedEnglish=${hasNoFrostedEnglish}`,
  );

  await page.screenshot({ path: path.join(OUT, '01_clean_blur_label.png'), fullPage: true });

  // 2. Generate a synthetic high-resolution image in page canvas (> 1.2 MB uncompressed)
  const testImgPath = path.join(OUT, 'test_large_wallpaper.png');
  // Create a 2000x1400 canvas with noise/gradient to produce a ~1.8 MB PNG file
  const imgBuffer = await page.evaluate(async () => {
    const canvas = document.createElement('canvas');
    canvas.width = 2000;
    canvas.height = 1400;
    const ctx = canvas.getContext('2d');
    const grad = ctx.createLinearGradient(0, 0, 2000, 1400);
    grad.addColorStop(0, '#ff7e5f');
    grad.addColorStop(0.5, '#feb47b');
    grad.addColorStop(1, '#86a8e7');
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, 2000, 1400);
    // Draw fine geometric shapes to ensure file size > 1MB
    ctx.fillStyle = 'rgba(255, 255, 255, 0.4)';
    for (let i = 0; i < 3000; i++) {
      ctx.fillRect((i * 37) % 2000, (i * 97) % 1400, 8, 8);
    }
    const blob = await new Promise((r) => canvas.toBlob(r, 'image/png'));
    const ab = await blob.arrayBuffer();
    const bytes = new Uint8Array(ab);
    let s = '';
    for (let i = 0; i < bytes.length; i += 0x8000) {
      s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
    }
    return { b64: btoa(s), size: blob.size };
  });

  fs.writeFileSync(testImgPath, Buffer.from(imgBuffer.b64, 'base64'));
  const rawSize = fs.statSync(testImgPath).size;
  record(
    `Synthetic test wallpaper created (> 1 MB, size = ${(rawSize / 1024 / 1024).toFixed(2)} MB)`,
    rawSize > 1024 * 1024,
    `size=${rawSize} bytes`,
  );

  // 3. Upload the > 1MB image via file input #app-wp-file
  const fileInput = page.locator('input#app-wp-file');
  await fileInput.setInputFiles(testImgPath);

  // Wait for upload and preference update (wait up to 10s)
  await page.waitForFunction(
    () => {
      const root = document.documentElement;
      const wp = root.getAttribute('data-app-wp');
      const err = document.querySelector('.status-badge.error');
      return wp === 'image' || (err && err.innerText.length > 0);
    },
    { timeout: 15000 },
  );

  await page.waitForTimeout(500);

  const uploadResult = await page.evaluate(() => {
    const root = document.documentElement;
    const styleBg = root.style.getPropertyValue('--app-wp-image');
    const errEl = document.querySelector('.status-badge.error');
    return {
      wpKind: root.getAttribute('data-app-wp'),
      wpImageVar: styleBg,
      errorText: errEl ? errEl.innerText : null,
    };
  });

  record(
    'Image > 1MB automatically compresses into .webp and uploads successfully without "Ảnh lớn hơn 2 MB" error',
    uploadResult.wpKind === 'image' &&
      uploadResult.wpImageVar.includes('/uploads/media/') &&
      uploadResult.wpImageVar.includes('.webp') &&
      uploadResult.errorText === null,
    JSON.stringify(uploadResult),
  );

  await page.screenshot({ path: path.join(OUT, '02_webp_wallpaper_uploaded.png'), fullPage: true });

  record('Zero unexpected runtime page errors', errors.length === 0, errors.join('; '));

} catch (err) {
  console.error('Fatal test error:', err);
  process.exitCode = 1;
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 043 Summary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length > 0) {
  process.exitCode = 1;
}
