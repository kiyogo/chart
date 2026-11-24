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

  // Conversations list + bottom tab bar audit
  const listAudit = await page.evaluate(() => {
    const rail = document.querySelector('.app-rail');
    const items = [...document.querySelectorAll('.rail-nav .rail-item')].map((el) => {
      const r = el.getBoundingClientRect();
      return { w: Math.round(r.width), h: Math.round(r.height) };
    });
    const acc = document.querySelector('.rail-account')?.getBoundingClientRect();
    return {
      scrollW: document.documentElement.scrollWidth,
      innerW: window.innerWidth,
      railH: rail ? Math.round(rail.getBoundingClientRect().height) : 0,
      minItemW: Math.min(...items.map((x) => x.w)),
      minItemH: Math.min(...items.map((x) => x.h)),
      accW: acc ? Math.round(acc.width) : 0,
      accH: acc ? Math.round(acc.height) : 0,
    };
  });
  console.log(`[${tag}] LIST_AUDIT:`, JSON.stringify(listAudit));
  if (listAudit.scrollW > listAudit.innerW) throw new Error(`[${tag}] List horizontal overflow`);
  if (listAudit.minItemW < 44 || listAudit.minItemH < 44) throw new Error(`[${tag}] Bottom rail touch targets too small: ${JSON.stringify(listAudit)}`);
  if (tag === '390x844') {
    await page.screenshot({ path: path.join(OUT_DIR, '02_conversations_390x844.png') });
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

    // Back to Conversations -> open Account menu bottom sheet -> Settings
    await page.locator('.back-btn').first().click();
    await page.waitForTimeout(350);
    await page.locator('.rail-account').first().click();
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
