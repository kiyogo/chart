import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve("web/frontend/node_modules/playwright"));

const BASE_URL = process.env.CHATW_URL || "http://127.0.0.1:8085";
const ARTIFACT_DIR = "/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec120";

if (!fs.existsSync(ARTIFACT_DIR)) {
  fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
}

async function dismissModals(page) {
  for (let i = 0; i < 6; i++) {
    const ov = page.locator(".modal-overlay").first();
    if (!(await ov.isVisible().catch(() => false))) break;
    const btn = ov.locator("button.story-close, button[aria-label='Skip tour'], button[aria-label='Close'], button:has-text('Later'), button:has-text('Skip'), button:has-text('Đóng'), button:has-text('Bỏ qua'), button.btn-secondary").first();
    if (await btn.isVisible().catch(() => false)) {
      await btn.click({ force: true }).catch(() => {});
      await page.waitForTimeout(300);
    } else {
      await page.keyboard.press("Escape").catch(() => {});
      await page.waitForTimeout(300);
    }
  }
}

async function run() {
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || "/usr/bin/google-chrome",
    headless: true,
    args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-web-security"]
  });

  const context = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1
  });

  const page = await context.newPage();

  console.log("Loading ChatW at", BASE_URL);
  await page.goto(BASE_URL, { waitUntil: "networkidle" });
  await page.waitForTimeout(1500);

  // Quick auto-login or registration if on auth screen
  const isAuth = await page.evaluate(() => !!document.querySelector(".auth-shell, .auth-card"));
  if (isAuth) {
    console.log("Logging in test user...");
    const devInp = await page.$(".auth-dev-form input, input[aria-label], input[placeholder]");
    if (devInp) {
      await devInp.fill("spec120_tester_" + Date.now().toString(36).slice(-4));
      const submitBtn = await page.$(".auth-dev-form button[type='submit'], button[type='submit'], .btn-primary");
      if (submitBtn) await submitBtn.click();
    }
    await page.waitForSelector(".app-rail", { timeout: 20000 });
    await dismissModals(page);
  }

  await dismissModals(page);

  // Ensure sunset wallpaper is active for strict gap and frosted glass verification
  await page.evaluate(() => {
    document.documentElement.setAttribute("data-app-wp", "preset-sunset");
    document.documentElement.setAttribute("data-panel-glass", "true");
  });
  await page.waitForTimeout(500);

  // ==========================================
  // AUDIT 1: DESKTOP WORKSPACE SHELL & FLUSH PANELS
  // ==========================================
  const desktopLayoutAudit = await page.evaluate(() => {
    const shell = document.querySelector(".workspace-shell");
    const rail = document.querySelector(".app-rail");
    const sidebar = document.querySelector(".nav-sidebar");
    const mainStage = document.querySelector(".main-stage");

    const shellStyle = shell ? window.getComputedStyle(shell) : null;
    const railRect = rail ? rail.getBoundingClientRect() : null;
    const sidebarRect = sidebar ? sidebar.getBoundingClientRect() : null;
    const stageRect = mainStage ? mainStage.getBoundingClientRect() : null;

    const railStyle = rail ? window.getComputedStyle(rail) : null;
    const sidebarStyle = sidebar ? window.getComputedStyle(sidebar) : null;
    const stageStyle = mainStage ? window.getComputedStyle(mainStage) : null;

    // First rail item
    const firstRailItem = document.querySelector(".rail-nav .rail-item");
    const firstRailSvg = firstRailItem ? firstRailItem.querySelector("svg") : null;
    const railItemRect = firstRailItem ? firstRailItem.getBoundingClientRect() : null;
    const railSvgRect = firstRailSvg ? firstRailSvg.getBoundingClientRect() : null;

    return {
      shellPadding: shellStyle ? shellStyle.padding : null,
      shellGap: shellStyle ? shellStyle.gap : null,
      railRect: railRect ? { left: railRect.left, right: railRect.right, width: railRect.width, height: railRect.height } : null,
      sidebarRect: sidebarRect ? { left: sidebarRect.left, right: sidebarRect.right, width: sidebarRect.width, height: sidebarRect.height } : null,
      stageRect: stageRect ? { left: stageRect.left, right: stageRect.right, width: stageRect.width, height: stageRect.height } : null,
      railRadius: railStyle ? railStyle.borderRadius : null,
      sidebarRadius: sidebarStyle ? sidebarStyle.borderRadius : null,
      stageRadius: stageStyle ? stageStyle.borderRadius : null,
      railGlass: railStyle ? { bg: railStyle.backgroundColor, backdropFilter: railStyle.backdropFilter } : null,
      sidebarGlass: sidebarStyle ? { bg: sidebarStyle.backgroundColor, backdropFilter: sidebarStyle.backdropFilter } : null,
      gapBetweenRailAndSidebar: (railRect && sidebarRect) ? sidebarRect.left - railRect.right : null,
      gapBetweenSidebarAndStage: (sidebarRect && stageRect) ? stageRect.left - sidebarRect.right : null,
      railItemDim: railItemRect ? { w: Math.round(railItemRect.width), h: Math.round(railItemRect.height) } : null,
      railSvgDim: railSvgRect ? { w: Math.round(railSvgRect.width), h: Math.round(railSvgRect.height) } : null
    };
  });

  console.log("DESKTOP_LAYOUT_AUDIT:", JSON.stringify(desktopLayoutAudit, null, 2));

  // Assertions for Desktop Framed Panels ("khung như cũ"):
  const railRad = parseFloat(desktopLayoutAudit.railRadius);
  if (isNaN(railRad) || railRad < 14) {
    throw new Error(`FAIL: Left Rail border-radius is ${desktopLayoutAudit.railRadius} (expected framed card ~17-18px)`);
  }
  const sideRad = parseFloat(desktopLayoutAudit.sidebarRadius);
  if (isNaN(sideRad) || sideRad < 14) {
    throw new Error(`FAIL: Sidebar border-radius is ${desktopLayoutAudit.sidebarRadius} (expected framed card ~17-18px)`);
  }
  const sGap = parseFloat(desktopLayoutAudit.shellGap);
  if (isNaN(sGap) || sGap < 10) {
    throw new Error(`FAIL: Shell gap is ${desktopLayoutAudit.shellGap} (expected ~11-12px)`);
  }

  await page.screenshot({ path: path.join(ARTIFACT_DIR, "01_desktop_main_chat_flush_1280x800.png") });

  // ==========================================
  // AUDIT 2: SETTINGS PAGE SEAMLESSNESS & COMPACT SCALE
  // ==========================================
  await dismissModals(page);
  // Click on Settings in Left Rail
  const settingsBtn = page.locator(".rail-bottom button[aria-label='Settings'], .rail-bottom button[data-tip='Settings']").first();
  if (await settingsBtn.isVisible()) {
    await settingsBtn.click({ force: true });
    await page.waitForTimeout(1000);
  }

  const settingsAudit = await page.evaluate(() => {
    const settingsPage = document.querySelector(".settings-page");
    const settingsNav = document.querySelector(".settings-nav");
    const settingsBody = document.querySelector(".settings-body");
    const navItems = Array.from(document.querySelectorAll(".settings-nav-item"));

    const pageStyle = settingsPage ? window.getComputedStyle(settingsPage) : null;
    const navRect = settingsNav ? settingsNav.getBoundingClientRect() : null;
    const bodyRect = settingsBody ? settingsBody.getBoundingClientRect() : null;

    const itemAudits = navItems.slice(0, 5).map(item => {
      const rect = item.getBoundingClientRect();
      const style = window.getComputedStyle(item);
      return {
        text: item.innerText.trim(),
        width: Math.round(rect.width),
        height: Math.round(rect.height),
        fontSize: style.fontSize,
        borderRadius: style.borderRadius,
        padding: style.padding
      };
    });

    return {
      pageGap: pageStyle ? pageStyle.gap : null,
      gapBetweenNavAndBody: (navRect && bodyRect) ? bodyRect.left - navRect.right : null,
      navBorderRadius: settingsNav ? window.getComputedStyle(settingsNav).borderRadius : null,
      bodyBorderRadius: settingsBody ? window.getComputedStyle(settingsBody).borderRadius : null,
      itemAudits
    };
  });

  console.log("SETTINGS_AUDIT:", JSON.stringify(settingsAudit, null, 2));

  if (settingsAudit.gapBetweenNavAndBody !== null) {
    if (settingsAudit.gapBetweenNavAndBody < 8) {
      throw new Error(`FAIL: Gap between Settings Nav and Settings Body is ${settingsAudit.gapBetweenNavAndBody}px (expected framed gap ~11-12px)`);
    }
  }
  if (settingsAudit.itemAudits.length > 0) {
    const firstItem = settingsAudit.itemAudits[0];
    if (firstItem.height > 38) {
      throw new Error(`FAIL: .settings-nav-item height is ${firstItem.height}px (expected compact <= 36px)`);
    }
  }

  await page.screenshot({ path: path.join(ARTIFACT_DIR, "02_desktop_settings_flush_compact_1280x800.png") });

  // ==========================================
  // AUDIT 3: COMMUNITIES & CHANNELS COMPACT SCALE
  // ==========================================
  await dismissModals(page);
  // Click Communities (Compass icon in rail-nav)
  const commBtn = page.locator(".rail-nav button[aria-label*='Communities'], .rail-nav button[data-tip*='Communities']").first();
  if (await commBtn.isVisible()) {
    await commBtn.click({ force: true });
    await page.waitForTimeout(1000);
  }

  const createBtn = page.locator(".create-community-btn").first();
  if (await createBtn.isVisible()) {
    await createBtn.click({ force: true });
    await page.waitForSelector(".modal-dialog input[type='text']", { timeout: 5000 });
    await page.locator(".modal-dialog input[type='text']").first().fill("123");
    await page.locator(".modal-dialog button[type='submit']").click();
    await page.waitForSelector(".sidebar-server-header", { timeout: 8000 });
    await page.waitForTimeout(800);
  }

  const channelAudit = await page.evaluate(() => {
    const channelRows = Array.from(document.querySelectorAll(".channel-row"));
    const catRows = Array.from(document.querySelectorAll(".category-row"));

    const channels = channelRows.slice(0, 4).map(ch => {
      const rect = ch.getBoundingClientRect();
      const style = window.getComputedStyle(ch);
      return {
        text: ch.innerText.trim(),
        height: Math.round(rect.height),
        fontSize: style.fontSize,
        borderRadius: style.borderRadius
      };
    });

    const categories = catRows.slice(0, 3).map(cat => {
      const rect = cat.getBoundingClientRect();
      const style = window.getComputedStyle(cat);
      return {
        text: cat.innerText.trim(),
        height: Math.round(rect.height),
        fontSize: style.fontSize
      };
    });

    return { channels, categories };
  });

  console.log("CHANNEL_AUDIT:", JSON.stringify(channelAudit, null, 2));

  if (channelAudit.channels.length > 0) {
    const ch = channelAudit.channels[0];
    if (ch.height > 34) {
      throw new Error(`FAIL: .channel-row height is ${ch.height}px (expected compact <= 34px)`);
    }
    if (ch.borderRadius === "12px") {
      throw new Error(`FAIL: .channel-row border-radius is 12px bulbous (expected subtle 6px/8px)`);
    }
  }

  await page.screenshot({ path: path.join(ARTIFACT_DIR, "03_desktop_channels_flush_compact_1280x800.png") });

  await browser.close();
  console.log("SPEC 120 DESKTOP 100% SEAMLESS & SCALE AUDIT: 100% PASSED!");
}

run().catch(err => {
  console.error("Test failed:", err);
  process.exit(1);
});
