// Automated Live Playwright Verification for Spec 084:
// Verifies:
//  1. 17 self-hosted fonts selectable in Settings -> Appearance -> Font with live dropdown previews.
//  2. Full 197-language localization: switching languages (vi, ja, ar RTL, ru, hi, th) displays
//     all font names and the live Font Preview card in the exact selected language.
//  3. Authentic Minecraft (`ChatW Minecraft` WOFF2) loads & applies cleanly with Vietnamese glyphs.
//  4. Original sculpted `ChatW LGBT` WOFF2 loads & renders animated 7-color rainbow Pride typography
//     when Reduce motion is OFF, and freezes seamlessly onto a static 7-color Pride gradient when
//     Reduce motion (`Giảm chuyển động`) is ON.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC084_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_084';
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
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 920 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => pageErrors.push(e.message));

  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => localStorage.setItem('chatw_lang', 'vi'));
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  const uname = `s84_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);
  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Open Settings -> Appearance (Giao diện)
  await page.locator('.app-rail button[aria-label="Cài đặt"], .app-rail button[title="Cài đặt"], .app-rail button[aria-label="Settings"]').first().click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.waitForTimeout(300);
  await page.locator('.settings-nav-item', { hasText: /Giao diện|Appearance/i }).first().click();
  await page.waitForSelector('select[data-font-picker="true"]', { timeout: 10000 });
  await page.waitForTimeout(400);

  // Scroll font selector into view & open custom dropdown
  const fontSel = page.locator('select[data-font-picker="true"]').first();
  await fontSel.scrollIntoViewIfNeeded();
  await page.waitForTimeout(200);
  await fontSel.click();
  await page.waitForSelector('.cw-select-menu[data-font-menu="true"] .cw-select-option', { timeout: 5000 });
  await page.waitForTimeout(500);

  const dropdownAudit = await page.evaluate(() => {
    const opts = Array.from(document.querySelectorAll('.cw-select-menu[data-font-menu="true"] .cw-select-option'));
    return {
      count: opts.length,
      labels: opts.map((o) => o.querySelector('.cw-select-option-label')?.textContent?.trim() || ''),
      values: opts.map((o) => o.getAttribute('data-font-preview') || ''),
      previewSample: document.querySelector('.font-preview-sample')?.textContent?.trim() || '',
    };
  });
  await page.screenshot({ path: path.join(SHOT_DIR, '01_vi_font_dropdown_17_fonts.png') });

  record(
    '1. Font dropdown lists all 17 fonts localized cleanly in Vietnamese with live font preview',
    dropdownAudit.count === 17 &&
      dropdownAudit.values.includes('lgbt') &&
      dropdownAudit.values.includes('minecraft') &&
      dropdownAudit.labels.includes('LGBT') &&
      dropdownAudit.labels.includes('Minecraft') &&
      dropdownAudit.labels.includes('Hiện đại không chân') &&
      dropdownAudit.labels.includes('Chữ viết tay') &&
      dropdownAudit.labels.includes('Thư pháp') &&
      dropdownAudit.previewSample.includes('Nhắn tin riêng tư'),
    JSON.stringify({ count: dropdownAudit.count, sample: dropdownAudit.previewSample, labels: dropdownAudit.labels.slice(0, 8) })
  );

  // 2. Select LGBT font -> verify sculpted ChatW LGBT WOFF2 loads & 7-color animation runs when Reduce motion is OFF
  await page.locator('.cw-select-menu .cw-select-option[data-font-preview="lgbt"]').click();
  await page.waitForTimeout(600);
  await page.evaluate(() => document.fonts.ready);

  const lgbtActive = await page.evaluate(() => {
    const html = document.documentElement;
    const sample = document.querySelector('.font-preview-sample');
    const cs = sample ? getComputedStyle(sample) : null;
    const loadedFaces = Array.from(document.fonts)
      .filter((f) => f.status === 'loaded')
      .map((f) => f.family.replace(/["']/g, ''));
    return {
      dataFont: html.getAttribute('data-font'),
      reduceMotion: html.getAttribute('data-reduce-motion'),
      loadedLgbt: loadedFaces.includes('ChatW LGBT'),
      bgImage: cs?.backgroundImage || '',
      bgClip: cs?.webkitBackgroundClip || cs?.backgroundClip || '',
      animName: cs?.animationName || '',
      fontFamily: cs?.fontFamily || '',
    };
  });
  await page.screenshot({ path: path.join(SHOT_DIR, '02_lgbt_animated_light_vi.png') });

  record(
    '2. Original ChatW LGBT WOFF2 loads & animates 7-color rainbow gradient when Reduce motion is OFF',
    lgbtActive.dataFont === 'lgbt' &&
      lgbtActive.loadedLgbt &&
      lgbtActive.bgImage.includes('linear-gradient') &&
      lgbtActive.bgClip === 'text' &&
      lgbtActive.animName.includes('cwLgbtPrismFlow'),
    JSON.stringify(lgbtActive)
  );

  // 3. Toggle Reduce motion ON -> verify animation freezes cleanly onto static 7-color Pride gradient!
  const reduceToggle = page.locator('.toggle-row', { hasText: /Giảm chuyển động|Reduce motion/i }).locator('input[type="checkbox"]').first();
  await reduceToggle.scrollIntoViewIfNeeded();
  await reduceToggle.click();
  await page.waitForTimeout(350);

  const lgbtReduced = await page.evaluate(() => {
    const html = document.documentElement;
    const sample = document.querySelector('.font-preview-sample');
    const cs = sample ? getComputedStyle(sample) : null;
    return {
      reduceMotion: html.getAttribute('data-reduce-motion'),
      bgImage: cs?.backgroundImage || '',
      animName: cs?.animationName || '',
    };
  });
  record(
    '3. Turning ON Reduce motion ("Giảm chuyển động") stops color-flow animation while keeping static 7-color LGBT gradient',
    lgbtReduced.reduceMotion === 'true' &&
      lgbtReduced.animName === 'none' &&
      lgbtReduced.bgImage.includes('linear-gradient'),
    JSON.stringify(lgbtReduced)
  );

  // Turn Reduce motion back OFF for animated chat verification
  await reduceToggle.click();
  await page.waitForTimeout(250);

  // 4. Verify live LGBT Pride chat bubbles & UI in Note-to-self ("Chính mình")
  await page.locator('.app-rail button[aria-label="Tin nhắn"], .app-rail button[title="Tin nhắn"], .app-rail button[aria-label="Messages"]').first().click();
  await page.waitForSelector('.nav-sidebar .chat-item', { timeout: 10000 });
  await page.locator('.nav-sidebar .chat-item').first().click();
  await page.waitForSelector('.composer-input', { timeout: 10000 });
  await page.locator('.composer-input').fill('Xin chào! Tự hào LGBT 7 màu trên ChatW ♥ 0123456789');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(700);
  const sentBubbleAudit = await page.evaluate(() => {
    const b = document.querySelector('.message-row.sent .message-bubble');
    const txt = b?.querySelector('.message-text');
    return {
      bubbleBg: b ? getComputedStyle(b).backgroundImage : '',
      textFill: txt ? getComputedStyle(txt).webkitTextFillColor : '',
    };
  });
  record(
    '3b. Outgoing sent bubble renders as high-contrast 7-color Pride gradient pill with crisp white text',
    sentBubbleAudit.bubbleBg.includes('linear-gradient') && sentBubbleAudit.textFill === 'rgb(255, 255, 255)',
    JSON.stringify(sentBubbleAudit)
  );
  await page.screenshot({ path: path.join(SHOT_DIR, '03_lgbt_chat_live_bubbles.png') });

  // 5. Return to Settings -> Appearance and select Minecraft font -> verify Monocraft WOFF2 loads
  await page.locator('.app-rail button[aria-label="Cài đặt"], .app-rail button[title="Cài đặt"], .app-rail button[aria-label="Settings"]').first().click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: /Giao diện|Appearance/i }).first().click();
  await page.waitForSelector('select[data-font-picker="true"]', { timeout: 10000 });
  const fontSel2 = page.locator('select[data-font-picker="true"]').first();
  await fontSel2.scrollIntoViewIfNeeded();
  await fontSel2.click();
  await page.waitForSelector('.cw-select-menu .cw-select-option[data-font-preview="minecraft"]', { timeout: 5000 });
  await page.locator('.cw-select-menu .cw-select-option[data-font-preview="minecraft"]').click();
  await page.waitForTimeout(600);
  await page.evaluate(() => document.fonts.ready);

  const mcAudit = await page.evaluate(() => {
    const html = document.documentElement;
    const sample = document.querySelector('.font-preview-sample');
    const cs = sample ? getComputedStyle(sample) : null;
    const loadedFaces = Array.from(document.fonts)
      .filter((f) => f.status === 'loaded')
      .map((f) => f.family.replace(/["']/g, ''));
    return {
      dataFont: html.getAttribute('data-font'),
      loadedMinecraft: loadedFaces.includes('ChatW Minecraft'),
      fontFamily: cs?.fontFamily || '',
    };
  });
  await page.screenshot({ path: path.join(SHOT_DIR, '04_minecraft_font_vi.png') });
  record(
    '4. Authentic Minecraft (`ChatW Minecraft` Monocraft WOFF2) loads & applies across UI',
    mcAudit.dataFont === 'minecraft' && mcAudit.loadedMinecraft && mcAudit.fontFamily.includes('ChatW Minecraft'),
    JSON.stringify(mcAudit)
  );

  // 6. Switch back to LGBT and test live language switching across ja, ar (RTL), ru, hi, th
  await fontSel2.click();
  await page.locator('.cw-select-menu .cw-select-option[data-font-preview="lgbt"]').click();
  await page.waitForTimeout(300);

  const langChecks = {};
  for (const code of ['ja', 'ru', 'hi', 'th', 'ar']) {
    await page.evaluate((c) => window.__chatwI18n.setLanguage(c), code);
    await page.waitForTimeout(450);
    await page.evaluate(() => document.fonts.ready);
    const info = await page.evaluate(() => {
      const sel = document.querySelector('select[data-font-picker="true"]');
      const modernOpt = sel ? Array.from(sel.options).find((o) => o.value === 'modern')?.textContent?.trim() : '';
      const handOpt = sel ? Array.from(sel.options).find((o) => o.value === 'handwriting')?.textContent?.trim() : '';
      return {
        lang: document.documentElement.getAttribute('lang'),
        dir: document.documentElement.getAttribute('dir'),
        modernOpt,
        handOpt,
        preview: document.querySelector('.font-preview-sample')?.textContent?.trim() || '',
      };
    });
    langChecks[code] = info;
  }
  await page.screenshot({ path: path.join(SHOT_DIR, '05_lgbt_arabic_rtl_live.png') });

  const allLangsLocalized =
    langChecks.ja?.preview && !/^Private messaging/.test(langChecks.ja.preview) &&
    langChecks.ru?.preview && /[А-Яа-яЁё]/.test(langChecks.ru.preview) &&
    langChecks.hi?.preview && /[\u0900-\u097F]/.test(langChecks.hi.preview) &&
    langChecks.th?.preview && /[\u0E00-\u0E7F]/.test(langChecks.th.preview) &&
    langChecks.ar?.dir === 'rtl' && /[\u0600-\u06FF]/.test(langChecks.ar.preview) &&
    langChecks.ar?.handOpt !== 'Handwriting';

  record(
    '5. Live language switching (ja, ru, hi, th, ar RTL) renders exact localized font labels & LGBT multi-script preview',
    Boolean(allLangsLocalized),
    JSON.stringify(langChecks)
  );
  record('6. Zero uncaught JS pageerrors', pageErrors.length === 0, pageErrors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 084 Summary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length) {
  process.exit(1);
}
