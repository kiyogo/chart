// Automated Live Playwright E2E Verification for Spec 076:
// Verifies:
//  1. Orphaned `local-*` pending bubbles in IndexedDB `chat_messages` are auto-pruned on chat open.
//  2. System `.message-bubble.error-bubble` and error toast translate 100% into Vietnamese (0 English leakage),
//     have crisp non-white timestamp contrast, and show ZERO `.pending-clock` icons.
//  3. Settings -> Riêng tư & bảo mật -> Bảo mật shows live red "Khóa không khớp" badge when a pending/mismatched
//     device logs in, and clicking "Đặt lại khóa thiết bị" -> "Xác nhận đặt lại khóa" promotes the device,
//     rotates keys, flips badge to green "Đang hoạt động an toàn", and immediately unblocks E2EE messaging.
//  4. Post-reset DM delivery succeeds cleanly and page reload has 0 resurrected ghost pending bubbles.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC076_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_076';
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

async function dismissTourIfOpen(page) {
  const tourClose = page.locator('.story-dialog .story-close, .story-dialog .btn-secondary').first();
  if (await tourClose.isVisible().catch(() => false)) {
    await tourClose.click({ force: true }).catch(() => {});
    await page.waitForTimeout(250);
  }
}

async function loginFresh(ctx, username, lang = 'en') {
  const page = await ctx.newPage();
  page.on('pageerror', (e) => pageErrors.push(`[${username}] ${e.message}`));
  await page.goto(BASE + '/', { waitUntil: 'networkidle' });
  if (lang !== 'en') {
    await page.evaluate((l) => {
      localStorage.setItem('chatw_lang', l);
    }, lang);
    await page.reload({ waitUntil: 'networkidle' });
  }
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Test username"], input[placeholder]').first().fill(username);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(900);
  await dismissTourIfOpen(page);
  return page;
}

