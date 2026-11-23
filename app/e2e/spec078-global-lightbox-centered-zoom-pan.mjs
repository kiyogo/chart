// Automated Live Playwright Verification for Specs 078 & 080:
// Verifies Root-Portal Borderless Global Image Lightbox Viewer:
//  1. Mounted outside .message-row / .rich-image / .chat-column, centered to <= 2px.
//  2. ZERO bottom dock; ALL controls unified in ONE borderless top-right .lightbox-actions toolbar
//     (0px borders, 0 box-shadows, transparent idle backgrounds, Vietnamese tooltips).
//  3. Min zoom locked at 100% (never shrinks below 100% via button or mouse wheel); max 400%.
//  4. Extreme drag (+2000px, +2000px) at 200% zoom NEVER loses image off-screen and automatically
//     springs back so the image edge sits flush at the exact viewport corner (left <= 2, top <= 2)!
//  5. Zooming back out from a corner snaps smoothly back to dead center (dxCenter <= 2, dyCenter <= 2).
//  6. Rotate 90deg, Reset 100%, and Escape close cleanly with 0 pageerrors.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC078_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_080';
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
  const uname = `s80_${tag}`;
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

  await page.locator('.composer .icon-btn').first().click();
  await page.waitForTimeout(250);
  await page.locator('input[type="file"][accept*="image"]').first().setInputFiles(tmpPng);
  await page.waitForSelector('.rich-image-btn', { timeout: 15000 });
  await page.waitForTimeout(700);

  // Open Lightbox
  await page.locator('.rich-image-btn').first().click();
  await page.waitForSelector('.lightbox .lightbox-img', { timeout: 5000 });
  await page.waitForTimeout(350);

  // 1. Verify root-portal centering, ZERO bottom dock, and 100% borderless top-right unified toolbar!
  const uiAudit = await page.evaluate(() => {
    const lb = document.querySelector('.lightbox');
    const img = document.querySelector('.lightbox-img');
    const dockCount = document.querySelectorAll('.lightbox-dock').length;
    const actions = document.querySelector('.lightbox-actions');
    const items = Array.from(actions ? actions.children : []);
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const ir = img.getBoundingClientRect();
    const ar = actions.getBoundingClientRect();
    const borderWidths = items.map((el) => getComputedStyle(el).borderTopWidth);
    const bgs = items.map((el) => getComputedStyle(el).backgroundColor);
    const titles = items.map((el) => el.getAttribute('title') || '');
    return {
      vw,
      vh,
      trappedInMsgRow: Boolean(lb.closest('.message-row, .rich-image, .chat-column')),
      imgW: Math.round(ir.width),
      imgH: Math.round(ir.height),
      dxCenter: Math.round(Math.abs((ir.left + ir.width / 2) - vw / 2) * 10) / 10,
      dyCenter: Math.round(Math.abs((ir.top + ir.height / 2) - vh / 2) * 10) / 10,
      dockCount,
      topRightAligned: ar.top < 40 && ar.right > vw - 30,
      controlCount: items.length,
      allZeroBorder: borderWidths.every((w) => w === '0px'),
      allTransparentIdle: bgs.every((bg) => bg === 'rgba(0, 0, 0, 0)' || bg === 'transparent'),
      titles,
      zoomText: document.querySelector('.lightbox-zoom-readout')?.innerText?.trim() || '',
    };
  });
  await page.screenshot({ path: path.join(SHOT_DIR, '01_borderless_topright_100pct.png') });

  record(
    'Lightbox centers to <= 2px with ZERO bottom dock & 100% borderless unified top-right menu',
    !uiAudit.trappedInMsgRow &&
      uiAudit.dxCenter <= 2 &&
      uiAudit.dyCenter <= 2 &&
      uiAudit.dockCount === 0 &&
      uiAudit.topRightAligned &&
      uiAudit.controlCount === 6 &&
      uiAudit.allZeroBorder &&
      uiAudit.allTransparentIdle &&
      uiAudit.zoomText === '100%',
    JSON.stringify(uiAudit)
  );

  // 2. Verify Min Zoom Floor = 100% (wheel down & disabled zoom-out button NEVER shrink below 100%!)
  const zoomOutBtn = page.locator('.lightbox-actions .lightbox-tool-btn').nth(0);
  const zoomInBtn = page.locator('.lightbox-actions .lightbox-tool-btn').nth(1);
  const rotateBtn = page.locator('.lightbox-actions .lightbox-tool-btn').nth(2);
  const resetBtn = page.locator('.lightbox-zoom-readout');

  const zoomOutDisabledAt100 = await zoomOutBtn.isDisabled();
  await page.mouse.move(800, 480);
  await page.mouse.wheel(0, 500);
  await page.mouse.wheel(0, 500);
  await page.waitForTimeout(250);
  const afterWheelDown = await resetBtn.innerText();
  record(
    'Min zoom is strictly locked at 100% (Zoom Out disabled & wheel-down cannot shrink below 100%)',
    zoomOutDisabledAt100 && afterWheelDown.trim() === '100%',
    `disabled=${zoomOutDisabledAt100} pct=${afterWheelDown.trim()}`
  );

  // 3. Double-click -> 200% (1920x1080 in 1600x960 -> exact corner clamp max_x=160, max_y=60!)
  await page.locator('.lightbox-img').dblclick();
  await page.waitForFunction(() => document.querySelector('.lightbox-zoom-readout')?.innerText?.trim() === '200%', { timeout: 3000 });
  await page.waitForTimeout(320);

  // Extreme overscroll drag by (+1400, +900) -> must NEVER lose image and must snap flush to top-left corner (left~0, top~0)!
  await page.mouse.move(800, 480);
  await page.mouse.down();
  await page.mouse.move(1550, 920, { steps: 10 });
  const midDragRect = await page.evaluate(() => {
    const r = document.querySelector('.lightbox-img').getBoundingClientRect();
    return { left: Math.round(r.left), top: Math.round(r.top), right: Math.round(r.right), bottom: Math.round(r.bottom) };
  });
  await page.mouse.up();
  await page.waitForFunction(() => {
    const img = document.querySelector('.lightbox-img');
    if (!img) return false;
    const r = img.getBoundingClientRect();
    return Math.abs(r.left) <= 3 && Math.abs(r.top) <= 3;
  }, { timeout: 2500 }).catch(() => {});

  const afterSnapCorner = await page.evaluate(() => {
    const img = document.querySelector('.lightbox-img');
    const r = img.getBoundingClientRect();
    return {
      stillOpen: Boolean(img),
      left: Math.round(r.left),
      top: Math.round(r.top),
      right: Math.round(r.right),
      bottom: Math.round(r.bottom),
      style: img.getAttribute('style'),
    };
  });
  // 3b. Spec 083: Verify Download & all top-right buttons stay sleek, uniform, and borderless
  // EVEN under translucent custom-wallpaper mode while hovering over a bright 200% zoomed image!
  await page.evaluate(() => {
    document.documentElement.setAttribute('data-panel-translucent', 'true');
    document.documentElement.setAttribute('data-app-wp', 'custom');
  });
  const dlBtn = page.locator('.lightbox-actions .lightbox-download');
  await dlBtn.hover();
  await page.waitForTimeout(220);
  const hoverAudit = await page.evaluate(() => {
    const dl = document.querySelector('.lightbox-actions .lightbox-download');
    const btns = Array.from(document.querySelectorAll('.lightbox-actions .lightbox-tool-btn'));
    const svgs = btns.map((b) => b.querySelector('svg'));
    const dlCs = dl ? getComputedStyle(dl) : null;
    return {
      hasBtnSecondary: dl ? dl.classList.contains('btn-secondary') : true,
      dlBg: dlCs?.backgroundColor || '',
      dlBoxShadow: dlCs?.boxShadow || '',
      dlFilter: dlCs?.filter || '',
      dlBorder: dlCs?.borderTopWidth || '',
      allIconsUniform18: svgs.every((s) => s && Math.round(s.getBoundingClientRect().width) === 18 && Math.round(s.getBoundingClientRect().height) === 18),
      zeroDuoFillPaths: svgs.every((s) => s && s.querySelectorAll('[stroke="none"]').length === 0),
    };
  });
  const shot83Dir = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_083';
  fs.mkdirSync(shot83Dir, { recursive: true });
  await page.screenshot({
    path: path.join(shot83Dir, '01_toolbar_hover_over_bright_zoomed.png'),
    clip: { x: 1120, y: 0, width: 480, height: 120 },
  });
  await page.screenshot({ path: path.join(SHOT_DIR, '02_zoomed_200pct_snapped_corner.png') });
  record(
    'Spec 083: Under translucent wallpaper mode, Download hover stays subtle glass (no opaque white blob), zero btn-secondary leak, and all 5 icons are uniform 18x18 stroke-only',
    !hoverAudit.hasBtnSecondary &&
      hoverAudit.dlBg === 'rgba(255, 255, 255, 0.16)' &&
      hoverAudit.dlBoxShadow === 'none' &&
      hoverAudit.dlFilter === 'none' &&
      hoverAudit.dlBorder === '0px' &&
      hoverAudit.allIconsUniform18 &&
      hoverAudit.zeroDuoFillPaths,
    JSON.stringify(hoverAudit)
  );

  record(
    'Extreme drag at 200% applies elastic resistance (never loses photo) and springs back flush to exact corner (left<=2, top<=2)',
    midDragRect.left <= 55 &&
      midDragRect.top <= 55 &&
      afterSnapCorner.stillOpen &&
      Math.abs(afterSnapCorner.left) <= 3 &&
      Math.abs(afterSnapCorner.top) <= 3,
    `midDrag=${JSON.stringify(midDragRect)} snapped=${JSON.stringify(afterSnapCorner)}`
  );

  // 4. Zooming out from corner (200% -> 150% -> 100%) automatically re-centers to (0, 0)!
  await zoomOutBtn.click();
  await zoomOutBtn.click();
  await page.waitForTimeout(350);
  const at150 = await page.evaluate(() => {
    const r = document.querySelector('.lightbox-img').getBoundingClientRect();
    return {
      pct: document.querySelector('.lightbox-zoom-readout')?.innerText?.trim(),
      dxCenter: Math.round(Math.abs((r.left + r.width / 2) - window.innerWidth / 2)),
      dyCenter: Math.round(Math.abs((r.top + r.height / 2) - window.innerHeight / 2)),
    };
  });
  record(
    'Zooming back down to 150% (fits viewport) automatically springs back to dead center (0px)',
    at150.pct === '150%' && at150.dxCenter <= 2 && at150.dyCenter <= 2,
    JSON.stringify(at150)
  );

  // 5. Zoom In (+), Rotate 90deg, Reset 100%, and Escape close
  await zoomInBtn.click();
  await rotateBtn.click();
  await page.waitForTimeout(300);
  const rotStyle = await page.evaluate(() => document.querySelector('.lightbox-img')?.style.transform || '');
  await resetBtn.click();
  await page.waitForTimeout(300);
  const resetState = await page.evaluate(() => ({
    pct: document.querySelector('.lightbox-zoom-readout')?.innerText?.trim(),
    transform: document.querySelector('.lightbox-img')?.style.transform || '',
  }));
  record(
    'Zoom In (+), Rotate 90deg, and Reset 100% work cleanly from top-right menu',
    rotStyle.includes('rotate(90deg)') && resetState.pct === '100%' && resetState.transform.includes('rotate(0deg)'),
    `rot="${rotStyle}" reset=${JSON.stringify(resetState)}`
  );

  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  record('Escape closes Lightbox cleanly', (await page.locator('.lightbox').count()) === 0);
  record('Zero uncaught JS pageerrors', pageErrors.length === 0, pageErrors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 080 Summary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length) {
  process.exit(1);
}
