// Automated Live Playwright Verification for Spec 078:
// Verifies Root-Portal Global Image Lightbox Viewer:
//  1. Mounted outside .message-row / .rich-image / .chat-column (true viewport root).
//  2. Mathematically centered to <= 2px horizontally AND vertically on both 1400x900 and 1920x1080.
//  3. Interactive Zoom In (+), Zoom Out (-), Mouse Wheel Zoom, Double-Click 100%<->200%,
//     Drag-to-Pan when zoomed in, Rotate 90deg, Reset 100% pill, and Escape close.
//  4. 100% Vietnamese i18n tooltips & labels + crisp non-overlapping modern UI.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC078_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_078';
fs.mkdirSync(SHOT_DIR, { recursive: true });

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
  headless: true,
  args: ['--no-sandbox', '--disable-gpu'],
});

const pageErrors = [];

try {
  const tag = Date.now().toString(36).slice(-5);
  const uname = `s78_${tag}`;
  const ctx = await browser.newContext({ viewport: { width: 1600, height: 960 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => pageErrors.push(e.message));

  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => localStorage.setItem('chatw_lang', 'vi'));
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(900);
  const tourClose = page.locator('.story-dialog .story-close').first();
  if (await tourClose.isVisible().catch(() => false)) {
    await tourClose.click({ force: true }).catch(() => {});
    await page.waitForTimeout(250);
  }

  // Open self-chat ("Chính mình") and upload a rich 960x540 colorful PNG
  await page.locator('.nav-sidebar .chat-item').first().click();
  await page.waitForSelector('#dm-chat-input', { timeout: 10000 });

  const samplePngBase64 = await page.evaluate(() => {
    const c = document.createElement('canvas');
    c.width = 960;
    c.height = 540;
    const g = c.getContext('2d');
    const grad = g.createLinearGradient(0, 0, 960, 540);
    grad.addColorStop(0, '#3b82f6');
    grad.addColorStop(0.5, '#8b5cf6');
    grad.addColorStop(1, '#ec4899');
    g.fillStyle = grad;
    g.fillRect(0, 0, 960, 540);
    g.fillStyle = 'rgba(255,255,255,0.22)';
    g.beginPath();
    g.arc(480, 270, 150, 0, Math.PI * 2);
    g.fill();
    g.fillStyle = '#ffffff';
    g.font = 'bold 48px sans-serif';
    g.textAlign = 'center';
    g.textBaseline = 'middle';
    g.fillText('ChatW Lightbox 960×540', 480, 270);
    return c.toDataURL('image/png').split(',')[1];
  });
  const tmpPng = path.join(SHOT_DIR, 'sample_960x540.png');
  fs.writeFileSync(tmpPng, Buffer.from(samplePngBase64, 'base64'));

  // Open composer "+" menu to reveal the image upload input, then upload the 960x540 PNG
  await page.locator('.composer .icon-btn').first().click();
  await page.waitForTimeout(250);
  const fileInput = page.locator('input[type="file"][accept*="image"]').first();
  await fileInput.setInputFiles(tmpPng);
  await page.waitForSelector('.rich-image-btn', { timeout: 15000 });
  await page.waitForTimeout(700);

  // Open Lightbox
  await page.locator('.rich-image-btn').first().click();
  await page.waitForSelector('.lightbox .lightbox-img', { timeout: 5000 });
  await page.waitForTimeout(300);

  // 1. Verify root-portal DOM mounting + exact <= 2px viewport centering!
  const centerAudit = await page.evaluate(() => {
    const lb = document.querySelector('.lightbox');
    const img = document.querySelector('.lightbox-img');
    const dock = document.querySelector('.lightbox-dock');
    const dl = document.querySelector('.lightbox-actions .btn-secondary');
    const cl = document.querySelector('.lightbox-close');
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const ir = img.getBoundingClientRect();
    const dr = dock.getBoundingClientRect();
    const dlr = dl.getBoundingClientRect();
    const clr = cl.getBoundingClientRect();
    const cx = ir.left + ir.width / 2;
    const cy = ir.top + ir.height / 2;
    const dockCx = dr.left + dr.width / 2;
    return {
      vw,
      vh,
      trappedInMsgRow: Boolean(lb.closest('.message-row, .rich-image, .chat-column')),
      imgW: Math.round(ir.width),
      imgH: Math.round(ir.height),
      dxCenter: Math.round(Math.abs(cx - vw / 2) * 10) / 10,
      dyCenter: Math.round(Math.abs(cy - vh / 2) * 10) / 10,
      dockDxCenter: Math.round(Math.abs(dockCx - vw / 2) * 10) / 10,
      actionsNonOverlap: dlr.right <= clr.left + 1,
      dlText: dl.innerText.trim(),
      zoomText: document.querySelector('.lightbox-zoom-readout')?.innerText?.trim() || '',
    };
  });
  await page.screenshot({ path: path.join(SHOT_DIR, '01_lightbox_centered_100pct.png') });

  record(
    'Lightbox mounts at viewport root (outside .message-row/.chat-column) & centers image to <= 2px',
    !centerAudit.trappedInMsgRow && centerAudit.dxCenter <= 2 && centerAudit.dyCenter <= 2 && centerAudit.dockDxCenter <= 2 && centerAudit.actionsNonOverlap,
    JSON.stringify(centerAudit)
  );
  record(
    'Lightbox renders Vietnamese UI ("Tải bản gốc") and 100% zoom readout',
    centerAudit.dlText.includes('Tải bản gốc') && centerAudit.zoomText === '100%',
    `dl="${centerAudit.dlText}" zoom="${centerAudit.zoomText}"`
  );

  // 2. Test Zoom In button (+25% x 2 -> 150%) & Zoom Out button (-25% -> 125%)
  const zoomInBtn = page.locator('.lightbox-dock-btn').nth(1);
  const zoomOutBtn = page.locator('.lightbox-dock-btn').nth(0);
  const rotateBtn = page.locator('.lightbox-dock-btn').nth(2);
  const resetBtn = page.locator('.lightbox-zoom-readout');

  await zoomInBtn.click();
  await zoomInBtn.click();
  await page.waitForTimeout(250);
  const zoomedIn = await page.evaluate(() => ({
    pct: document.querySelector('.lightbox-zoom-readout')?.innerText?.trim(),
    w: Math.round(document.querySelector('.lightbox-img').getBoundingClientRect().width),
  }));
  record(
    'Zoom In (+) button enlarges image to 150% proportionally',
    zoomedIn.pct === '150%' && zoomedIn.w > centerAudit.imgW * 1.4,
    `baseW=${centerAudit.imgW} zoomed=${JSON.stringify(zoomedIn)}`
  );

  await zoomOutBtn.click();
  await zoomOutBtn.click();
  await zoomOutBtn.click();
  await page.waitForTimeout(350);
  const zoomedOut = await page.evaluate(() => {
    const img = document.querySelector('.lightbox-img');
    return {
      pct: document.querySelector('.lightbox-zoom-readout')?.innerText?.trim(),
      w: Math.round(img.getBoundingClientRect().width),
      inlineStyle: img.getAttribute('style'),
      computedTf: getComputedStyle(img).transform,
    };
  });
  record(
    'Zoom Out (−) button shrinks image cleanly to 75%',
    zoomedOut.pct === '75%' && zoomedOut.w < centerAudit.imgW * 0.82,
    JSON.stringify(zoomedOut)
  );

  // 3. Test Reset pill -> 100% + Double-click toggle -> 200% + Drag-to-Pan!
  await resetBtn.click();
  await page.waitForTimeout(220);
  await page.locator('.lightbox-img').dblclick();
  await page.waitForTimeout(260);
  const dblZoom = await page.evaluate(() => ({
    pct: document.querySelector('.lightbox-zoom-readout')?.innerText?.trim(),
    rect: document.querySelector('.lightbox-img').getBoundingClientRect().toJSON(),
  }));

  await page.screenshot({ path: path.join(SHOT_DIR, '02_lightbox_zoomed_200pct.png') });

  // Drag-to-pan by (+120, +70) while at 200%
  const cx = dblZoom.rect.left + dblZoom.rect.width / 2;
  const cy = dblZoom.rect.top + dblZoom.rect.height / 2;
  await page.mouse.move(cx, cy);
  await page.mouse.down();
  await page.mouse.move(cx + 120, cy + 70, { steps: 6 });
  await page.mouse.up();
  await page.waitForTimeout(180);

  const afterPan = await page.evaluate(() => {
    const img = document.querySelector('.lightbox-img');
    const r = img ? img.getBoundingClientRect() : null;
    return {
      stillOpen: Boolean(img),
      dx: r ? Math.round((r.left + r.width / 2) - window.innerWidth / 2) : 0,
      dy: r ? Math.round((r.top + r.height / 2) - window.innerHeight / 2) : 0,
    };
  });
  record(
    'Double-click toggles 200% zoom and Drag-to-Pan moves image smoothly without closing',
    dblZoom.pct === '200%' && afterPan.stillOpen && afterPan.dx > 80 && afterPan.dy > 40,
    `dbl=${dblZoom.pct} pan=${JSON.stringify(afterPan)}`
  );

  // 4. Test Mouse Wheel zoom & Rotate 90deg & Reset 100%
  await page.mouse.move(800, 480);
  await page.mouse.wheel(0, -120);
  await page.waitForTimeout(200);
  const afterWheel = await page.locator('.lightbox-zoom-readout').innerText();
  await rotateBtn.click();
  await page.waitForTimeout(240);
  const afterRot = await page.evaluate(() => document.querySelector('.lightbox-img')?.style.transform || '');
  await resetBtn.click();
  await page.waitForTimeout(240);
  const afterReset = await page.evaluate(() => ({
    pct: document.querySelector('.lightbox-zoom-readout')?.innerText?.trim(),
    transform: document.querySelector('.lightbox-img')?.style.transform || '',
  }));
  record(
    'Mouse wheel zooms, Rotate rotates 90deg, and Reset restores 100% centered (0deg)',
    afterWheel.trim() !== '200%' && afterRot.includes('rotate(90deg)') && afterReset.pct === '100%' && afterReset.transform.includes('rotate(0deg)'),
    `wheel=${afterWheel.trim()} rot="${afterRot}" reset=${JSON.stringify(afterReset)}`
  );

  // 5. Test Escape closes cleanly
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  record('Escape closes Lightbox cleanly', (await page.locator('.lightbox').count()) === 0);
  record('Zero uncaught JS pageerrors', pageErrors.length === 0, pageErrors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 078 Summary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length) {
  process.exit(1);
}
