// Comprehensive Live End-to-End Verification for ChatW Plugin Platform (Spec 037)
// Tests:
// 1. Third-party JS/HTML/CSS SDK plugin (acme.task_tracker): Publish -> Consent Modal -> Sandboxed iframe UI -> Storage -> Settings -> Isolation probe -> Permission denial -> /track command -> Dynamic Message Action -> Event Hook -> Teardown
// 2. Third-party compiled Rust-to-WASM SDK plugin (acme.calc_wasm): Publish real 121KB .wasm -> Install -> Execute /wcalc (12 + 30) * 2 - 4 = 80
// 3. Discord-style Incoming Webhook -> Real Channel Message -> Visual .discord-embed-card rendering
// 4. Multi-device Realtime Sync across 2 independent Chromium contexts (Device A -> Device B via WsEvent::PluginSync)
// 5. Privacy Dashboard -> Instant Grant Revocation
// 6. Real Cryptographic E2EE Verification in Cyber Neon (replaces old fake check)

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE_URL = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_037';
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

async function dismissModals(page) {
  for (let i = 0; i < 6; i++) {
    const ov = page.locator('.modal-overlay').first();
    if (!(await ov.isVisible().catch(() => false))) break;
    const txt = await ov.innerText().catch(() => '');
    const btn = ov
      .locator(
        'button[aria-label="Bỏ qua giới thiệu"], button[aria-label="Đóng"], button:has-text("Chấp nhận đồng bộ"), button:has-text("Để sau"), button:has-text("Bỏ qua"), button:has-text("Đóng"), button:has-text("Tạo khóa mới"), button:has-text("Xong"), button.btn-secondary, button.btn-ghost'
      )
      .first();
    if (await btn.isVisible().catch(() => false)) {
      await btn.click({ force: true }).catch(() => {});
      await page.waitForTimeout(300);
    } else {
      console.log('MODAL_OVERLAY_TEXT:', txt.slice(0, 200));
      await page.keyboard.press('Escape').catch(() => {});
      await page.waitForTimeout(300);
    }
  }
}

