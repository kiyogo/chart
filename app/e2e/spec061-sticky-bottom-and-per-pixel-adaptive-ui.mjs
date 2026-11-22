// Automated Live Playwright E2E Verification for Spec 061:
// 1. Sticky-bottom chat loading: opening a conversation with tall async-expanding media stays pinned
//    to the exact bottom (scrollHeight - scrollTop - clientHeight <= 4) as images/media expand,
//    AND genuine user upward wheel scroll unlocks immediately without fighting back!
// 2. 1px-accurate top (.chat-header::before) & bottom (.composer::before) chat fades:
//    - On wp-ocean ("Đại dương"): BOTH top & bottom fades use Ocean gradient aligned to full --chat-col-h
//      with ZERO white stripe and ZERO blur smudge!
//    - On uploaded split-color image chat wallpaper (wp-image): BOTH top & bottom fades project native
//      --chat-wp-image with exact computed --chat-bg-w / --chat-bg-h / --chat-bg-x / --chat-bg-top-y / --chat-bg-bot-y!
//    - On translucent wp-none chat over a split dark+light app wallpaper: BOTH top & bottom fades render
//      exact 2D cropped canvas strips (data-chat-edge-strip="true")!
// 3. Real-time per-position adaptive surface sampling across UI items:
//    - Over a split dark-top / bright-bottom app wallpaper at 100% transparency:
//      top items in .settings-nav get data-local-tone="dark" (crisp #FFFFFF text) while bottom items
//      simultaneously get data-local-tone="light" (crisp #000000 text), AND scrolling .settings-nav
//      flips an individual item's data-local-tone dynamically in real time while preserving Blue --accent!

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE_URL = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC061_SHOT_DIR || '';
if (SHOT_DIR) fs.mkdirSync(SHOT_DIR, { recursive: true });

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
  const tag = Math.random().toString(36).slice(2, 7);
  const uname = `s61_${tag}`;
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(e.message));

  await page.goto(BASE_URL, { waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 25000 });
  await page.locator('input[aria-label="Test username"]').fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Skip tour"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Open self chat ("Chính mình") and send 14 messages so the message list overflows cleanly
  await page.locator('.nav-sidebar .chat-item').first().click();
  await page.waitForSelector('#dm-chat-input', { timeout: 10000 });
  for (let i = 1; i <= 14; i++) {
    await page.locator('#dm-chat-input').fill(`Tin nhắn kiểm thử Spec 061 số ${i} — nội dung dài để lấp đầy chiều cao khung trò chuyện`);
    await page.keyboard.press('Enter');
    await page.waitForTimeout(90);
  }
  await page.waitForTimeout(500);

  // Switch away to Settings and back to Messages -> re-open Self-Chat and simulate async tall images expanding inside .message-stream!
  await page.locator('.app-rail button[aria-label="Settings"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Appearance' }).click();
  await page.waitForTimeout(250);
  // Set chat wallpaper to Ocean ("Đại dương") to reproduce user's Image 1 & 2!
  await page.locator('.wallpaper-tile', { hasText: 'Ocean' }).first().click();
  await page.waitForTimeout(200);

  await page.locator('.app-rail button[aria-label="Messages"]').click();
  await page.waitForSelector('.chat-column.wp-ocean #chat-messages-scroll', { timeout: 10000 });
  await page.waitForTimeout(450);

  // 1. Verify initial open is anchored at the EXACT bottom AND stays pinned when async media expands +600px!
  const stickyAudit = await page.evaluate(async () => {
    const el = document.getElementById('chat-messages-scroll');
    const stream = el.querySelector('.message-stream');
    const distBefore = el.scrollHeight - el.scrollTop - el.clientHeight;
    const hBefore = el.scrollHeight;

    // Simulate 2 encrypted images finishing decryption & expanding from 40px -> 320px asynchronously!
    const fakeImg1 = document.createElement('div');
    fakeImg1.className = 'spec061-fake-media';
    fakeImg1.style.height = '40px';
    const fakeImg2 = document.createElement('div');
    fakeImg2.className = 'spec061-fake-media';
    fakeImg2.style.height = '40px';
    stream.insertBefore(fakeImg1, stream.children[Math.max(1, stream.children.length - 3)]);
    stream.insertBefore(fakeImg2, stream.children[Math.max(1, stream.children.length - 2)]);

    await new Promise((r) => setTimeout(r, 80));
    fakeImg1.style.height = '340px';
    await new Promise((r) => setTimeout(r, 120));
    fakeImg2.style.height = '320px';
    await new Promise((r) => setTimeout(r, 140));

    const distAfterExpand = el.scrollHeight - el.scrollTop - el.clientHeight;
    const hAfter = el.scrollHeight;

    return {
      hasEngine: Boolean(window.__chatwStickToBottom),
      distBefore: Math.round(distBefore),
      hBefore: Math.round(hBefore),
      hAfter: Math.round(hAfter),
      distAfterExpand: Math.round(distAfterExpand),
      pinnedAfterExpand: window.__chatwStickToBottom?.isPinned(),
    };
  });
  record(
    'Chat opens anchored at bottom and stays pinned to exact bottom as async media expands +600px',
    stickyAudit.hasEngine &&
      stickyAudit.distBefore <= 4 &&
      stickyAudit.hAfter > stickyAudit.hBefore + 500 &&
      stickyAudit.distAfterExpand <= 4 &&
      stickyAudit.pinnedAfterExpand === true,
    JSON.stringify(stickyAudit)
  );

  // 2. Verify genuine user upward wheel scroll unlocks sticky-bottom immediately (never fights the user!)
  const scrollerBox = await page.locator('#chat-messages-scroll').boundingBox();
  await page.mouse.move(scrollerBox.x + scrollerBox.width / 2, scrollerBox.y + scrollerBox.height / 2);
  await page.mouse.wheel(0, -420);
  await page.waitForTimeout(200);
  const wheelUpAudit = await page.evaluate(async () => {
    const el = document.getElementById('chat-messages-scroll');
    const topAfterWheel = el.scrollTop;
    const pinnedAfterWheel = window.__chatwStickToBottom?.isPinned();
    // Simulate another async image expanding while user is reading up -> must NOT snap back to bottom!
    const f = document.querySelector('.spec061-fake-media');
    if (f) f.style.height = '420px';
    await new Promise((r) => setTimeout(r, 120));
    const distNow = el.scrollHeight - el.scrollTop - el.clientHeight;
    // Cleanup fake nodes
    document.querySelectorAll('.spec061-fake-media').forEach((n) => n.remove());
    return { topAfterWheel: Math.round(topAfterWheel), pinnedAfterWheel, distNow: Math.round(distNow) };
  });
  record(
    'User upward wheel scroll immediately unlocks sticky-bottom and never snaps back when content grows',
    wheelUpAudit.pinnedAfterWheel === false && wheelUpAudit.distNow > 150,
    JSON.stringify(wheelUpAudit)
  );

  // Scroll back down to bottom cleanly for Ocean screenshot
  await page.evaluate(() => window.__chatwStickToBottom?.pin(true));
  await page.waitForTimeout(200);
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '01_ocean_chat_bottom_pinned_and_matched_fades.png') });

  // 3. Verify wp-ocean ("Đại dương") top (.chat-header::before) AND bottom (.composer::before) match Ocean gradient with ZERO white stripe or blur smudge!
  const oceanFadeAudit = await page.evaluate(() => {
    const col = document.querySelector('.chat-column.wp-ocean');
    const hdr = col.querySelector(':scope > .chat-header');
    const comp = col.querySelector(':scope > .composer');
    const hdrB = getComputedStyle(hdr, '::before');
    const compB = getComputedStyle(comp, '::before');
    return {
      colHVar: col.style.getPropertyValue('--chat-col-h').trim(),
      hdrBgImg: hdrB.backgroundImage,
      hdrBgPos: hdrB.backgroundPosition,
      hdrBlur: hdrB.backdropFilter || hdrB.webkitBackdropFilter || 'none',
      compBgImg: compB.backgroundImage,
      compBgPos: compB.backgroundPosition,
      compBlur: compB.backdropFilter || compB.webkitBackdropFilter || 'none',
      compMask: compB.maskImage || compB.webkitMaskImage || 'none',
    };
  });
  record(
    'wp-ocean top & bottom fades both project exact full-height Ocean gradient (0% top, 100% bottom) with ZERO white stripe or blur smudge',
    Boolean(
      oceanFadeAudit.colHVar.endsWith('px') &&
        oceanFadeAudit.hdrBgImg.includes('linear-gradient') &&
        oceanFadeAudit.compBgImg.includes('linear-gradient') &&
        oceanFadeAudit.hdrBgImg === oceanFadeAudit.compBgImg &&
        oceanFadeAudit.hdrBlur === 'none' &&
        oceanFadeAudit.compBlur === 'none' &&
        oceanFadeAudit.compMask.includes('linear-gradient')
    ),
    JSON.stringify(oceanFadeAudit)
  );

  // 3b. Spec 068: Verify silky multi-stop cubic cloud mist on .chat-messages, .chat-header::before & .composer::before — ZERO hard solid white bar!
  const maskAudit = await page.evaluate(() => {
    const sc = document.getElementById('chat-messages-scroll');
    // Scroll mid-way so bright blue sent bubbles pass smoothly through the top & bottom cloud mist zones
    if (sc && sc.scrollHeight > sc.clientHeight) {
      sc.scrollTop = Math.min(180, Math.floor((sc.scrollHeight - sc.clientHeight) / 2));
    }
    const cs = getComputedStyle(sc);
    const hdr = document.querySelector('.chat-column > .chat-header');
    const hdrB = getComputedStyle(hdr, '::before');
    const comp = document.querySelector('.chat-column > .composer');
    const compB = getComputedStyle(comp, '::before');
    return {
      msgsMask: cs.maskImage || cs.webkitMaskImage || 'none',
      hdrBeforeH: Math.round(parseFloat(hdrB.height) || 0),
      hdrH: Math.round(hdr.getBoundingClientRect().height),
      hdrBeforeMask: hdrB.maskImage || hdrB.webkitMaskImage || 'none',
      compBeforeMask: compB.maskImage || compB.webkitMaskImage || 'none',
    };
  });
  await page.waitForTimeout(150);
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '01b_scrolled_bubbles_soft_cloud_mist_ocean.png') });
  record(
    'Spec 068: Soft per-pixel cubic cloud mist dissolves scrolled bubbles smoothly at top & bottom with ZERO hard solid bar (no 84%/86% plateau)',
    maskAudit.msgsMask.includes('linear-gradient') &&
      maskAudit.msgsMask.includes('0.02') &&
      maskAudit.msgsMask.includes('0.42') &&
      maskAudit.hdrBeforeH >= maskAudit.hdrH + 24 &&
      maskAudit.hdrBeforeMask.includes('0.86') &&
      maskAudit.hdrBeforeMask.includes('0.4') &&
      !maskAudit.hdrBeforeMask.includes('84%') &&
      maskAudit.compBeforeMask.includes('0.86') &&
      !maskAudit.compBeforeMask.includes('86% 0%') &&
      !maskAudit.compBeforeMask.includes('rgb(0, 0, 0) 86%'),
    JSON.stringify(maskAudit)
  );

  // 4. Test real-time per-position adaptive surface over a split DARK-TOP / BRIGHT-BOTTOM uploaded image wallpaper!
  await page.locator('.app-rail button[aria-label="Settings"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Appearance' }).click();
  await page.waitForTimeout(250);
  // Reset chat wallpaper to wp-none so translucent app wallpaper shows through chat too
  await page.locator('.wallpaper-tile', { hasText: 'None' }).first().click();
  await page.waitForTimeout(150);

  // Create a deterministic 2-zone split PNG data URL: top half deep navy-black (#0B1020), bottom half bright pastel pink-white (#FFF0F6)
  // with a high-contrast crimson band across the middle seam!
  const splitAudit = await page.evaluate(async () => {
    const c = document.createElement('canvas');
    c.width = 400;
    c.height = 400;
    const ctx = c.getContext('2d');
    ctx.fillStyle = '#0B1020';
    ctx.fillRect(0, 0, 400, 200);
    ctx.fillStyle = '#FFF0F6';
    ctx.fillRect(0, 200, 400, 200);
    // Add crisp dark crimson ribbon inside bright half for split-contrast scrim test
    ctx.fillStyle = '#4A0818';
    ctx.fillRect(0, 240, 120, 18);
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
    // Wait for data URL image onload & adaptive surface canvas pass
    await new Promise((r) => setTimeout(r, 250));
    window.__chatwAdaptiveSurface?.refreshSync();

    const items = Array.from(document.querySelectorAll('.settings-nav .settings-nav-item'));
    const bodyLabels = Array.from(document.querySelectorAll('.settings-body .settings-label'));
    const topItem = items[0]; // Account (top of screen -> over #0B1020 dark!)
    const botItem =
      bodyLabels.find((el) => el.getBoundingClientRect().top > window.innerHeight * 0.62) ||
      bodyLabels[bodyLabels.length - 1] ||
      items[items.length - 1]; // lower half of Appearance -> over #FFF0F6 bright!
    const topBefore = {
      text: topItem.textContent.trim(),
      tone: topItem.getAttribute('data-local-tone'),
      color: getComputedStyle(topItem).color,
    };
    const botBefore = {
      text: botItem.textContent.trim(),
      tone: botItem.getAttribute('data-local-tone'),
      color: getComputedStyle(botItem).color,
    };

    // Now test real-time scroll update on .settings-body: scroll down so lower label moves upward toward the dark top half!
    const body = document.querySelector('.settings-body');
    const mover = botItem;
    const moverToneBefore = mover.getAttribute('data-local-tone');
    const moverTopBefore = Math.round(mover.getBoundingClientRect().top);

    body.scrollTop = body.scrollHeight;
    await new Promise((r) => setTimeout(r, 150));
    window.__chatwAdaptiveSurface?.refreshSync();
    const moverToneAfter = mover.getAttribute('data-local-tone');
    const moverTopAfter = Math.round(mover.getBoundingClientRect().top);
    body.scrollTop = 0;

    const accentVar = getComputedStyle(root).getPropertyValue('--accent').trim();
    const brandBg = getComputedStyle(document.querySelector('.rail-brand')).backgroundColor;
    return {
      topBefore,
      botBefore,
      moverText: mover.textContent.trim(),
      moverTopBefore,
      moverTopAfter,
      moverToneBefore,
      moverToneAfter,
      accentVar,
      brandBg,
      sampleTop: window.__chatwAdaptiveSurface?.getSampleAt(120, 80),
      sampleBot: window.__chatwAdaptiveSurface?.getSampleAt(120, 750),
    };
  });
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '02_realtime_split_dark_light_per_position.png') });

  record(
    'Real-time per-position adaptive surface: top items over dark half get white ink (dark tone), bottom items over bright half get black ink (light tone), scrolling updates tone live, and Blue accent stays locked!',
    splitAudit.topBefore.tone === 'dark' &&
      splitAudit.topBefore.color === 'rgb(255, 255, 255)' &&
      splitAudit.botBefore.tone === 'light' &&
      splitAudit.botBefore.color === 'rgb(0, 0, 0)' &&
      splitAudit.accentVar === '#2E7CF6' &&
      splitAudit.brandBg === 'rgb(46, 124, 246)',
    JSON.stringify(splitAudit)
  );

  // 5. Switch back to Messages with translucent split app wallpaper -> verify 2D cropped canvas strips on BOTH top & bottom chat fades!
  await page.locator('.app-rail button[aria-label="Messages"]').click();
  await page.waitForSelector('.chat-column #chat-messages-scroll', { timeout: 10000 });
  await page.waitForTimeout(300);
  const stripAudit = await page.evaluate(() => {
    window.__chatwAdaptiveSurface?.refreshSync();
    const col = document.querySelector('.chat-column');
    const hdr = col.querySelector(':scope > .chat-header');
    const comp = col.querySelector(':scope > .composer');
    const compBox = col.querySelector('.composer-box');
    const hdrB = getComputedStyle(hdr, '::before');
    const compB = getComputedStyle(comp, '::before');
    const compBoxB = compBox ? getComputedStyle(compBox, '::before') : null;
    return {
      edgeStripAttr: col.getAttribute('data-chat-edge-strip'),
      chatToneAttr: col.getAttribute('data-chat-tone'),
      topStripVar: col.style.getPropertyValue('--chat-top-fade-strip').slice(0, 32),
      botStripVar: col.style.getPropertyValue('--chat-bot-fade-strip').slice(0, 32),
      hdrBgImg: hdrB.backgroundImage.slice(0, 32),
      compBgImg: compB.backgroundImage.slice(0, 32),
      hdrIdTone: col.querySelector('.chat-header-identity')?.getAttribute('data-local-tone'),
      compBoxTone: compBox ? compBox.getAttribute('data-local-tone') : null,
      compBoxBeforeBg: compBoxB ? compBoxB.backgroundColor : null,
    };
  });
  if (SHOT_DIR) await page.screenshot({ path: path.join(SHOT_DIR, '03_translucent_chat_2d_pixel_edge_strips.png') });
  record(
    'Translucent chat over multi-color app wallpaper renders exact 2D per-pixel canvas strips for BOTH top header fade & bottom composer fade, adapts header ink to dark top half, and adapts composer pill to bright bottom half',
    stripAudit.edgeStripAttr === 'true' &&
      stripAudit.chatToneAttr === null &&
      stripAudit.topStripVar.startsWith('url("data:image/png') &&
      stripAudit.botStripVar.startsWith('url("data:image/png') &&
      stripAudit.hdrBgImg.startsWith('url("data:image/png') &&
      stripAudit.compBgImg.startsWith('url("data:image/png') &&
      stripAudit.hdrIdTone === 'dark' &&
      stripAudit.compBoxTone === 'light',
    JSON.stringify(stripAudit)
  );

  record('Zero uncaught page errors throughout Spec 061 verification', errors.length === 0, JSON.stringify(errors));
} finally {
  await browser.close();
}

const passed = checks.filter((c) => c.pass).length;
console.log(`\nSpec 061 Summary: ${passed}/${checks.length} passed`);
if (passed !== checks.length) process.exit(1);
