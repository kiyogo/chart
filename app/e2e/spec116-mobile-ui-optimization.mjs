// Spec 116 E2E Mobile UI Optimization Verification:
// Audits 390x844 (iPhone 14 Pro) and 360x800 (Android) across Auth, Conversations,
// Active DM Chat (Header + Single-line Composer), Details Drawer, Account Bottom Sheet,
// Settings List/Detail, and Mobile Lightbox.
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const TEST_PNG = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_cnn_sr/ChatGPT_Anime_1672x941.png';
const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec116';
fs.mkdirSync(OUT_DIR, { recursive: true });

async function auditViewport(browser, vw, vh, tag, alice, bob, aliceLs, aliceCookies) {
  const ctx = await browser.newContext({
    viewport: { width: vw, height: vh },
    hasTouch: true,
    isMobile: true,
    deviceScaleFactor: 2,
  });
  const page = await ctx.newPage();
  const pageErrors = [];
  page.on('pageerror', (e) => pageErrors.push(e.message));

  // 1. Audit Auth page first (before injecting cookies)
  await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  await page.evaluate(() => {
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'dark');
  });
  await page.reload({ waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.auth-card', { timeout: 25000 });
  await page.waitForTimeout(250);

  const authCheck = await page.evaluate(() => ({
    scrollW: document.documentElement.scrollWidth,
    innerW: window.innerWidth,
  }));
  if (authCheck.scrollW > authCheck.innerW) {
    throw new Error(`[${tag}] Auth horizontal overflow: ${JSON.stringify(authCheck)}`);
  }
  if (tag === '390x844') {
    await page.screenshot({ path: path.join(OUT_DIR, '01_auth_390x844.png') });
  }

  // 2. Restore Alice session + E2EE keys
  await ctx.addCookies(aliceCookies);
  await page.evaluate((ls) => {
    for (const [k, v] of Object.entries(ls)) localStorage.setItem(k, v);
    localStorage.setItem('chatw_lang', 'vi');
    localStorage.setItem('chatw_theme', 'dark');
  }, aliceLs);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  const ct = page.locator('.story-dialog .story-close').first();
  if (await ct.isVisible().catch(() => false)) await ct.click({ force: true }).catch(() => {});
  await page.waitForTimeout(350);

  // Conversations list + LEFT vertical feature rail + CORE bottom 5-tab bar audit (Spec 117)
  const listAudit = await page.evaluate(() => {
    const rail = document.querySelector('.app-rail');
    const railRect = rail ? rail.getBoundingClientRect() : null;
    const bottomBar = document.querySelector('.mobile-bottom-bar');
    const bottomRect = bottomBar ? bottomBar.getBoundingClientRect() : null;
    const leftItems = [...document.querySelectorAll('.app-rail .rail-item')].filter((el) => getComputedStyle(el).display !== 'none').map((el) => {
      const r = el.getBoundingClientRect();
      return { label: el.getAttribute('aria-label') || '', w: Math.round(r.width), h: Math.round(r.height) };
    });
    const bottomTabs = [...document.querySelectorAll('.mobile-bottom-bar .mobile-tab')].filter((el) => getComputedStyle(el).display !== 'none').map((el) => {
      const r = el.getBoundingClientRect();
      return { label: el.getAttribute('aria-label') || '', w: Math.round(r.width), h: Math.round(r.height) };
    });
    const leftOutlineSizes = [...document.querySelectorAll('.app-rail .rail-item:not(.has-logo) svg')]
      .filter((el) => { const r = el.getBoundingClientRect(); return r.width > 0 || r.height > 0; })
      .map((el) => { const r = el.getBoundingClientRect(); return `${Math.round(r.width)}x${Math.round(r.height)}`; });
    const leftLogoSizes = [...document.querySelectorAll('.app-rail .rail-brand svg, .app-rail .rail-item.has-logo svg')]
      .filter((el) => getComputedStyle(el).display !== 'none')
      .map((el) => { const r = el.getBoundingClientRect(); return `${Math.round(r.width)}x${Math.round(r.height)}`; });
    const bottomIconSizes = [...document.querySelectorAll('.mobile-bottom-bar .mobile-tab-icon > svg, .mobile-bottom-bar .mobile-tab-icon .user-avatar')]
      .map((el) => { const r = el.getBoundingClientRect(); return `${Math.round(r.width)}x${Math.round(r.height)}`; });
    const railBorderR = rail ? getComputedStyle(rail).borderRightWidth : null;
    const bottomBorderT = bottomBar ? getComputedStyle(bottomBar).borderTopWidth : null;
    const railCs = rail ? getComputedStyle(rail) : null;
    const railBeforeCs = rail ? getComputedStyle(rail, '::before') : null;
    const sidebar = document.querySelector('.nav-sidebar');
    const sidebarRect = sidebar ? sidebar.getBoundingClientRect() : null;
    const canvasScrollbar = document.getElementById('canvas-scrollbar-overlay');
    const canvasCs = canvasScrollbar ? getComputedStyle(canvasScrollbar) : null;
    return {
      scrollW: document.documentElement.scrollWidth,
      innerW: window.innerWidth,
      innerH: window.innerHeight,
      leftRail: railRect ? { left: Math.round(railRect.left), w: Math.round(railRect.width), h: Math.round(railRect.height), bottom: Math.round(railRect.bottom) } : null,
      sidebar: sidebarRect ? { left: Math.round(sidebarRect.left), w: Math.round(sidebarRect.width), h: Math.round(sidebarRect.height), bottom: Math.round(sidebarRect.bottom) } : null,
      bottomBar: bottomRect ? { top: Math.round(bottomRect.top), bottom: Math.round(bottomRect.bottom), w: Math.round(bottomRect.width), h: Math.round(bottomRect.height) } : null,
      leftItemCount: leftItems.length,
      bottomTabCount: bottomTabs.length,
      minBottomTabW: Math.min(...bottomTabs.map((x) => x.w)),
      minBottomTabH: Math.min(...bottomTabs.map((x) => x.h)),
      railBorderR,
      bottomBorderT,
      leftOutlineSizes,
      leftLogoSizes,
      bottomIconSizes,
      railBeforeDisplay: railBeforeCs ? railBeforeCs.display : 'none',
      canvasScrollbarDisplay: canvasCs ? canvasCs.display : 'none',
    };
  });
  console.log(`[${tag}] LIST_AUDIT:`, JSON.stringify(listAudit));
  if (listAudit.scrollW > listAudit.innerW) throw new Error(`[${tag}] List horizontal overflow`);
  if (!listAudit.leftRail || listAudit.leftRail.left !== 0 || listAudit.leftRail.w > 60 || listAudit.leftRail.h < 400) {
    throw new Error(`[${tag}] Left feature rail geometry invalid: ${JSON.stringify(listAudit.leftRail)}`);
  }
  if (!listAudit.bottomBar || listAudit.bottomBar.w !== listAudit.innerW || listAudit.bottomTabCount !== 5) {
    throw new Error(`[${tag}] Bottom core 5-tab bar invalid: ${JSON.stringify(listAudit)}`);
  }
  if (listAudit.minBottomTabW < 64 || listAudit.minBottomTabH < 44) {
    throw new Error(`[${tag}] Bottom tab touch targets too small: ${JSON.stringify(listAudit)}`);
  }
  if (listAudit.railBorderR !== '0px' || listAudit.bottomBorderT !== '0px') {
    throw new Error(`[${tag}] Harsh divider borders still present: rail=${listAudit.railBorderR} bottom=${listAudit.bottomBorderT}`);
  }
  if (listAudit.leftOutlineSizes.some((s) => s !== '24x24')) {
    throw new Error(`[${tag}] Left outline icons not uniform 24x24: ${JSON.stringify(listAudit.leftOutlineSizes)}`);
  }
  if (listAudit.leftLogoSizes.some((s) => s !== '24x24')) {
    throw new Error(`[${tag}] Left logo icons not uniform 24x24: ${JSON.stringify(listAudit.leftLogoSizes)}`);
  }
  if (listAudit.bottomIconSizes.some((s) => s !== '22x22')) {
    throw new Error(`[${tag}] Bottom tab icons not uniform 22x22: ${JSON.stringify(listAudit.bottomIconSizes)}`);
  }
  if (listAudit.railBeforeDisplay !== 'none') {
    throw new Error(`[${tag}] Left rail ::before pseudo-element not disabled: ${listAudit.railBeforeDisplay}`);
  }
  if (listAudit.canvasScrollbarDisplay !== 'none') {
    throw new Error(`[${tag}] Canvas scrollbars overlay not disabled on touch mobile: ${listAudit.canvasScrollbarDisplay}`);
  }
  // Spec 119: Both rail and sidebar must stop cleanly above the bottom bar with ZERO vertical seam
  if (listAudit.sidebar && Math.abs(listAudit.sidebar.bottom - listAudit.leftRail.bottom) > 2) {
    throw new Error(`[${tag}] Rail and sidebar heights mismatched: railBottom=${listAudit.leftRail.bottom} sidebarBottom=${listAudit.sidebar.bottom}`);
  }
  if (tag === '390x844') {
    await page.screenshot({ path: path.join(OUT_DIR, '02_conversations_390x844.png') });
    // Also verify Light + Frosted Glass Wallpaper mode (Spec 118)
    const glassAudit = await page.evaluate(() => {
      const h = document.documentElement;
      h.setAttribute('data-theme', 'light');
      h.setAttribute('data-panel-glass', 'true');
      h.setAttribute('data-app-wp', 'sunset');
      h.style.setProperty('--panel-opacity', '62%');
      h.style.setProperty('--panel-blur', '18px');
      document.body.style.background = 'linear-gradient(135deg, #f5c6aa 0%, #c89f8c 50%, #7d8da6 100%)';
      const railCs = getComputedStyle(document.querySelector('.app-rail'));
      const barCs = getComputedStyle(document.querySelector('.mobile-bottom-bar'));
      return {
        railBg: railCs.backgroundColor,
        railBlur: railCs.backdropFilter,
        barBg: barCs.backgroundColor,
        barBlur: barCs.backdropFilter,
      };
    });
    console.log(`[${tag}] LIGHT_GLASS_AUDIT:`, JSON.stringify(glassAudit));
    if (!glassAudit.barBlur || glassAudit.barBlur === 'none' || !glassAudit.railBlur || glassAudit.railBlur === 'none') {
      throw new Error(`[${tag}] Frosted glass blur missing on rail/bottom bar: ${JSON.stringify(glassAudit)}`);
    }
    await page.screenshot({ path: path.join(OUT_DIR, '11_light_glass_wallpaper_390x844.png') });
    await page.evaluate(() => {
      const h = document.documentElement;
      h.setAttribute('data-theme', 'dark');
      h.removeAttribute('data-panel-glass');
      h.removeAttribute('data-app-wp');
      h.style.removeProperty('--panel-opacity');
      h.style.removeProperty('--panel-blur');
      document.body.style.background = '';
    });
  }

  // Open DM chat with Bob
  await page.locator('.conversation-item, .chat-item, .conv-item', { hasText: bob }).first().click();
  await page.waitForSelector('#dm-chat-input', { timeout: 15000 });
  await page.waitForTimeout(250);

  const chatAudit = await page.evaluate(() => {
    const hdr = document.querySelector('.chat-header')?.getBoundingClientRect();
    const metaText = document.querySelector('.chat-header-meta-text');
    const compBox = document.querySelector('.composer-box')?.getBoundingClientRect();
    const inp = document.querySelector('#dm-chat-input');
    const inpRect = inp?.getBoundingClientRect();
    const shortcutsVisible = [...document.querySelectorAll('.composer-media-shortcut')].filter((el) => getComputedStyle(el).display !== 'none').length;
    return {
      scrollW: document.documentElement.scrollWidth,
      innerW: window.innerWidth,
      hdrH: hdr ? Math.round(hdr.height) : 0,
      metaOverflow: metaText ? getComputedStyle(metaText).textOverflow : null,
      compBoxH: compBox ? Math.round(compBox.height) : 0,
      inpW: inpRect ? Math.round(inpRect.width) : 0,
      inpH: inpRect ? Math.round(inpRect.height) : 0,
      inpFontSize: inp ? getComputedStyle(inp).fontSize : null,
      shortcutsVisible,
    };
  });
  console.log(`[${tag}] CHAT_AUDIT:`, JSON.stringify(chatAudit));
  if (chatAudit.scrollW > chatAudit.innerW) throw new Error(`[${tag}] Chat horizontal overflow`);
  if (chatAudit.shortcutsVisible !== 0) throw new Error(`[${tag}] Expected redundant GIF/Sticker shortcuts hidden on mobile`);
  if (chatAudit.compBoxH > 64) throw new Error(`[${tag}] Composer too tall (wrapped?): ${chatAudit.compBoxH}px`);
  if (chatAudit.inpW < (vw >= 390 ? 180 : 150)) throw new Error(`[${tag}] Composer input too narrow: ${chatAudit.inpW}px`);
  if (chatAudit.inpFontSize !== '16px') throw new Error(`[${tag}] Composer input font-size should be 16px to prevent iOS zoom, got ${chatAudit.inpFontSize}`);
  if (chatAudit.metaOverflow !== 'ellipsis') throw new Error(`[${tag}] Header meta text must use text-overflow: ellipsis`);

  if (tag === '390x844') {
    await page.screenshot({ path: path.join(OUT_DIR, '03_chat_dm_390x844.png') });

    // Open Details drawer
    await page.locator('[data-testid="toggle-details"]').first().click();
    await page.waitForTimeout(350);
    await page.screenshot({ path: path.join(OUT_DIR, '04_details_390x844.png') });
    await page.locator('.details-panel .profile-close, .partner-info-sidebar .profile-close').first().click({ force: true });
    await page.waitForTimeout(250);

    // Send an image & open Mobile Lightbox to verify topbar zoom controls hidden and zero overflow
    const plus = page.locator('.composer-tools button').first();
    await plus.click();
    await page.waitForTimeout(200);
    await page.screenshot({ path: path.join(OUT_DIR, '05_tools_sheet_390x844.png') });
    await page.locator('.tools-menu input[type="file"][accept]').first().setInputFiles(TEST_PNG);
    await page.waitForSelector('.rich-image-btn', { timeout: 25000 });
    await page.waitForTimeout(400);
    await page.locator('.rich-image-btn').first().click();
    await page.waitForSelector('.lightbox-img', { timeout: 10000 });
    await page.waitForTimeout(300);

    const lbAudit = await page.evaluate(() => {
      const zoomVisible = [...document.querySelectorAll('.lightbox-zoom-btn, .lightbox-zoom-readout')].filter((el) => getComputedStyle(el).display !== 'none').length;
      const topbar = document.querySelector('.lightbox-topbar')?.getBoundingClientRect();
      const closeBtn = document.querySelector('.lightbox-close')?.getBoundingClientRect();
      return {
        scrollW: document.documentElement.scrollWidth,
        innerW: window.innerWidth,
        zoomVisible,
        topbarW: topbar ? Math.round(topbar.width) : 0,
        closeRight: closeBtn ? Math.round(closeBtn.right) : 0,
      };
    });
    console.log(`[${tag}] LIGHTBOX_AUDIT:`, JSON.stringify(lbAudit));
    if (lbAudit.zoomVisible !== 0) throw new Error(`[${tag}] Desktop zoom controls should hide on <= 640px Lightbox`);
    if (lbAudit.closeRight > vw) throw new Error(`[${tag}] Lightbox close button overflows viewport: ${lbAudit.closeRight} > ${vw}`);
    await page.screenshot({ path: path.join(OUT_DIR, '06_lightbox_390x844.png') });
    await page.keyboard.press('Escape');
    await page.waitForTimeout(200);

    // Back to Conversations -> open Account menu bottom sheet via bottom bar -> Settings
    await page.locator('.back-btn').first().click();
    await page.waitForTimeout(350);
    await page.locator('.mobile-tab-account').first().click();
    await page.waitForTimeout(250);
    await page.screenshot({ path: path.join(OUT_DIR, '07_account_sheet_390x844.png') });
    const settingsBtn = page.locator('.account-menu .menu-item', { hasText: /Cài đặt|Settings/i }).first();
    if (await settingsBtn.isVisible().catch(() => false)) {
      await settingsBtn.click();
      await page.waitForTimeout(350);
      await page.screenshot({ path: path.join(OUT_DIR, '08_settings_list_390x844.png') });
      await page.locator('.settings-nav-item').first().click();
      await page.waitForTimeout(350);
      await page.screenshot({ path: path.join(OUT_DIR, '09_settings_detail_390x844.png') });
    }
  } else {
    await page.screenshot({ path: path.join(OUT_DIR, '10_chat_dm_360x800.png') });
  }

  if (pageErrors.length > 0) {
    throw new Error(`[${tag}] Page errors: ${pageErrors.join(' | ')}`);
  }
  await ctx.close();
}

(async () => {
  const b = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox'],
  });
  try {
    const sfx = Date.now().toString(36).slice(-5);
    const alice = 'm116a_' + sfx;
    const bob = 'm116b_' + sfx;

    // Seed friendship between Alice & Bob on desktop helper contexts first
    const ctxA = await b.newContext({ viewport: { width: 1280, height: 800 } });
    const ctxB = await b.newContext({ viewport: { width: 1280, height: 800 } });
    const pA = await ctxA.newPage();
    const pB = await ctxB.newPage();
    for (const [p, u] of [[pA, alice], [pB, bob]]) {
      await p.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
      await p.waitForSelector('.auth-card', { timeout: 25000 });
      await p.locator('input[aria-label], input[placeholder]').first().fill(u);
      await p.locator('.auth-dev-form button[type="submit"]').click();
      await p.waitForSelector('.app-rail', { timeout: 25000 });
    }
    await pA.evaluate(async (t) => {
      await fetch('/api/friends/requests', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({ username: t }),
      });
    }, bob);
    await pB.evaluate(async (f) => {
      const l = await (await fetch('/api/friends/requests', { credentials: 'include' })).json();
      const arr = Array.isArray(l) ? l : (l.incoming || l.requests || []);
      const h = arr.find((x) => (x.from_username || x.username || x.sender_username || x.requester?.username) === f);
      if (h) {
        await fetch('/api/friends/requests/' + h.id + '/accept', {
          method: 'POST',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: '{}',
        });
      }
    }, alice);
    const aliceLs = await pA.evaluate(() => {
      const o = {};
      for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i);
        o[k] = localStorage.getItem(k);
      }
      return o;
    });
    const aliceCookies = await ctxA.cookies();
    await ctxA.close();
    await ctxB.close();

    await auditViewport(b, 390, 844, '390x844', alice, bob, aliceLs, aliceCookies);
    await auditViewport(b, 360, 800, '360x800', alice, bob, aliceLs, aliceCookies);

    console.log('SPEC 116 MOBILE UI OPTIMIZATION: 100% PASSED ON 390x844 & 360x800!');
  } finally {
    await b.close();
  }
})();
