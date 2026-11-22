// Automated Live Playwright E2E Verification for Spec 062:
// 1. All 32 /locales/{code}.json packs load HTTP 200 with full 298 exact + 16 template entries, zero empty strings, zero "..."
// 2. All 33 language tiles render full native names with zero "..." truncation ("Bahasa Indonesia", "Português Brasil")
// 3. Real-time end-to-end switching across 10 diverse languages (en, ja, ar RTL, ko, zh-CN, th, ru, de, hi, vi) + reload persistence
// 4. Busy high-contrast colorful anime-style wallpaper @ 100% transparency:
//    - ZERO data-local-scrim="true" patchy white boxes on nav items
//    - ZERO white-on-white text on "Tải ảnh của bạn", "Khôi phục giao diện mặc định", or segmented buttons (WCAG >= 4.5:1)
//    - 100% accent synchronization across active nav item, active rail item, active segmented pills, active theme/wallpaper rings (Blue & Violet)
//    - Unified sleek .segmented pill tracks & refined inputs ("Tìm cài đặt", focus accent border)

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE_URL = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC062_SHOT_DIR || '';
if (SHOT_DIR) fs.mkdirSync(SHOT_DIR, { recursive: true });

const ALL_32_TARGET_LANGS = [
  'en', 'zh-CN', 'zh-TW', 'ja', 'ko', 'th', 'id', 'ms', 'fil', 'km', 'lo',
  'hi', 'bn', 'ar', 'fa', 'he', 'ur', 'tr', 'ru', 'uk', 'pl', 'cs', 'de',
  'fr', 'es', 'pt-BR', 'it', 'nl', 'sv', 'ro', 'hu', 'el',
];

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

function parseRgb(str) {
  const m = (str || '').match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?/i);
  if (!m) return null;
  return { r: +m[1], g: +m[2], b: +m[3], a: m[4] == null ? 1 : +m[4] };
}

function relLum(c) {
  const f = (v) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b);
}

