// Live Chromium verification for Spec 045: Adaptive Contrast & Foolproof Aesthetic Compensation Engine
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_045';
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
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`s045_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Helper: evaluate color contrast & luminance in page
  const evalContrastHelpers = async () => {
    await page.evaluate(() => {
      window.__testParseColor = (css) => {
        if (!css) return [0, 0, 0, 0];
        const m = css.match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\s*\)/i);
        if (m) return [Number(m[1]), Number(m[2]), Number(m[3]), m[4] === undefined ? 1 : Number(m[4])];
        const sm = css.match(/color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+))?\)/i);
        if (sm) return [Number(sm[1]) * 255, Number(sm[2]) * 255, Number(sm[3]) * 255, sm[4] === undefined ? 1 : Number(sm[4])];
        const hm = css.trim().match(/^#([0-9a-f]{6})$/i);
        if (hm) return [parseInt(hm[1].slice(0, 2), 16), parseInt(hm[1].slice(2, 4), 16), parseInt(hm[1].slice(4, 6), 16), 1];
        return [0, 0, 0, 0];
      };
      window.__testLum = ([r, g, b]) => {
        const lin = (v) => { const c = v / 255; return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4); };
        return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
      };
      window.__testBlend = (fg, bg) => {
        const a = fg[3];
        return [fg[0] * a + bg[0] * (1 - a), fg[1] * a + bg[1] * (1 - a), fg[2] * a + bg[2] * (1 - a), 1];
      };
      window.__testContrast = (c1, c2) => {
        const l1 = window.__testLum(c1), l2 = window.__testLum(c2);
        return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
      };
    });
  };
  await evalContrastHelpers();

  const setRange = async (ariaLabel, value) => {
    const loc = page.locator(`input[type="range"][aria-label="${ariaLabel}"]`);
    await loc.evaluate((el, v) => {
      const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
      setter.call(el, String(v));
      el.dispatchEvent(new Event('input', { bubbles: true }));
    }, value);
  };
  const setColor = async (ariaLabel, hex) => {
    const loc = page.locator(`input[type="color"][aria-label="${ariaLabel}"]`);
    await loc.evaluate((el, v) => {
      const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
      setter.call(el, v);
      el.dispatchEvent(new Event('change', { bubbles: true }));
    }, hex);
  };

  // Open Settings > Giao diện and force Light Theme first
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  await page.waitForTimeout(300);
  await page.locator('.theme-card', { hasText: 'Sáng' }).click();
  await page.waitForTimeout(250);

  // Scroll to bottom of Appearance settings
  const scrollBottom = () => page.evaluate(() => {
    const el = document.querySelector('.settings-body');
    if (el) el.scrollTop = el.scrollHeight;
  });
  await scrollBottom();
  await page.waitForTimeout(200);

  // =========================================================================
  // SCENARIO 1: Light Theme + "Đêm sao" + 24% transparency (Screenshot 1 fix)
  // =========================================================================
  // Click "Đêm sao" in App Wallpaper group (second wallpaper-grid)
  await page.locator('.wallpaper-grid').nth(1).locator('.wallpaper-tile', { hasText: 'Đêm sao' }).click();
  await setRange('Độ trong suốt khung trắng', 24);
  await page.waitForTimeout(300);

  const s1 = await page.evaluate(() => {
    const root = document.documentElement;
    const railIcon = document.querySelector('.app-rail .rail-item:not(.active):not(.has-logo)');
    const railColor = railIcon ? window.__testParseColor(getComputedStyle(railIcon).color) : [0, 0, 0, 1];
    const title = Array.from(document.querySelectorAll('.settings-group-title')).find((e) => e.textContent.includes('Hình nền toàn ứng dụng'));
    const titleRect = title ? title.getBoundingClientRect() : { height: 999 };
    const row = Array.from(document.querySelectorAll('.settings-row')).find((e) => e.textContent.includes('Độ trong suốt'));
    const rowBg = row ? window.__testParseColor(getComputedStyle(row).backgroundColor) : [0, 0, 0, 0];
    return {
      railTone: root.getAttribute('data-rail-tone'),
      surfaceTone: root.getAttribute('data-surface-tone'),
      railLum: Number(window.__testLum(railColor).toFixed(3)),
      titleHeight: Math.round(titleRect.height),
      rowBgAlpha: Number(rowBg[3].toFixed(2)),
      rowBgLum: Number(window.__testLum(rowBg).toFixed(2)),
    };
  });
  record(
    'Scenario 1 (Light + Đêm sao + 24%): Rail auto-flips to crisp light icons, heading icon inline, luminous cards',
    s1.railTone === 'dark' && s1.railLum > 0.65 && s1.titleHeight <= 36 && s1.rowBgLum > 0.9,
    JSON.stringify(s1),
  );
  await page.screenshot({ path: path.join(OUT, '01_light_night_24pct.png') });

  // =========================================================================
  // SCENARIO 2: Light Theme + "Chấm bi" + 100% transparency + 80% dim (Screenshots 3 & 4 fix)
  // =========================================================================
  await page.locator('.wallpaper-grid').nth(1).locator('.wallpaper-tile', { hasText: 'Chấm bi' }).click();
  await setRange('Độ trong suốt khung trắng', 100);
  await setRange('Độ tối phủ lên ảnh', 80);
  await page.waitForTimeout(300);

  const s2 = await page.evaluate(() => {
    const root = document.documentElement;
    const row = Array.from(document.querySelectorAll('.settings-row')).find((e) => e.textContent.includes('Độ trong suốt'));
    const label = row ? row.querySelector('.settings-label') : null;
    const labelColor = label ? window.__testParseColor(getComputedStyle(label).color) : [0, 0, 0, 1];
    const rowBg = row ? window.__testParseColor(getComputedStyle(row).backgroundColor) : [0, 0, 0, 0];
    const compRowBg = window.__testBlend(rowBg, [48, 48, 48, 1]);
    const resetBtn = Array.from(document.querySelectorAll('button')).find((b) => b.textContent.includes('Khôi phục giao diện'));
    const btnBg = resetBtn ? window.__testParseColor(getComputedStyle(resetBtn).backgroundColor) : [255, 255, 255, 1];
    const btnFg = resetBtn ? window.__testParseColor(getComputedStyle(resetBtn).color) : [0, 0, 0, 1];
    const compBtnBg = window.__testBlend(btnBg, [48, 48, 48, 1]);
    return {
      surfaceTone: root.getAttribute('data-surface-tone'),
      railTone: root.getAttribute('data-rail-tone'),
      labelLum: Number(window.__testLum(labelColor).toFixed(3)),
      labelContrast: Number(window.__testContrast(labelColor, compRowBg).toFixed(2)),
      btnNotBlindingWhite: window.__testLum(compBtnBg) < 0.25,
      btnContrast: Number(window.__testContrast(btnFg, compBtnBg).toFixed(2)),
    };
  });
  record(
    'Scenario 2 (Light + Chấm bi + 100% trans + 80% dim): Auto-flips to dark surface, >= 8:1 text contrast, dark buttons',
    s2.surfaceTone === 'dark' && s2.railTone === 'dark' && s2.labelLum > 0.7 && s2.labelContrast >= 8 && s2.btnNotBlindingWhite && s2.btnContrast >= 7,
    JSON.stringify(s2),
  );
  await page.screenshot({ path: path.join(OUT, '02_light_dots_100trans_80dim.png') });

  // =========================================================================
  // SCENARIO 3: Light Theme + Busy High-Contrast Photo (White/Pink + Dark Navy) + 100% trans + 0% dim + Blur OFF (Screenshot 6 fix)
  // =========================================================================
  await setRange('Độ tối phủ lên ảnh', 0);
  const busyImgPath = path.join(OUT, 'busy_anime_contrast.png');
  const busyBuf = await page.evaluate(async () => {
    const c = document.createElement('canvas');
    c.width = 1200; c.height = 800;
    const g = c.getContext('2d');
    // Left + upper-right bright cream/white/pink (like anime hair + bear), lower-center & bottom dark navy uniform
    g.fillStyle = '#FAF6F0';
    g.fillRect(0, 0, 1200, 800);
    g.fillStyle = '#F7B7C8';
    g.beginPath(); g.arc(880, 260, 210, 0, Math.PI * 2); g.fill();
    g.fillStyle = '#192138';
    g.fillRect(280, 380, 920, 420);
    // High-contrast ink stripes on left behind nav
    g.fillStyle = '#1E2538';
    for (let y = 0; y < 800; y += 60) {
      g.fillRect(40, y, 220, 26);
    }
    const blob = await new Promise((r) => c.toBlob(r, 'image/png'));
    return Array.from(new Uint8Array(await blob.arrayBuffer()));
  });
  fs.writeFileSync(busyImgPath, Buffer.from(busyBuf));
  await page.locator('#app-wp-file').setInputFiles(busyImgPath);
  await page.waitForFunction(() => document.documentElement.getAttribute('data-app-wp') === 'image', null, { timeout: 15000 });
  await page.waitForFunction(() => document.documentElement.getAttribute('data-wp-busy') === 'true', null, { timeout: 10000 });
  await page.waitForTimeout(350);

  const s3 = await page.evaluate(() => {
    const root = document.documentElement;
    const bodyBlur = getComputedStyle(document.querySelector('.settings-body')).backdropFilter;
    const row = Array.from(document.querySelectorAll('.settings-row')).find((e) => e.textContent.includes('Độ trong suốt'));
    const rowBg = window.__testParseColor(getComputedStyle(row).backgroundColor);
    const labelFg = window.__testParseColor(getComputedStyle(row.querySelector('.settings-label')).color);
    // Worst-case pixel contrast: over pitch-dark navy (#192138) AND over pure white (#FFFFFF)!
    const compOverNavy = window.__testBlend(rowBg, [25, 33, 56, 1]);
    const compOverWhite = window.__testBlend(rowBg, [255, 255, 255, 1]);
    const navItem = document.querySelector('.settings-nav-item:not(.active)');
    const navBg = window.__testParseColor(getComputedStyle(navItem).backgroundColor);
    const navFg = window.__testParseColor(getComputedStyle(navItem).color);
    const compNavNavy = window.__testBlend(navBg, [25, 33, 56, 1]);
    const compNavWhite = window.__testBlend(navBg, [255, 255, 255, 1]);
    return {
      wpBusy: root.getAttribute('data-wp-busy'),
      panelScrim: root.getAttribute('data-panel-scrim'),
      panelGlass: root.getAttribute('data-panel-glass'),
      surfaceTone: root.getAttribute('data-surface-tone'),
      bodyBlur,
      rowContrastNavy: Number(window.__testContrast(labelFg, compOverNavy).toFixed(2)),
      rowContrastWhite: Number(window.__testContrast(labelFg, compOverWhite).toFixed(2)),
      navContrastNavy: Number(window.__testContrast(navFg, compNavNavy).toFixed(2)),
      navContrastWhite: Number(window.__testContrast(navFg, compNavWhite).toFixed(2)),
    };
  });
  record(
    'Scenario 3 (Busy anime-like photo + 100% trans + 0% dim + Blur OFF): No forced blur & >= 8:1 contrast over BOTH navy and white',
    s3.wpBusy === 'true' && s3.panelScrim === 'true' && s3.panelGlass === 'false' && s3.bodyBlur === 'none' &&
      s3.rowContrastNavy >= 8 && s3.rowContrastWhite >= 8 && s3.navContrastNavy >= 7.5 && s3.navContrastWhite >= 7.5,
    JSON.stringify(s3),
  );
  await page.screenshot({ path: path.join(OUT, '03_light_busy_photo_100trans_0dim.png') });

  // =========================================================================
  // SCENARIO 4: Same Busy Photo + 100% trans + 35% dim (Screenshot 7 fix)
  // =========================================================================
  await setRange('Độ tối phủ lên ảnh', 35);
  await page.waitForTimeout(300);
  const s4 = await page.evaluate(() => {
    const root = document.documentElement;
    const row = Array.from(document.querySelectorAll('.settings-row')).find((e) => e.textContent.includes('Độ trong suốt'));
    const rowBg = window.__testParseColor(getComputedStyle(row).backgroundColor);
    const labelFg = window.__testParseColor(getComputedStyle(row.querySelector('.settings-label')).color);
    const compOverWhite = window.__testBlend(rowBg, [255, 255, 255, 1]);
    const compOverNavy = window.__testBlend(rowBg, [16, 20, 32, 1]);
    return {
      surfaceTone: root.getAttribute('data-surface-tone'),
      contrastOverWhite: Number(window.__testContrast(labelFg, compOverWhite).toFixed(2)),
      contrastOverNavy: Number(window.__testContrast(labelFg, compOverNavy).toFixed(2)),
    };
  });
  record(
    'Scenario 4 (Busy photo + 35% dim + 100% trans): Auto-flips to crisp dark surface with >= 8:1 contrast everywhere',
    s4.surfaceTone === 'dark' && s4.contrastOverWhite >= 8 && s4.contrastOverNavy >= 10,
    JSON.stringify(s4),
  );
  await page.screenshot({ path: path.join(OUT, '04_light_busy_photo_100trans_35dim.png') });

  // =========================================================================
  // SCENARIO 5: Dark Theme + Pure White Wallpaper (#FFFFFF) + 100% trans + 0% dim
  // =========================================================================
  await page.evaluate(() => { document.querySelector('.settings-body').scrollTop = 0; });
  await page.locator('.theme-card', { hasText: 'Tối' }).click();
  await page.waitForTimeout(200);
  await scrollBottom();
  await setRange('Độ tối phủ lên ảnh', 0);
  await setColor('Màu nền ứng dụng', '#FFFFFF');
  await page.waitForTimeout(300);

  const s5 = await page.evaluate(() => {
    const root = document.documentElement;
    const row = Array.from(document.querySelectorAll('.settings-row')).find((e) => e.textContent.includes('Độ trong suốt'));
    const labelFg = window.__testParseColor(getComputedStyle(row.querySelector('.settings-label')).color);
    return {
      theme: root.getAttribute('data-theme'),
      surfaceTone: root.getAttribute('data-surface-tone'),
      railTone: root.getAttribute('data-rail-tone'),
      contrastOnWhite: Number(window.__testContrast(labelFg, [255, 255, 255, 1]).toFixed(2)),
    };
  });
  record(
    'Scenario 5 (Dark Theme + Pure White #FFFFFF wallpaper + 100% trans): Auto-flips to light surface (>= 12:1 contrast)',
    s5.theme === 'dark' && s5.surfaceTone === 'light' && s5.railTone === 'light' && s5.contrastOnWhite >= 12,
    JSON.stringify(s5),
  );
  await page.screenshot({ path: path.join(OUT, '05_dark_white_wp_100trans.png') });

  // =========================================================================
  // SCENARIO 6: Worst-case Ugly Custom Accent (#FFFF00 / #FFFFFF / #000000) & Bubbles (#FFFFFF / #000000)
  // =========================================================================
  await page.locator('button', { hasText: 'Khôi phục giao diện mặc định' }).click();
  await page.waitForTimeout(250);
  await page.evaluate(() => { document.querySelector('.settings-body').scrollTop = 0; });
  await page.locator('.theme-card', { hasText: 'Sáng' }).click();
  await page.waitForTimeout(200);

  // Set blinding neon yellow accent #FFFF00, pure white sent bubble #FFFFFF, pure black recv bubble #000000
  await setColor('Màu nhấn', '#FFFF00');
  await setColor('Màu bong bóng của bạn', '#FFFFFF');
  await setColor('Màu bong bóng người khác', '#000000');
  await page.waitForTimeout(300);

  const s6Light = await page.evaluate(() => {
    const root = document.documentElement;
    const cs = getComputedStyle(root);
    const accent = window.__testParseColor(cs.getPropertyValue('--accent').trim());
    const onAccent = window.__testParseColor(cs.getPropertyValue('--on-accent').trim());
    const accentText = window.__testParseColor(cs.getPropertyValue('--accent-text').trim());
    const sentBg = window.__testParseColor(cs.getPropertyValue('--msg-sent').trim());
    const sentFg = window.__testParseColor(cs.getPropertyValue('--msg-sent-text').trim());
    const sentMeta = window.__testParseColor(cs.getPropertyValue('--msg-sent-meta').trim());
    const recvBg = window.__testParseColor(cs.getPropertyValue('--msg-recv').trim());
    const recvFg = window.__testParseColor(cs.getPropertyValue('--msg-recv-text').trim());
    const recvMeta = window.__testParseColor(cs.getPropertyValue('--msg-recv-meta').trim());
    return {
      accentContrastVsWhite: Number(window.__testContrast(accent, [255, 255, 255, 1]).toFixed(2)),
      onAccentContrast: Number(window.__testContrast(onAccent, accent).toFixed(2)),
      accentTextContrastVsWhite: Number(window.__testContrast(accentText, [255, 255, 255, 1]).toFixed(2)),
      sentSeparationFromWhite: Number(window.__testContrast(sentBg, [255, 255, 255, 1]).toFixed(2)),
      sentTextContrast: Number(window.__testContrast(sentFg, sentBg).toFixed(2)),
      sentMetaContrast: Number(window.__testContrast(window.__testBlend(sentMeta, sentBg), sentBg).toFixed(2)),
      recvTextContrast: Number(window.__testContrast(recvFg, recvBg).toFixed(2)),
      recvMetaContrast: Number(window.__testContrast(window.__testBlend(recvMeta, recvBg), recvBg).toFixed(2)),
    };
  });
  record(
    'Scenario 6a (Light Mode + Neon Yellow Accent + White/Black Bubbles): WCAG AA contrast & separation auto-compensated',
    s6Light.accentContrastVsWhite >= 3.4 &&
      s6Light.onAccentContrast >= 4.5 &&
      s6Light.accentTextContrastVsWhite >= 5.0 &&
      s6Light.sentSeparationFromWhite >= 1.15 &&
      s6Light.sentTextContrast >= 12 &&
      s6Light.sentMetaContrast >= 4.5 &&
      s6Light.recvTextContrast >= 15 &&
      s6Light.recvMetaContrast >= 7,
    JSON.stringify(s6Light),
  );
  // Scroll so preview + advanced colors are visible for screenshot
  await page.evaluate(() => {
    const el = document.querySelector('.settings-preview-chat');
    if (el) el.scrollIntoView({ block: 'center' });
  });
  await page.waitForTimeout(200);
  await page.screenshot({ path: path.join(OUT, '06_ugly_colors_compensated.png') });

  // Scenario 6b: Dark Theme + Pure Black Accent #000000
  await page.evaluate(() => { document.querySelector('.settings-body').scrollTop = 0; });
  await page.locator('.theme-card', { hasText: 'Tối' }).click();
  await setColor('Màu nhấn', '#000000');
  await page.waitForTimeout(250);
  const s6Dark = await page.evaluate(() => {
    const cs = getComputedStyle(document.documentElement);
    const accent = window.__testParseColor(cs.getPropertyValue('--accent').trim());
    const accentText = window.__testParseColor(cs.getPropertyValue('--accent-text').trim());
    const darkBg = [23, 27, 36, 1];
    return {
      accentContrastVsDark: Number(window.__testContrast(accent, darkBg).toFixed(2)),
      accentTextContrastVsDark: Number(window.__testContrast(accentText, darkBg).toFixed(2)),
    };
  });
  record(
    'Scenario 6b (Dark Mode + Pure Black #000000 Accent): Auto-lifted so buttons & links never vanish',
    s6Dark.accentContrastVsDark >= 3.4 && s6Dark.accentTextContrastVsDark >= 5.0,
    JSON.stringify(s6Dark),
  );

  // =========================================================================
  // SCENARIO 7: Chat Wallpaper "Đêm sao" in Light Mode (Live Preview + Chat Header)
  // =========================================================================
  await scrollBottom();
  await page.locator('button', { hasText: 'Khôi phục giao diện mặc định' }).click();
  await page.waitForTimeout(250);
  await page.evaluate(() => { document.querySelector('.settings-body').scrollTop = 0; });
  await page.locator('.theme-card', { hasText: 'Sáng' }).click();
  await page.waitForTimeout(200);
  await scrollBottom();
  await page.locator('.wallpaper-grid').nth(0).locator('.wallpaper-tile', { hasText: 'Đêm sao' }).click();
  await page.waitForTimeout(250);

  const previewHasNight = await page.evaluate(() => {
    const el = document.querySelector('.settings-preview-chat');
    return el ? el.classList.contains('wp-night') : false;
  });

  // Switch to Messages & open first chat to verify .chat-column.wp-night header text contrast
  await page.locator('.app-rail button[aria-label="Tin nhắn"]').click();
  await page.waitForSelector('.chat-item', { timeout: 10000 });
  await page.locator('.chat-item').first().click();
  await page.waitForSelector('.chat-column.wp-night', { timeout: 10000 });
  await page.waitForTimeout(250);

  const s7 = await page.evaluate(() => {
    const title = document.querySelector('.chat-column.wp-night .chat-header-title');
    const titleFg = title ? window.__testParseColor(getComputedStyle(title).color) : [0, 0, 0, 1];
    const notice = document.querySelector('.chat-column.wp-night .e2ee-notice');
    const noticeFg = notice ? window.__testParseColor(getComputedStyle(notice).color) : [0, 0, 0, 1];
    const noticeBg = notice ? window.__testParseColor(getComputedStyle(notice).backgroundColor) : [255, 255, 255, 1];
    const compNoticeBg = window.__testBlend(noticeBg, [14, 21, 43, 1]);
    const compBefore = document.querySelector('.chat-column.wp-night .composer-box');
    const compBg = compBefore ? window.__testParseColor(getComputedStyle(compBefore, '::before').backgroundColor) : [255, 255, 255, 1];
    return {
      titleLum: Number(window.__testLum(titleFg).toFixed(3)),
      contrastOnNight: Number(window.__testContrast(titleFg, [14, 21, 43, 1]).toFixed(2)),
      noticeContrast: Number(window.__testContrast(noticeFg, compNoticeBg).toFixed(2)),
      composerIsDark: window.__testLum(compBg) < 0.15,
    };
  });
  record(
    'Scenario 7 (Light Mode + Chat Wallpaper Đêm sao): Live preview, crisp white header, dark composer & high-contrast E2EE notice',
    previewHasNight && s7.titleLum > 0.7 && s7.contrastOnNight >= 12 && s7.noticeContrast >= 8 && s7.composerIsDark,
    JSON.stringify({ previewHasNight, ...s7 }),
  );
  await page.screenshot({ path: path.join(OUT, '07_chat_wp_night_light_theme.png') });

  record('Zero unexpected runtime page errors', errors.length === 0, errors.join('; '));
} catch (e) {
  record('Fatal run error', false, e.message);
  console.error(e);
} finally {
  await browser.close();
}

const passed = checks.filter((c) => c.pass).length;
console.log(`\nSpec 045 Summary: ${passed}/${checks.length} checks passed.`);
if (passed !== checks.length) process.exit(1);
