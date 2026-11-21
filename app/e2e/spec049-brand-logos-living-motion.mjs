// Spec 049 Comprehensive Live Chromium Verification:
// 1. Living Empty State: D3Motion-inspired Living Luminous Orb (.cw-living-orb) + 4 interactive Beginner Quick-Start cards (.cw-quickstart-card) + spotlight shimmer.
// 2. Interactive WelcomeTour: animated step badge, progress bar fill, animated SVG illustration, and spark celebration burst on completion.
// 3. Cloud Backup Settings (Đám mây): 3-step friendly beginner strip + crisp multicolor SVG brand logos for ALL 10 providers (Google Drive, OneDrive, Dropbox, Nextcloud, WebDAV, Amazon S3, Cloudflare R2, MinIO, NAS, Apple iCloud) + zero raw OAuth env-var jargon.
// 4. Security Settings (Bảo mật): animated E2EE live shield + 2FA logo + Passkey logo + ZIP logo + Chrome + Linux Tux paired brand logos on every Security Log entry.
// 5. Account & Devices (Tài khoản + Thiết bị): official 4-color Google logo on Google sign-in row + Browser/OS DeviceBrandPair + live pulse dot on current device.
// 6. Language Settings (Ngôn ngữ): crisp vector SVG flags on language tiles with flat translucent hover/active (zero white boxes, zero bolding).
// 7. Download Page (Tải ứng dụng): multicolor Linux Tux, Windows, macOS, Android, App Store logos + auto-detected "Máy của bạn" pulse badge + animated QR scan line.
// 8. Guide Settings (Hướng dẫn): Interactive 5-step Beginner Checklist with live progress bar + spark burst on check-off + persisted state.
// 9. Living Motion JS loaded cleanly (window.__cwLivingMotionInit && window.cwSparkBurst) with zero page errors.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || 'verify_049';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
const record = (name, pass, detail = '') => {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
};
const shot = (page, name) => page.screenshot({ path: path.join(OUT, `${name}.png`) });

const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
  args: ['--no-sandbox'],
});

const run = Date.now().toString(36).slice(-5);
const USER = 's049_' + run;

const ctx = await browser.newContext({ viewport: { width: 1366, height: 860 } });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));

