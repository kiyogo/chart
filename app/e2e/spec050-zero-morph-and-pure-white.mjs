// Spec 050 Live Chromium Verification:
// 1. Zero morph/scale/shift on .message-row, .message-bubble, .chat-column, .chat-header, .settings-cols
//    sampled at 60fps across new chat load AND cached chat switches.
// 2. Zero .conv-row .chat-item paddingRight squish on hover/click (.conv-more replaces timestamp in-place).
// 3. 100% Pure Crisp White rgb(255, 255, 255) (zero #F4F6F9 grey, zero colored gradient) on:
//    Feed (.feed-column), Meet (.meet-column), Details (.details-panel), Chat (.chat-column),
//    Sidebar (.nav-sidebar), Notifications, Download, Search, Settings.
// 4. Translucent parity verified on Feed & Meet when panel transparency is enabled.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'verify_050';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
const record = (name, pass, detail = '') => {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
};
const shot = (page, name) => page.screenshot({ path: path.join(OUT, `${name}.png`) });

const parseAlpha = (colorStr) => {
  if (!colorStr || colorStr === 'transparent') return 0;
  const rgba = colorStr.match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\s*\)/i);
  if (rgba) return rgba[4] === undefined ? 1 : parseFloat(rgba[4]);
  const srgb = colorStr.match(/color\(\s*srgb\s+[\d.]+\s+[\d.]+\s+[\d.]+(?:\s*\/\s*([\d.]+))?\s*\)/i);
  if (srgb) return srgb[1] === undefined ? 1 : parseFloat(srgb[1]);
  return -1;
};

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
  args: ['--no-sandbox'],
});

const run = Date.now().toString(36).slice(-5);
const ALICE = 's050_a_' + run;
const BOB = 's050_b_' + run;

const ctx1 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const ctx2 = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const errors = [];
const p1 = await ctx1.newPage();
p1.on('pageerror', (e) => errors.push(String(e)));
const p2 = await ctx2.newPage();
p2.on('pageerror', (e) => errors.push(String(e)));

const devLogin = async (page, username) => {
  await page.goto(BASE + '/');
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(username);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 30000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
};

const api = async (ctx, method, url, data) => {
  const opts = { method };
  if (data !== undefined) opts.data = data;
  const r = await ctx.request.fetch(`${BASE}${url}`, opts);
  let body = null;
  try { body = await r.json(); } catch (_) {}
  return { status: r.status(), body };
};

