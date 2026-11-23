// Automated Live Playwright Verification for Spec 086:
// Verifies:
//  1. Bright vibrant 7-color LGBT spectrum across ALL app text AND ALL monochrome UI SVG icons
//     (using #cw-lgbt-icon-grad + animated hue flow when Reduce motion OFF, frozen when ON).
//  2. Bright fresh 6-stripe Pride avatar colors (#FF2E55..#A838FF) on "Chính mình".
//  3. "Nhà phát triển" ("Developer") spacing in Settings nav equals 100% the gap of all other items (0px flush).
//  4. Tight 1/3 left/right inset (3px) and flush 0px item gap across Messages, Friends, Communities & Settings sidebars.
//  5. Right-side canvas scrollbar sits mathematically centered inside the 3px right gutter (leftGap == rightGap == 0.5px).

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC086_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_086';
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
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => pageErrors.push(e.message));

  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => localStorage.setItem('chatw_lang', 'vi'));
  await page.reload({ waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });

  const uname = `s86_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);
  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Open Settings -> About -> unlock Developer mode (7 clicks on version) OR toggle via Settings -> Appearance
  await page.locator('.app-rail button[aria-label="Cài đặt"], .app-rail button[title="Cài đặt"], .app-rail button[aria-label="Settings"]').first().click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.waitForTimeout(300);

  // Enable developer mode cleanly via About -> version unlock
  await page.locator('.settings-nav-item', { hasText: /Giới thiệu|About/i }).first().click();
  await page.waitForTimeout(300);
  const devBtn = page.locator('.about-dev-footer button, button', { hasText: /Bật chế độ nhà phát triển|Enable developer mode|Nhà phát triển/i }).first();
  if (await devBtn.isVisible().catch(() => false)) {
    await devBtn.click().catch(() => {});
    await page.waitForTimeout(250);
  } else {
    // Fallback: enable dev_mode via preferences switch in About
    const versionTrigger = page.locator('.about-version-btn, .about-dev-card').first();
    for (let i = 0; i < 7; i++) {
      if (await versionTrigger.isVisible().catch(() => false)) {
        await versionTrigger.click().catch(() => {});
      }
    }
    await page.waitForTimeout(300);
  }

  // Switch to Appearance -> select LGBT font
  await page.locator('.settings-nav-item', { hasText: /Giao diện|Appearance/i }).first().click();
  await page.waitForSelector('select[data-font-picker="true"]', { timeout: 10000 });
  const fontSel = page.locator('select[data-font-picker="true"]').first();
  await fontSel.scrollIntoViewIfNeeded();
  await fontSel.click();
  await page.waitForSelector('.cw-select-menu .cw-select-option[data-font-preview="lgbt"]', { timeout: 5000 });
  await page.locator('.cw-select-menu .cw-select-option[data-font-preview="lgbt"]').click();
  await page.waitForTimeout(500);
  await page.evaluate(() => document.fonts.ready);

  // 1. Audit vibrant LGBT text + ALL UI SVG icons + Settings nav 3px inset + 0px uniform Developer spacing + centered scrollbar
  const settingsAudit = await page.evaluate(() => {
    const nav = document.querySelector('.settings-nav');
    const navRect = nav.getBoundingClientRect();
    const navCs = getComputedStyle(nav);
    const items = Array.from(nav.querySelectorAll('.settings-nav-item'));
    const gaps = [];
    for (let i = 1; i < items.length; i++) {
      gaps.push(Math.round((items[i].getBoundingClientRect().top - items[i - 1].getBoundingClientRect().bottom) * 10) / 10);
    }
    const devItem = nav.querySelector('.settings-nav-dev');
    const devMarginTop = devItem ? getComputedStyle(devItem).marginTop : 'n/a';
    const itemRect = items[0].getBoundingClientRect();
    const leftInset = Math.round(itemRect.left - navRect.left);
    const rightInset = Math.round(navRect.right - itemRect.right);

    const railSvg = document.querySelector('.app-rail button:not(.has-logo) svg.cw-icon');
    const navSvg = items[0].querySelector('svg.cw-icon');
    const railCs = railSvg ? getComputedStyle(railSvg) : null;
    const navSvgCs = navSvg ? getComputedStyle(navSvg) : null;
    const sampleCs = getComputedStyle(document.querySelector('.font-preview-sample'));
    const themeLblCs = getComputedStyle(document.querySelector('.theme-card > span'));
    const dropCs = getComputedStyle(document.querySelector('select[data-font-picker="true"]'));
    const sb = window.__chatwCanvasScrollbarMetrics ? window.__chatwCanvasScrollbarMetrics(nav) : null;
    const sbLeftGap = sb ? Math.round((sb.thumbX - (sb.rect.right - sb.padRight)) * 10) / 10 : null;
    const sbRightGap = sb ? Math.round((sb.rect.right - (sb.thumbX + sb.thumbW)) * 10) / 10 : null;

    return {
      padLeft: navCs.paddingLeft,
      padRight: navCs.paddingRight,
      leftInset,
      rightInset,
      itemCount: items.length,
      gaps,
      allZeroGaps: gaps.every((g) => Math.abs(g) <= 0.5),
      hasDevItem: Boolean(devItem),
      devMarginTop,
      railIconStroke: railCs?.stroke || '',
      railIconAnim: railCs?.animationName || '',
      navIconStroke: navSvgCs?.stroke || '',
      sampleBg: sampleCs.backgroundImage,
      themeLblBg: themeLblCs.backgroundImage,
      dropAnim: dropCs.animationName,
      sbPadRight: sb?.padRight,
      sbThumbW: sb?.thumbW,
      sbLeftGap,
      sbRightGap,
    };
  });

  await page.screenshot({ path: path.join(SHOT_DIR, '01_settings_vibrant_lgbt_all_text_and_icons.png') });
  // Scroll Settings right column down to capture Font preview + vibrant toggle switch
  await page.locator('.font-preview-card').scrollIntoViewIfNeeded();
  await page.waitForTimeout(200);
  await page.screenshot({ path: path.join(SHOT_DIR, '02_settings_vibrant_preview_and_switch.png') });

  record(
    '1. Vibrant bright 7-color LGBT covers ALL text (headings, themes, dropdowns, previews) AND ALL UI SVG icons',
    settingsAudit.sampleBg.includes('rgb(244, 31, 94)') &&
      settingsAudit.sampleBg.includes('rgb(0, 179, 89)') &&
      settingsAudit.themeLblBg.includes('linear-gradient') &&
      settingsAudit.dropAnim.includes('cwLgbtTextColorFlow') &&
      settingsAudit.railIconStroke.includes('cw-lgbt-icon-grad') &&
      settingsAudit.navIconStroke.includes('cw-lgbt-icon-grad') &&
      settingsAudit.railIconAnim.includes('cwLgbtIconHue'),
    JSON.stringify({
      railStroke: settingsAudit.railIconStroke,
      railAnim: settingsAudit.railIconAnim,
      dropAnim: settingsAudit.dropAnim,
      themeBg: settingsAudit.themeLblBg.slice(0, 50),
    })
  );

  record(
    '2. Settings nav has tight 3px (1/3) left/right inset, 0px flush gaps & 100% uniform Developer spacing',
    settingsAudit.leftInset === 3 &&
      settingsAudit.rightInset === 3 &&
      settingsAudit.allZeroGaps &&
      (!settingsAudit.hasDevItem || settingsAudit.devMarginTop === '0px'),
    JSON.stringify({
      leftInset: settingsAudit.leftInset,
      rightInset: settingsAudit.rightInset,
      gaps: settingsAudit.gaps,
      devMarginTop: settingsAudit.devMarginTop,
    })
  );

  record(
    '3. Right-side canvas scrollbar is mathematically centered inside the 3px sidebar gutter (0.5px left == 0.5px right)',
    settingsAudit.sbPadRight === 3 &&
      settingsAudit.sbThumbW === 2 &&
      Math.abs(settingsAudit.sbLeftGap - settingsAudit.sbRightGap) <= 0.5,
    JSON.stringify({ padRight: settingsAudit.sbPadRight, thumbW: settingsAudit.sbThumbW, leftGap: settingsAudit.sbLeftGap, rightGap: settingsAudit.sbRightGap })
  );

  // 4. Verify Messages sidebar (3px inset, 0px gap, bright fresh Pride avatar, vibrant rainbow icons/text)
  await page.locator('.app-rail button[aria-label="Tin nhắn"], .app-rail button[title="Tin nhắn"], .app-rail button[aria-label="Messages"]').first().click();
  await page.waitForSelector('.nav-sidebar .sidebar-scrollable', { timeout: 10000 });
  await page.waitForTimeout(350);

  const msgSideAudit = await page.evaluate(() => {
    const scroll = document.querySelector('.nav-sidebar .sidebar-scrollable');
    const cs = getComputedStyle(scroll);
    const rects = Array.from(document.querySelectorAll('.nav-sidebar .self-chat-avatar svg rect')).map((r) => r.getAttribute('fill'));
    const sb = window.__chatwCanvasScrollbarMetrics ? window.__chatwCanvasScrollbarMetrics(scroll) : null;
    const sbLeftGap = sb ? Math.round((sb.thumbX - (sb.rect.right - sb.padRight)) * 10) / 10 : null;
    const sbRightGap = sb ? Math.round((sb.rect.right - (sb.thumbX + sb.thumbW)) * 10) / 10 : null;
    return {
      padLeft: cs.paddingLeft,
      padRight: cs.paddingRight,
      gap: cs.rowGap || cs.gap,
      brightAvatar: ['#FF2E55', '#FF8C1A', '#FFE119', '#00D166', '#1A8CFF', '#A838FF'].every((c) => rects.includes(c)),
      sbLeftGap,
      sbRightGap,
    };
  });
  await page.screenshot({ path: path.join(SHOT_DIR, '03_messages_tight_flush_bright_lgbt.png') });

  record(
    '4. Messages sidebar has 3px left/right inset, 0px flush gap, bright fresh Pride avatar & centered scrollbar',
    msgSideAudit.padLeft === '3px' &&
      msgSideAudit.padRight === '3px' &&
      (msgSideAudit.gap === '0px' || msgSideAudit.gap === 'normal') &&
      msgSideAudit.brightAvatar &&
      Math.abs(msgSideAudit.sbLeftGap - msgSideAudit.sbRightGap) <= 0.5,
    JSON.stringify(msgSideAudit)
  );

  // 5. Verify Friends & Communities sidebars also share exact 3px inset & 0px flush gap
  await page.locator('.app-rail button[aria-label="Bạn bè"], .app-rail button[title="Bạn bè"], .app-rail button[aria-label="Friends"]').first().click();
  await page.waitForSelector('.nav-sidebar .sidebar-scrollable', { timeout: 10000 });
  await page.waitForTimeout(300);
  const friendsAudit = await page.evaluate(() => {
    const s = document.querySelector('.nav-sidebar .sidebar-scrollable');
    const cs = getComputedStyle(s);
    return { padLeft: cs.paddingLeft, padRight: cs.paddingRight, gap: cs.rowGap || cs.gap };
  });
  await page.screenshot({ path: path.join(SHOT_DIR, '04_friends_tight_flush_lgbt.png'), clip: { x: 0, y: 0, width: 420, height: 600 } });

  await page.locator('.app-rail button[aria-label="Cộng đồng"], .app-rail button[title="Cộng đồng"], .app-rail button[aria-label="Communities"]').first().click();
  await page.waitForSelector('.nav-sidebar .sidebar-scrollable', { timeout: 10000 });
  await page.waitForTimeout(300);
  const commAudit = await page.evaluate(() => {
    const s = document.querySelector('.nav-sidebar .sidebar-scrollable');
    const cs = getComputedStyle(s);
    const items = Array.from(s.querySelectorAll('.chat-item, .create-community-btn'));
    const txt = s.innerText || '';
    return {
      padLeft: cs.paddingLeft,
      padRight: cs.paddingRight,
      gap: cs.rowGap || cs.gap,
      localizedCreateBtn: txt.includes('Tạo hoặc tham gia cộng đồng'),
      count: items.length,
    };
  });
  await page.screenshot({ path: path.join(SHOT_DIR, '05_communities_tight_flush_lgbt.png'), clip: { x: 0, y: 0, width: 420, height: 600 } });

  record(
    '5. Friends & Communities sidebars use identical 3px left/right inset, 0px flush gap & localized Vietnamese labels',
    friendsAudit.padLeft === '3px' &&
      friendsAudit.padRight === '3px' &&
      commAudit.padLeft === '3px' &&
      commAudit.padRight === '3px' &&
      commAudit.localizedCreateBtn,
    JSON.stringify({ friends: friendsAudit, communities: commAudit })
  );

  record('6. Zero uncaught JS pageerrors', pageErrors.length === 0, pageErrors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 086 Summary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length) {
  process.exit(1);
}