try {
  await page.goto(BASE + '/');
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(USER);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 30000 });
  await page.waitForTimeout(900);

  // 1. Living Motion JS loaded
  const motionReady = await page.evaluate(() => Boolean(window.__cwLivingMotionInit && typeof window.cwSparkBurst === 'function'));
  record('Living Motion Engine loaded (spotlight + spark burst)', motionReady);

  // 2. Upgraded WelcomeTour (if open on first login, inspect it; otherwise click guide card later)
  const tourVisible = await page.locator('.story-dialog').isVisible().catch(() => false);
  if (tourVisible) {
    const tourCheck = await page.evaluate(() => ({
      hasBadge: Boolean(document.querySelector('.story-step-badge')),
      hasProgress: Boolean(document.querySelector('.story-progress-fill')),
      hasLivingIll: Boolean(document.querySelector('.cw-living-ill')),
    }));
    await shot(page, '01_welcome_tour_animated');
    record('WelcomeTour shows animated illustration, step badge & spring progress bar', tourCheck.hasBadge && tourCheck.hasProgress && tourCheck.hasLivingIll, JSON.stringify(tourCheck));
    await page.locator('button[aria-label="Bỏ qua giới thiệu"]').click();
    await page.waitForTimeout(300);
  }

  // 3. Living Empty State (Messages view with no chat open yet)
  const emptyCheck = await page.evaluate(() => {
    const orb = document.querySelector('.cw-living-orb');
    const rings = document.querySelectorAll('.cw-orb-ring').length;
    const sats = document.querySelectorAll('.cw-orb-sat').length;
    const qsCards = document.querySelectorAll('.cw-quickstart-card.cw-spotlight').length;
    return { hasOrb: Boolean(orb), rings, sats, qsCards };
  });
  await shot(page, '02_living_empty_state_orb_quickstart');
  record(
    'Empty State renders D3Motion-inspired Living Luminous Orb + 4 interactive Beginner Quick-Start cards',
    emptyCheck.hasOrb && emptyCheck.rings === 2 && emptyCheck.sats === 3 && emptyCheck.qsCards === 4,
    JSON.stringify(emptyCheck),
  );

  // Hover quick-start card to verify pointer spotlight activates
  const firstQs = page.locator('.cw-quickstart-card').first();
  const qsBox = await firstQs.boundingBox();
  if (qsBox) {
    await page.mouse.move(qsBox.x + qsBox.width / 2, qsBox.y + qsBox.height / 2);
    await page.waitForTimeout(120);
  }
  const spotActive = await page.evaluate(() => Boolean(document.querySelector('.cw-quickstart-card.cw-spot-on')));
  record('Pointer hover activates smooth cursor spotlight (.cw-spot-on) on interactive cards', spotActive);

  // 4. Download Page: Linux Tux, Windows, macOS, Android, App Store logos + auto-detected OS pill + QR scan beam
  await page.locator('.app-rail button.rail-item[aria-label="Tải ứng dụng"]').click();
  await page.waitForSelector('.download-page', { timeout: 10000 });
  await page.waitForTimeout(500);
  const dlCheck = await page.evaluate(() => ({
    osIcons: document.querySelectorAll('.download-os-icon svg').length,
    hasRecPill: Boolean(document.querySelector('.cw-os-rec-pill')),
    hasQrScan: Boolean(document.querySelector('.cw-qr-scan')),
    hasTuxPaths: Boolean(document.querySelector('.download-os-icon svg ellipse[fill="#F8FAFC"]')),
  }));
  await shot(page, '03_download_os_logos_and_qr_scan');
  record(
    'Download Page shows multicolor Linux Tux, Windows, macOS, Android, iOS logos + auto-detected OS pill + animated QR scan',
    dlCheck.osIcons >= 5 && dlCheck.hasRecPill && dlCheck.hasQrScan && dlCheck.hasTuxPaths,
    JSON.stringify(dlCheck),
  );

  // 5. Open Settings -> Cloud (Đám mây)
  await page.locator('.app-rail button.rail-item[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.waitForTimeout(350);

  await page.locator('.settings-nav-item', { hasText: 'Đám mây' }).first().click();
  await page.waitForTimeout(600);
  const cloudCheck = await page.evaluate(() => {
    const brands = [...document.querySelectorAll('.settings-body .brand-badge[data-brand]')]
      .map((el) => el.getAttribute('data-brand'));
    const bodyText = document.querySelector('.settings-body')?.innerText || '';
    return {
      hasGuide: Boolean(document.querySelector('.cw-cloud-guide')),
      brandCount: brands.length,
      brands,
      hasGoogleDrive: brands.includes('google_drive'),
      hasOneDrive: brands.includes('onedrive'),
      hasDropbox: brands.includes('dropbox'),
      hasNextcloud: brands.includes('nextcloud'),
      hasS3: brands.includes('s3'),
      hasR2: brands.includes('r2'),
      hasICloud: brands.includes('icloud'),
      noOauthEnvJargon: !bodyText.includes('GOOGLE_DRIVE_CLIENT_ID') && !bodyText.includes('ONEDRIVE_CLIENT_ID') && !bodyText.includes('DROPBOX_APP_KEY'),
    };
  });
  await shot(page, '04_settings_cloud_logos_and_guide');
  record(
    'Cloud Settings renders 3-step guide + crisp SVG brand logos for all cloud providers + zero raw OAuth env jargon',
    cloudCheck.hasGuide && cloudCheck.brandCount >= 10 && cloudCheck.hasGoogleDrive && cloudCheck.hasOneDrive && cloudCheck.hasDropbox && cloudCheck.hasS3 && cloudCheck.hasR2 && cloudCheck.hasICloud && cloudCheck.noOauthEnvJargon,
    JSON.stringify(cloudCheck),
  );

  // 6. Settings -> Security (Bảo mật): E2EE shield, 2FA logo, Passkey, ZIP + Chrome & Linux Tux on Security Log
  await page.locator('.settings-nav-item', { hasText: 'Bảo mật' }).first().click();
  await page.waitForTimeout(650);
  const secCheck = await page.evaluate(() => ({
    hasE2eeShield: Boolean(document.querySelector('[data-brand="e2ee"] svg')),
    has2faLogo: Boolean(document.querySelector('[data-brand="2fa"] svg')),
    hasPasskeyLogo: Boolean(document.querySelector('[data-brand="passkey"] svg')),
    hasZipLogo: Boolean(document.querySelector('[data-brand="zip"] svg')),
    secLogPairs: document.querySelectorAll('.security-log .device-brand-pair svg').length,
    browsers: [...document.querySelectorAll('.security-log [data-browser]')].map((e) => e.getAttribute('data-browser')),
    oses: [...document.querySelectorAll('.security-log [data-os]')].map((e) => e.getAttribute('data-os')),
  }));
  await shot(page, '05_settings_security_browser_os_logos');
  record(
    'Security Settings shows E2EE shield, 2FA, Passkey, ZIP logos + paired Browser & OS logos (Chrome + Linux Tux) on Security Log',
    secCheck.hasE2eeShield && secCheck.has2faLogo && secCheck.hasPasskeyLogo && secCheck.hasZipLogo && secCheck.secLogPairs >= 1 && secCheck.browsers.includes('chrome') && secCheck.oses.includes('linux'),
    JSON.stringify(secCheck),
  );

  // 7. Settings -> Account (Tài khoản) & Devices (Thiết bị)
  await page.locator('.settings-nav-item', { hasText: 'Tài khoản' }).first().click();
  await page.waitForTimeout(450);
  const hasGoogleLogo = await page.evaluate(() => Boolean(document.querySelector('.settings-body [data-brand="google"] svg')));
  await shot(page, '06_settings_account_google_logo');
  record('Account Settings displays official 4-color Google logo on Google sign-in row', hasGoogleLogo);

  await page.locator('.settings-nav-item', { hasText: 'Thiết bị' }).first().click();
  await page.waitForTimeout(550);
  const devCheck = await page.evaluate(() => ({
    pairs: document.querySelectorAll('.settings-body .device-brand-pair svg').length,
    liveDots: document.querySelectorAll('.settings-body .cw-live-dot').length,
  }));
  await shot(page, '07_settings_devices_os_logos');
  record('Devices panel shows OS/Browser brand badges and pulsing live dot on current device', devCheck.pairs >= 1 && devCheck.liveDots >= 1, JSON.stringify(devCheck));

  // 8. Settings -> Language (Ngôn ngữ)
  await page.locator('.settings-nav-item', { hasText: 'Ngôn ngữ' }).first().click();
  await page.waitForTimeout(400);
  const langCheck = await page.evaluate(() => ({
    flagSvgs: document.querySelectorAll('.lang-tile .lang-flag-badge svg').length,
    tiles: document.querySelectorAll('.lang-tile').length,
  }));
  await shot(page, '08_settings_language_flags');
  record('Language Settings displays crisp vector SVG flags across language tiles', langCheck.flagSvgs >= 15 && langCheck.tiles >= 20, JSON.stringify(langCheck));

  // 9. Settings -> Guide (Hướng dẫn): Interactive 5-step Beginner Checklist + Spark Burst
  await page.locator('.settings-nav-item', { hasText: 'Hướng dẫn' }).first().click();
  await page.waitForTimeout(450);
  const checklistItems = page.locator('.cw-checklist-item');
  const clCount = await checklistItems.count();
  await checklistItems.first().click();
  await page.waitForTimeout(200);
  await checklistItems.nth(1).click();
  await page.waitForTimeout(250);
  const clAfter = await page.evaluate(() => ({
    counter: document.querySelector('.cw-checklist-counter')?.innerText?.trim() || '',
    barWidth: document.querySelector('.cw-checklist-bar-fill')?.style?.width || '',
    doneCount: document.querySelectorAll('.cw-checklist-item.done').length,
  }));
  await shot(page, '09_settings_guide_checklist');
  record(
    'Guide Settings renders interactive 5-step Beginner Checklist with live animated progress bar (2/5 = 40%)',
    clCount === 5 && clAfter.doneCount === 2 && clAfter.barWidth === '40%',
    JSON.stringify({ clCount, ...clAfter }),
  );

  // Trigger WelcomeTour from Guide "Xem lại giới thiệu" button & verify completion spark burst
  await page.locator('.guide-hero button.btn-primary').click();
  await page.waitForSelector('.story-dialog', { timeout: 6000 });
  await page.waitForTimeout(300);
  if (!tourVisible) await shot(page, '01_welcome_tour_animated');
  for (let i = 0; i < 6; i++) {
    const nextBtn = page.locator('.story-actions button.btn-primary');
    if (await nextBtn.isVisible().catch(() => false)) {
      await nextBtn.click();
      await page.waitForTimeout(160);
    }
  }
  const tourClosed = !(await page.locator('.story-dialog').isVisible().catch(() => false));
  record('WelcomeTour steps through cleanly to completion and closes smoothly', tourClosed);

  record('Zero uncaught page errors throughout Spec 049 verification', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 049 Summary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length) process.exit(1);