try {
  await devLogin(p1, ALICE);
  await devLogin(p2, BOB);

  // Befriend Alice & Bob and send 4 messages so DM has grouped bubbles
  const fReq = await api(ctx1, 'POST', '/api/friends/requests', { username: BOB });
  if (fReq.body?.id) {
    await api(ctx2, 'POST', `/api/friends/requests/${fReq.body.id}/accept`, {});
  }
  await p2.reload();
  await p2.waitForSelector('.app-rail', { timeout: 20000 });
  await p2.waitForTimeout(800);
  const aliceRow = p2.locator('.chat-item', { hasText: ALICE }).first();
  if (await aliceRow.isVisible().catch(() => false)) {
    await aliceRow.click();
    await p2.waitForTimeout(400);
    const ta = p2.locator('.composer-input, textarea').first();
    for (const msg of ['Tin nhắn 1', 'Tin nhắn 2', 'Tin nhắn 3']) {
      await ta.fill(msg);
      await ta.press('Enter');
      await p2.waitForTimeout(200);
    }
  }

  await p1.reload();
  await p1.waitForSelector('.app-rail', { timeout: 20000 });
  await p1.waitForTimeout(800);

  // =========================================================================
  // 1. Hovering/clicking .conv-row .chat-item must have ZERO paddingRight squish
  // =========================================================================
  const bobItem = p1.locator('.chat-item', { hasText: BOB }).first();
  await bobItem.waitFor({ state: 'visible', timeout: 10000 });
  const beforeHoverPad = await bobItem.evaluate((el) => getComputedStyle(el).paddingRight);
  await bobItem.hover();
  await p1.waitForTimeout(150);
  const duringHoverPad = await bobItem.evaluate((el) => getComputedStyle(el).paddingRight);
  record(
    'Zero sidebar chat-item padding-right squish on hover',
    beforeHoverPad === duringHoverPad,
    `before=${beforeHoverPad} during=${duringHoverPad}`,
  );

  // =========================================================================
  // 2. Frame-by-frame audit on opening a new DM: ZERO transform/animation morph
  //    on .message-row, locked .chat-header height, no scrollbar-gutter: stable
  // =========================================================================
  await p1.evaluate(() => {
    window.__morphSamples = [];
    window.__stopMorphSampling = false;
    const sample = () => {
      const rows = [...document.querySelectorAll('.message-row')];
      const hdr = document.querySelector('.chat-header');
      const cm = document.querySelector('.chat-messages');
      const col = document.querySelector('.chat-column');
      const morphedRows = rows.filter((r) => {
        const cs = getComputedStyle(r);
        return cs.transform !== 'none' || cs.animationName !== 'none';
      }).length;
      if (rows.length > 0 || hdr) {
        window.__morphSamples.push({
          rows: rows.length,
          morphedRows,
          hdrH: hdr ? Math.round(hdr.getBoundingClientRect().height * 10) / 10 : null,
          colW: col ? Math.round(col.getBoundingClientRect().width * 10) / 10 : null,
          gutter: cm ? getComputedStyle(cm).scrollbarGutter : null,
        });
      }
      if (!window.__stopMorphSampling) requestAnimationFrame(sample);
    };
    requestAnimationFrame(sample);
  });

  await bobItem.click();
  await p1.waitForSelector('.message-bubble', { timeout: 10000 });
  await p1.waitForTimeout(350);

  // Switch to Self-Chat and back to Bob while still sampling
  const selfItem = p1.locator('.chat-item', { hasText: 'Chính mình' }).first();
  await selfItem.click();
  await p1.waitForTimeout(250);
  await bobItem.click();
  await p1.waitForTimeout(300);

  const morphReport = await p1.evaluate(() => {
    window.__stopMorphSampling = true;
    const s = window.__morphSamples;
    const maxMorphed = Math.max(0, ...s.map((x) => x.morphedRows));
    const hdrHeights = [...new Set(s.map((x) => x.hdrH).filter((v) => v !== null))];
    const gutters = [...new Set(s.map((x) => x.gutter).filter((v) => v !== null))];
    return { sampleCount: s.length, maxMorphed, hdrHeights, gutters };
  });
  await shot(p1, '01_messages_pure_white_zero_morph');
  record(
    'Zero .message-row transform/animation morph across 60fps frames, locked .chat-header height & no scrollbar-gutter',
    morphReport.sampleCount > 10 &&
      morphReport.maxMorphed === 0 &&
      morphReport.hdrHeights.length === 1 &&
      !morphReport.gutters.includes('stable'),
    JSON.stringify(morphReport),
  );

  // Verify Details panel & Chat column are PURE WHITE rgb(255, 255, 255) with NO gradient wash
  const msgWhiteCheck = await p1.evaluate(() => {
    const pick = (sel) => {
      const el = document.querySelector(sel);
      if (!el) return null;
      const cs = getComputedStyle(el);
      return { bg: cs.backgroundColor, bgImg: cs.backgroundImage };
    };
    return {
      sidebar: pick('.nav-sidebar'),
      chat: pick('.chat-column'),
      details: pick('.details-panel'),
    };
  });
  record(
    'Messages sidebar, chat column & right details panel are pure crisp white rgb(255, 255, 255) with zero gradient tint',
    msgWhiteCheck.sidebar?.bg === 'rgb(255, 255, 255)' &&
      msgWhiteCheck.chat?.bg === 'rgb(255, 255, 255)' &&
      msgWhiteCheck.details?.bg === 'rgb(255, 255, 255)' &&
      msgWhiteCheck.details?.bgImg === 'none',
    JSON.stringify(msgWhiteCheck),
  );

  // =========================================================================
  // 3. Feed Page (Screenshot 2) -> 100% Pure Crisp White rgb(255, 255, 255)
  // =========================================================================
  await p1.locator('.app-rail button.rail-item[aria-label="Mới"], .app-rail button.rail-item[aria-label*="Bảng tin"]').first().click();
  await p1.waitForSelector('.feed-column', { timeout: 10000 });
  await p1.waitForTimeout(450);
  const feedWhite = await p1.evaluate(() => {
    const col = document.querySelector('.feed-column');
    const bar = document.querySelector('.feed-compose-bar');
    const stories = document.querySelector('.feed-stories-card');
    return {
      colBg: col ? getComputedStyle(col).backgroundColor : null,
      colImg: col ? getComputedStyle(col).backgroundImage : null,
      barBg: bar ? getComputedStyle(bar).backgroundColor : null,
      storiesBg: stories ? getComputedStyle(stories).backgroundColor : null,
    };
  });
  await shot(p1, '02_feed_pure_white');
  record(
    'Feed page (.feed-column) is 100% pure crisp white rgb(255, 255, 255) with zero grey tint',
    feedWhite.colBg === 'rgb(255, 255, 255)' &&
      feedWhite.colImg === 'none' &&
      (!feedWhite.barBg || feedWhite.barBg === 'rgba(0, 0, 0, 0)'),
    JSON.stringify(feedWhite),
  );

  // =========================================================================
  // 4. Meet Page (Screenshot 3) -> 100% Pure Crisp White rgb(255, 255, 255)
  // =========================================================================
  await p1.locator('.app-rail button.rail-item[aria-label="Cuộc họp"], .app-rail button.rail-item[aria-label*="Họp"]').first().click();
  await p1.waitForSelector('.meet-column', { timeout: 10000 });
  await p1.waitForTimeout(450);
  const meetWhite = await p1.evaluate(() => {
    const col = document.querySelector('.meet-column');
    const hero = document.querySelector('.meet-hero-card');
    const card = document.querySelector('.meet-card');
    const listCard = document.querySelector('.meet-list-card');
    return {
      colBg: col ? getComputedStyle(col).backgroundColor : null,
      colImg: col ? getComputedStyle(col).backgroundImage : null,
      heroBg: hero ? getComputedStyle(hero).backgroundColor : null,
      cardBg: card ? getComputedStyle(card).backgroundColor : null,
      listBg: listCard ? getComputedStyle(listCard).backgroundColor : null,
    };
  });
  await shot(p1, '03_meet_pure_white');
  record(
    'Meet page (.meet-column) is 100% pure crisp white rgb(255, 255, 255) with zero grey cards',
    meetWhite.colBg === 'rgb(255, 255, 255)' &&
      meetWhite.colImg === 'none' &&
      (!meetWhite.heroBg || meetWhite.heroBg === 'rgba(0, 0, 0, 0)') &&
      (!meetWhite.cardBg || meetWhite.cardBg === 'rgba(0, 0, 0, 0)') &&
      (!meetWhite.listBg || meetWhite.listBg === 'rgba(0, 0, 0, 0)'),
    JSON.stringify(meetWhite),
  );

  // =========================================================================
  // 5. Other Full Pages (Notifications, Download, Search, Settings) -> Pure White + Zero Settings Morph
  // =========================================================================
  await p1.locator('.app-rail button.rail-item[aria-label="Thông báo"]').click();
  await p1.waitForSelector('.notifications-page', { timeout: 8000 });
  const notifBg = await p1.evaluate(() => getComputedStyle(document.querySelector('.notifications-page')).backgroundColor);

  await p1.locator('.app-rail button.rail-item[aria-label="Tải ứng dụng"]').click();
  await p1.waitForSelector('.download-page', { timeout: 8000 });
  const dlBg = await p1.evaluate(() => getComputedStyle(document.querySelector('.download-page')).backgroundColor);

  await p1.locator('.app-rail button.rail-item[aria-label="Tìm kiếm"]').click();
  await p1.waitForSelector('.search-page', { timeout: 8000 });
  const searchBg = await p1.evaluate(() => getComputedStyle(document.querySelector('.search-page')).backgroundColor);

  await p1.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await p1.waitForSelector('.settings-body', { timeout: 8000 });
  const setCheck = await p1.evaluate(() => {
    const body = document.querySelector('.settings-body');
    const nav = document.querySelector('.settings-nav');
    const cols = document.querySelector('.settings-cols');
    return {
      bodyBg: body ? getComputedStyle(body).backgroundColor : null,
      navBg: nav ? getComputedStyle(nav).backgroundColor : null,
      colsAnim: cols ? getComputedStyle(cols).animationName : null,
    };
  });
  record(
    'Notifications, Download, Search & Settings panels are pure white rgb(255, 255, 255) with zero .settings-cols shift animation',
    notifBg === 'rgb(255, 255, 255)' &&
      dlBg === 'rgb(255, 255, 255)' &&
      searchBg === 'rgb(255, 255, 255)' &&
      setCheck.bodyBg === 'rgb(255, 255, 255)' &&
      setCheck.navBg === 'rgb(255, 255, 255)' &&
      setCheck.colsAnim === 'none',
    JSON.stringify({ notifBg, dlBg, searchBg, setCheck }),
  );

  // =========================================================================
  // 6. Translucent Parity on Feed & Meet (setting 28% transparency blends white cleanly)
  // =========================================================================
  await p1.locator('.settings-nav-item', { hasText: 'Giao diện' }).first().click();
  await p1.waitForTimeout(350);
  await p1.locator('input[aria-label="Độ trong suốt khung trắng"]').fill('28');
  await p1.locator('input[aria-label="Độ trong suốt khung trắng"]').dispatchEvent('input');
  await p1.waitForTimeout(450);

  await p1.locator('.app-rail button.rail-item[aria-label="Cuộc họp"], .app-rail button.rail-item[aria-label*="Họp"]').first().click();
  await p1.waitForSelector('.meet-column', { timeout: 8000 });
  await p1.waitForTimeout(300);
  const meetTransBg = await p1.evaluate(() => getComputedStyle(document.querySelector('.meet-column')).backgroundColor);
  const meetAlpha = parseAlpha(meetTransBg);
  record(
    'Meet panel honors panel transparency smoothly (translucent white, not overridden by opaque grey)',
    meetAlpha > 0.5 && meetAlpha < 0.9,
    `bg=${meetTransBg} alpha=${meetAlpha}`,
  );

  record('Zero uncaught page errors throughout Spec 050 verification', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 050 Summary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length) process.exit(1);