try {
  const tag = Date.now().toString(36).slice(-5);
  const ALICE = `s76_a_${tag}`;
  const BOB = `s76_b_${tag}`;

  const ctxA1 = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const ctxB = await browser.newContext({ viewport: { width: 1400, height: 900 } });

  // 1. Bootstrap trusted Device A1 for ALICE (Vietnamese account) and BOB, and make them friends.
  const pageA1 = await loginFresh(ctxA1, ALICE, 'vi');
  const pageB = await loginFresh(ctxB, BOB, 'en');

  const meA = await (await ctxA1.request.get(`${BASE}/api/auth/me`)).json();
  const meB = await (await ctxB.request.get(`${BASE}/api/auth/me`)).json();

  // Persist Vietnamese on ALICE's server prefs just like a real Vietnamese user account
  await ctxA1.request.put(`${BASE}/api/prefs`, { data: { language: 'vi' } }).catch(() => {});

  const frResp = await ctxA1.request.post(`${BASE}/api/friends/requests`, { data: { username: BOB } });
  const frJson = await frResp.json().catch(() => ({}));
  if (frJson?.id) {
    await ctxB.request.post(`${BASE}/api/friends/requests/${frJson.id}/accept`, { data: {} });
  }

  // Wait briefly so Device A1 registers as trusted device & keys exist on server
  await pageA1.waitForTimeout(800);

  // Capture ALICE Device A1's valid local keypair (keys1) for ALICE's own user_id,
  // then rotate ALICE's server key on Device A1 to keys2 via Settings -> Security!
  // Doing this before Device A2 logs in avoids Device A1's incoming-approval modal intercepting Settings.
  const aliceKeys1 = await pageA1.evaluate((uid) => {
    const wrap = localStorage.getItem('__chatw_device_kwrap_v1');
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i);
      if (k && k.includes(uid) && k.includes('e2ee_keys')) {
        return { k, v: localStorage.getItem(k), wrap };
      }
    }
    return null;
  }, meA.id);

  await dismissTourIfOpen(pageA1);
  await pageA1.locator('.app-rail button[aria-label*="Cài đặt"], .app-rail button[title*="Cài đặt"]').first().click();
  await pageA1.waitForSelector('.settings-nav', { timeout: 10000 });
  await pageA1.locator('.settings-nav-item', { hasText: /Riêng tư & bảo mật|Privacy & security/ }).first().click();
  await pageA1.waitForTimeout(250);
  await pageA1.locator('.settings-subtab', { hasText: /Bảo mật|Security/ }).first().click();
  await pageA1.waitForTimeout(500);
  await pageA1.locator('button', { hasText: /Đặt lại khóa thiết bị|Reset device key/ }).first().click();
  await pageA1.waitForTimeout(250);
  await pageA1.locator('button', { hasText: /Xác nhận đặt lại khóa|Confirm reset key/ }).first().click();
  await pageA1.waitForSelector('.status-badge.success', { timeout: 10000 });

  // 2. Open secondary browser context (Device A2) for ALICE in Vietnamese (`vi`)
  //    Simulating a real-world mismatched/pending Chrome browser holding stale keys1 while server has keys2!
  const ctxA2 = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const pageA2 = await loginFresh(ctxA2, ALICE, 'vi');
  await pageA2.waitForTimeout(1200);

  const recModal1 = pageA2.locator('.modal-dialog:not(.story-dialog)').first();
  if (await recModal1.isVisible().catch(() => false)) {
    await recModal1.locator('.btn-ghost').first().click();
    await pageA2.waitForTimeout(300);
  }
  await dismissTourIfOpen(pageA2);

  // Inject keys1 (matching aliceId AAD, mismatched vs server keys2!) + 2 orphaned `local-*` pending bubbles into Device A2
  await pageA2.evaluate(async ({ aliceId, bobId, aliceKeys1 }) => {
    if (aliceKeys1 && aliceKeys1.v) {
      if (aliceKeys1.wrap) localStorage.setItem('__chatw_device_kwrap_v1', aliceKeys1.wrap);
      localStorage.setItem(aliceKeys1.k, aliceKeys1.v);
    }
    localStorage.setItem('chatw_lang', 'vi');
    if (window.__chatwStorage && window.__chatwStorage.putMessages) {
      const convKey = `dm:${aliceId}:${bobId}`;
      const now = new Date().toISOString();
      await window.__chatwStorage.putMessages(convKey, [
        {
          id: 'local-stale-orphan-1',
          seq: 1,
          sender_id: aliceId,
          recipient_id: bobId,
          text: 'STALE_GHOST_1',
          created_at: now,
          is_self: true,
          status: 'Pending',
          reply_to_id: null,
          reactions: [],
          is_pinned: false,
        },
        {
          id: 'local-stale-orphan-2',
          seq: 2,
          sender_id: aliceId,
          recipient_id: bobId,
          text: 'STALE_GHOST_2',
          created_at: now,
          is_self: true,
          status: 'Pending',
          reply_to_id: null,
          reactions: [],
          is_pinned: false,
        },
      ], false);
    }
  }, { aliceId: meA.id, bobId: meB.id, aliceKeys1 });

  // Reload Device A2 so it boots in Vietnamese with the mismatched key + orphaned IndexedDB messages
  await pageA2.reload({ waitUntil: 'load' });
  await pageA2.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA2.waitForTimeout(1500);

  // Dismiss recovery modal ("Để sau") and welcome tour so we can test opening the chat & triggering the mismatch error
  const recModal2 = pageA2.locator('.modal-dialog:not(.story-dialog)').first();
  if (await recModal2.isVisible().catch(() => false)) {
    const modalText = await recModal2.innerText();
    record('Recovery modal renders in Vietnamese on mismatch', /Đồng bộ hoặc khôi phục khóa E2EE/.test(modalText), modalText.slice(0, 80));
    await recModal2.locator('.btn-ghost').first().click();
    await pageA2.waitForTimeout(300);
  }
  await dismissTourIfOpen(pageA2);

  // Open BOB's DM on Device A2
  await pageA2.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA2.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA2.waitForTimeout(1000);

  // Check 1: Orphaned `local-stale-orphan-*` pending bubbles are GONE from DOM and IndexedDB!
  const ghostDomCount = await pageA2.evaluate(() => {
    const body = document.body.innerText || '';
    const hasGhostText = body.includes('STALE_GHOST_1') || body.includes('STALE_GHOST_2');
    const pendingRows = document.querySelectorAll('.message-row.pending').length;
    return { hasGhostText, pendingRows };
  });
  const idbAfterHydrate = await pageA2.evaluate(async ({ aliceId, bobId }) => {
    if (!window.__chatwStorage?.getMessages) return [];
    const rec = await window.__chatwStorage.getMessages(`dm:${aliceId}:${bobId}`);
    return (rec?.messages || []).map((m) => m.id);
  }, { aliceId: meA.id, bobId: meB.id });

  record(
    'Orphaned local-* pending bubbles auto-pruned from DOM and IndexedDB',
    !ghostDomCount.hasGhostText && ghostDomCount.pendingRows === 0 && !idbAfterHydrate.some((id) => id.startsWith('local-')),
    `dom=${JSON.stringify(ghostDomCount)} idbIds=${JSON.stringify(idbAfterHydrate)}`
  );

  // Check 2: Sending a message on mismatched Device A2 produces 100% Vietnamese error bubble + toast,
  // ZERO clock icon on error bubble, and readable non-white timestamp contrast!
  await pageA2.locator('#dm-chat-input').fill('test mismatch message');
  await pageA2.locator('#dm-chat-input').press('Enter');
  await pageA2.waitForSelector('.message-bubble.error-bubble', { timeout: 10000 });
  await pageA2.waitForTimeout(600);

  const errInspect = await pageA2.evaluate(() => {
    const bubble = document.querySelector('.message-bubble.error-bubble');
    const toastEl = document.querySelector('.toast-banner, .toast, [role="alert"]');
    const meta = bubble?.querySelector('.bubble-meta');
    const clockInErr = bubble?.querySelector('.pending-clock');
    const metaColor = meta ? getComputedStyle(meta).color : '';
    const bgColor = bubble ? getComputedStyle(bubble).backgroundColor : '';
    return {
      bubbleText: (bubble?.innerText || '').trim(),
      toastText: (toastEl?.innerText || '').trim(),
      hasClockInErr: Boolean(clockInErr),
      metaColor,
      bgColor,
    };
  });

  await pageA2.screenshot({ path: path.join(SHOT_DIR, '01_vi_error_bubble_clean_no_stale_pending.png') });

  const expectedViPhrase = 'Khóa mã hóa trên thiết bị này không khớp với tài khoản của bạn';
  const hasEnglishLeak = /This device's encryption key does not match/.test(errInspect.bubbleText + ' ' + errInspect.toastText);
  // Check timestamp color isn't faint near-white (handles both rgb(...) and color(srgb ...))
  let metaReadable = false;
  const rgbMatch = errInspect.metaColor.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/);
  const srgbMatch = errInspect.metaColor.match(/color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)/);
  if (rgbMatch) {
    metaReadable = Number(rgbMatch[2]) < 150 && Number(rgbMatch[3]) < 150;
  } else if (srgbMatch) {
    metaReadable = Number(srgbMatch[2]) < 0.6 && Number(srgbMatch[3]) < 0.6;
  }

  record(
    'Error bubble & toast render 100% in Vietnamese (0 English leakage)',
    errInspect.bubbleText.includes(expectedViPhrase) && !hasEnglishLeak,
    `bubble="${errInspect.bubbleText.slice(0, 90)}" toast="${errInspect.toastText.slice(0, 90)}"`
  );
  record(
    'Error bubble has ZERO pending clock icon and crisp readable timestamp',
    !errInspect.hasClockInErr && metaReadable,
    `hasClock=${errInspect.hasClockInErr} metaColor=${errInspect.metaColor} bg=${errInspect.bgColor}`
  );

  // Check 3: Open Settings -> Riêng tư & bảo mật -> Bảo mật
  await pageA2.locator('.app-rail button[title="Cài đặt"], .app-rail button[aria-label="Cài đặt"], .app-rail .rail-btn').nth(7).click().catch(async () => {
    await pageA2.locator('button[aria-label*="Cài đặt"], button[title*="Cài đặt"]').first().click();
  });
  await pageA2.waitForSelector('.settings-nav', { timeout: 10000 });
  await pageA2.locator('.settings-nav-item', { hasText: /Riêng tư & bảo mật|Privacy & security/ }).first().click();
  await pageA2.waitForTimeout(300);
  await pageA2.locator('.settings-subtab', { hasText: /Bảo mật|Security/ }).first().click();
  await pageA2.waitForTimeout(1000);

  const secBefore = await pageA2.locator('.settings-content').innerText();
  await pageA2.screenshot({ path: path.join(SHOT_DIR, '02_vi_settings_security_key_mismatch.png') });

  record(
    'Settings > Privacy & security > Security shows live Vietnamese "Khóa không khớp" + Sync & Reset controls',
    secBefore.includes('Khóa không khớp')
      && secBefore.includes('Đồng bộ từ thiết bị tin cậy')
      && secBefore.includes('Đặt lại khóa thiết bị')
      && !secBefore.includes('Key mismatch')
      && !secBefore.includes('Reset device key'),
    secBefore.slice(0, 220).replace(/\s+/g, ' ')
  );

  // Click "Đặt lại khóa thiết bị" -> "Xác nhận đặt lại khóa"
  await pageA2.locator('button', { hasText: 'Đặt lại khóa thiết bị' }).first().click();
  await pageA2.waitForTimeout(300);
  const confirmBtn = pageA2.locator('button', { hasText: 'Xác nhận đặt lại khóa' }).first();
  record('Two-step reset confirmation appears in Vietnamese', await confirmBtn.isVisible());

  await confirmBtn.click();
  await pageA2.waitForSelector('.settings-value.tag-active', { timeout: 12000 });
  await pageA2.waitForTimeout(600);

  const secAfter = await pageA2.locator('.settings-content').innerText();
  await pageA2.screenshot({ path: path.join(SHOT_DIR, '03_vi_settings_security_key_reset_ok.png') });

  record(
    '1-click device key reset promotes pending device, updates keys, and shows Vietnamese success',
    secAfter.includes('Đang hoạt động an toàn')
      && secAfter.includes('Đã đặt lại khóa mã hóa thiết bị. Bạn có thể gửi tin nhắn ngay bây giờ.'),
    secAfter.slice(0, 220).replace(/\s+/g, ' ')
  );

  // Check 4: Exit Settings back to Messages and immediately send an E2EE DM to BOB -> succeeds with 0 errors!
  await pageA2.locator('.app-rail button[aria-label*="Tin nhắn"], .app-rail button[aria-label*="Messages"]').first().click();
  await pageA2.waitForTimeout(400);
  await pageA2.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA2.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA2.waitForTimeout(500);

  const liveMsg = `Xin chào sau khi đặt lại khóa ${tag}`;
  await pageA2.locator('#dm-chat-input').fill(liveMsg);
  await pageA2.locator('#dm-chat-input').press('Enter');
  await pageA2.waitForTimeout(1800);

  const postSend = await pageA2.evaluate((msgText) => {
    const rows = Array.from(document.querySelectorAll('.message-row'));
    const matching = rows.filter((r) => (r.innerText || '').includes(msgText));
    const pending = rows.filter((r) => r.classList.contains('pending')).length;
    const errors = document.querySelectorAll('.message-bubble.error-bubble').length;
    return { matched: matching.length, pending, errors };
  }, liveMsg);

  await pageA2.screenshot({ path: path.join(SHOT_DIR, '04_dm_sent_after_key_reset.png') });

  record(
    'DM sends cleanly right after key reset (0 errors, 0 pending clocks)',
    postSend.matched === 1 && postSend.pending === 0 && postSend.errors === 0,
    JSON.stringify(postSend)
  );

  // Reload to verify 0 resurrected ghost pending bubbles or rejected error bubbles!
  await pageA2.reload({ waitUntil: 'load' });
  await pageA2.waitForSelector('.app-rail', { timeout: 25000 });
  await pageA2.waitForTimeout(1200);
  await pageA2.locator('.nav-sidebar .chat-item', { hasText: BOB }).first().click();
  await pageA2.waitForSelector('#dm-chat-input', { timeout: 10000 });
  await pageA2.waitForTimeout(1000);

  const afterReload = await pageA2.evaluate((msgText) => {
    const body = document.body.innerText || '';
    const pending = document.querySelectorAll('.message-row.pending').length;
    return {
      hasLiveMsg: body.includes(msgText),
      hasStaleGhost: body.includes('STALE_GHOST_1') || body.includes('STALE_GHOST_2'),
      pending,
    };
  }, liveMsg);

  record(
    'Reload preserves real message with 0 ghost pending bubbles',
    afterReload.hasLiveMsg && !afterReload.hasStaleGhost && afterReload.pending === 0,
    JSON.stringify(afterReload)
  );

  record('Zero uncaught JS pageerrors', pageErrors.length === 0, pageErrors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length) {
  process.exit(1);
}
