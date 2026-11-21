// Live Chromium verification for Spec 040:
// - Fixed range sliders (6px slim track, always-visible knob, zero 37.5px gray pill bug)
// - All visual appearance & wallpaper settings unified inside Settings > "Giao diện"
// - New "Độ mờ đục khung trắng" (0-100%) + "Làm mờ kính khung" (0-32px) controls
// - Wallpaper dim works on preset wallpapers too, and unified reset restores solid defaults
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_040';
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
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`s040_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Open Settings
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });

  // 1. Check "Tùy chỉnh" tab is clean (retains rail pins, no duplicate app wallpaper)
  await page.locator('.settings-nav-item', { hasText: 'Tùy chỉnh' }).click();
  await page.waitForTimeout(200);
  const custText = await page.locator('.settings-body').innerText();
  record(
    'Customize tab ("Tùy chỉnh") keeps rail pin controls and removed duplicate appearance/wallpaper clutter',
    custText.includes('Ghim cộng đồng') && !custText.includes('Hình nền toàn ứng dụng'),
    `hasPins=${custText.includes('Ghim cộng đồng')}, hasAppWp=${custText.includes('Hình nền toàn ứng dụng')}`,
  );

  // 2. Open "Giao diện" tab — verify all unified look controls + panel opacity/blur controls live here
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  await page.waitForTimeout(250);
  const appText = await page.locator('.settings-body').innerText();
  record(
    'Appearance tab ("Giao diện") unifies Colors, Shapes, Chat Wallpaper, App Wallpaper & White Panel Opacity controls',
    appText.includes('Màu sắc nâng cao') &&
      appText.includes('Hình dạng và bố cục') &&
      appText.includes('Hình nền trò chuyện mặc định') &&
      appText.includes('Hình nền toàn ứng dụng') &&
      appText.includes('Độ mờ đục khung trắng') &&
      appText.includes('Làm mờ kính khung'),
    'all unified sections present',
  );

  // 3. Select "Đại dương" app wallpaper so wallpaper dim & blur sliders also appear
  const appWpSection = page.locator('.settings-group', { hasText: 'Hình nền toàn ứng dụng' });
  await appWpSection.locator('.wallpaper-tile', { hasText: 'Đại dương' }).click();
  await page.waitForTimeout(250);

  // Scroll down to show wallpaper & sliders clearly
  await page.evaluate(() => {
    const body = document.querySelector('.settings-body');
    if (body) body.scrollTop = body.scrollHeight;
  });
  await page.waitForTimeout(200);

  // 4. Verify ALL range sliders are crisp 6px slim tracks (NOT broken 37.5px giant gray pills)
  const sliderAudit = await page.evaluate(() => {
    const ranges = Array.from(document.querySelectorAll('.settings-body input[type="range"]'));
    return ranges.map((r) => {
      const rect = r.getBoundingClientRect();
      const cs = getComputedStyle(r);
      return {
        label: r.getAttribute('aria-label'),
        height: Math.round(rect.height),
        minHeight: cs.minHeight,
        width: Math.round(rect.width),
        val: r.value,
      };
    });
  });
  record(
    'All range sliders render as slim 6px tracks (zero giant gray pill bug)',
    sliderAudit.length >= 5 && sliderAudit.every((s) => s.height <= 8 && s.width >= 140),
    JSON.stringify(sliderAudit),
  );

  // 5. Adjust "Độ mờ đục khung trắng" -> 58%, "Làm mờ kính khung" -> 16px, "Độ tối phủ lên ảnh" -> 20%
  const setRange = async (ariaLabel, value) => {
    const loc = page.locator(`input[type="range"][aria-label="${ariaLabel}"]`);
    await loc.evaluate((el, v) => {
      const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
      setter.call(el, String(v));
      el.dispatchEvent(new Event('input', { bubbles: true }));
    }, value);
  };
  await setRange('Độ mờ đục khung trắng', 58);
  await setRange('Làm mờ kính khung', 16);
  await setRange('Độ tối phủ lên ảnh', 20);
  await page.waitForTimeout(300);

  const glassAudit = await page.evaluate(() => {
    const root = document.documentElement;
    const body = document.querySelector('.settings-body');
    const nav = document.querySelector('.settings-nav');
    const shell = document.querySelector('.workspace-shell');
    const bcs = getComputedStyle(body);
    const ncs = getComputedStyle(nav);
    const afterCs = getComputedStyle(shell, '::after');
    return {
      appWp: root.getAttribute('data-app-wp'),
      glass: root.getAttribute('data-panel-glass'),
      panelOpacityVar: root.style.getPropertyValue('--panel-opacity').trim(),
      panelBlurVar: root.style.getPropertyValue('--panel-blur').trim(),
      bodyBg: bcs.backgroundColor,
      bodyBackdrop: bcs.backdropFilter,
      navBg: ncs.backgroundColor,
      afterBg: afterCs.backgroundColor,
    };
  });
  record(
    'White panel opacity & frosted glass blur apply live across settings panels and dim works on preset wallpaper',
    glassAudit.appWp === 'preset-ocean' &&
      glassAudit.glass === 'true' &&
      glassAudit.panelOpacityVar === '58%' &&
      glassAudit.panelBlurVar === '16px' &&
      glassAudit.bodyBackdrop.includes('blur(16px)') &&
      !glassAudit.bodyBg.startsWith('rgb(255, 255, 255)') &&
      glassAudit.afterBg !== 'rgba(0, 0, 0, 0)',
    JSON.stringify(glassAudit),
  );

  await page.screenshot({ path: path.join(OUT, '01_appearance_sliders_glass.png') });

  // 6. Switch to Messages to verify translucent sidebar & chat column live + header & composer polish
  await page.locator('.app-rail button[aria-label="Tin nhắn"]').click();
  await page.waitForSelector('.chat-item', { timeout: 10000 });
  await page.locator('.chat-item').first().click();
  await page.waitForSelector('.chat-column', { timeout: 10000 });
  await page.waitForTimeout(250);
  const chatGlass = await page.evaluate(() => {
    const sb = getComputedStyle(document.querySelector('.nav-sidebar'));
    const cc = getComputedStyle(document.querySelector('.chat-column'));
    const header = document.querySelector('.chat-header');
    const headerCs = header ? getComputedStyle(header) : null;
    const headerBeforeCs = header ? getComputedStyle(header, '::before') : null;
    const composer = document.querySelector('.composer');
    const composerBeforeCs = composer ? getComputedStyle(composer, '::before') : null;

    return {
      sidebarBg: sb.backgroundColor,
      sidebarBlur: sb.backdropFilter,
      chatBg: cc.backgroundColor,
      chatBlur: cc.backdropFilter,
      headerBg: headerCs ? headerCs.backgroundColor : null,
      headerBorder: headerCs ? headerCs.borderBottomWidth : null,
      headerBeforeDisplay: headerBeforeCs ? headerBeforeCs.display : null,
      composerBeforeBlur: composerBeforeCs ? composerBeforeCs.backdropFilter : null,
    };
  });
  record(
    'Translucent white panels & frosted blur apply live to Messages sidebar and Chat column',
    chatGlass.sidebarBlur.includes('blur(16px)') &&
      chatGlass.chatBlur.includes('blur(16px)') &&
      !chatGlass.sidebarBg.startsWith('rgb(255, 255, 255)'),
    JSON.stringify({ sidebarBg: chatGlass.sidebarBg, chatBg: chatGlass.chatBg }),
  );
  record(
    'Top header is completely transparent without blurred/opaque dividing stripe',
    (chatGlass.headerBg === 'rgba(0, 0, 0, 0)' || chatGlass.headerBg === 'transparent') &&
      chatGlass.headerBeforeDisplay === 'none',
    JSON.stringify({ headerBg: chatGlass.headerBg, headerBeforeDisplay: chatGlass.headerBeforeDisplay }),
  );
  record(
    'Chat composer has faint frosted white translucent mist blur backdrop',
    chatGlass.composerBeforeBlur && chatGlass.composerBeforeBlur.includes('blur(14px)'),
    `composerBeforeBlur=${chatGlass.composerBeforeBlur}`,
  );
  await page.screenshot({ path: path.join(OUT, '02_chat_translucent_glass_ocean.png') });

  // 7. Return to Giao diện and click "Khôi phục giao diện mặc định" -> restores opaque solid defaults
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  await page.waitForTimeout(200);
  await page.locator('button', { hasText: 'Khôi phục giao diện mặc định' }).click();
  await page.waitForTimeout(250);
  const resetAudit = await page.evaluate(() => ({
    appWp: document.documentElement.getAttribute('data-app-wp'),
    glass: document.documentElement.getAttribute('data-panel-glass'),
    panelOpacityVar: document.documentElement.style.getPropertyValue('--panel-opacity').trim(),
    bodyBg: getComputedStyle(document.querySelector('.settings-body')).backgroundColor,
  }));
  record(
    'Unified "Khôi phục giao diện mặc định" restores solid opaque white panels and clears wallpaper',
    resetAudit.appWp === 'none' &&
      resetAudit.glass === 'false' &&
      resetAudit.panelOpacityVar === '100%' &&
      resetAudit.bodyBg === 'rgb(255, 255, 255)',
    JSON.stringify(resetAudit),
  );

  record('Zero uncaught page errors', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 040 check: ${checks.length - failed.length}/${checks.length} passed.`);
if (failed.length > 0) process.exit(1);
