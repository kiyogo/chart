import fs from 'fs';
import path from 'path';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE_URL = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_032';
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
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));

const FORBIDDEN = ['🤖 Bot', '📄 Trang', '🎨 Giao diện', '⏱ HUD', 'Trang & HUD', 'Bot & Lệnh', 'Hiển thị HUD nổi', 'Chế độ Bot'];

try {
  // Login fresh dev user
  const uname = `p032_${Math.random().toString(36).slice(2, 8)}`;
  await page.goto(BASE_URL, { waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 20000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) {
    await skip.click();
    await page.waitForTimeout(250);
  }

  // 0. Sidebar has NO fake "Trợ lý lệnh nhanh" bot item & NO floating HUD
  const sidebarCheck = await page.evaluate(() => {
    const txt = document.querySelector('.nav-sidebar')?.innerText || '';
    const hasHud = !!document.querySelector('.plugin-floating-hud');
    return { hasFakeBot: txt.includes('Trợ lý lệnh nhanh'), hasHud };
  });
  record('Messages sidebar has zero fake Bot items and zero floating HUD', !sidebarCheck.hasFakeBot && !sidebarCheck.hasHud, JSON.stringify(sidebarCheck));

  // 1. Open Plugin Manager
  await page.locator('.app-rail button[aria-label="Plugin"]').click();
  await page.waitForSelector('.mod-cards-grid .mod-card', { timeout: 10000 });
  await page.waitForTimeout(300);

  const storeAudit = await page.evaluate((forbidden) => {
    const stage = document.querySelector('.plugin-custom-stage');
    const txt = stage?.innerText || '';
    const foundForbidden = forbidden.filter((f) => txt.includes(f));
    const capPills = document.querySelectorAll('.mod-capability-pills').length;
    const descEls = document.querySelectorAll('.mod-card-desc').length;
    const hudEls = document.querySelectorAll('.plugin-floating-hud').length;
    const cardTitles = Array.from(document.querySelectorAll('.mod-card-title')).map((e) => e.textContent?.trim());
    return { foundForbidden, capPills, descEls, hudEls, cardTitles };
  }, FORBIDDEN);
  record(
    'Plugin store has ZERO Bot/Trang/Giao diện/HUD badges, ZERO card description bloat, ZERO HUD',
    storeAudit.foundForbidden.length === 0 && storeAudit.capPills === 0 && storeAudit.descEls === 0 && storeAudit.hudEls === 0,
    JSON.stringify(storeAudit),
  );
  await page.screenshot({ path: path.join(OUT, '00_plugin_store.png') });

  // 2. Open "Tùy chỉnh" for "Sổ tay & Việc làm"
  await page.locator('.mod-card', { hasText: 'Sổ tay & Việc làm' }).locator('button:has-text("Tùy chỉnh")').click();
  await page.waitForSelector('[data-plugin-editor-kind="notes"]', { timeout: 5000 });
  await page.waitForTimeout(250);
  const notesAudit = await page.evaluate((forbidden) => {
    const root = document.querySelector('[data-plugin-editor-kind="notes"]');
    const txt = root?.innerText || '';
    return {
      foundForbidden: forbidden.filter((f) => txt.includes(f)),
      hasDescLabel: txt.includes('Mô tả'),
      hasPomodoro: txt.includes('Cấu hình Pomodoro'),
      hasReorder: root?.querySelectorAll('.plugin-order-row').length || 0,
      hasLogoBtn: txt.includes('Đổi logo'),
    };
  }, FORBIDDEN);
  record(
    'Sổ tay & Việc làm shows its own Pomodoro + Reorderable Layout UI (no Mô tả / Bot / HUD)',
    notesAudit.foundForbidden.length === 0 && !notesAudit.hasDescLabel && notesAudit.hasPomodoro && notesAudit.hasReorder === 3 && notesAudit.hasLogoBtn,
    JSON.stringify(notesAudit),
  );
  await page.screenshot({ path: path.join(OUT, '01_plugin_notes_settings.png') });

  // Reorder Pomodoro DOWN below Todos, then open live Sổ tay workspace and verify DOM order!
  await page.locator('button[aria-label="Đưa Đồng hồ Pomodoro xuống"]').click();
  await page.waitForTimeout(200);
  await page.locator('[data-plugin-editor-kind="notes"] button:has-text("Mở")').click();
  await page.waitForSelector('[data-plugin-section="todos"]', { timeout: 5000 });
  await page.waitForTimeout(200);
  const liveOrder = await page.evaluate(() =>
    Array.from(document.querySelectorAll('[data-plugin-section]')).map((el) => el.getAttribute('data-plugin-section')),
  );
  record(
    'Reordering Sổ tay sections immediately updates live Sổ tay workspace order',
    liveOrder[0] === 'todos' && liveOrder[1] === 'pomodoro',
    JSON.stringify(liveOrder),
  );
  await page.screenshot({ path: path.join(OUT, '02_plugin_notes_live_reordered.png') });

  // 3. Return to Plugin Manager, install & configure "Soundboard 8-Bit"
  await page.locator('.app-rail button[aria-label="Plugin"]').click();
  await page.waitForSelector('.mod-cards-grid', { timeout: 5000 });
  await page.locator('.mod-card', { hasText: 'Soundboard 8-Bit' }).locator('button:has-text("Cài đặt")').click();
  await page.waitForTimeout(250);
  await page.locator('.mod-card', { hasText: 'Soundboard 8-Bit' }).locator('button:has-text("Tùy chỉnh")').click();
  await page.waitForSelector('[data-plugin-editor-kind="soundboard"]', { timeout: 5000 });
  await page.waitForTimeout(250);
  const soundAudit = await page.evaluate((forbidden) => {
    const root = document.querySelector('[data-plugin-editor-kind="soundboard"]');
    const txt = root?.innerText || '';
    return {
      foundForbidden: forbidden.filter((f) => txt.includes(f)),
      hasDescLabel: txt.includes('Mô tả'),
      hasMixer: txt.includes('Bộ trộn âm thanh'),
      ranges: root?.querySelectorAll('input[type="range"]').length || 0,
      padRows: root?.querySelectorAll('.plugin-order-row').length || 0,
    };
  }, FORBIDDEN);
  record(
    'Soundboard 8-Bit shows its own Audio Mixer + 8 Reorderable Pads UI (distinct from Sổ tay)',
    soundAudit.foundForbidden.length === 0 && !soundAudit.hasDescLabel && soundAudit.hasMixer && soundAudit.ranges === 2 && soundAudit.padRows === 8,
    JSON.stringify(soundAudit),
  );
  await page.screenshot({ path: path.join(OUT, '03_plugin_soundboard_settings.png') });

  // 4. Back to Store -> install & configure "Bảng vẽ Voxel" (logo owned by plugin: allow_custom_logo=false)
  await page.locator('button:has-text("Quay lại")').first().click();
  await page.waitForSelector('.mod-cards-grid', { timeout: 5000 });
  await page.locator('.mod-card', { hasText: 'Bảng vẽ Voxel' }).locator('button:has-text("Cài đặt")').click();
  await page.waitForTimeout(250);
  await page.locator('.mod-card', { hasText: 'Bảng vẽ Voxel' }).locator('button:has-text("Tùy chỉnh")').click();
  await page.waitForSelector('[data-plugin-editor-kind="voxel"]', { timeout: 5000 });
  await page.waitForTimeout(250);
  const voxelAudit = await page.evaluate((forbidden) => {
    const root = document.querySelector('[data-plugin-editor-kind="voxel"]');
    const txt = root?.innerText || '';
    return {
      foundForbidden: forbidden.filter((f) => txt.includes(f)),
      hasSwatches: root?.querySelectorAll('.plugin-swatch-btn').length || 0,
      hasLogoBtn: txt.includes('Đổi logo'),
      hasHud: !!document.querySelector('.plugin-floating-hud'),
    };
  }, FORBIDDEN);
  record(
    'Bảng vẽ Voxel shows its own Brush Swatch & Grid UI, owns its logo (no Đổi logo), zero HUD',
    voxelAudit.foundForbidden.length === 0 && voxelAudit.hasSwatches === 12 && !voxelAudit.hasLogoBtn && !voxelAudit.hasHud,
    JSON.stringify(voxelAudit),
  );
  await page.screenshot({ path: path.join(OUT, '04_plugin_voxel_settings.png') });

  // 5. Back to Store -> configure "Công cụ Lệnh nhanh" & live-test /calc 12*8+4 => 100
  await page.locator('button:has-text("Quay lại")').first().click();
  await page.waitForSelector('.mod-cards-grid', { timeout: 5000 });
  await page.locator('.mod-card', { hasText: 'Công cụ Lệnh nhanh' }).locator('button:has-text("Tùy chỉnh")').click();
  await page.waitForSelector('[data-plugin-editor-kind="tools"]', { timeout: 5000 });
  await page.locator('[data-plugin-editor-kind="tools"] button:has-text("Chạy")').click();
  await page.waitForTimeout(200);
  const toolsOut = await page.locator('.plugin-calc-result').first().innerText().catch(() => '');
  record('Công cụ Lệnh nhanh runs real /calc 12*8+4 => 100 live', toolsOut.includes('100'), toolsOut);
  await page.screenshot({ path: path.join(OUT, '05_plugin_tools_settings.png') });

  // 6. Back to Store -> install & configure "Sakura Kính mờ" -> live CSS accent applies
  await page.locator('button:has-text("Quay lại")').first().click();
  await page.waitForSelector('.mod-cards-grid', { timeout: 5000 });
  await page.locator('.mod-card', { hasText: 'Sakura Kính mờ' }).locator('button:has-text("Cài đặt")').click();
  await page.waitForTimeout(250);
  await page.locator('.mod-card', { hasText: 'Sakura Kính mờ' }).locator('button:has-text("Tùy chỉnh")').click();
  await page.waitForSelector('[data-plugin-editor-kind="theme_sakura"]', { timeout: 5000 });
  await page.waitForTimeout(250);
  const sakuraAudit = await page.evaluate(() => {
    const css = document.getElementById('chatw-plugin-mods-css')?.textContent || '';
    return {
      hasAccent: css.includes('#EC4899'),
      hasHud: !!document.querySelector('.plugin-floating-hud'),
    };
  });
  record('Sakura Kính mờ applies live CSS theme without any HUD', sakuraAudit.hasAccent && !sakuraAudit.hasHud, JSON.stringify(sakuraAudit));
  await page.screenshot({ path: path.join(OUT, '06_plugin_sakura_settings.png') });

  record('Zero uncaught page errors', errors.length === 0, errors.join('; '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