async function loginFresh(ctx, username) {
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(`[${username}] ${e.message}`));
  await page.goto(BASE_URL, { waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(username);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(900);
  await dismissModals(page);
  return page;
}

try {
  const tag = Math.random().toString(36).slice(2, 7);
  const uname = `plg_${tag}`;
  const pubSlug = `acme${tag}`;

  // Device A & Device B (two independent browser contexts logged into SAME user account for realtime sync!)
  const ctxA = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const pageA = await loginFresh(ctxA, uname);

  const ctxB = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const pageB = await loginFresh(ctxB, uname);

  // 1. Register Publisher & Publish BOTH real SDK bundles (JS Task Tracker + Compiled Rust WASM Calculator)
  const jsBundleRaw = JSON.parse(
    fs.readFileSync('sdk/examples/task-tracker-js/acme.task_tracker-1.0.0.chatw-plugin.json', 'utf8')
  );
  jsBundleRaw.manifest.plugin_key = `${pubSlug}.task_tracker`;
  jsBundleRaw.manifest.publisher = pubSlug;
  delete jsBundleRaw.sha256; // server computes & seals canonical sha256 for this unique publisher namespace

  const wasmBundleRaw = JSON.parse(
    fs.readFileSync('sdk/examples/calc-wasm-rust/acme.calc_wasm-1.0.0.chatw-plugin.json', 'utf8')
  );
  wasmBundleRaw.manifest.plugin_key = `${pubSlug}.calc_wasm`;
  wasmBundleRaw.manifest.publisher = pubSlug;
  delete wasmBundleRaw.sha256;

  const publishRes = await pageA.evaluate(
    async ({ pubSlug, jsBundleRaw, wasmBundleRaw }) => {
      const api = async (method, url, body) => {
        const r = await fetch(url, {
          method,
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: body ? JSON.stringify(body) : undefined,
        });
        return { status: r.status, data: await r.json().catch(() => ({})) };
      };
      const pub = await api('POST', '/api/plugins/publishers', {
        slug: pubSlug,
        display_name: `Acme Studio ${pubSlug}`,
        website_url: 'https://example.com',
      });
      const jsPkg = await api('POST', '/api/plugins/packages', jsBundleRaw);
      const wasmPkg = await api('POST', '/api/plugins/packages', wasmBundleRaw);
      return { pub, jsPkg, wasmPkg };
    },
    { pubSlug, jsBundleRaw, wasmBundleRaw }
  );

  record(
    'Publish real JS SDK plugin + real compiled Rust WASM plugin via Platform API',
    publishRes.pub.status === 200 && publishRes.jsPkg.status === 200 && publishRes.wasmPkg.status === 200,
    JSON.stringify({ pub: publishRes.pub.status, js: publishRes.jsPkg.status, wasm: publishRes.wasmPkg.status })
  );

  // 2. Open Plugin Hub UI on Device A -> Marketplace shows both + official chatw.focus_board
  await dismissModals(pageA);
  await dismissModals(pageB);
  await pageA.locator('.app-rail button[aria-label="Plugin"]').click();
  await pageA.waitForSelector('[data-plugin-hub="v2"]', { timeout: 15000 });
  await pageA.waitForSelector(`[data-install-plugin="${pubSlug}.task_tracker"]`, { timeout: 15000 });
  await pageA.screenshot({ path: path.join(OUT, '01_marketplace.png') });

  // 3. Click Install on JS Task Tracker -> Verify Consent Modal enforces explicit checkbox confirmation!
  await pageA.locator(`[data-install-plugin="${pubSlug}.task_tracker"]`).click();
  await pageA.waitForSelector(`[data-consent-modal="${pubSlug}.task_tracker"]`, { timeout: 10000 });
  const submitBeforeCheck = await pageA.locator('[data-consent-submit="true"]').isDisabled();
  await pageA.screenshot({ path: path.join(OUT, '02_consent_modal.png') });
  await pageA.locator('[data-consent-checkbox="true"]').check();
  const submitAfterCheck = await pageA.locator('[data-consent-submit="true"]').isDisabled();
  await pageA.locator('[data-consent-submit="true"]').click();

  record(
    'Permission Consent Modal blocks install until explicit user confirmation',
    submitBeforeCheck === true && submitAfterCheck === false,
    `disabledBefore=${submitBeforeCheck} disabledAfter=${submitAfterCheck}`
  );

  // 4. Verify Sandboxed Custom HTML/CSS/JS Iframe UI loads, isolates parent/localStorage, saves storage & denies unauthorized E2EE!
  await pageA.waitForSelector('#plugin-hub-sandbox-stage iframe.plugin-sandbox-frame', { timeout: 15000 });
  const iframeAttrs = await pageA.evaluate(() => {
    const f = document.querySelector('#plugin-hub-sandbox-stage iframe.plugin-sandbox-frame');
    return f ? { sandbox: f.getAttribute('sandbox'), hasSrcdoc: !!(f.getAttribute('srcdoc') || f.src) } : null;
  });
  record(
    'Custom UI runs inside opaque-origin sandbox="allow-scripts" iframe (NO allow-same-origin)',
    iframeAttrs && iframeAttrs.sandbox === 'allow-scripts' && iframeAttrs.hasSrcdoc,
    JSON.stringify(iframeAttrs)
  );

  const frame = pageA.frameLocator('#plugin-hub-sandbox-stage iframe.plugin-sandbox-frame');
  await frame.locator('#task-input').waitFor({ timeout: 10000 });
  await frame.locator('#task-input').fill('Hoàn thiện ChatW Plugin SDK');
  await frame.locator('#add-btn').click();
  await pageA.waitForTimeout(500);
  const taskCardVisible = await frame.locator('.card', { hasText: 'Hoàn thiện ChatW Plugin SDK' }).isVisible();

  await frame.locator('#probe-sandbox-btn').click();
  await pageA.waitForTimeout(200);
  const sandboxProbe = await frame.locator('#probe-out').innerText();

  await frame.locator('#probe-perm-btn').click();
  await pageA.waitForTimeout(300);
  const permProbe = await frame.locator('#probe-out').innerText();

  await frame.locator('#save-settings-btn').click();
  await pageA.waitForTimeout(400);
  const settingsProbe = await frame.locator('#probe-out').innerText();
  await pageA.screenshot({ path: path.join(OUT, '03_sandbox_custom_ui.png') });

  record(
    'Custom JS/HTML/CSS plugin UI creates persistent task, proves sandbox isolation, denies unauthorized E2EE, saves settings',
    taskCardVisible &&
      sandboxProbe.includes('parentBlocked=true') &&
      sandboxProbe.includes('localStorageBlocked=true') &&
      permProbe.includes('DENIED_OK') &&
      settingsProbe.includes('SETTINGS_SAVED'),
    JSON.stringify({ taskCardVisible, sandboxProbe, permProbe, settingsProbe })
  );

  // 5. Install compiled Rust WASM Calculator plugin via Consent Modal
  await pageA.locator('[data-plugin-hub="v2"] button[role="tab"]', { hasText: 'Chợ Plugin' }).click();
  await pageA.waitForSelector(`[data-install-plugin="${pubSlug}.calc_wasm"]`, { timeout: 10000 });
  await pageA.locator(`[data-install-plugin="${pubSlug}.calc_wasm"]`).click();
  await pageA.waitForSelector(`[data-consent-modal="${pubSlug}.calc_wasm"]`, { timeout: 10000 });
  await pageA.locator('[data-consent-checkbox="true"]').check();
  await pageA.locator('[data-consent-submit="true"]').click();
  await pageA.waitForSelector(`[data-installed-key="${pubSlug}.calc_wasm"]`, { timeout: 10000 });
  await pageA.waitForTimeout(600);

  // 6. Verify Multi-Device Realtime Sync: Device B automatically received BOTH installations via WsEvent::PluginSync WITHOUT reloading!
  await pageB.waitForTimeout(1000);
  const deviceBStatuses = await pageB.evaluate(() =>
    window.ChatwPluginHost ? window.ChatwPluginHost.listStatuses() : []
  );
  const syncedOnB =
    deviceBStatuses.some((s) => s.plugin_key === `${pubSlug}.task_tracker` && s.enabled) &&
    deviceBStatuses.some((s) => s.plugin_key === `${pubSlug}.calc_wasm` && s.enabled);
  record(
    'Multi-device realtime sync delivers new installations to Device B via WebSocket without reload',
    syncedOnB,
    JSON.stringify(deviceBStatuses.map((s) => ({ key: s.plugin_key, enabled: s.enabled })))
  );

  // 7. Create a real Server + Channel via UI -> Fire Discord Incoming Webhook live via WebSocket -> Execute JS /track & Rust WASM /wcalc!
  await pageA.locator('.app-rail button[aria-label="Cộng đồng"]').click();
  await pageA.waitForSelector('.create-community-btn', { timeout: 10000 });
  await pageA.locator('.create-community-btn').click();
  await pageA.waitForSelector('.modal-dialog input[type="text"]', { timeout: 8000 });
  await pageA.locator('.modal-dialog input[type="text"]').first().fill('Plugin Verify Guild');
  await pageA.locator('.modal-dialog button[type="submit"]').click();
  await pageA.waitForSelector('.sidebar-server-header', { timeout: 10000 });
  await pageA.waitForTimeout(500);

  const srvSetup = await pageA.evaluate(async () => {
    const api = async (method, url, body) => {
      const r = await fetch(url, {
        method,
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: body ? JSON.stringify(body) : undefined,
      });
      return r.json();
    };
    const srvs = await api('GET', '/api/servers');
    const srv = srvs.find((s) => s.name === 'Plugin Verify Guild') || srvs[0];
    const chs = await api('GET', `/api/servers/${srv.id}/channels`);
    const textCh = chs.find((c) => c.channel_type === 'text') || chs[0];
    const chId = textCh.id;
    const wh = await api('POST', '/api/webhooks', {
      direction: 'incoming',
      scope_type: 'channel',
      scope_id: chId,
      channel_id: chId,
      name: 'Release CI Bot',
      rate_limit_per_min: 30,
    });
    const postResp = await fetch(`/api/webhooks/${wh.id}/${wh.token}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        username: 'GitHub Actions',
        content: 'Bản dựng ChatW Plugin Platform đã hoàn tất',
        embeds: [
          {
            title: 'Spec 037 Release Check',
            description: 'Tất cả bài kiểm thử WASM, Sandbox và Webhook đã thông qua.',
            url: 'https://github.com/kiyogo/chart',
            color: 2450411,
            fields: [
              { name: 'Runtime', value: 'JS Worker + Rust WASM', inline: true },
              { name: 'Security', value: 'Opaque Sandbox + SSRF', inline: true },
            ],
            footer: { text: 'ChatW Webhook Engine' },
          },
        ],
      }),
    });
    return { srv, chId, whOk: postResp.ok };
  });

  await pageA.waitForSelector('.discord-embed-card', { timeout: 15000 });
  await pageA.waitForTimeout(400);
  const embedAudit = await pageA.evaluate(() => ({
    hasCard: !!document.querySelector('.discord-embed-card'),
    hasBadge: !!document.querySelector('.webhook-badge'),
    titleText: document.querySelector('.discord-embed-title')?.textContent || '',
    fieldCount: document.querySelectorAll('.discord-embed-field').length,
  }));
  record(
    'Discord-style Incoming Webhook posts real channel message and renders .discord-embed-card visually',
    srvSetup.whOk && embedAudit.hasCard && embedAudit.hasBadge && embedAudit.titleText.includes('Spec 037') && embedAudit.fieldCount === 2,
    JSON.stringify(embedAudit)
  );

  // Execute JS plugin slash command `/track` AND compiled Rust WASM command `/wcalc (12 + 30) * 2 - 4` in channel composer!
  const composer = pageA.locator('textarea.composer-input').first();
  await composer.fill('/track Kiểm thử lệnh từ Worker');
  await composer.press('Enter');
  await pageA.waitForTimeout(900);

  await composer.fill('/wcalc (12 + 30) * 2 - 4');
  await composer.press('Enter');
  await pageA.waitForTimeout(1100);

  const chatText = await pageA.locator('.message-stream').innerText();
  const jsCmdWorked = chatText.includes('Đã tạo thẻ công việc') && chatText.includes('Kiểm thử lệnh từ Worker');
  const wasmCmdWorked = chatText.includes('WASM Calc: (12 + 30) * 2 - 4 = 80');
  record(
    'JS Sandbox slash command (/track) executes live in chat composer',
    jsCmdWorked,
    jsCmdWorked ? 'rendered task creation output' : chatText.slice(-300)
  );
  record(
    'Compiled Rust WebAssembly slash command (/wcalc) executes live in chat composer',
    wasmCmdWorked,
    wasmCmdWorked ? 'WASM Calc: (12 + 30) * 2 - 4 = 80' : chatText.slice(-300)
  );

  // Dynamic Message Action button from third-party plugin ("Tạo thẻ")
  const hasMsgAction = (await pageA.locator('[data-plugin-msg-action="create_task_card"]').count()) > 0;
  if (hasMsgAction) {
    await pageA.locator('[data-plugin-msg-action="create_task_card"]').first().click({ force: true });
    await pageA.waitForTimeout(500);
  }
  record('Third-party plugin dynamically injects Message Action button ("Tạo thẻ") into chat toolbar', hasMsgAction);
  await pageA.screenshot({ path: path.join(OUT, '04_channel_embed_and_wasm_cmd.png') });

  // 8. Privacy Dashboard -> Instant Grant Revocation & Teardown on Disable/Uninstall
  await pageA.locator('.app-rail button[aria-label="Plugin"]').click();
  await pageA.waitForSelector('[data-plugin-hub="v2"]', { timeout: 10000 });
  await pageA.locator('[data-plugin-hub="v2"] button[role="tab"]', { hasText: 'Quyền riêng tư' }).click();
  await pageA.waitForSelector('[data-revoke-grant]', { timeout: 10000 });
  const grantsBefore = await pageA.locator('[data-revoke-grant]').count();
  await pageA.screenshot({ path: path.join(OUT, '05_privacy_dashboard.png') });
  await pageA.locator('[data-revoke-grant]').first().click();
  await pageA.waitForTimeout(600);
  const grantsAfter = await pageA.locator('[data-revoke-grant]').count();
  record(
    'Privacy Dashboard lists active grants and revokes grant immediately',
    grantsBefore > 0 && grantsAfter === grantsBefore - 1,
    `before=${grantsBefore} after=${grantsAfter}`
  );

  // Disable Task Tracker -> Verify teardown immediately stops worker & syncs to Device B!
  await pageA.locator('[data-plugin-hub="v2"] button[role="tab"]', { hasText: 'Đã cài đặt' }).click();
  await pageA.waitForSelector(`[data-toggle-plugin="${pubSlug}.task_tracker"]`, { timeout: 10000 });
  await pageA.locator(`[data-toggle-plugin="${pubSlug}.task_tracker"]`).uncheck();
  await pageA.waitForTimeout(900);

  const afterDisableA = await pageA.evaluate((k) => {
    const st = window.ChatwPluginHost ? window.ChatwPluginHost.listStatuses() : [];
    const found = st.find((x) => x.plugin_key === k);
    const acts = window.ChatwPluginHost ? window.ChatwPluginHost.getMessageActions(null) : [];
    return { found, actsCount: acts.length };
  }, `${pubSlug}.task_tracker`);

  const afterDisableB = await pageB.evaluate((k) => {
    const st = window.ChatwPluginHost ? window.ChatwPluginHost.listStatuses() : [];
    return st.find((x) => x.plugin_key === k) || null;
  }, `${pubSlug}.task_tracker`);

  record(
    'Disabling plugin terminates sandbox Worker, removes Message Action button, and syncs disable to Device B',
    !afterDisableA.found && afterDisableA.actsCount === 0 && !afterDisableB,
    JSON.stringify({ afterDisableA, afterDisableB })
  );

  // 9. Verify Cyber Neon's "Kiểm tra E2EE" runs REAL P-256 + AES-256-GCM cryptographic check (not old hardcoded string!)
  await pageA.locator('.mod-card', { hasText: 'Cyber Neon' }).locator('button:has-text("Cài đặt"), button:has-text("Tùy chỉnh")').first().click();
  await pageA.waitForTimeout(300);
  const customBtn = pageA.locator('.mod-card', { hasText: 'Cyber Neon' }).locator('button:has-text("Tùy chỉnh")');
  if (await customBtn.isVisible().catch(() => false)) await customBtn.click();
  await pageA.waitForSelector('button:has-text("Kiểm tra E2EE")', { timeout: 8000 });
  await pageA.locator('button:has-text("Kiểm tra E2EE")').click();
  await pageA.waitForTimeout(250);
  const e2eeOut = await pageA.locator('.plugin-calc-result').first().innerText();
  record(
    'Cyber Neon "Kiểm tra E2EE" executes real P-256 fingerprint & AES-256-GCM check',
    e2eeOut.includes('vân tay ') && e2eeOut.includes('AES-256-GCM'),
    e2eeOut
  );

  record('Zero unhandled page errors during entire E2E flow', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
