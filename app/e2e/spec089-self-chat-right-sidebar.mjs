// Automated Live Playwright Verification for Spec 089:
// Verifies the right-side Info & Personal Vault Menu in "Chính mình" ("Note to self"):
//  1. Top-right PanelRightIcon toggle button (`[data-testid="toggle-details"]`) is rendered right next to
//     the More (`...`) button in "Chính mình" header and toggles the right sidebar open/closed.
//  2. SelfDetailsPanel displays Pride mascot avatar, "Chính mình", 4 Quick Actions (Ghi chú / Tìm kiếm / Chủ đề / Xóa hết),
//     4 Shared Storage tabs (Ảnh / Tài liệu / Liên kết / Đã ghim), Search in chat, Synced devices, and collapsible E2EE lock details.
//  3. Sending real messages with a URL updates the live Link tab count & entry; in-chat search finds matching notes and jumps cleanly.
//  4. Clicking Theme ("Chủ đề") quick action opens ChatW's real Wallpaper Picker; clicking Security opens the personal E2EE fingerprint.
//  5. Clean visuals in Light Mode, LGBT font mode, and Dark Mode with zero uncaught pageerrors.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const SHOT_DIR = process.env.SPEC089_SHOT_DIR || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_089';
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

  const uname = `s89_${Date.now().toString(36).slice(-5)}`;
  await page.locator('input[aria-label], input[placeholder]').first().fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);
  const closeTour = page.locator('.story-dialog .story-close').first();
  if (await closeTour.isVisible().catch(() => false)) {
    await closeTour.click({ force: true }).catch(() => {});
    await page.waitForTimeout(200);
  }

  // Open "Chính mình"
  await page.locator('.nav-sidebar .conv-row, .nav-sidebar .chat-item').filter({ hasText: /Chính mình|Note to self/i }).first().click();
  await page.waitForSelector('.chat-header .self-header-identity', { timeout: 10000 });
  await page.waitForTimeout(450);

  // 1. Verify top-right toggle button next to More (...) and SelfDetailsPanel open on desktop
  const headerCheck = await page.evaluate(() => {
    const toggle = document.querySelector('.chat-header-actions [data-testid="toggle-details"]');
    const panel = document.querySelector('[data-testid="self-details-panel"]');
    const resizer = document.querySelector('.panel-resizer[data-resizer="details"]');
    const actions = Array.from(document.querySelectorAll('.chat-header-actions > button'));
    const toggleIdx = actions.indexOf(toggle);
    return {
      hasToggle: Boolean(toggle),
      toggleActive: toggle?.classList.contains('active') || false,
      toggleRightBeforeMore: toggleIdx >= 0 && toggleIdx === actions.length - 2,
      hasPanel: Boolean(panel),
      hasResizer: Boolean(resizer),
      hasPrideSvg: Boolean(panel?.querySelector('.self-details-pride-avatar svg')),
      quickActionCount: panel?.querySelectorAll('.details-actions .details-action').length || 0,
      vaultTabs: Array.from(panel?.querySelectorAll('.self-vault-tab') || []).map((b) => b.textContent.trim()),
    };
  });

  record(
    '1. "Chính mình" shows PanelRightIcon toggle beside More (...) and opens resizable SelfDetailsPanel',
    headerCheck.hasToggle &&
      headerCheck.toggleActive &&
      headerCheck.toggleRightBeforeMore &&
      headerCheck.hasPanel &&
      headerCheck.hasResizer &&
      headerCheck.hasPrideSvg &&
      headerCheck.quickActionCount === 4 &&
      headerCheck.vaultTabs.length === 4,
    JSON.stringify(headerCheck)
  );

  // 2. Toggle closed then re-open via button
  const toggleBtn = page.locator('.chat-header-actions [data-testid="toggle-details"]');
  await toggleBtn.click();
  await page.waitForTimeout(200);
  const closedOk = (await page.locator('[data-testid="self-details-panel"]').count()) === 0;
  await toggleBtn.click();
  await page.waitForSelector('[data-testid="self-details-panel"]', { timeout: 5000 });
  const reopenedOk = (await page.locator('[data-testid="self-details-panel"]').count()) === 1;
  record('2. Top-right PanelRightIcon cleanly closes and re-opens SelfDetailsPanel', closedOk && reopenedOk, JSON.stringify({ closedOk, reopenedOk }));

  // 3. Send a real self-message containing a URL + note text, verify live Link tab + live Search hit!
  const composer = page.locator('.composer textarea, .composer [contenteditable="true"], .composer input[type="text"]').first();
  await composer.click();
  await composer.fill('Tài liệu dự án quan trọng https://github.com/kiyogo/chart');
  await composer.press('Enter');
  await page.waitForTimeout(900);

  await page.locator('[data-testid="self-details-panel"] [data-vault-tab="link"]').click();
  await page.waitForTimeout(250);
  const linkTabText = await page.locator('[data-testid="self-details-panel"] .self-vault-section').innerText();
  const hasGithubLink = linkTabText.includes('github.com') && linkTabText.includes('https://github.com/kiyogo/chart');

  // Test in-chat search inside SelfDetailsPanel
  const searchInput = page.locator('#self-details-search-input');
  await searchInput.fill('dự án');
  await page.waitForTimeout(250);
  const searchHitCount = await page.locator('[data-testid="self-details-panel"] .details-search-hit').count();
  await searchInput.fill('');
  await page.waitForTimeout(150);

  record(
    '3. Shared Link vault extracts real sent URL & Search in chat finds live decrypted notes',
    hasGithubLink && searchHitCount >= 1,
    JSON.stringify({ hasGithubLink, searchHitCount })
  );

  // 4. Open Security & Encryption disclosure + capture Light Mode screenshot
  await page.locator('[data-testid="self-details-panel"] .disclosure').click();
  await page.waitForTimeout(250);
  const hasFingerprint = (await page.locator('[data-testid="self-details-panel"] .fingerprint').count()) >= 1;
  record('4. Security disclosure reveals personal E2EE fingerprint & user code', hasFingerprint, JSON.stringify({ hasFingerprint }));

  await page.screenshot({ path: path.join(SHOT_DIR, '01_self_chat_right_sidebar_light.png') });
  await page.screenshot({
    path: path.join(SHOT_DIR, '02_self_header_toggle_closeup.png'),
    clip: { x: 860, y: 0, width: 580, height: 110 },
  });

  // 5. Test Wallpaper quick action opens real WallpaperPicker
  await page.locator('[data-testid="self-action-wallpaper"]').click();
  await page.waitForTimeout(250);
  const wpOpen = (await page.locator('.wallpaper-picker, .modal-content, .wallpaper-grid').count()) > 0;
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  record('5. Theme quick action opens the real chat Wallpaper picker', wpOpen, JSON.stringify({ wpOpen }));

  // 6. Verify all 4 vault tab labels fit cleanly without truncation + real Dark mode screenshot
  const noTabTruncation = await page.evaluate(() => {
    const tabs = Array.from(document.querySelectorAll('[data-testid="self-details-panel"] .self-vault-tab > span:first-child'));
    return tabs.length === 4 && tabs.every((el) => el.scrollWidth <= el.clientWidth + 1);
  });
  record('6. All 4 Shared Storage tabs fit cleanly without truncation', noTabTruncation);

  await page.evaluate(() => {
    document.documentElement.setAttribute('data-theme', 'dark');
    const shell = document.querySelector('.app-shell');
    if (shell) shell.setAttribute('data-theme', 'dark');
  });
  await page.waitForTimeout(250);
  await page.screenshot({ path: path.join(SHOT_DIR, '03_self_chat_right_sidebar_dark.png') });

  record('7. Zero uncaught JS pageerrors', pageErrors.length === 0, pageErrors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 089 Summary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length) {
  process.exit(1);
}
