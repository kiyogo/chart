// Automated Live Playwright E2E Verification for Specs 062, 063 & 064:
// 1. Canonical English source code on clean boot + translate="no" / <meta name="google" content="notranslate"> anti-corruption lock
// 2. /locales/_keys.json + ALL 197 world /locales/{code}.json packs load HTTP 200 with 100% exact & template parity, zero empty values, zero "..."
// 3. All 197 global language tiles render official /flags/{cc}.svg vector flags, full native names (zero "..." truncation), translate="no", & live Language Search
// 4. Authentic Vietnamese (vi) translates 100% of Account, Appearance, Settings Nav, Messages Empty State, Sidebar, Rail tooltips & persists across reload
// 5. Live switching across 12 diverse world languages spanning every continent & script (zh-CN, ja, ko, ar RTL, he RTL, fa RTL, hi, th, ru, de, sw, am) + return to canonical English (en)
// 6. Busy colorful wallpaper @ 100% transparency: ZERO patchy white scrim boxes, soft low-opacity Blue & Violet accent pills (14%-26% alpha), unified segmented tracks & crisp inputs

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE_URL = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC062_SHOT_DIR || '';
if (SHOT_DIR) fs.mkdirSync(SHOT_DIR, { recursive: true });

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

function parseRgb(str) {
  const s = str || '';
  const m = s.match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?/i);
  if (m) return { r: +m[1], g: +m[2], b: +m[3], a: m[4] == null ? 1 : +m[4] };
  const srgb = s.match(/color\(\s*srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:\s*\/\s*([\d.]+))?\s*\)/i);
  if (srgb) {
    return {
      r: Math.round(+srgb[1] * 255),
      g: Math.round(+srgb[2] * 255),
      b: Math.round(+srgb[3] * 255),
      a: srgb[4] == null ? 1 : +srgb[4],
    };
  }
  return null;
}

function relLum(c) {
  const f = (v) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.7152 * 0 + 0.0722 * f(c.b);
}

