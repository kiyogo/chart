import fs from 'fs';
import path from 'path';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE_URL = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_033';
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

async function auditNoNumbersOrArrows(scopeSelector) {
  return page.evaluate((sel) => {
    const root = document.querySelector(sel) || document.body;
    const indexCount = root.querySelectorAll('.plugin-order-index').length;
    const stepBtnCount = root.querySelectorAll('.plugin-step-btn').length;
    const arrowBtns = Array.from(root.querySelectorAll('button')).filter((b) => {
      const t = (b.textContent || '').trim();
      const title = (b.getAttribute('title') || '') + ' ' + (b.getAttribute('aria-label') || '');
      return t === '↑' || t === '↓' || title.includes('Đưa lên') || title.includes('Đưa xuống');
    }).length;
    return { indexCount, stepBtnCount, arrowBtns };
  }, scopeSelector);
}

try {
  // Login fresh dev user
  const uname = `p033_${Math.random().toString(36).slice(2, 8)}`;
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

  // 0. Sidebar has NO fake Bot item & NO floating HUD
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
    return { foundForbidden, capPills, descEls, hudEls };
  }, FORBIDDEN);
  record(
    'Plugin store has ZERO Bot/Trang/Giao diện/HUD badges, ZERO card description bloat, ZERO HUD',
    storeAudit.foundForbidden.length === 0 && storeAudit.capPills === 0 && storeAudit.descEls === 0 && storeAudit.hudEls === 0,
    JSON.stringify(storeAudit),
  );
  await page.screenshot({ path: path.join(OUT, '00_plugin_store.png') });

  // 2. Sổ tay & Việc làm — Zero numbers, zero arrows, mouse Drag-and-Drop updates live workspace!
  await page.locator('.mod-card', { hasText: 'Sổ tay & Việc làm' }).locator('button:has-text("Tùy chỉnh")').click();
  await page.waitForSelector('[data-plugin-editor-kind="notes"]', { timeout: 5000 });
  await page.waitForTimeout(250);
  const notesNoArrows = await auditNoNumbersOrArrows('[data-plugin-editor-kind="notes"]');
  const notesDragCount = await page.locator('[data-plugin-editor-kind="notes"] [data-drag-item="true"]').count();
  record(
    'Sổ tay settings has ZERO numbers, ZERO arrows, and 3 draggable rows',
    notesNoArrows.indexCount === 0 && notesNoArrows.stepBtnCount === 0 && notesNoArrows.arrowBtns === 0 && notesDragCount === 3,
    JSON.stringify({ ...notesNoArrows, notesDragCount }),
  );
  await page.screenshot({ path: path.join(OUT, '01_plugin_notes_dnd.png') });

  // Drag Pomodoro onto Ghi chú nhanh via mouse dragTo
  await page
    .locator('[data-plugin-editor-kind="notes"] [data-drag-key="pomodoro"]')
    .dragTo(page.locator('[data-plugin-editor-kind="notes"] [data-drag-key="notes"]'));
  await page.waitForTimeout(250);
  await page.locator('[data-plugin-editor-kind="notes"] button:has-text("Mở")').click();
  await page.waitForSelector('[data-plugin-section="todos"]', { timeout: 5000 });
  await page.waitForTimeout(200);
  const liveOrder = await page.evaluate(() =>
    Array.from(document.querySelectorAll('[data-plugin-section]')).map((el) => el.getAttribute('data-plugin-section')),
  );
  record(
    'Mouse drag-and-drop in Sổ tay updates live workspace section order',
    liveOrder[0] === 'todos' && liveOrder[liveOrder.length - 1] === 'pomodoro',
    JSON.stringify(liveOrder),
  );

  // 3. Soundboard 8-Bit — Visual Draggable Launchpad Pad Grid (NO vertical order rows, NO arrows/numbers!)
  await page.locator('.app-rail button[aria-label="Plugin"]').click();
  await page.waitForSelector('.mod-cards-grid', { timeout: 5000 });
  await page.locator('.mod-card', { hasText: 'Soundboard 8-Bit' }).locator('button:has-text("Cài đặt")').click();
  await page.waitForTimeout(250);
  await page.locator('.mod-card', { hasText: 'Soundboard 8-Bit' }).locator('button:has-text("Tùy chỉnh")').click();
  await page.waitForSelector('[data-plugin-editor-kind="soundboard"]', { timeout: 5000 });
  await page.waitForTimeout(250);
  const sbNoArrows = await auditNoNumbersOrArrows('[data-plugin-editor-kind="soundboard"]');
  const sbBefore = await page.evaluate(() =>
    Array.from(document.querySelectorAll('.soundboard-cfg-grid .soundboard-cfg-pad')).map((e) => e.getAttribute('data-drag-key')),
  );
  const sbListRows = await page.locator('[data-plugin-editor-kind="soundboard"] .plugin-order-row').count();
  await page
    .locator('.soundboard-cfg-pad')
    .nth(0)
    .dragTo(page.locator('.soundboard-cfg-pad').nth(2));
  await page.waitForTimeout(250);
  const sbAfter = await page.evaluate(() =>
    Array.from(document.querySelectorAll('.soundboard-cfg-grid .soundboard-cfg-pad')).map((e) => e.getAttribute('data-drag-key')),
  );
  record(
    'Soundboard uses distinct 8-pad visual draggable grid (0 vertical list rows, 0 arrows/numbers) & mouse drag reorders pads',
    sbNoArrows.indexCount === 0 &&
      sbNoArrows.arrowBtns === 0 &&
      sbListRows === 0 &&
      sbBefore.length === 8 &&
      sbAfter[2] === sbBefore[0] &&
      sbAfter[0] === sbBefore[1],
    JSON.stringify({ sbNoArrows, sbListRows, firstBefore: sbBefore.slice(0, 3), firstAfter: sbAfter.slice(0, 3) }),
  );
  await page.screenshot({ path: path.join(OUT, '02_plugin_soundboard_grid.png') });

  // 4. Bảng vẽ Voxel — NO cloned "Sắp xếp khu vực bảng vẽ" list; renders live 192-cell interactive mini canvas!
  await page.locator('button:has-text("Quay lại")').first().click();
  await page.waitForSelector('.mod-cards-grid', { timeout: 5000 });
  await page.locator('.mod-card', { hasText: 'Bảng vẽ Voxel' }).locator('button:has-text("Cài đặt")').click();
  await page.waitForTimeout(250);
  await page.locator('.mod-card', { hasText: 'Bảng vẽ Voxel' }).locator('button:has-text("Tùy chỉnh")').click();
  await page.waitForSelector('[data-plugin-editor-kind="voxel"]', { timeout: 5000 });
  await page.waitForTimeout(250);
  await page.locator('[data-plugin-editor-kind="voxel"] button:has-text("Xóa trắng")').click();
  await page.waitForTimeout(150);
  const clearedFilled = await page.evaluate(() =>
    document.querySelectorAll('[data-plugin-editor-kind="voxel"] .voxel-mini-cell.filled').length,
  );
  await page.locator('[data-plugin-editor-kind="voxel"] button:has-text("Lâu đài")').click();
  await page.waitForTimeout(200);
  const voxelAudit = await page.evaluate(() => {
    const root = document.querySelector('[data-plugin-editor-kind="voxel"]');
    const txt = root?.innerText || '';
    const cells = root?.querySelectorAll('.voxel-mini-cell').length || 0;
    const filled = root?.querySelectorAll('.voxel-mini-cell.filled').length || 0;
    const orderRows = root?.querySelectorAll('.plugin-order-row').length || 0;
    return {
      hasOrderHeading: txt.includes('Sắp xếp khu vực'),
      cells,
      filled,
      orderRows,
      hasLogoBtn: txt.includes('Đổi logo'),
    };
  });
  record(
    'Bảng vẽ Voxel removed cloned reorder list and shows live 192-cell mini canvas with working templates & clear',
    !voxelAudit.hasOrderHeading && voxelAudit.orderRows === 0 && voxelAudit.cells === 192 && clearedFilled === 0 && voxelAudit.filled === 72 && !voxelAudit.hasLogoBtn,
    JSON.stringify({ ...voxelAudit, clearedFilled }),
  );
  await page.screenshot({ path: path.join(OUT, '03_plugin_voxel_minicanvas.png') });

  // 5. Cyber Neon — NO cloned "Bố cục Trạm lệnh" reorder list; has live E2EE check & CSS editor
  await page.locator('button:has-text("Quay lại")').first().click();
  await page.waitForSelector('.mod-cards-grid', { timeout: 5000 });
  await page.locator('.mod-card', { hasText: 'Cyber Neon' }).locator('button:has-text("Cài đặt")').click();
  await page.waitForTimeout(250);
  await page.locator('.mod-card', { hasText: 'Cyber Neon' }).locator('button:has-text("Tùy chỉnh")').click();
  await page.waitForSelector('[data-plugin-editor-kind="cyber"]', { timeout: 5000 });
  await page.locator('[data-plugin-editor-kind="cyber"] button:has-text("Kiểm tra E2EE")').click();
  await page.waitForTimeout(200);
  const cyberAudit = await page.evaluate(() => {
    const root = document.querySelector('[data-plugin-editor-kind="cyber"]');
    const txt = root?.innerText || '';
    return {
      orderRows: root?.querySelectorAll('.plugin-order-row').length || 0,
      hasDiag: txt.includes('AES-256-GCM') && txt.includes('vân tay'),
    };
  });
  record('Cyber Neon removed cloned reorder list and runs live E2EE diagnostic', cyberAudit.orderRows === 0 && cyberAudit.hasDiag, JSON.stringify(cyberAudit));
  await page.screenshot({ path: path.join(OUT, '04_plugin_cyber.png') });

  // 6. Công cụ Lệnh nhanh — NO cloned section list, clickable utility chips + draggable commands (zero arrows)
  await page.locator('button:has-text("Quay lại")').first().click();
  await page.waitForSelector('.mod-cards-grid', { timeout: 5000 });
  await page.locator('.mod-card', { hasText: 'Công cụ Lệnh nhanh' }).locator('button:has-text("Tùy chỉnh")').click();
  await page.waitForSelector('[data-plugin-editor-kind="tools"]', { timeout: 5000 });
  await page.locator('[data-plugin-editor-kind="tools"] button:has-text("Chạy")').click();
  await page.waitForTimeout(200);
  const toolsNoArrows = await auditNoNumbersOrArrows('[data-plugin-editor-kind="tools"]');
  const toolsOut = await page.locator('.plugin-calc-result').first().innerText().catch(() => '');
  const toolsHasOrderTitle = await page.locator('[data-plugin-editor-kind="tools"]').innerText().then((t) => t.includes('Sắp xếp tiện ích'));
  record(
    'Công cụ Lệnh nhanh has zero arrows/numbers, no cloned reorder list, and runs /calc live',
    toolsNoArrows.indexCount === 0 && toolsNoArrows.arrowBtns === 0 && !toolsHasOrderTitle && toolsOut.includes('100'),
    JSON.stringify({ toolsNoArrows, toolsHasOrderTitle, toolsOut }),
  );
  await page.screenshot({ path: path.join(OUT, '05_plugin_tools.png') });

  // 7. Enterprise Settings › Thanh bên — Zero arrows, mouse Drag-and-Drop reorders modules!
  await page.locator('.app-rail button[aria-label="Cộng đồng"]').click();
  await page.waitForSelector('.create-community-btn', { timeout: 8000 });
  await page.locator('.create-community-btn').click();
  await page.waitForSelector('.modal-dialog input[type="text"]', { timeout: 5000 });
  await page.locator('.modal-dialog input[type="text"]').first().fill('DoanhNghiep033');
  await page.locator('.modal-dialog input.switch[type="checkbox"]').click();
  await page.locator('.modal-dialog button[type="submit"]').click();
  await page.waitForSelector('.sidebar-server-header', { timeout: 10000 });
  await page.waitForTimeout(350);
  await page.locator('.sidebar-server-header button[aria-label="Cài đặt cộng đồng"]').click();
  await page.waitForSelector('.server-settings-stage', { timeout: 10000 });
  await page.locator('.server-settings-stage .settings-nav-item', { hasText: 'Thiết lập' }).click();
  await page.waitForSelector('[data-drag-group="ent-sidebar-mods"]', { timeout: 8000 });
  await page.waitForTimeout(350);
  const entAudit = await auditNoNumbersOrArrows('[data-drag-group="ent-sidebar-mods"]');
  const beforeOrder = await page.evaluate(() =>
    Array.from(document.querySelectorAll('[data-drag-group="ent-sidebar-mods"] [data-drag-item="true"]')).map((e) => e.getAttribute('data-drag-key')),
  );
  await page
    .locator('[data-drag-group="ent-sidebar-mods"] [data-drag-item="true"]')
    .nth(2)
    .dragTo(page.locator('[data-drag-group="ent-sidebar-mods"] [data-drag-item="true"]').nth(1));
  await page.waitForTimeout(600);
  const afterOrder = await page.evaluate(() =>
    Array.from(document.querySelectorAll('[data-drag-group="ent-sidebar-mods"] [data-drag-item="true"]')).map((e) => e.getAttribute('data-drag-key')),
  );
  record(
    'Enterprise Thanh bên has ZERO arrows and mouse Drag-and-Drop reorders modules',
    entAudit.arrowBtns === 0 && beforeOrder.length >= 4 && afterOrder[1] === beforeOrder[2],
    JSON.stringify({ entAudit, before: beforeOrder.slice(0, 4), after: afterOrder.slice(0, 4) }),
  );
  await page.screenshot({ path: path.join(OUT, '06_enterprise_sidebar_dnd.png') });

  record('Zero uncaught page errors', errors.length === 0, errors.join('; '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
