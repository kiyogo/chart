// Live Chromium verification for Spec 024: Canvas Scrollbars & Toggle Switches
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_024';
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
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`ui_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // 1. Open User Settings -> Privacy (has multiple switches & scrollable container)
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav button', { hasText: 'Quyền riêng tư' }).click();
  await page.waitForSelector('input[type="checkbox"].switch', { timeout: 10000 });
  await page.waitForTimeout(300);

  const settingsSwitchStats = await page.evaluate(() => {
    const els = Array.from(document.querySelectorAll('input[type="checkbox"].switch'));
    return els.map((el) => {
      const r = el.getBoundingClientRect();
      const cs = getComputedStyle(el);
      const after = getComputedStyle(el, '::after');
      return {
        checked: el.checked,
        w: Math.round(r.width * 100) / 100,
        h: Math.round(r.height * 100) / 100,
        kw: after.width,
        kh: after.height,
        kleft: after.left,
        ktop: after.top,
        ktransform: after.transform,
        pad: cs.padding,
      };
    });
  });

  const allSettingsSwitchesUniform =
    settingsSwitchStats.length > 0 &&
    settingsSwitchStats.every(
      (s) => s.w === 42 && s.h === 24 && s.kw === '20px' && s.kh === '20px' && s.kleft === '2px' && s.ktop === '2px',
    );
  record(
    'Settings switches have exact 42x24 track & 20x20 knob at equidistant 2px inset',
    allSettingsSwitchesUniform,
    JSON.stringify(settingsSwitchStats.slice(0, 3)),
  );
  await page.screenshot({ path: path.join(OUT, '01_settings_switches.png') });

  // 2. Test Enterprise Settings ("Thanh bên" switches that were previously stretched to 115px!)
  await page.locator('.settings-nav button', { hasText: 'Quay lại' }).first().click().catch(() => {});
  await page.locator('.app-rail button[aria-label="Cộng đồng"]').click();
  await page.waitForTimeout(300);
  await page.locator('.create-community-btn').click();
  await page.waitForSelector('.modal-dialog', { timeout: 10000 });
  await page.locator('.modal-dialog input[type="text"]').first().fill('Doanh nghiệp Test UI');
  await page.locator('.modal-dialog input.switch[type="checkbox"]').click();
  await page.waitForTimeout(200);
  await page.screenshot({ path: path.join(OUT, '02_create_modal_switch.png') });
  await page.locator('.modal-dialog button[type="submit"]').click();
  await page.waitForSelector('.sidebar-server-header', { timeout: 10000 });
  await page.waitForTimeout(400);

  // Open Enterprise Server Settings -> Thiết lập
  await page.locator('.sidebar-server-header button[aria-label="Cài đặt cộng đồng"], .sidebar-server-header button[title*="Cài đặt"]').first().click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav button', { hasText: 'Thiết lập' }).first().click();
  await page.waitForSelector('.settings-list input.switch', { timeout: 10000 });
  await page.waitForTimeout(400);

  const entSwitchStats = await page.evaluate(() => {
    const els = Array.from(document.querySelectorAll('input[type="checkbox"].switch'));
    return els.map((el) => {
      const r = el.getBoundingClientRect();
      return {
        checked: el.checked,
        w: Math.round(r.width * 100) / 100,
        h: Math.round(r.height * 100) / 100,
      };
    });
  });
  const entSwitchesExact = entSwitchStats.length >= 5 && entSwitchStats.every((s) => s.w === 42 && s.h === 24);
  record(
    'Enterprise Settings ("Thanh bên") switches are all 42x24 (no stretching)',
    entSwitchesExact,
    `count=${entSwitchStats.length}, sample=${JSON.stringify(entSwitchStats.slice(0, 4))}`,
  );
  await page.screenshot({ path: path.join(OUT, '03_enterprise_settings_switches.png') });

  // 3. Measure Canvas Scrollbar width in Scroll/Proximity (3px) vs Hover (8px) on .settings-body & .sidebar-scrollable
  async function testScrollbarContainer(selector, label) {
    const scrollInfo = await page.evaluate((sel) => {
      const el = document.querySelector(sel);
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
        scrollH: el.scrollHeight,
        clientH: el.clientHeight,
      };
    }, selector);
    record(`${label} is scrollable`, !!(scrollInfo && scrollInfo.scrollH > scrollInfo.clientH), JSON.stringify(scrollInfo));
    if (!scrollInfo || scrollInfo.scrollH <= scrollInfo.clientH) return;

    const thumbY = Math.round(scrollInfo.top + (scrollInfo.bottom - scrollInfo.top) * 0.45);

    async function measureScrollbarPixels(rectRight, sampleY) {
      return await page.evaluate(
        ({ right, y }) => {
          const c = document.getElementById('canvas-scrollbar-overlay');
          if (!c) return { found: false };
          const ctx2 = c.getContext('2d');
          const dpr = window.devicePixelRatio || 1;
          const startX = Math.max(0, Math.floor((right - 20) * dpr));
          const endX = Math.min(c.width - 1, Math.ceil((right + 4) * dpr));
          const py = Math.floor(y * dpr);
          const data = ctx2.getImageData(startX, py, endX - startX + 1, 1).data;
          let minCol = -1;
          let maxCol = -1;
          let maxAlpha = 0;
          for (let i = 0; i <= endX - startX; i++) {
            const a = data[i * 4 + 3];
            if (a > 10) {
              if (minCol === -1) minCol = i;
              maxCol = i;
              if (a > maxAlpha) maxAlpha = a;
            }
          }
          return {
            found: minCol !== -1,
            pixelWidth: minCol !== -1 ? (maxCol - minCol + 1) / dpr : 0,
            maxAlpha,
          };
        },
        { right: rectRight, y: sampleY },
      );
    }

    // Trigger scroll (cursor away from right edge) -> expect thin ~3px scrollbar
    await page.mouse.move(scrollInfo.left + 80, thumbY);
    await page.evaluate((sel) => {
      const el = document.querySelector(sel);
      el.scrollTop = Math.round((el.scrollHeight - el.clientHeight) * 0.45);
    }, selector);
    await page.waitForTimeout(180);
    const thinSample = await measureScrollbarPixels(scrollInfo.right, thumbY);
    await page.screenshot({ path: path.join(OUT, `04_${selector.replace(/[^a-z]/g, '')}_thin.png`) });
    record(
      `${label}: scrollbar on scroll is thin (~3px)`,
      thinSample.found && thinSample.pixelWidth >= 2 && thinSample.pixelWidth <= 4.2,
      JSON.stringify(thinSample),
    );

    // Move cursor directly onto the scrollbar track/thumb (5px from right edge) -> expect expanded ~6px
    await page.mouse.move(scrollInfo.right - 5, thumbY);
    await page.waitForTimeout(380);
    const hoverSample = await measureScrollbarPixels(scrollInfo.right, thumbY);
    await page.screenshot({ path: path.join(OUT, `05_${selector.replace(/[^a-z]/g, '')}_hover.png`) });
    record(
      `${label}: scrollbar on direct hover expands visibly (~6px)`,
      hoverSample.found && hoverSample.pixelWidth >= 5.2 && hoverSample.pixelWidth <= 7.5 && hoverSample.pixelWidth > thinSample.pixelWidth + 2.0,
      `thin=${thinSample.pixelWidth}px -> hover=${hoverSample.pixelWidth}px (alpha ${thinSample.maxAlpha} -> ${hoverSample.maxAlpha})`,
    );

    // Drag thumb down 80px and verify scrollTop increases
    const beforeDrag = await page.evaluate((sel) => document.querySelector(sel).scrollTop, selector);
    await page.mouse.down();
    await page.mouse.move(scrollInfo.right - 5, thumbY + 80, { steps: 6 });
    await page.mouse.up();
    await page.waitForTimeout(100);
    const afterDrag = await page.evaluate((sel) => document.querySelector(sel).scrollTop, selector);
    record(
      `${label}: dragging canvas scrollbar thumb scrolls container`,
      afterDrag > beforeDrag + 20,
      `scrollTop ${beforeDrag} -> ${afterDrag}`,
    );

    // Move cursor away and wait for fade out
    await page.mouse.move(scrollInfo.left + 80, thumbY);
    await page.waitForTimeout(1300);
    const idleSample = await measureScrollbarPixels(scrollInfo.right, thumbY);
    record(
      `${label}: scrollbar fades out cleanly when idle`,
      !idleSample.found || idleSample.maxAlpha === 0,
      JSON.stringify(idleSample),
    );
  }

  await testScrollbarContainer('.settings-body', 'Settings body');
  await testScrollbarContainer('.sidebar-scrollable', 'Sidebar channels');

  record('No uncaught page errors', errors.length === 0, errors.join('; '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
