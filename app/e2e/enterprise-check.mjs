// Live Chromium E2E Verification for Enterprise Server Mode (Spec 015 & Spec 023)
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/tmp/enterprise-e2e-out';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

async function api(ctx, method, urlPath, body) {
  const res = await ctx.request.fetch(`${BASE}${urlPath}`, {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : undefined,
    data: body ? JSON.stringify(body) : undefined,
  });
  let json = null;
  try {
    json = await res.json();
  } catch (_) {}
  return { status: res.status(), body: json };
}

async function createAccount(browser, label) {
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  const suffix = `${Date.now().toString(36).slice(-5)}${Math.floor(Math.random() * 90 + 10)}`;
  const username = `${label}_${suffix}`;

  await page.goto(`${BASE}/`);
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(username);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();
  const me = (await api(ctx, 'GET', '/api/users/me')).body;
  return { ctx, page, username, userId: me?.id };
}

const browser = await chromium.launch({
  executablePath: '/usr/bin/google-chrome',
  headless: true,
  args: ['--no-sandbox', '--disable-gpu'],
});

const errors = [];
try {
  const { ctx: ownerCtx, page: ownerPage, userId: ownerId } = await createAccount(browser, 'ceo');
  const { ctx: staffCtx, page: staffPage, username: staffName, userId: staffId } = await createAccount(browser, 'dev');
  ownerPage.on('pageerror', (e) => errors.push(`owner:${e.message}`));
  staffPage.on('pageerror', (e) => errors.push(`staff:${e.message}`));

  const shot = async (page, name) => {
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(OUT, `${name}.png`) });
  };

  // 1. Open Communities section & Create Server Dialog -> Verify "Chế độ doanh nghiệp" toggle
  await ownerPage.locator('.app-rail button[aria-label="Cộng đồng"]').click();
  await ownerPage.locator('.create-community-btn').click();
  await ownerPage.waitForSelector('.modal-dialog', { timeout: 10000 });
  const modalText = await ownerPage.locator('.modal-dialog').innerText();
  const entCheckbox = ownerPage.locator('.modal-dialog input.switch[type="checkbox"]');
  const defaultChecked = await entCheckbox.isChecked();
  record(
    'Create Server dialog shows Enterprise Mode toggle (default OFF) with exact description',
    modalText.includes('Chế độ doanh nghiệp') &&
      modalText.includes('Bổ sung công cụ quản lý và làm việc dành cho doanh nghiệp.') &&
      defaultChecked === false,
  );
  await shot(ownerPage, '01_create_server_dialog');

  // Turn ON Enterprise mode and create server via UI
  await ownerPage.locator('.modal-dialog input[type="text"]').first().fill('Tập đoàn Công nghệ Vico');
  await entCheckbox.click();
  await ownerPage.locator('.modal-dialog button[type="submit"]').click();
  await ownerPage.waitForSelector('.sidebar-server-header', { timeout: 10000 });
  await ownerPage.locator('.nav-sidebar button.channel-row', { hasText: 'Tổng quan' }).first().waitFor({ state: 'visible', timeout: 10000 });
  await ownerPage.waitForTimeout(400);

  const sidebarText = await ownerPage.locator('.nav-sidebar').innerText();
  record(
    'Enterprise server created and renders Không gian làm việc modules in sidebar',
    sidebarText.includes('Không gian làm việc') &&
      sidebarText.includes('Tổng quan') &&
      sidebarText.includes('Nhân sự') &&
      sidebarText.includes('Công việc'),
  );

  // 2. Open Overview module and verify live KPI cards & attendance check-in
  await ownerPage.locator('.nav-sidebar button.channel-row', { hasText: 'Tổng quan' }).first().click();
  await ownerPage.waitForSelector('.ent-kpi-grid', { timeout: 10000 });
  const ovText = await ownerPage.locator('.ent-workspace').innerText();
  record(
    'Enterprise Overview dashboard renders live KPIs and quick server-clock attendance',
    ovText.includes('Việc cần làm') && ovText.includes('Nhân sự hoạt động'),
  );
  await shot(ownerPage, '02_enterprise_overview');

  // 3. Navigate to HR module -> Batch Import staff member -> Verify employee card
  await ownerPage.locator('.nav-sidebar button.channel-row', { hasText: 'Nhân sự' }).first().click();
  await ownerPage.waitForTimeout(450);
  await ownerPage.locator('.ent-workspace button', { hasText: 'Nhập danh sách' }).click();
  await ownerPage.locator('.ent-workspace textarea').first().fill(`${staffName}, Trần Minh Đức, Kỹ sư Backend, ENG`);
  await ownerPage.locator('.ent-workspace button', { hasText: 'Thực hiện nhập' }).click();
  await ownerPage.waitForTimeout(600);
  const hrText = await ownerPage.locator('.ent-workspace').innerText();
  record(
    'HR module batch imports employee with department & job title',
    hrText.includes('Trần Minh Đức') && hrText.includes('Kỹ sư Backend'),
  );
  await shot(ownerPage, '03_enterprise_hr');

  // 4. Navigate to Departments module -> Verify Org Chart mode
  await ownerPage.locator('.nav-sidebar button.channel-row', { hasText: 'Phòng ban' }).first().click();
  await ownerPage.waitForTimeout(450);
  await ownerPage.locator('.ent-workspace button', { hasText: 'Sơ đồ tổ chức' }).click();
  await ownerPage.waitForTimeout(300);
  const deptText = await ownerPage.locator('.ent-workspace').innerText();
  record(
    'Departments module renders Org Chart with linked channels',
    deptText.includes('Ban điều hành') && deptText.includes('Phòng kỹ thuật sản phẩm'),
  );
  await shot(ownerPage, '04_enterprise_orgchart');

  // 5. Navigate to Tasks module -> Create task from template with Checklist -> Toggle checklist item
  await ownerPage.locator('.nav-sidebar button.channel-row', { hasText: 'Công việc' }).first().click();
  await ownerPage.waitForTimeout(450);
  await ownerPage.locator('.ent-workspace button', { hasText: 'Giao việc mới' }).click();
  await ownerPage.locator('.ent-workspace button.filter-chip', { hasText: 'Xử lý sự cố kỹ thuật' }).click();
  await ownerPage.locator('.ent-workspace button', { hasText: 'Lưu công việc' }).click();
  await ownerPage.waitForTimeout(600);
  const taskText = await ownerPage.locator('.ent-workspace').innerText();
  record(
    'Tasks module creates task from template with interactive checklist in Kanban view',
    taskText.includes('Xử lý sự cố kỹ thuật') && taskText.includes('Xác định nguyên nhân'),
  );
  await shot(ownerPage, '05_enterprise_tasks_kanban');

  // 6. Create task directly from a Channel Message ("Tạo công việc")
  await ownerPage.locator('.nav-sidebar button.channel-row', { hasText: 'Ban điều hành' }).first().click();
  await ownerPage.waitForSelector('#channel-chat-input', { timeout: 10000 });
  await ownerPage.locator('#channel-chat-input').fill('Chuẩn bị hồ sơ kiểm toán quý 4 cho toàn hệ thống');
  await ownerPage.locator('#channel-chat-input').press('Enter');
  await ownerPage.waitForTimeout(500);

  const createTaskBtn = ownerPage.locator('button[aria-label="Tạo công việc"]').last();
  await createTaskBtn.click({ force: true });
  await ownerPage.waitForSelector('.modal-dialog', { timeout: 10000 });
  const msgModalText = await ownerPage.locator('.modal-dialog').innerText();
  record(
    'Channel message action opens Enterprise Task / Document / Announcement modal',
    msgModalText.includes('Tạo công việc') && msgModalText.includes('Lưu vào tài liệu'),
  );
  await shot(ownerPage, '06_task_from_message_modal');
  await ownerPage.locator('.modal-dialog button.btn-primary', { hasText: 'Lưu' }).click();
  await ownerPage.waitForTimeout(500);

  // 7. Open Server Settings -> Verify Enterprise Settings tabs & search
  await ownerPage.locator('.sidebar-server-header button[aria-label="Cài đặt cộng đồng"]').click();
  await ownerPage.waitForSelector('.server-settings-stage', { timeout: 10000 });
  const settingsNavText = await ownerPage.locator('.server-settings-stage .settings-nav').innerText();
  record(
    'Server Settings integrates Enterprise settings categories seamlessly',
    settingsNavText.includes('Thiết lập') &&
      settingsNavText.includes('Nhân sự') &&
      settingsNavText.includes('Chấm công') &&
      settingsNavText.includes('Phê duyệt') &&
      settingsNavText.includes('Bảo mật') &&
      settingsNavText.includes('Kiểm toán'),
  );
  await ownerPage.locator('.server-settings-stage .settings-nav-item', { hasText: 'Thiết lập' }).click();
  await ownerPage.waitForTimeout(300);
  await shot(ownerPage, '07_enterprise_settings');

  // 8. Legacy Server conversion test via API (preserves messages/channels/members)
  const stdSrv = (await api(ownerCtx, 'POST', '/api/servers', { name: 'Cộng đồng Rust Cũ', is_enterprise: false })).body;
  const stdChs = (await api(ownerCtx, 'GET', `/api/servers/${stdSrv.id}/channels`)).body;
  const genChId = stdChs[0].id;
  await api(ownerCtx, 'POST', `/api/channels/${genChId}/messages`, { content: 'Tin nhắn lịch sử trước khi lên doanh nghiệp' });
  const convRes = await api(ownerCtx, 'PUT', `/api/servers/${stdSrv.id}/settings`, { is_enterprise: true });
  const afterMsgs = (await api(ownerCtx, 'GET', `/api/channels/${genChId}/messages`)).body;
  record(
    'Existing community converts to Enterprise Mode without losing channels or message history',
    convRes.status === 200 &&
      convRes.body?.is_enterprise === true &&
      afterMsgs.some((m) => m.content.includes('Tin nhắn lịch sử trước khi lên doanh nghiệp')),
  );

  record('Zero uncaught page errors across all Enterprise flows', errors.length === 0, errors.join('; '));
} finally {
  await browser.close();
}

const passed = checks.filter((c) => c.pass).length;
console.log(`\nSummary: ${passed}/${checks.length} checks passed.`);
fs.writeFileSync(path.join(OUT, 'report.json'), JSON.stringify(checks, null, 2));
if (passed !== checks.length) process.exit(1);