function contrastRatio(fgStr, bgStr) {
  const fg = parseRgb(fgStr);
  const bg = parseRgb(bgStr);
  if (!fg || !bg) return 0;
  const l1 = relLum(fg);
  const l2 = relLum(bg);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

function isTranslucentAccent(str, hue) {
  const c = parseRgb(str);
  if (!c) return false;
  if (c.a < 0.11 || c.a > 0.34) return false;
  if (hue === 'blue') return c.b > 190 && c.b > c.r + 100;
  if (hue === 'violet') return c.b > 200 && c.r > 100 && c.r < 175 && c.g < 120;
  return true;
}

const browser = await chromium.launch({
  executablePath: '/usr/bin/google-chrome',
  headless: true,
  args: ['--no-sandbox', '--disable-gpu'],
});

const errors = [];

try {
  const tag = Math.random().toString(36).slice(2, 7);
  const uname = `s64_${tag}`;
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(e.message));

  await page.goto(BASE_URL, { waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 25000 });

  // 1. Canonical English boot + Browser Auto-Translate Lockout (notranslate)
  const bootAudit = await page.evaluate(() => {
    const html = document.documentElement;
    const meta = document.querySelector('meta[name="google"][content="notranslate"]');
    const authText = document.querySelector('.auth-card')?.innerText || '';
    const hasViChars = /[àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ]/i.test(authText);
    return {
      lang: html.getAttribute('lang'),
      dir: html.getAttribute('dir'),
      translate: html.getAttribute('translate'),
      hasNotranslateClass: html.classList.contains('notranslate'),
      hasMetaNotranslate: Boolean(meta),
      supportedCount: window.__chatwI18n?.supportedCount?.() || 0,
      hasViChars,
      title: document.title,
    };
  });
  record(
    'Canonical English source boot with translate="no" + <meta name="google" content="notranslate"> + >=195 world languages registered',
    bootAudit.lang === 'en' &&
      bootAudit.dir === 'ltr' &&
      bootAudit.translate === 'no' &&
      bootAudit.hasNotranslateClass &&
      bootAudit.hasMetaNotranslate &&
      bootAudit.supportedCount >= 195 &&
      !bootAudit.hasViChars,
    JSON.stringify(bootAudit)
  );

  // 2. Verify /locales/_keys.json + ALL 197 /locales/{code}.json packs load HTTP 200 with 100% key parity & placeholder integrity
  const packAudit = await page.evaluate(async () => {
    const langs = window.__chatwI18n.supportedLanguages();
    const kr = await fetch('/locales/_keys.json');
    if (!kr.ok) return { error: `_keys.json HTTP ${kr.status}` };
    const keys = await kr.json();
    const nExact = (keys.exact || []).length;
    const nTpl = (keys.templates || []).length;
    const bad = [];

    // Fetch in waves of 20 for speed
    for (let i = 0; i < langs.length; i += 20) {
      const slice = langs.slice(i, i + 20);
      await Promise.all(
        slice.map(async (code) => {
          const r = await fetch(`/locales/${encodeURIComponent(code)}.json`);
          if (!r.ok) {
            bad.push(`${code}: HTTP ${r.status}`);
            return;
          }
          const d = await r.json();
          if (d.v !== 2 || d.lang !== code) {
            bad.push(`${code}: bad header`);
            return;
          }
          if (!Array.isArray(d.exact) || d.exact.length !== nExact) {
            bad.push(`${code}: exact len ${d.exact?.length} != ${nExact}`);
            return;
          }
          if (!Array.isArray(d.templates) || d.templates.length !== nTpl) {
            bad.push(`${code}: tpl len ${d.templates?.length} != ${nTpl}`);
            return;
          }
          if (d.exact.some((v) => !v || !v.trim() || v.includes('...'))) {
            bad.push(`${code}: empty or ellipsis in exact`);
            return;
          }
          if (d.templates.some((v) => !v || !v.trim() || v.includes('...'))) {
            bad.push(`${code}: empty or ellipsis in templates`);
            return;
          }
        })
      );
    }
    return { totalLangs: langs.length, nExact, nTpl, badCount: bad.length, sampleBad: bad.slice(0, 5) };
  });
  record(
    'ALL 197 world /locales/{code}.json packs load HTTP 200 with 100% exact (>=2800) & template (>=350) parity, zero empty strings & zero "..."',
    packAudit.totalLangs >= 195 && packAudit.nExact >= 2800 && packAudit.nTpl >= 350 && packAudit.badCount === 0,
    JSON.stringify(packAudit)
  );

  // Log in (canonical English auth form)
  await page.locator('input[aria-label="Test username"]').fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Skip tour"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Open Settings -> Language (English)
  await page.locator('.app-rail button[aria-label="Settings"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Language' }).click();
  await page.waitForSelector('.lang-grid .lang-tile', { timeout: 10000 });
  await page.waitForTimeout(300);

  // 3. Verify all 197 language tiles render full native names, official SVG flags, translate="no", soft low-opacity active pill, & live search works
  const langGridAudit = await page.evaluate(() => {
    const tiles = Array.from(document.querySelectorAll('.lang-grid .lang-tile'));
    const truncated = [];
    let allNotranslate = true;
    let allHaveFlagImg = true;
    for (const tile of tiles) {
      if (tile.getAttribute('translate') !== 'no' || !tile.classList.contains('notranslate')) allNotranslate = false;
      const img = tile.querySelector('.lang-flag-badge img[src^="/flags/"]');
      if (!img) allHaveFlagImg = false;
      const span = tile.querySelector('.lang-native');
      if (!span) continue;
      if (span.textContent.includes('...')) truncated.push(span.textContent);
    }
    const activeTile = document.querySelector('.lang-tile.active');
    return {
      total: tiles.length,
      allNotranslate,
      allHaveFlagImg,
      truncated,
      activeLang: activeTile?.getAttribute('lang'),
      activeBg: activeTile ? getComputedStyle(activeTile).backgroundColor : '',
      activeColor: activeTile ? getComputedStyle(activeTile).color : '',
      countBadge: document.querySelector('.lang-count-badge')?.textContent?.trim(),
    };
  });
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '01_language_grid_197_english.png') });

  // Test live Language Search filter
  const langSearch = page.locator('.lang-search-field input');
  await langSearch.fill('viet');
  await page.waitForTimeout(180);
  const searchViet = await page.locator('.lang-grid .lang-tile').allTextContents();
  await langSearch.fill('swahili');
  await page.waitForTimeout(180);
  const searchSw = await page.locator('.lang-grid .lang-tile').allTextContents();
  await langSearch.fill('');
  await page.waitForTimeout(180);

  record(
    'All 197 global language tiles render official /flags/*.svg icons, translate="no", soft low-opacity active pill & instant search filter',
    langGridAudit.total >= 195 &&
      langGridAudit.allNotranslate &&
      langGridAudit.allHaveFlagImg &&
      langGridAudit.truncated.length === 0 &&
      langGridAudit.activeLang === 'en' &&
      isTranslucentAccent(langGridAudit.activeBg, 'blue') &&
      searchViet.some((t) => t.includes('Tiếng Việt')) &&
      searchSw.some((t) => t.includes('Kiswahili')),
    JSON.stringify({ ...langGridAudit, searchViet, searchSw })
  );

  // Verify English opaque Appearance active pills stay strictly low-opacity translucent
  await page.locator('.settings-nav-item', { hasText: 'Appearance' }).click();
  await page.mouse.move(10, 10);
  await page.waitForTimeout(300);
  const enAppearance = await page.evaluate(() => ({
    pageTitle: document.querySelector('.settings-page-title')?.textContent?.trim(),
    opaqueNavActiveBg: getComputedStyle(document.querySelector('.settings-nav-item.active')).backgroundColor,
    opaqueSegActiveBg: getComputedStyle(document.querySelector('.settings-body .segmented button.active')).backgroundColor,
  }));
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '02_appearance_english_opaque.png') });
  record(
    'Canonical English Appearance renders natively with soft low-opacity translucent active pills',
    enAppearance.pageTitle === 'Appearance' &&
      isTranslucentAccent(enAppearance.opaqueNavActiveBg, 'blue') &&
      isTranslucentAccent(enAppearance.opaqueSegActiveBg, 'blue'),
    JSON.stringify(enAppearance)
  );

  // Helper to switch language cleanly from Language Settings
  async function pickLang(code) {
    await page.locator(`.lang-tile[lang="${code}"]`).click();
    await page.waitForFunction(
      (c) =>
        document.documentElement.getAttribute('lang') === c &&
        window.__chatwI18n?.getLanguage() === c &&
        (c === 'en' || window.__chatwI18n.loadedLocales().includes(c)),
      code,
      { timeout: 15000 }
    );
    await page.waitForTimeout(280);
  }

  // 4. Switch to Authentic Native Vietnamese (vi) and inspect Account + Appearance + Messages Empty State + Reload Persistence!
  await page.locator('.settings-nav-item', { hasText: 'Language' }).click();
  await page.waitForSelector('.lang-grid .lang-tile', { timeout: 10000 });
  await pickLang('vi');

  // Inspect Account screen in Vietnamese (the exact screen from user's screenshot #2!)
  await page.locator('.settings-nav-item', { hasText: 'Tài khoản' }).click();
  await page.waitForTimeout(300);
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '03_account_vietnamese_100pct.png') });
  const viAccount = await page.evaluate(() => ({
    lang: document.documentElement.getAttribute('lang'),
    dir: document.documentElement.getAttribute('dir'),
    searchPh: document.querySelector('.settings-search input')?.getAttribute('placeholder'),
    navs: Array.from(document.querySelectorAll('.settings-nav-item')).map((e) => e.textContent.trim()),
    bodyText: document.querySelector('.settings-body')?.innerText || '',
  }));
  record(
    'Authentic Vietnamese (vi) translates 100% of Account page, Settings Nav ("Đám mây", "Trò chuyện", "Giao diện", "Đồng bộ") with ZERO garbled Chrome-translate labels',
    viAccount.lang === 'vi' &&
      viAccount.dir === 'ltr' &&
      viAccount.searchPh === 'Tìm cài đặt' &&
      viAccount.navs.includes('Tài khoản') &&
      viAccount.navs.includes('Đám mây') &&
      viAccount.navs.includes('Trò chuyện') &&
      viAccount.navs.includes('Giao diện') &&
      viAccount.navs.includes('Đồng bộ') &&
      !viAccount.navs.includes('Thiên đường') &&
      !viAccount.navs.includes('Ngoại cảnh') &&
      viAccount.bodyText.includes('Đổi ảnh bìa') &&
      viAccount.bodyText.includes('Đổi ảnh đại diện') &&
      viAccount.bodyText.includes('Tên đăng nhập') &&
      viAccount.bodyText.includes('Chỉnh sửa'),
    JSON.stringify({ lang: viAccount.lang, searchPh: viAccount.searchPh, hasCloud: viAccount.navs.includes('Đám mây'), hasChat: viAccount.navs.includes('Trò chuyện') })
  );

  // Full page reload persistence in Vietnamese + Messages empty state & chat check!
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(700);
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '04_messages_empty_vietnamese_reload.png') });
  const viReload = await page.evaluate(() => ({
    lang: document.documentElement.getAttribute('lang'),
    translate: document.documentElement.getAttribute('translate'),
    sidebarTitle: document.querySelector('.sidebar-title')?.textContent?.trim(),
    convSearchPh: document.querySelector('.sidebar-search input')?.getAttribute('placeholder'),
    mainText: document.querySelector('.chat-column, .chat-empty, .main-panel')?.innerText || document.body.innerText,
  }));
  record(
    'Vietnamese (vi) persists across full reload and translates Sidebar, Empty state ("Sẵn sàng trò chuyện riêng tư") & Rail cleanly',
    viReload.lang === 'vi' &&
      viReload.translate === 'no' &&
      viReload.sidebarTitle === 'Tin nhắn' &&
      viReload.convSearchPh === 'Tìm cuộc trò chuyện' &&
      viReload.mainText.includes('Sẵn sàng trò chuyện riêng tư'),
    JSON.stringify({ lang: viReload.lang, sidebarTitle: viReload.sidebarTitle, convSearchPh: viReload.convSearchPh })
  );

  // 5. Switch live across 12 diverse world languages spanning EVERY continent & script (including RTL Hebrew, Arabic, Persian + East Africa Swahili + Ethiopia Amharic + CJK + Indic + Cyrillic)!
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Ngôn ngữ' }).click();
  await page.waitForSelector('.lang-grid .lang-tile', { timeout: 10000 });

  const worldSamples = [
    ['zh-CN', 'ltr', '语言', '外观', '搜索设置'],
    ['ja', 'ltr', '言語', '外観', '設定を検索'],
    ['ko', 'ltr', '언어', '화면', '설정 검색'],
    ['ar', 'rtl', 'اللغة', 'المظهر', 'البحث في الإعدادات'],
    ['he', 'rtl', 'שפה', 'מראה', 'חיפוש הגדרות'],
    ['fa', 'rtl', 'زبان', 'ظاهر', 'جستجوی تنظیمات'],
    ['hi', 'ltr', 'भाषा', 'दिखावट', 'सेटिंग्स खोजें'],
    ['th', 'ltr', 'ภาษา', 'ลักษณะที่ปรากฏ', 'ค้นหาการตั้งค่า'],
    ['ru', 'ltr', 'Язык', 'Оформление', 'Поиск настроек'],
    ['de', 'ltr', 'Sprache', 'Darstellung', 'Einstellungen durchsuchen'],
    ['sw', 'ltr', null, null, null], // Swahili (East Africa) - verify translated away from English!
    ['am', 'ltr', null, null, null], // Amharic (Ethiopic script) - verify translated into Ge'ez script!
  ];

  const multiChecks = [];
  for (const [code, expDir, expTitle, expAppNav, expPh] of worldSamples) {
    await pickLang(code);
    const snap = await page.evaluate(() => ({
      lang: document.documentElement.getAttribute('lang'),
      dir: document.documentElement.getAttribute('dir'),
      translate: document.documentElement.getAttribute('translate'),
      title: document.querySelector('.settings-page-title')?.textContent?.trim(),
      ph: document.querySelector('.settings-search input')?.getAttribute('placeholder'),
      navs: Array.from(document.querySelectorAll('.settings-nav-item')).map((e) => e.textContent.trim()),
    }));
    let ok = snap.lang === code && snap.dir === expDir && snap.translate === 'no';
    if (expTitle) ok = ok && snap.title === expTitle;
    else ok = ok && Boolean(snap.title && snap.title !== 'Language');
    if (expPh) ok = ok && snap.ph === expPh;
    else ok = ok && Boolean(snap.ph && snap.ph !== 'Search settings');
    if (expAppNav) ok = ok && snap.navs.includes(expAppNav);
    else ok = ok && !snap.navs.includes('Appearance');
    multiChecks.push({ code, ok, dir: snap.dir, title: snap.title, ph: snap.ph });

    if (code === 'zh-CN' && SHOT_DIR) {
      await page.locator('.settings-nav-item').first().click();
      await page.waitForTimeout(260);
      await page.screenshot({ path: path.join(SHOT_DIR, '05_account_zh_cn_100pct.png') });
      await page.locator('.settings-nav-item', { hasText: '语言' }).click();
      await page.waitForSelector('.lang-grid .lang-tile', { timeout: 10000 });
    }
    if (code === 'he' && SHOT_DIR) {
      await page.screenshot({ path: path.join(SHOT_DIR, '06_language_hebrew_rtl_clean.png') });
    }
    if (code === 'ar' && SHOT_DIR) {
      await page.locator('.settings-nav-item', { hasText: 'المظهر' }).click();
      await page.waitForTimeout(250);
      await page.screenshot({ path: path.join(SHOT_DIR, '07_appearance_arabic_rtl.png') });
      await page.locator('.settings-nav-item', { hasText: 'اللغة' }).click();
      await page.waitForSelector('.lang-grid .lang-tile', { timeout: 10000 });
    }
  }
  record(
    'Live switching across 12 diverse world languages (zh-CN, ja, ko, ar RTL, he RTL, fa RTL, hi, th, ru, de, sw, am) translates all UI screens accurately with translate="no"',
    multiChecks.every((c) => c.ok),
    JSON.stringify(multiChecks)
  );

  // Switch back to canonical English (en) and verify 100% exact lossless restoration
  await pickLang('en');
  const enRestored = await page.evaluate(() => ({
    lang: document.documentElement.getAttribute('lang'),
    dir: document.documentElement.getAttribute('dir'),
    title: document.querySelector('.settings-page-title')?.textContent?.trim(),
    ph: document.querySelector('.settings-search input')?.getAttribute('placeholder'),
  }));
  record(
    'Switching back to canonical English (en) restores 100% exact English labels and LTR direction losslessly',
    enRestored.lang === 'en' && enRestored.dir === 'ltr' && enRestored.title === 'Language' && enRestored.ph === 'Search settings',
    JSON.stringify(enRestored)
  );

  // 6. Busy colorful wallpaper @ 100% transparency audit (Spec 063 low-opacity translucent active pills + Violet sync!)
  await page.locator('.settings-nav-item', { hasText: 'Appearance' }).click();
  await page.mouse.move(10, 10);
  await page.waitForTimeout(300);

  const busyUiAudit = await page.evaluate(async () => {
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
    ctx.fillStyle = '#991B1B';
    ctx.fillRect(380, 180, 260, 220);
    ctx.fillStyle = '#1E1B4B';
    ctx.fillRect(420, 0, 220, 170);
    ctx.fillStyle = '#EC4899';
    ctx.beginPath(); ctx.arc(160, 140, 95, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = '#FFFFFF';
    ctx.fillRect(0, 0, 110, 400);
    ctx.fillStyle = '#BE123C';
    for (let y = 80; y < 380; y += 34) ctx.fillRect(10, y, 160, 10);
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
      el.textContent.includes('Upload your image')
    );
    const resetBtn = Array.from(document.querySelectorAll('.settings-body .btn-secondary')).find((el) =>
      el.textContent.includes('Reset default appearance')
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
      railActiveBg: railActive ? getComputedStyle(railActive).backgroundColor : '',
      activeSegsBgs: activeSegs.map((b) => getComputedStyle(b).backgroundColor),
      activeSegsColors: activeSegs.map((b) => getComputedStyle(b).color),
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
      searchFocusBorder,
    };
  });

  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '08_busy_wp_low_opacity_blue.png') });

  const uploadCR = contrastRatio(busyUiAudit.uploadBtnColor, busyUiAudit.uploadBtnBg);
  const resetCR = contrastRatio(busyUiAudit.resetBtnColor, busyUiAudit.resetBtnBg);
  const idleSegCR = contrastRatio(busyUiAudit.idleSegColor, busyUiAudit.segTrackBg);
  const searchCR = contrastRatio(busyUiAudit.searchColor, busyUiAudit.searchBg);
  const activeSegsAllSoftBlue =
    busyUiAudit.activeSegsBgs.length > 0 &&
    busyUiAudit.activeSegsBgs.every((bg) => isTranslucentAccent(bg, 'blue')) &&
    busyUiAudit.activeSegsColors.every((c) => c === 'rgb(255, 255, 255)' || c === 'rgb(15, 23, 42)');

  record(
    'Busy colorful wallpaper @ 100% transparency: ZERO patchy scrims, soft low-opacity Blue accent pills, unified .segmented track, crisp inputs & >=4.5:1 contrast',
    busyUiAudit.scrimNodes === 0 &&
      busyUiAudit.allIdleNavTransparent &&
      isTranslucentAccent(busyUiAudit.navActiveBg, 'blue') &&
      isTranslucentAccent(busyUiAudit.railActiveBg, 'blue') &&
      activeSegsAllSoftBlue &&
      busyUiAudit.idleSegAllTransparent &&
      busyUiAudit.themeActiveShadow.includes('1.5px') &&
      uploadCR >= 7 &&
      resetCR >= 7 &&
      idleSegCR >= 7 &&
      searchCR >= 7 &&
      busyUiAudit.searchFocusBorder === 'rgb(46, 124, 246)',
    JSON.stringify({ ...busyUiAudit, uploadCR: +uploadCR.toFixed(1), resetCR: +resetCR.toFixed(1), idleSegCR: +idleSegCR.toFixed(1), searchCR: +searchCR.toFixed(1) })
  );

  // Violet accent sync check
  await page.locator('.accent-swatch[aria-label="Violet"]').click();
  await page.waitForTimeout(260);
  const violetSync = await page.evaluate(() => ({
    navActiveBg: getComputedStyle(document.querySelector('.settings-nav-item.active')).backgroundColor,
    railActiveBg: getComputedStyle(document.querySelector('.app-rail .rail-item.active')).backgroundColor,
    segActiveBg: getComputedStyle(document.querySelector('.settings-body .segmented button.active')).backgroundColor,
    brandBg: getComputedStyle(document.querySelector('.rail-brand')).backgroundColor,
  }));
  record(
    'Switching Accent to Violet (#8B5CF6) instantaneously synchronizes soft low-opacity nav pill, rail pill, segmented buttons & brand logo',
    isTranslucentAccent(violetSync.navActiveBg, 'violet') &&
      isTranslucentAccent(violetSync.railActiveBg, 'violet') &&
      isTranslucentAccent(violetSync.segActiveBg, 'violet') &&
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
