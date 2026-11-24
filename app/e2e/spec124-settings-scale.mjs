import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve("web/frontend/node_modules/playwright"));

const BASE_URL = process.env.CHATW_URL || "http://127.0.0.1:8085";
const ARTIFACT_DIR = "/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec124";

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
  console.log("=== Spec 124: Normalize Settings Scale, Navigation Proportions and Typography ===");
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

  console.log("1. Navigating to ChatW at", BASE_URL);
  await page.goto(BASE_URL, { waitUntil: "networkidle" });
  await page.waitForTimeout(1500);

  // Auto-login or dev auth
  const isAuth = await page.evaluate(() => !!document.querySelector(".auth-shell, .auth-card"));
  if (isAuth) {
    console.log("Logging in test user...");
    const devInp = await page.$(".auth-dev-form input, input[aria-label], input[placeholder]");
    if (devInp) {
      await devInp.fill("spec124_tester_" + Date.now().toString(36).slice(-4));
      const submitBtn = await page.$(".auth-dev-form button[type='submit'], button[type='submit'], .btn-primary");
      if (submitBtn) await submitBtn.click();
    }
    await page.waitForSelector(".app-rail", { timeout: 20000 });
    await dismissModals(page);
  }

  await dismissModals(page);

  // Set sunset wallpaper and glass panel for beautiful screenshot
  await page.evaluate(() => {
    document.documentElement.setAttribute("data-app-wp", "preset-sunset");
    document.documentElement.setAttribute("data-panel-glass", "true");
  });
  await page.waitForTimeout(500);

  // Open Settings view
  console.log("2. Navigating to Settings view...");
  const settingsBtn = page.locator(".rail-footer button[aria-label*='Settings'], .rail-footer button[data-tip*='Settings'], .rail-item[aria-label*='Settings'], .rail-item:has(svg.lucide-settings)").first();
  if (await settingsBtn.isVisible()) {
    await settingsBtn.click({ force: true });
  } else {
    // Try rail footer gear icon or direct shortcut
    const gearBtn = page.locator(".rail-footer .rail-item").last();
    if (await gearBtn.isVisible()) {
      await gearBtn.click({ force: true });
    }
  }
  await page.waitForTimeout(1000);
  await dismissModals(page);

  // Audit Settings Dimensions
  console.log("3. Auditing Settings Scale & Proportions on 1280x800 Desktop...");
  const audit = await page.evaluate(() => {
    const nav = document.querySelector(".settings-nav");
    const body = document.querySelector(".settings-body");
    const searchInp = document.querySelector(".settings-search input");
    const items = Array.from(document.querySelectorAll(".settings-nav-item"));

    if (!nav) return { error: "No .settings-nav found" };

    const navRect = nav.getBoundingClientRect();
    const navStyle = window.getComputedStyle(nav);

    const searchRect = searchInp ? searchInp.getBoundingClientRect() : null;
    const searchStyle = searchInp ? window.getComputedStyle(searchInp) : null;

    const itemsData = items.map(item => {
      const rect = item.getBoundingClientRect();
      const style = window.getComputedStyle(item);
      const span = item.querySelector("span:not(.chevron)");
      const spanRect = span ? span.getBoundingClientRect() : null;
      const svg = item.querySelector("svg");
      const svgRect = svg ? svg.getBoundingClientRect() : null;
      return {
        text: item.innerText.trim(),
        width: Math.round(rect.width),
        height: Math.round(rect.height),
        fontSize: style.fontSize,
        padding: style.padding,
        gap: style.gap,
        borderRadius: style.borderRadius,
        spanRightPadding: (rect && spanRect) ? Math.round(rect.right - spanRect.right) : null,
        svgSize: svgRect ? `${Math.round(svgRect.width)}x${Math.round(svgRect.height)}` : null
      };
    });

    return {
      navWidth: Math.round(navRect.width),
      navHeight: Math.round(navRect.height),
      navPadding: navStyle.padding,
      search: searchRect ? {
        width: Math.round(searchRect.width),
        height: Math.round(searchRect.height),
        fontSize: searchStyle.fontSize,
        padding: searchStyle.padding
      } : null,
      itemsCount: items.length,
      items: itemsData
    };
  });

  console.log("DESKTOP SETTINGS AUDIT RESULTS:", JSON.stringify(audit, null, 2));

  if (audit.error) {
    throw new Error(`FAIL: ${audit.error}`);
  }

  // Assertions
  console.log(`Checking .settings-nav width: ${audit.navWidth}px (expected >= 270px)`);
  if (audit.navWidth < 270) {
    throw new Error(`FAIL: .settings-nav width is too narrow: ${audit.navWidth}px (expected >= 270px)`);
  }

  if (audit.items.length === 0) {
    throw new Error("FAIL: No .settings-nav-item found in .settings-nav");
  }

  const sampleItem = audit.items[0];
  console.log(`Checking .settings-nav-item height: ${sampleItem.height}px (expected >= 40px)`);
  if (sampleItem.height < 40) {
    throw new Error(`FAIL: .settings-nav-item height is too short: ${sampleItem.height}px (expected >= 40px)`);
  }

  const fontSizeNum = parseFloat(sampleItem.fontSize);
  console.log(`Checking .settings-nav-item font size: ${sampleItem.fontSize} (${fontSizeNum}px, expected >= 14px)`);
  if (fontSizeNum < 14) {
    throw new Error(`FAIL: .settings-nav-item font size is too small: ${sampleItem.fontSize}`);
  }

  if (audit.search) {
    console.log(`Checking .settings-search input height: ${audit.search.height}px (expected >= 36px)`);
    if (audit.search.height < 36) {
      throw new Error(`FAIL: .settings-search input height is too small: ${audit.search.height}px`);
    }
  }

  // Check that no item is clipped against the right edge
  for (const item of audit.items) {
    if (item.spanRightPadding !== null && item.spanRightPadding < 8) {
      throw new Error(`FAIL: Item "${item.text}" has insufficient right padding: ${item.spanRightPadding}px`);
    }
  }

  // Capture Desktop Screenshot
  const desktopPic = path.join(ARTIFACT_DIR, "01_settings_normalized_proportions_1280x800.png");
  await page.screenshot({ path: desktopPic });
  console.log("Saved desktop screenshot:", desktopPic);

  // Test tablet viewport 960x800
  console.log("4. Testing Tablet Viewport (960x800)...");
  await page.setViewportSize({ width: 960, height: 800 });
  await page.waitForTimeout(500);

  const tabletNavWidth = await page.evaluate(() => {
    const nav = document.querySelector(".settings-nav");
    return nav ? Math.round(nav.getBoundingClientRect().width) : null;
  });
  console.log(`Tablet .settings-nav width: ${tabletNavWidth}px (expected >= 250px)`);
  if (tabletNavWidth !== null && tabletNavWidth < 250) {
    throw new Error(`FAIL: Tablet .settings-nav collapsed too narrow: ${tabletNavWidth}px (expected >= 250px)`);
  }

  const tabletPic = path.join(ARTIFACT_DIR, "02_settings_tablet_960x800.png");
  await page.screenshot({ path: tabletPic });
  console.log("Saved tablet screenshot:", tabletPic);

  // Test mobile viewport 390x844
  console.log("5. Testing Mobile Viewport (390x844)...");
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForTimeout(500);

  const mobileNavAudit = await page.evaluate(() => {
    const nav = document.querySelector(".settings-nav");
    const item = document.querySelector(".settings-nav-item");
    return {
      navVisible: nav ? nav.getBoundingClientRect().width > 0 : false,
      itemHeight: item ? Math.round(item.getBoundingClientRect().height) : null
    };
  });
  console.log("Mobile audit:", mobileNavAudit);

  const mobilePic = path.join(ARTIFACT_DIR, "03_settings_mobile_390x844.png");
  await page.screenshot({ path: mobilePic });
  console.log("Saved mobile screenshot:", mobilePic);

  await browser.close();
  console.log("=== SPEC 124 VERIFICATION PASSED 100% ===");
}

run().catch(err => {
  console.error("TEST FAILED:", err);
  process.exit(1);
});
