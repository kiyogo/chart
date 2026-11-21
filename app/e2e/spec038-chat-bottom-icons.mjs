// Live Chromium verification for Spec 038:
// - Chat column & message stream flush to bottom edge of viewport (0px bottom clipping strip)
// - Redesigned "ChatW Duo v2" sculpted icons enlarged across rail (30/28/26px), composer (24px), header (22px)
// - Transparent evenodd cutout on SettingsIcon across Light & Dark themes
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_038';
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
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`s038_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
  await page.waitForSelector('.chat-item', { timeout: 10000 });

  // Open "Chính mình" self-chat
  await page.locator('.chat-item').first().click();
  await page.waitForSelector('.chat-column .composer-input', { timeout: 10000 });

  // Send enough messages so the stream overflows behind the composer
  const input = page.locator('.composer-input');
  for (let i = 1; i <= 14; i++) {
    await input.fill(`Tin nhắn kiểm thử chạm mép dưới số ${i} — kiểm tra tràn toàn màn hình`);
    await input.press('Enter');
    await page.waitForTimeout(90);
  }
  await page.waitForTimeout(400);

  // Scroll up slightly so newer bubbles sit behind and below the floating composer pill
  await page.evaluate(() => {
    const sc = document.querySelector('.chat-messages');
    if (sc) sc.scrollTop = -120;
  });
  await page.waitForTimeout(250);

  const audit = await page.evaluate(() => {
    const vh = window.innerHeight;
    const col = document.querySelector('.chat-column');
    const msgs = document.querySelector('.chat-messages');
    const comp = document.querySelector('.composer');
    const compBox = document.querySelector('.composer-box');
    if (!col || !msgs || !comp || !compBox) return null;

    const colR = col.getBoundingClientRect();
    const msgR = msgs.getBoundingClientRect();
    const compR = comp.getBoundingClientRect();
    const colCs = getComputedStyle(col);
    const compCs = getComputedStyle(comp);

    const brandSvg = document.querySelector('.rail-brand svg.cw-icon');
    const topNavSvgs = Array.from(document.querySelectorAll('.rail-nav .rail-item:not(.has-logo) svg.cw-icon'));
    const bottomNavSvgs = Array.from(document.querySelectorAll('.rail-bottom .rail-item svg.cw-icon'));
    const composerSvgs = Array.from(document.querySelectorAll('.composer-box svg.cw-icon'));
    const settingsSvg = document.querySelector('.rail-bottom button[aria-label="Cài đặt"] svg.cw-icon');
    const settingsFillPath = settingsSvg?.querySelector('path[fill-rule="evenodd"]');
    const inactiveDuoFills = Array.from(document.querySelectorAll('.app-rail .rail-item:not(.active):not(.has-logo) svg.cw-icon [fill-opacity]'))
      .map((el) => parseFloat(getComputedStyle(el).fillOpacity));
    const activeDuoFills = Array.from(document.querySelectorAll('.app-rail .rail-item.active:not(.has-logo) svg.cw-icon [fill-opacity]'))
      .map((el) => parseFloat(getComputedStyle(el).fillOpacity));

    const dims = (els) => els.map((s) => `${Math.round(s.getBoundingClientRect().width)}x${Math.round(s.getBoundingClientRect().height)}`);

    // Check whether at least one message row extends below composer-box bottom toward viewport bottom
    const boxR = compBox.getBoundingClientRect();
    const rows = Array.from(document.querySelectorAll('.chat-messages .message-row'));
    const lowestRowBottom = rows.reduce((mx, r) => Math.max(mx, r.getBoundingClientRect().bottom), 0);

    return {
      vh,
      colBottom: Math.round(colR.bottom * 10) / 10,
      msgBottom: Math.round(msgR.bottom * 10) / 10,
      compBottom: Math.round(compR.bottom * 10) / 10,
      colBottomGap: Math.round((vh - colR.bottom) * 10) / 10,
      msgBottomGap: Math.round((vh - msgR.bottom) * 10) / 10,
      colBottomLeftRadius: colCs.borderBottomLeftRadius,
      colBottomRightRadius: colCs.borderBottomRightRadius,
      compBg: compCs.backgroundColor,
      boxBottom: Math.round(boxR.bottom),
      lowestRowBottom: Math.round(lowestRowBottom),
      brandSize: brandSvg ? `${Math.round(brandSvg.getBoundingClientRect().width)}x${Math.round(brandSvg.getBoundingClientRect().height)}` : null,
      topNavSizes: dims(topNavSvgs),
      bottomNavSizes: dims(bottomNavSvgs),
      composerSizes: dims(composerSvgs),
      settingsHasEvenOddCutout: !!settingsFillPath,
      inactiveDuoFills,
      activeDuoFills,
    };
  });

  record(
    'Chat column extends flush to viewport bottom (0px strip, 0px bottom border-radius)',
    !!(
      audit &&
      Math.abs(audit.colBottomGap) <= 1 &&
      Math.abs(audit.msgBottomGap) <= 1 &&
      audit.colBottomLeftRadius === '0px' &&
      audit.colBottomRightRadius === '0px' &&
      audit.compBg === 'rgba(0, 0, 0, 0)'
    ),
    JSON.stringify({
      vh: audit?.vh,
      colBottom: audit?.colBottom,
      msgBottom: audit?.msgBottom,
      colBottomGap: audit?.colBottomGap,
      radii: [audit?.colBottomLeftRadius, audit?.colBottomRightRadius],
      compBg: audit?.compBg,
    }),
  );

  record(
    'Messages visibly scroll behind floating composer down toward bottom viewport edge',
    !!(audit && audit.lowestRowBottom > audit.boxBottom),
    JSON.stringify({ boxBottom: audit?.boxBottom, lowestRowBottom: audit?.lowestRowBottom, vh: audit?.vh }),
  );

  record(
    'Rail brand (30px), top nav (28px), bottom nav (26px), and composer (24px) icons enlarged cleanly',
    !!(
      audit &&
      audit.brandSize === '30x30' &&
      audit.topNavSizes.length >= 6 &&
      audit.topNavSizes.every((s) => s === '28x28') &&
      audit.bottomNavSizes.length === 3 &&
      audit.bottomNavSizes.every((s) => s === '26x26') &&
      audit.composerSizes.length >= 4 &&
      audit.composerSizes.every((s) => s === '24x24')
    ),
    JSON.stringify({
      brand: audit?.brandSize,
      top: audit?.topNavSizes,
      bottom: audit?.bottomNavSizes,
      composer: audit?.composerSizes,
    }),
  );

  record(
    'SettingsIcon uses transparent SVG evenodd cutout (no white center patch)',
    !!(audit && audit.settingsHasEvenOddCutout),
    JSON.stringify({ settingsHasEvenOddCutout: audit?.settingsHasEvenOddCutout }),
  );

  record(
    'Inactive icons have 0% muddy gray interior fill while active rail icon gets clean luminous accent tint',
    !!(
      audit &&
      audit.inactiveDuoFills.length > 0 &&
      audit.inactiveDuoFills.every((v) => v === 0) &&
      audit.activeDuoFills.length > 0 &&
      audit.activeDuoFills.every((v) => v > 0.1 && v <= 0.22)
    ),
    JSON.stringify({ inactiveCount: audit?.inactiveDuoFills?.length, inactiveMax: Math.max(...(audit?.inactiveDuoFills || [0])), activeFills: audit?.activeDuoFills }),
  );

  // Hover one message row so we also capture the redesigned message hover action bar
  await page.locator('.message-row').nth(8).hover().catch(() => {});
  await page.waitForTimeout(150);

  await page.screenshot({ path: path.join(OUT, '01_full_chat_light.png') });
  await page.screenshot({
    path: path.join(OUT, '02_chat_bottom_zoom_light.png'),
    clip: { x: 350, y: 630, width: 1090, height: 270 },
  });
  await page.screenshot({
    path: path.join(OUT, '03_rail_zoom_light.png'),
    clip: { x: 0, y: 0, width: 120, height: 900 },
  });

  // Switch to Dark theme to verify icons & bottom flush in Dark mode too
  await page.evaluate(() => {
    document.documentElement.setAttribute('data-theme', 'dark');
  });
  await page.waitForTimeout(200);
  await page.screenshot({ path: path.join(OUT, '04_full_chat_dark.png') });
  await page.screenshot({
    path: path.join(OUT, '05_rail_zoom_dark.png'),
    clip: { x: 0, y: 0, width: 120, height: 900 },
  });

  record('No uncaught page errors', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 038 check: ${checks.length - failed.length}/${checks.length} passed.`);
if (failed.length > 0) process.exit(1);
