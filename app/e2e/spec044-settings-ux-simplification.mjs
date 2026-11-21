// Live Chromium verification for Spec 044: Ultra-Intuitive Settings UI/UX Simplification
// - Zero technical jargon (PBKDF2, salt, ECDH, HKDF, AES-256, ECDSA, PKCE, OAuth)
// - Zero ellipses "..." in all settings tabs
// - Zero unwanted parentheses "(...)" in settings labels, descriptions, and buttons
// - Plain, short, everyday Vietnamese for ultra-easy use (IQ 60 level friendly)
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_044';
fs.mkdirSync(OUT, { recursive: true });

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
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(e.message));

  const suffix = `${Date.now().toString(36).slice(-5)}${Math.floor(Math.random() * 90 + 10)}`;
  await page.goto(`${BASE}/`);
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`s044_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Open Settings modal
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });

  // Function to inspect current settings-body text
  async function inspectBody(tabName) {
    await page.waitForTimeout(300);
    const bodyText = await page.locator('.settings-body').innerText();
    const hasEllipsis = bodyText.includes('...') || bodyText.includes('…');
    // Check for unwanted parentheses in labels / descriptions (excluding formatted variables)
    const parenMatches = (bodyText.match(/\([a-zA-ZÀ-ỹ0-9\s]{3,}\)/g) || []).filter(
      (m) => !m.includes('@') // ignore handle like (@username) if any
    );
    return { bodyText, hasEllipsis, parenMatches };
  }

  // 1. Account Tab
  await page.locator('.settings-nav-item', { hasText: 'Tài khoản' }).click();
  const acc = await inspectBody('Tài khoản');
  record(
    'Account Tab: No technical jargon (PKCE, OAuth) and clean labels',
    !acc.bodyText.includes('PKCE') && !acc.bodyText.includes('OAuth') && acc.bodyText.includes('Không lưu email') && acc.bodyText.includes('Không cần số điện thoại'),
    `hasPKCE=${acc.bodyText.includes('PKCE')}, hasOAuth=${acc.bodyText.includes('OAuth')}`
  );
  record('Account Tab: Zero ellipses (...)', !acc.hasEllipsis, `hasEllipsis=${acc.hasEllipsis}`);
  await page.screenshot({ path: path.join(OUT, '01_account_tab.png') });

  // 2. Security Tab
  await page.locator('.settings-nav-item', { hasText: 'Bảo mật' }).click();
  const sec = await inspectBody('Bảo mật');
  const hasCryptoJargon =
    sec.bodyText.includes('ECDH P-256') ||
    sec.bodyText.includes('HKDF-SHA-256') ||
    sec.bodyText.includes('AES-256-GCM') ||
    sec.bodyText.includes('ECDSA P-256');
  const hasFriendlySecurity =
    sec.bodyText.includes('Tiêu chuẩn bảo vệ') ||
    sec.bodyText.includes('Mã hóa hai đầu chuẩn cao nhất') ||
    sec.bodyText.includes('Vân tay bảo mật');
  record(
    'Security Tab: Zero raw crypto strings and friendly Vietnamese wording',
    !hasCryptoJargon && hasFriendlySecurity,
    `hasCryptoJargon=${hasCryptoJargon}, hasFriendlySecurity=${hasFriendlySecurity}`
  );
  record('Security Tab: Zero ellipses (...)', !sec.hasEllipsis, `hasEllipsis=${sec.hasEllipsis}`);
  record('Security Tab: Zero unwanted parentheses', sec.parenMatches.length === 0, `parens=${sec.parenMatches.join(', ')}`);
  await page.screenshot({ path: path.join(OUT, '02_security_tab.png') });

  // 3. Lock Tab ("Khóa ứng dụng")
  await page.locator('.settings-nav-item', { hasText: 'Khóa ứng dụng' }).click();
  const lock = await inspectBody('Khóa ứng dụng');
  const hasLockJargon = lock.bodyText.includes('PBKDF2') || lock.bodyText.includes('salt');
  const hasFriendlyLock =
    lock.bodyText.includes('Khóa ứng dụng') &&
    (lock.bodyText.includes('Mật khẩu mới từ 4 đến 8 số') || lock.bodyText.includes('Đặt mật khẩu khóa'));
  record(
    'Lock Tab: Zero PBKDF2/salt technical jargon, plain everyday Vietnamese',
    !hasLockJargon && hasFriendlyLock,
    `hasLockJargon=${hasLockJargon}, hasFriendlyLock=${hasFriendlyLock}`
  );
  record('Lock Tab: Zero ellipses (...)', !lock.hasEllipsis, `hasEllipsis=${lock.hasEllipsis}`);
  await page.screenshot({ path: path.join(OUT, '03_lock_tab.png') });

  // 4. Chat Tab ("Trò chuyện")
  await page.locator('.settings-nav-item', { hasText: 'Trò chuyện' }).click();
  const chat = await inspectBody('Trò chuyện');
  const hasPunchyChatToggles =
    chat.bodyText.includes('Bấm hai lần để trả lời') &&
    chat.bodyText.includes('Nhấn Enter để gửi') &&
    chat.bodyText.includes('Gợi ý hình dán') &&
    chat.bodyText.includes('Gợi ý nhắc tên') &&
    chat.bodyText.includes('Gợi ý mặt cười') &&
    chat.bodyText.includes('Báo đang gõ chữ') &&
    chat.bodyText.includes('Báo đã xem');
  record(
    'Chat Tab: Short, punchy toggle labels with everyday wording',
    hasPunchyChatToggles,
    `hasPunchyChatToggles=${hasPunchyChatToggles}`
  );
  record('Chat Tab: Zero ellipses (...)', !chat.hasEllipsis, `hasEllipsis=${chat.hasEllipsis}`);
  await page.screenshot({ path: path.join(OUT, '04_chat_tab.png') });

  // 5. Notifications Tab ("Thông báo")
  await page.locator('.settings-nav-item', { hasText: 'Thông báo' }).click();
  const notif = await inspectBody('Thông báo');
  const hasCleanNotif =
    notif.bodyText.includes('Bật thông báo tin nhắn mới') &&
    notif.bodyText.includes('Hiện nội dung trong thông báo') &&
    notif.bodyText.includes('Phát âm thanh khi có thông báo');
  record('Notifications Tab: Clean, concise controls', hasCleanNotif, `hasCleanNotif=${hasCleanNotif}`);
  record('Notifications Tab: Zero ellipses (...)', !notif.hasEllipsis, `hasEllipsis=${notif.hasEllipsis}`);
  await page.screenshot({ path: path.join(OUT, '05_notifications_tab.png') });

  // 6. Privacy Tab ("Quyền riêng tư")
  await page.locator('.settings-nav-item', { hasText: 'Quyền riêng tư' }).click();
  const priv = await inspectBody('Quyền riêng tư');
  const hasCleanPriv =
    priv.bodyText.includes('Ai được gửi lời mời kết bạn') &&
    priv.bodyText.includes('Ai được gọi cho bạn') &&
    priv.bodyText.includes('Danh sách chặn');
  record('Privacy Tab: Clear everyday questions without technical jargon', hasCleanPriv, `hasCleanPriv=${hasCleanPriv}`);
  record('Privacy Tab: Zero ellipses (...)', !priv.hasEllipsis, `hasEllipsis=${priv.hasEllipsis}`);
  await page.screenshot({ path: path.join(OUT, '06_privacy_tab.png') });

  // 7. Appearance Tab ("Giao diện")
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  // Scroll down to advanced controls
  await page.evaluate(() => {
    const el = document.querySelector('.settings-body');
    if (el) el.scrollTop = el.scrollHeight;
  });
  await page.waitForTimeout(200);
  const appTab = await inspectBody('Giao diện');
  const hasCleanAppearance =
    appTab.bodyText.includes('Độ trong suốt khung trắng') &&
    appTab.bodyText.includes('Làm mờ kính khung') &&
    !appTab.bodyText.toLowerCase().includes('frosted glass') &&
    !appTab.bodyText.includes('(0px đến 32px)');
  record(
    'Appearance Tab: Clean sliders without (Frosted glass) or (0px đến 32px)',
    hasCleanAppearance,
    `hasCleanAppearance=${hasCleanAppearance}`
  );
  record('Appearance Tab: Zero ellipses (...)', !appTab.hasEllipsis, `hasEllipsis=${appTab.hasEllipsis}`);
  await page.screenshot({ path: path.join(OUT, '07_appearance_tab.png') });

  // 8. Customize Tab ("Tùy chỉnh")
  await page.locator('.settings-nav-item', { hasText: 'Tùy chỉnh' }).click();
  const cust = await inspectBody('Tùy chỉnh');
  const hasCleanCust =
    cust.bodyText.includes('Thanh lối tắt') &&
    cust.bodyText.includes('Ghim cộng đồng') &&
    cust.bodyText.includes('Ghim nhóm') &&
    cust.bodyText.includes('Âm báo tin nhắn') &&
    cust.bodyText.includes('Hiệu ứng hồ sơ');
  record(
    'Customize Tab: Simplified navigation bar customization and sound effect controls',
    hasCleanCust,
    `hasCleanCust=${hasCleanCust}`
  );
  record('Customize Tab: Zero ellipses (...)', !cust.hasEllipsis, `hasEllipsis=${cust.hasEllipsis}`);
  await page.screenshot({ path: path.join(OUT, '08_customize_tab.png') });

} catch (e) {
  record('Fatal run error', false, e.message);
  console.error(e);
} finally {
  await browser.close();
}

console.log('=== SUMMARY ===');
const failed = checks.filter((c) => !c.pass);
console.log(`Passed: ${checks.length - failed.length}/${checks.length}`);
if (failed.length > 0) {
  console.error('Failed checks:', failed);
  process.exit(1);
} else {
  console.log('All Spec 044 checks PASSED.');
  process.exit(0);
}