function contrastRatio(fgStr, bgStr) {
  const fg = parseRgb(fgStr);
  const bg = parseRgb(bgStr);
  if (!fg || !bg) return 0;
  const l1 = relLum(fg);
  const l2 = relLum(bg);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

const browser = await chromium.launch({
  executablePath: '/usr/bin/google-chrome',
  headless: true,
  args: ['--no-sandbox', '--disable-gpu'],
});

const errors = [];

try {
  const tag = Math.random().toString(36).slice(2, 7);
  const uname = `s62_${tag}`;
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(e.message));

  await page.goto(BASE_URL, { waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 25000 });

  // 1. Verify all 32 JSON locale packs are served cleanly via HTTP 200 with zero empty or "..." strings
  const packAudit = await page.evaluate(async (langs) => {
    const bad = [];
    let minExact = 9999;
    let minTpl = 9999;
    for (const code of langs) {
      const r = await fetch(`/locales/${encodeURIComponent(code)}.json`);
      if (!r.ok) {
        bad.push(`${code}: HTTP ${r.status}`);
        continue;
      }
      const d = await r.json();
      const exVals = Object.values(d.exact || {});
      const tpVals = Object.values(d.templates || {});
      minExact = Math.min(minExact, exVals.length);
      minTpl = Math.min(minTpl, tpVals.length);
      if (exVals.some((v) => !v || !v.trim() || v.includes('...'))) bad.push(`${code}: invalid exact`);
      if (tpVals.some((v) => !v || !v.trim() || v.includes('...'))) bad.push(`${code}: invalid tpl`);
    }
    return { count: langs.length, bad, minExact, minTpl };
  }, ALL_32_TARGET_LANGS);
  record(
    'All 32 /locales/{code}.json packs load HTTP 200 with >=290 exact + 16 templates, zero empty values & zero "..."',
    packAudit.bad.length === 0 && packAudit.minExact >= 290 && packAudit.minTpl >= 16,
    JSON.stringify(packAudit)
  );

  // Log in
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Open Settings -> Ngôn ngữ
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Ngôn ngữ' }).click();
  await page.waitForSelector('.lang-grid .lang-tile', { timeout: 10000 });
  await page.waitForTimeout(250);

  // 2. Verify all 33 language tiles render full native names with zero truncation ("Bahasa Indonesia", "Português Brasil")
  const langGridAudit = await page.evaluate(() => {
    const tiles = Array.from(document.querySelectorAll('.lang-grid .lang-tile'));
    const truncated = [];
    for (const tile of tiles) {
      const span = tile.querySelector('.lang-native');
      if (!span) continue;
      const tR = tile.getBoundingClientRect();
      const sR = span.getBoundingClientRect();
      if (sR.right > tR.right + 1 || span.scrollWidth > span.clientWidth + 2 || span.textContent.includes('...')) {
        truncated.push({ text: span.textContent, sw: span.scrollWidth, cw: span.clientWidth, sRight: Math.round(sR.right), tRight: Math.round(tR.right) });
      }
    }
    const activeTile = document.querySelector('.lang-tile.active');
    return {
      total: tiles.length,
      truncated,
      activeBg: activeTile ? getComputedStyle(activeTile).backgroundColor : '',
      activeColor: activeTile ? getComputedStyle(activeTile).color : '',
    };
  });
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '01_language_grid_33_untruncated.png') });
  record(
    'All 33 language tiles display full native names (Bahasa Indonesia, Português Brasil) with zero "..." truncation & vivid accent active pill',
    langGridAudit.total === 33 &&
      langGridAudit.truncated.length === 0 &&
      langGridAudit.activeBg === 'rgb(46, 124, 246)' &&
      langGridAudit.activeColor === 'rgb(255, 255, 255)',
    JSON.stringify(langGridAudit)
  );

  // 3. Test live switching across diverse languages (en, ja, ar RTL, ko, zh-CN, th, ru, de, hi) + reload persistence + return to vi
  async function pickLang(code) {
    await page.locator(`.lang-tile[lang="${code}"]`).click();
    await page.waitForFunction((c) => document.documentElement.getAttribute('lang') === c && window.__chatwI18n?.getLanguage() === c, code, { timeout: 10000 });
    await page.waitForTimeout(260);
  }

  // English
  await pickLang('en');
  const enLangPage = await page.evaluate(() => ({
    lang: document.documentElement.getAttribute('lang'),
    dir: document.documentElement.getAttribute('dir'),
    pageTitle: document.querySelector('.settings-page-title')?.textContent?.trim(),
    searchPh: document.querySelector('.settings-search input')?.getAttribute('placeholder'),
    navTexts: Array.from(document.querySelectorAll('.settings-nav-item')).map((el) => el.textContent.trim()),
  }));

  // Click Appearance while in English
  await page.locator('.settings-nav-item', { hasText: 'Appearance' }).click();
  await page.waitForTimeout(300);
  const enAppearance = await page.evaluate(() => ({
    pageTitle: document.querySelector('.settings-page-title')?.textContent?.trim(),
    groupTitles: Array.from(document.querySelectorAll('.settings-body .settings-group-title')).map((el) => el.textContent.trim()),
    themeNames: Array.from(document.querySelectorAll('.theme-card')).map((el) => el.textContent.trim()),
    hasUploadEn: document.querySelector('.settings-body')?.innerText?.includes('Upload your image') || false,
    hasResetEn: document.querySelector('.settings-body')?.innerText?.includes('Reset default appearance') || false,
    hasTransparencyEn: document.querySelector('.settings-body')?.innerText?.includes('Panel transparency: 0%') || false,
  }));
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '02_appearance_english_opaque.png') });

  record(
    'English (en) immediately translates Settings Nav, Search placeholder, Appearance headings, Themes, Wallpapers, Sliders & Buttons',
    enLangPage.lang === 'en' &&
      enLangPage.dir === 'ltr' &&
      enLangPage.pageTitle === 'Language' &&
      enLangPage.searchPh === 'Search settings' &&
      enLangPage.navTexts.includes('Appearance') &&
      enLangPage.navTexts.includes('Account') &&
      enLangPage.navTexts.includes('Security') &&
      enAppearance.pageTitle === 'Appearance' &&
      enAppearance.groupTitles.includes('Theme') &&
      enAppearance.groupTitles.includes('Accent color') &&
      enAppearance.themeNames.includes('Light') &&
      enAppearance.themeNames.includes('Dark') &&
      enAppearance.hasUploadEn &&
      enAppearance.hasResetEn &&
      enAppearance.hasTransparencyEn,
    JSON.stringify({ enLangPage: { lang: enLangPage.lang, pageTitle: enLangPage.pageTitle, searchPh: enLangPage.searchPh }, enAppearance })
  );

  // Switch back to Language tab and test ja, ar (RTL), ko, zh-CN, th, ru, de, hi
  await page.locator('.settings-nav-item', { hasText: 'Language' }).click();
  await page.waitForSelector('.lang-grid .lang-tile', { timeout: 10000 });

  const multiChecks = [];
  const samples = [
    ['ja', 'ltr', '言語', '外観', '設定を検索'],
    ['ar', 'rtl', 'اللغة', 'المظهر', 'البحث في الإعدادات'],
    ['ko', 'ltr', '언어', '화면', '설정 검색'],
    ['zh-CN', 'ltr', '语言', '外观', '搜索设置'],
    ['th', 'ltr', 'ภาษา', 'ลักษณะที่ปรากฏ', 'ค้นหาการตั้งค่า'],
    ['ru', 'ltr', 'Язык', 'Оформление', 'Поиск настроек'],
    ['de', 'ltr', 'Sprache', 'Darstellung', 'Einstellungen durchsuchen'],
    ['hi', 'ltr', 'भाषा', 'दिखावट', 'सेटिंग्स खोजें'],
  ];

  for (const [code, expDir, expTitle, expAppNav, expPh] of samples) {
    await pickLang(code);
    const snap = await page.evaluate(() => ({
      lang: document.documentElement.getAttribute('lang'),
      dir: document.documentElement.getAttribute('dir'),
      title: document.querySelector('.settings-page-title')?.textContent?.trim(),
      ph: document.querySelector('.settings-search input')?.getAttribute('placeholder'),
      navs: Array.from(document.querySelectorAll('.settings-nav-item')).map((e) => e.textContent.trim()),
    }));
    const ok = snap.lang === code && snap.dir === expDir && snap.title === expTitle && snap.ph === expPh && snap.navs.includes(expAppNav);
    multiChecks.push({ code, ok, gotTitle: snap.title, gotDir: snap.dir, gotPh: snap.ph });
    if (code === 'ar' && SHOT_DIR) {
      await page.locator('.settings-nav-item', { hasText: 'المظهر' }).click();
      await page.waitForTimeout(250);
      await page.screenshot({ path: path.join(SHOT_DIR, '03_appearance_arabic_rtl.png') });
      await page.locator('.settings-nav-item', { hasText: 'اللغة' }).click();
      await page.waitForSelector('.lang-grid .lang-tile', { timeout: 10000 });
    }
  }
  record(
    'Switching live across 8 diverse scripts (ja, ar RTL, ko, zh-CN, th, ru, de, hi) translates all visible UI cleanly with RTL direction on Arabic',
    multiChecks.every((c) => c.ok),
    JSON.stringify(multiChecks)
  );

  // Test reload persistence in English + Messages/Chat translation
  await pickLang('en');
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(700);
  const reloadEnAudit = await page.evaluate(() => ({
    lang: document.documentElement.getAttribute('lang'),
    sidebarTitle: document.querySelector('.sidebar-title')?.textContent?.trim(),
    convSearchPh: document.querySelector('.sidebar-search input')?.getAttribute('placeholder'),
    railTips: Array.from(document.querySelectorAll('.app-rail [data-tip]')).map((el) => el.getAttribute('data-tip')),
  }));
  await page.locator('.nav-sidebar .chat-item').first().click();
  await page.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await page.waitForTimeout(250);
  const chatEnPh = await page.locator('#dm-chat-input').getAttribute('placeholder');
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '06_messages_chat_english_and_inputs.png') });
  record(
    'Selected language (en) persists across full page reload and translates Sidebar, Rail tooltips & Chat composer',
    reloadEnAudit.lang === 'en' &&
      reloadEnAudit.sidebarTitle === 'Messages' &&
      reloadEnAudit.convSearchPh === 'Search conversations' &&
      reloadEnAudit.railTips.includes('Messages') &&
      reloadEnAudit.railTips.includes('Meetings') &&
      chatEnPh === 'Note to self',
    JSON.stringify({ reloadEnAudit, chatEnPh })
  );

  // Switch back to Vietnamese (vi) and verify exact restoration
  await page.locator('.app-rail button[aria-label="Settings"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Language' }).click();
  await page.waitForSelector('.lang-grid .lang-tile', { timeout: 10000 });
  await pickLang('vi');
  const viRestored = await page.evaluate(() => ({
    lang: document.documentElement.getAttribute('lang'),
    dir: document.documentElement.getAttribute('dir'),
    title: document.querySelector('.settings-page-title')?.textContent?.trim(),
    ph: document.querySelector('.settings-search input')?.getAttribute('placeholder'),
  }));
  record(
    'Switching back to Tiếng Việt (vi) restores exact original Vietnamese labels and LTR direction',
    viRestored.lang === 'vi' && viRestored.dir === 'ltr' && viRestored.title === 'Ngôn ngữ' && viRestored.ph === 'Tìm cài đặt',
    JSON.stringify(viRestored)
  );

  // 4. Busy high-contrast colorful anime-style wallpaper @ 100% transparency audit!
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  await page.mouse.move(10, 10);
  await page.waitForTimeout(300);

  const busyUiAudit = await page.evaluate(async () => {
    // Draw a vibrant high-contrast busy illustration mimicking user's anime wallpaper (bright pink, dark crimson, purple, cream, cyan)
    const c = document.createElement('canvas');
    c.width = 640;
    c.height = 400;
    const ctx = c.getContext('2d');
    const bg = ctx.createLinearGradient(0, 0, 640, 400);
    bg.addColorStop(0, '#FFE4F1');
    bg.addColorStop(0.5, '#C084FC');
    bg.addColorStop(1, '#FFF3D6');
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, 640, 400);
    // High-contrast dark crimson & black hair/dress regions over nav, segmented controls, and bottom-right upload button!
    ctx.fillStyle = '#991B1B';
    ctx.fillRect(380, 180, 260, 220);
    ctx.fillStyle = '#1E1B4B';
    ctx.fillRect(420, 0, 220, 170);
    ctx.fillStyle = '#EC4899';
    ctx.beginPath(); ctx.arc(160, 140, 95, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = '#FFFFFF';
    ctx.fillRect(0, 0, 110, 400);
    ctx.fillStyle = '#BE123C';
    for (let y = 80; y < 380; y += 34) {
      ctx.fillRect(10, y, 160, 10);
    }
    const dataUrl = c.toDataURL('image/png');

    const root = document.documentElement;
    root.setAttribute('data-app-wp', 'image');
    root.setAttribute('data-panel-translucent', 'true');
    root.style.setProperty('--app-wp-image', `url('${dataUrl}')`);
    root.style.setProperty('--panel-opacity', '0%');
    window.__chatwCustom = Object.assign({}, window.__chatwCustom || {}, {
      wpKind: 'image',
      wpImageUrl: dataUrl,
      panelOpacity: 0,
      panelGlass: false,
      dim: 0,
    });
    window.__chatwRecomputeContrast();
    await new Promise((r) => setTimeout(r, 320));
    window.__chatwAdaptiveSurface?.refreshSync();

    const scrimNodes = document.querySelectorAll('[data-local-scrim="true"]').length;
    const idleNavBgs = Array.from(document.querySelectorAll('.settings-nav .settings-nav-item:not(.active)')).map(
      (el) => getComputedStyle(el).backgroundColor
    );
    const allIdleNavTransparent = idleNavBgs.every((bg) => bg === 'rgba(0, 0, 0, 0)');

    const navActive = document.querySelector('.settings-nav-item.active');
    const railActive = document.querySelector('.app-rail .rail-item.active');
    const segTracks = Array.from(document.querySelectorAll('.settings-body .segmented'));
    const activeSegs = Array.from(document.querySelectorAll('.settings-body .segmented button.active'));
    const idleSegs = Array.from(document.querySelectorAll('.settings-body .segmented button:not(.active)'));
    const themeActive = document.querySelector('.theme-card.active');
    const uploadBtn = Array.from(document.querySelectorAll('.settings-body .btn-secondary')).find((el) =>
      el.textContent.includes('Tải ảnh của bạn')
    );
    const resetBtn = Array.from(document.querySelectorAll('.settings-body .btn-secondary')).find((el) =>
      el.textContent.includes('Khôi phục giao diện mặc định')
    );
    const searchInput = document.querySelector('.settings-search input');
    searchInput.focus();
    await new Promise((r) => setTimeout(r, 200));
    const searchFocusBorder = getComputedStyle(searchInput).borderTopColor;
    searchInput.blur();

    return {
      scrimNodes,
      allIdleNavTransparent,
      navActiveBg: navActive ? getComputedStyle(navActive).backgroundColor : '',
      navActiveColor: navActive ? getComputedStyle(navActive).color : '',
      railActiveBg: railActive ? getComputedStyle(railActive).backgroundColor : '',
      railActiveColor: railActive ? getComputedStyle(railActive).color : '',
      activeSegsAllBlueWhite: activeSegs.every(
        (b) => getComputedStyle(b).backgroundColor === 'rgb(46, 124, 246)' && getComputedStyle(b).color === 'rgb(255, 255, 255)'
      ),
      segTrackBg: segTracks[0] ? getComputedStyle(segTracks[0]).backgroundColor : '',
      idleSegAllTransparent: idleSegs.every((b) => getComputedStyle(b).backgroundColor === 'rgba(0, 0, 0, 0)'),
      idleSegColor: idleSegs[0] ? getComputedStyle(idleSegs[0]).color : '',
      themeActiveShadow: themeActive ? getComputedStyle(themeActive).boxShadow : '',
      uploadBtnBg: uploadBtn ? getComputedStyle(uploadBtn).backgroundColor : '',
      uploadBtnColor: uploadBtn ? getComputedStyle(uploadBtn).color : '',
      resetBtnBg: resetBtn ? getComputedStyle(resetBtn).backgroundColor : '',
      resetBtnColor: resetBtn ? getComputedStyle(resetBtn).color : '',
      searchBg: getComputedStyle(searchInput).backgroundColor,
      searchColor: getComputedStyle(searchInput).color,
      searchRadius: getComputedStyle(searchInput).borderRadius,
      searchFocusBorder,
    };
  });

  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '04_busy_colorful_wp_100pct_vietnamese.png') });

  const uploadCR = contrastRatio(busyUiAudit.uploadBtnColor, busyUiAudit.uploadBtnBg);
  const resetCR = contrastRatio(busyUiAudit.resetBtnColor, busyUiAudit.resetBtnBg);
  const idleSegCR = contrastRatio(busyUiAudit.idleSegColor, busyUiAudit.segTrackBg);
  const searchCR = contrastRatio(busyUiAudit.searchColor, busyUiAudit.searchBg);

  record(
    'Busy colorful wallpaper @ 100% transparency: ZERO patchy white scrim boxes, 100% Blue accent sync across active pills/rail/theme ring, unified .segmented track, crisp inputs & >=4.5:1 button contrast',
    busyUiAudit.scrimNodes === 0 &&
      busyUiAudit.allIdleNavTransparent &&
      busyUiAudit.navActiveBg === 'rgb(46, 124, 246)' &&
      busyUiAudit.navActiveColor === 'rgb(255, 255, 255)' &&
      busyUiAudit.railActiveBg === 'rgb(46, 124, 246)' &&
      busyUiAudit.activeSegsAllBlueWhite &&
      busyUiAudit.idleSegAllTransparent &&
      busyUiAudit.themeActiveShadow.includes('46, 124, 246') &&
      uploadCR >= 7 &&
      resetCR >= 7 &&
      idleSegCR >= 7 &&
      searchCR >= 7 &&
      busyUiAudit.searchFocusBorder === 'rgb(46, 124, 246)',
    JSON.stringify({ ...busyUiAudit, uploadCR: +uploadCR.toFixed(1), resetCR: +resetCR.toFixed(1), idleSegCR: +idleSegCR.toFixed(1), searchCR: +searchCR.toFixed(1) })
  );

  // 5. Switch accent to Violet (#8B5CF6 -> rgb(139, 92, 246)) and verify ALL active controls sync instantaneously!
  await page.locator('.accent-swatch[aria-label="Tím"]').click();
  await page.waitForTimeout(260);
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '05_busy_colorful_wp_100pct_violet_accent.png') });
  const violetSync = await page.evaluate(() => ({
    navActiveBg: getComputedStyle(document.querySelector('.settings-nav-item.active')).backgroundColor,
    railActiveBg: getComputedStyle(document.querySelector('.app-rail .rail-item.active')).backgroundColor,
    segActiveBg: getComputedStyle(document.querySelector('.settings-body .segmented button.active')).backgroundColor,
    brandBg: getComputedStyle(document.querySelector('.rail-brand')).backgroundColor,
  }));
  record(
    'Switching Accent to Violet (#8B5CF6) instantaneously synchronizes nav pill, rail pill, segmented buttons & brand logo',
    violetSync.navActiveBg === 'rgb(139, 92, 246)' &&
      violetSync.railActiveBg === 'rgb(139, 92, 246)' &&
      violetSync.segActiveBg === 'rgb(139, 92, 246)' &&
      violetSync.brandBg === 'rgb(139, 92, 246)',
    JSON.stringify(violetSync)
  );

  record('Zero uncaught page runtime errors', errors.length === 0, errors.join(' | '));

  await ctx.close();
} finally {
  await browser.close();
}

const passed = checks.filter((c) => c.pass).length;
console.log(`\nSummary: ${passed}/${checks.length} passed`);
if (passed !== checks.length) process.exit(1);
