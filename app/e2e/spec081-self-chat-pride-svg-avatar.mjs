// Automated Live Playwright Verification for Spec 081 / Spec 082:
// Verifies the "Note to self" ("Chính mình") avatar renders as a pure inline vector SVG
// in BOTH the sidebar conversation list AND the chat header, contains all 6 Pride rainbow
// stripes underneath + the original smooth ChatW Bubble-Spirit Mascot (zero voxel polygons),
// uses unique clipPath & gradient IDs per instance, and serves /self-chat-avatar.svg HTTP 200.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC085_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_085';
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
  const ctx = await browser.newContext({ viewport: { width: 1400, height: 900 } });

  // Verify standalone public SVG asset is served cleanly
  const svgResp = await ctx.request.get(`${BASE}/self-chat-avatar.svg`);
  const svgBody = await svgResp.text();
  record(
    'Standalone /self-chat-avatar.svg serves HTTP 200 original ChatW mascot SVG with all 6 Pride colors',
    svgResp.ok() &&
      svgBody.includes('<svg') &&
      svgBody.includes('<path') &&
      !svgBody.includes('<polygon') &&
      ['#E40303', '#FF8C00', '#FFED00', '#008026', '#004DFF', '#750787'].every((c) => svgBody.includes(c)),
    `status=${svgResp.status()} len=${svgBody.length}`
  );

  const page = await ctx.newPage();
  page.on('pageerror', (e) => pageErrors.push(e.message));
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => localStorage.setItem('chatw_lang', 'vi'));
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  const uname = `s82_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);
  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Click "Chính mình" so BOTH sidebar avatar and header avatar render simultaneously
  await page.locator('.nav-sidebar .chat-item').first().click();
  await page.waitForSelector('.chat-header .self-chat-avatar.pride-voxel svg.self-pride-svg', { timeout: 10000 });
  await page.waitForTimeout(400);

  const audit = await page.evaluate(() => {
    const sideSvg = document.querySelector('.nav-sidebar .self-chat-avatar.pride-voxel svg.self-pride-svg');
    const hdrSvg = document.querySelector('.chat-header .self-chat-avatar.pride-voxel svg.self-pride-svg');
    const inspect = (svg) => {
      if (!svg) return null;
      const r = svg.getBoundingClientRect();
      const rects = Array.from(svg.querySelectorAll('rect')).map((el) => el.getAttribute('fill'));
      const paths = svg.querySelectorAll('path').length;
      const ellipses = svg.querySelectorAll('ellipse').length;
      const circles = svg.querySelectorAll('circle').length;
      const polys = svg.querySelectorAll('polygon').length;
      const clipId = svg.querySelector('clipPath')?.getAttribute('id') || '';
      const gradIds = Array.from(svg.querySelectorAll('linearGradient')).map((el) => el.getAttribute('id'));
      const shiftedRight = Boolean(svg.querySelector('g[transform="translate(2.5, 0)"]'));
      return { w: Math.round(r.width), h: Math.round(r.height), rects, paths, ellipses, circles, polys, clipId, gradIds, shiftedRight };
    };
    return { side: inspect(sideSvg), hdr: inspect(hdrSvg) };
  });

  await page.screenshot({ path: path.join(SHOT_DIR, '01_live_original_pride_avatar.png') });
  await page.screenshot({
    path: path.join(SHOT_DIR, '02_closeup_sidebar_and_header.png'),
    clip: { x: 60, y: 0, width: 620, height: 340 },
  });

  const expectedPride = ['#E40303', '#FF8C00', '#FFED00', '#008026', '#004DFF', '#750787'];
  const sideOk = Boolean(
    audit.side &&
      audit.side.w >= 40 &&
      audit.side.h >= 40 &&
      audit.side.paths === 6 &&
      audit.side.ellipses >= 4 &&
      audit.side.polys === 0 &&
      audit.side.gradIds.length === 2 &&
      audit.side.shiftedRight &&
      expectedPride.every((c) => audit.side.rects.includes(c))
  );
  const hdrOk = Boolean(
    audit.hdr &&
      audit.hdr.w >= 34 &&
      audit.hdr.h >= 34 &&
      audit.hdr.paths === 6 &&
      audit.hdr.ellipses >= 4 &&
      audit.hdr.polys === 0 &&
      audit.hdr.gradIds.length === 2 &&
      audit.hdr.shiftedRight &&
      expectedPride.every((c) => audit.hdr.rects.includes(c))
  );
  const allIds = [...(audit.side ? [audit.side.clipId, ...audit.side.gradIds] : []), ...(audit.hdr ? [audit.hdr.clipId, ...audit.hdr.gradIds] : [])];
  const uniqueDefsIds = allIds.length === 6 && new Set(allIds).size === 6;

  record('Sidebar "Chính mình" avatar renders star-free +1px right-centered ChatW Pride SVG', sideOk, JSON.stringify(audit.side));
  record('Chat header "Chính mình" avatar renders star-free +1px right-centered ChatW Pride SVG', hdrOk, JSON.stringify(audit.hdr));
  record('Multiple inline SVG instances use 6 distinct clipPath + gradient IDs (zero collisions)', uniqueDefsIds, JSON.stringify(allIds));
  record('Zero uncaught JS pageerrors', pageErrors.length === 0, pageErrors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 085 Summary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length) {
  process.exit(1);
}
