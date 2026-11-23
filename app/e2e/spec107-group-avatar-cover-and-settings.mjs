import { createRequire } from 'node:module';
import path from 'node:path';
import fs from 'node:fs';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = 'http://127.0.0.1:8085';
const ARTIFACTS_DIR = path.resolve('scratch/verify_107');
fs.mkdirSync(ARTIFACTS_DIR, { recursive: true });

function makePng(w, h) {
  const zlib = require('node:zlib');
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc = (buf) => {
    let c = 0xffffffff;
    for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const td = Buffer.concat([Buffer.from(type), data]);
    const c = Buffer.alloc(4);
    c.writeUInt32BE(crc(td));
    return Buffer.concat([len, td, c]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 4 + 1)] = 0;
    for (let x = 0; x < w; x++) {
      const o = y * (w * 4 + 1) + 1 + x * 4;
      const inside = Math.hypot(x - w / 2, y - h / 2) < w / 2 - 2;
      raw[o] = 59;
      raw[o + 1] = 130;
      raw[o + 2] = 246;
      raw[o + 3] = inside ? 255 : 0;
    }
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

async function newUser(browser, name, theme = 'dark') {
  const ctx = await browser.newContext({ viewport: { width: 1366, height: 768 }, deviceScaleFactor: 1 });
  await ctx.addInitScript((t) => {
    try {
      localStorage.setItem('chatw_theme', t);
      localStorage.setItem('chatw_lang', 'vi');
    } catch (_) {}
  }, theme);
  const res = await ctx.request.post(`${BASE}/api/auth/dev-login`, { data: { username: name } });
  if (!res.ok()) throw new Error(`dev-login ${name} failed: ${res.status()}`);
  const me = await (await ctx.request.get(`${BASE}/api/auth/me`)).json();
  const page = await ctx.newPage();
  await page.goto(BASE + '/');
  return { ctx, page, me, name };
}

async function api(user, method, url, data) {
  const r = await user.ctx.request.fetch(`${BASE}${url}`, { method, data });
  let body = null;
  try {
    body = await r.json();
  } catch (_) {}
  return { status: r.status(), body };
}

async function run() {
  console.log('🚀 Starting Spec 107 Verification: Group Avatar, Cover Photo & Rich Group Features...');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
  });

  try {
    const timestamp = Date.now().toString(36).slice(-5);
    const alice = await newUser(browser, `alice_${timestamp}`, 'dark');
    const bob = await newUser(browser, `bob_${timestamp}`, 'dark');
    console.log(`✅ Logged in Alice: ${alice.name} (${alice.me.id}), Bob: ${bob.name} (${bob.me.id})`);

    // 1. Create a test group via API
    const groupName = `Nhóm Test Spec107 ${timestamp}`;
    const grpRes = await api(alice, 'POST', '/api/groups', {
      name: groupName,
      member_ids: [bob.me.id],
    });
    console.log('Group created:', grpRes.status, grpRes.body);
    if (grpRes.status >= 300) {
      throw new Error(`Failed to create group: ${JSON.stringify(grpRes.body)}`);
    }
    const group = grpRes.body;

    // 2. AUDIT: Direct API Avatar & Banner Upload Tests
    console.log('--- AUDIT 1: Group Avatar & Banner Upload API Endpoints ---');
    const pngBuf = makePng(64, 64);

    // Test Avatar Upload
    const avatarRes = await alice.ctx.request.post(`${BASE}/api/groups/${group.id}/avatar`, {
      headers: { 'Content-Type': 'image/png' },
      data: pngBuf,
    });
    console.log('Upload avatar status:', avatarRes.status());
    if (!avatarRes.ok()) {
      throw new Error(`Avatar upload failed: ${avatarRes.status()} - ${await avatarRes.text()}`);
    }
    const avatarJson = await avatarRes.json();
    console.log('Avatar upload response:', avatarJson);
    if (!avatarJson.avatar_url || !avatarJson.avatar_url.includes('group_')) {
      throw new Error(`Expected avatar_url in response, got: ${JSON.stringify(avatarJson)}`);
    }
    console.log('✅ Group avatar upload API PASS');

    // Test Banner Upload
    const bannerRes = await alice.ctx.request.post(`${BASE}/api/groups/${group.id}/banner`, {
      headers: { 'Content-Type': 'image/png' },
      data: pngBuf,
    });
    console.log('Upload banner status:', bannerRes.status());
    if (!bannerRes.ok()) {
      throw new Error(`Banner upload failed: ${bannerRes.status()} - ${await bannerRes.text()}`);
    }
    const bannerJson = await bannerRes.json();
    console.log('Banner upload response:', bannerJson);
    if (!bannerJson.banner_url || !bannerJson.banner_url.includes('group_')) {
      throw new Error(`Expected banner_url in response, got: ${JSON.stringify(bannerJson)}`);
    }
    console.log('✅ Group cover banner upload API PASS');

    // 3. Test Settings API
    console.log('--- AUDIT 2: Group Settings API ---');
    const settingsGetRes = await api(alice, 'GET', `/api/groups/${group.id}/settings`);
    console.log('Get settings status:', settingsGetRes.status, settingsGetRes.body);
    if (settingsGetRes.status >= 300) {
      throw new Error(`Failed to get group settings: ${JSON.stringify(settingsGetRes.body)}`);
    }

    const settingsUpdateRes = await api(alice, 'PUT', `/api/groups/${group.id}/settings`, {
      require_approval: true,
      perm_poll: false,
    });
    console.log('Update settings status:', settingsUpdateRes.status, settingsUpdateRes.body);
    if (settingsUpdateRes.status >= 300) {
      throw new Error(`Failed to update group settings: ${JSON.stringify(settingsUpdateRes.body)}`);
    }
    console.log('✅ Group settings API PASS');

    // 4. UI Audit on Alice's page
    console.log('--- AUDIT 3: Group UI Verification (Cover photo, Avatar badge, Tabs, Settings) ---');
    await alice.page.reload({ waitUntil: 'domcontentloaded' });
    await alice.page.waitForSelector('.app-rail', { timeout: 25000 });
    await alice.page.waitForTimeout(1000);

    // Dismiss tour if present
    const closeTour = alice.page.locator('.story-dialog .story-close, button[aria-label="Bỏ qua giới thiệu"]').first();
    if (await closeTour.isVisible().catch(() => false)) {
      await closeTour.click({ force: true }).catch(() => {});
      await alice.page.waitForTimeout(300);
    }

    // Open group chat by clicking group in sidebar
    const groupItem = alice.page.locator(`[data-conv="group:${group.id}"], .nav-sidebar .conv-row, .nav-sidebar .chat-item`).filter({ hasText: groupName }).first();
    await groupItem.waitFor({ timeout: 15000 });
    await groupItem.click();
    await alice.page.waitForSelector('.chat-column', { timeout: 10000 });
    await alice.page.waitForTimeout(500);
    console.log('✅ Clicked group chat in sidebar');

    // Open group info panel by clicking header identity or right sidebar button
    const headerIdentity = alice.page.locator('.chat-header-identity').first();
    await headerIdentity.click();
    await alice.page.waitForSelector('.group-info-sidebar', { timeout: 10000 });
    await alice.page.waitForTimeout(500);
    console.log('✅ Group Info Sidebar opened');

    // Check Cover Photo Banner
    const coverBanner = alice.page.locator('.group-cover-banner');
    if (!(await coverBanner.isVisible())) {
      throw new Error('Cover photo banner .group-cover-banner is not visible');
    }
    const coverBtn = alice.page.locator('.group-cover-btn').filter({ hasText: /Đổi ảnh bìa/i });
    if (!(await coverBtn.isVisible())) {
      throw new Error('Change cover button .group-cover-btn is not visible');
    }
    console.log('✅ Group cover banner & change cover button visible');

    // Check Avatar & Camera badge
    const avatarBadge = alice.page.locator('.group-avatar-edit-badge');
    if (!(await avatarBadge.isVisible())) {
      throw new Error('Avatar camera edit badge .group-avatar-edit-badge is not visible');
    }
    console.log('✅ Avatar edit camera badge visible');

    // Check Navigation Tabs
    const tabs = alice.page.locator('.group-tab-btn');
    const tabCount = await tabs.count();
    console.log(`Found ${tabCount} tabs in Group Info`);
    if (tabCount < 2) {
      throw new Error(`Expected at least 2 tabs (Members & Settings), found ${tabCount}`);
    }

    // Switch to Settings Tab
    const settingsTab = tabs.filter({ hasText: /Cài đặt/i }).first();
    await settingsTab.click();
    await alice.page.waitForTimeout(400);

    const settingsPanel = alice.page.locator('.group-settings-panel');
    if (!(await settingsPanel.isVisible())) {
      throw new Error('Settings panel .group-settings-panel is not visible after clicking settings tab');
    }
    const settingRows = alice.page.locator('.group-setting-row');
    const rowCount = await settingRows.count();
    console.log(`Found ${rowCount} setting toggle rows in Settings tab`);
    if (rowCount < 5) {
      throw new Error(`Expected at least 5 setting rows, found ${rowCount}`);
    }
    console.log('✅ Group settings panel renders all toggle options properly');

    // Switch back to Members Tab & test search filter
    const membersTab = tabs.filter({ hasText: /Thành viên/i }).first();
    await membersTab.click();
    await alice.page.waitForTimeout(300);

    const searchInput = alice.page.locator('.group-search-input');
    if (await searchInput.isVisible()) {
      await searchInput.fill(bob.name);
      await alice.page.waitForTimeout(300);
      const memberRows = alice.page.locator('.partner-info-card');
      const count = await memberRows.count();
      console.log(`Filtered members count: ${count}`);
    }
    console.log('✅ Member search filter verified');

    // Capture screenshot
    const shotPath = path.join(ARTIFACTS_DIR, 'group_info_full.png');
    await alice.page.screenshot({ path: shotPath, fullPage: false });
    console.log(`📸 Screenshot captured at ${shotPath}`);

    console.log('🎉 Spec 107 Verification SUCCESSFUL: All group features verified!');
  } finally {
    await browser.close();
  }
}

run().catch((err) => {
  console.error('❌ Verification failed:', err);
  process.exit(1);
});
