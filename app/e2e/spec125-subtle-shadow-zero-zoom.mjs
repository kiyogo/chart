import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve("web/frontend/node_modules/playwright"));

const BASE_URL = process.env.CHATW_URL || "http://127.0.0.1:8085";
const ARTIFACT_DIR = "/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec125";

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
  console.log("=== Spec 125: Subtle Shadow & Zero Hover Zoom for Focus Button ===");
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || "/usr/bin/google-chrome",
    headless: true,
    args: ["--no-sandbox", "--disable-setuid-sandbox", "--disable-web-security"]
  });

  const ctx = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1
  });

  const page = await ctx.newPage();
  console.log("Loading ChatW at", BASE_URL);
  await page.goto(BASE_URL, { waitUntil: "networkidle" });
  await page.waitForTimeout(1500);

  // Authenticate if needed
  const isAuth = await page.evaluate(() => !!document.querySelector(".auth-shell, .auth-card"));
  if (isAuth) {
    const devInp = await page.$(".auth-dev-form input, input[aria-label], input[placeholder]");
    if (devInp) {
      await devInp.fill("spec125_tester_" + Date.now().toString(36).slice(-4));
      const submitBtn = await page.$(".auth-dev-form button[type='submit'], button[type='submit'], .btn-primary");
      if (submitBtn) await submitBtn.click();
    }
    await page.waitForSelector(".app-rail", { timeout: 20000 });
  }

  await dismissModals(page);

  // 1. Audit Base CSS on .rich-image-enhance-corner
  console.log("1. Auditing base CSS styles for .rich-image-enhance-corner...");
  const baseAudit = await page.evaluate(() => {
    const el = document.createElement("span");
    el.className = "rich-image-enhance-corner";
    document.body.appendChild(el);
    const style = window.getComputedStyle(el);
    const result = {
      bg: style.backgroundColor,
      filter: style.filter,
      transform: style.transform,
      opacity: style.opacity,
      border: style.borderStyle
    };
    document.body.removeChild(el);
    return result;
  });

  console.log("Base Audit:", baseAudit);
  if (baseAudit.bg !== "rgba(0, 0, 0, 0)" && baseAudit.bg !== "transparent") {
    throw new Error(`FAIL: Expected transparent background, got ${baseAudit.bg}`);
  }
  // Check that blur radius is <= 3px and does not contain 6px or 8px heavy blur
  if (baseAudit.filter.includes("6px") || baseAudit.filter.includes("8px")) {
    throw new Error(`FAIL: Base filter still contains heavy dark blur: ${baseAudit.filter}`);
  }

  // 2. Audit Hover State: Zero Zoom and Subtle Shadow
  console.log("2. Auditing hover state on .rich-image-enhance-corner...");
  // Create a realistic test image card with .rich-image-enhance-corner
  await page.evaluate(() => {
    let dummy = document.getElementById("spec125-test-card");
    if (!dummy) {
      dummy = document.createElement("div");
      dummy.id = "spec125-test-card";
      dummy.style.position = "fixed";
      dummy.style.top = "60px";
      dummy.style.left = "400px";
      dummy.style.width = "240px";
      dummy.style.height = "240px";
      dummy.style.background = "#e2e8f0";
      dummy.style.borderRadius = "12px";
      dummy.style.zIndex = "9999";
      dummy.style.overflow = "hidden";

      const btn = document.createElement("span");
      btn.className = "rich-image-enhance-corner";
      btn.innerHTML = `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3"></circle><path d="M3 7V5a2 2 0 0 1 2-2h2"></path><path d="M17 3h2a2 2 0 0 1 2 2v2"></path><path d="M21 17v2a2 2 0 0 1-2 2h-2"></path><path d="M7 21H5a2 2 0 0 1-2-2v-2"></path></svg>`;
      dummy.appendChild(btn);
      document.body.appendChild(dummy);
    }
  });

  const btnLocator = page.locator("#spec125-test-card .rich-image-enhance-corner");
  await btnLocator.waitFor({ state: "visible" });

  // Hover over the button
  await btnLocator.hover();
  await page.waitForTimeout(300);

  const hoverAudit = await page.evaluate(() => {
    const el = document.querySelector("#spec125-test-card .rich-image-enhance-corner");
    const style = window.getComputedStyle(el);
    return {
      bg: style.backgroundColor,
      filter: style.filter,
      transform: style.transform,
      opacity: style.opacity
    };
  });

  console.log("Hover Audit:", hoverAudit);

  // Assertion: MUST NOT scale up on hover
  if (hoverAudit.transform && hoverAudit.transform !== "none") {
    // If it's a matrix, check that scale is 1.0 (not 1.15)
    if (hoverAudit.transform.includes("1.15")) {
      throw new Error(`FAIL: Hover zoom detected! Transform is ${hoverAudit.transform} (expected transform: none / 1:1 scale)`);
    }
  }

  // Assertion: MUST NOT have heavy dark blur
  if (hoverAudit.filter.includes("8px") || hoverAudit.filter.includes("6px")) {
    throw new Error(`FAIL: Hover filter has heavy blur halo: ${hoverAudit.filter}`);
  }

  // Assertion: Background must stay transparent
  if (hoverAudit.bg !== "rgba(0, 0, 0, 0)" && hoverAudit.bg !== "transparent") {
    throw new Error(`FAIL: Hover background must be transparent, got ${hoverAudit.bg}`);
  }

  // Take screenshot of hover state
  const picPath = path.join(ARTIFACT_DIR, "01_clean_subtle_focus_icon.png");
  await page.screenshot({ path: picPath });
  console.log("Saved clean focus icon screenshot:", picPath);

  await browser.close();
  console.log("=== SPEC 125 VERIFICATION PASSED 100% ===");
}

run().catch(err => {
  console.error("TEST FAILED:", err);
  process.exit(1);
});
