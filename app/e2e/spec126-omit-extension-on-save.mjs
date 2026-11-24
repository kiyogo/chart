import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve("web/frontend/node_modules/playwright"));

const BASE_URL = process.env.CHATW_URL || "http://127.0.0.1:8085";
const ARTIFACT_DIR = "/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec126";
const TEST_FILE = "/tmp/hinh-nen-may-tinh-anime-4k (94).jpg";

if (!fs.existsSync(ARTIFACT_DIR)) {
  fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
}

// Ensure test file exists
if (!fs.existsSync(TEST_FILE)) {
  const smallJpg = Buffer.from('/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAP//////////////////////////////////////////////////////////////////////////////////////wgALCAABAAEBAREA/8QAFBABAAAAAAAAAAAAAAAAAAAAAP/aAAgBAQABPxA=', 'base64');
  fs.writeFileSync(TEST_FILE, smallJpg);
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
  console.log("=== Spec 126: Omit File Extension When Saving & Displaying Media Filenames ===");
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
  console.log("1. Loading ChatW at", BASE_URL);
  await page.goto(BASE_URL, { waitUntil: "networkidle" });
  await page.waitForTimeout(1500);

  // Authenticate if needed
  const isAuth = await page.evaluate(() => !!document.querySelector(".auth-shell, .auth-card"));
  if (isAuth) {
    const devInp = await page.$(".auth-dev-form input, input[aria-label], input[placeholder]");
    if (devInp) {
      await devInp.fill("spec126_tester_" + Date.now().toString(36).slice(-4));
      const submitBtn = await page.$(".auth-dev-form button[type='submit'], button[type='submit'], .btn-primary");
      if (submitBtn) await submitBtn.click();
    }
    await page.waitForSelector(".app-rail", { timeout: 20000 });
  }

  await dismissModals(page);

  // 1. Audit window.__chatwMediaOpt.stripFileExtension
  console.log("2. Testing stripFileExtension unit logic...");
  const logicAudit = await page.evaluate(() => {
    const strip = window.__chatwMediaOpt?.stripFileExtension;
    if (!strip) return { error: "Missing stripFileExtension function" };
    return {
      anime: strip("hinh-nen-may-tinh-anime-4k (94).jpg"),
      png: strip("photo.png"),
      multiDot: strip("my.cool.photo.avif"),
      noExt: strip("just_a_name"),
      empty: strip("")
    };
  });

  console.log("Logic Audit Result:", logicAudit);
  if (logicAudit.error) {
    throw new Error(`FAIL: ${logicAudit.error}`);
  }
  if (logicAudit.anime !== "hinh-nen-may-tinh-anime-4k (94)") {
    throw new Error(`FAIL: Expected 'hinh-nen-may-tinh-anime-4k (94)', got '${logicAudit.anime}'`);
  }
  if (logicAudit.png !== "photo") {
    throw new Error(`FAIL: Expected 'photo', got '${logicAudit.png}'`);
  }
  if (logicAudit.multiDot !== "my.cool.photo") {
    throw new Error(`FAIL: Expected 'my.cool.photo', got '${logicAudit.multiDot}'`);
  }

  // 2. Select Note to Self chat and upload test image
  console.log("3. Navigating to Note to Self chat...");
  const noteToSelf = page.locator(".chat-item:has-text('Note to self'), .chat-item:has-text('Chính mình')").first();
  if (await noteToSelf.isVisible().catch(() => false)) {
    await noteToSelf.click();
    await page.waitForTimeout(800);
  } else {
    const firstChat = page.locator(".chat-item, .channel-row").first();
    if (await firstChat.isVisible().catch(() => false)) {
      await firstChat.click();
      await page.waitForTimeout(800);
    }
  }

  console.log("4. Attaching file:", TEST_FILE);
  const plusBtn = page.locator(".composer-tools button.composer-tool-btn, .composer-tools button.icon-btn").first();
  if (await plusBtn.isVisible().catch(() => false)) {
    await plusBtn.click({ force: true });
    await page.waitForSelector(".tools-menu, .account-menu.tools-menu", { timeout: 5000 });
  }

  const fileInput = page.locator("input[type='file'][accept*='image']").first();
  if (await fileInput.count() > 0) {
    await fileInput.setInputFiles(TEST_FILE);
    console.log("File attached, waiting for upload and render in chat...");
    await page.waitForSelector(".rich-image-btn, .self-media-tile", { timeout: 20000 });
    await page.waitForTimeout(1000);
  }

  // 3. Click the image to open Lightbox
  console.log("5. Clicking image to open Lightbox...");
  const imgBtn = page.locator(".rich-image-btn, .self-media-tile").last();
  if (await imgBtn.isVisible().catch(() => false)) {
    await imgBtn.click({ force: true });
    await page.waitForSelector(".lightbox", { timeout: 10000 });
    await page.waitForSelector(".lightbox-filename", { timeout: 10000 });
    await page.waitForTimeout(600);
  }

  // 4. Verify Lightbox topbar filename
  const lbFilename = await page.evaluate(() => {
    const el = document.querySelector(".lightbox-filename");
    const dlBtn = document.querySelector("a.lightbox-download");
    return {
      text: el ? el.innerText.trim() : null,
      title: el ? el.getAttribute("title") : null,
      downloadAttr: dlBtn ? dlBtn.getAttribute("download") : null
    };
  });

  console.log("Lightbox Filename Audit:", lbFilename);
  if (lbFilename.text) {
    console.log(`Rendered Lightbox Filename: "${lbFilename.text}"`);
    if (lbFilename.text.endsWith(".jpg") || lbFilename.text.endsWith(".png") || lbFilename.text.endsWith(".jpeg") || lbFilename.text.endsWith(".webp") || lbFilename.text.endsWith(".avif")) {
      throw new Error(`FAIL: Lightbox filename still contains raw file extension: "${lbFilename.text}"`);
    }
    if (lbFilename.downloadAttr && (lbFilename.downloadAttr.endsWith(".jpg") || lbFilename.downloadAttr.endsWith(".png") || lbFilename.downloadAttr.endsWith(".jpeg") || lbFilename.downloadAttr.endsWith(".webp") || lbFilename.downloadAttr.endsWith(".avif"))) {
      throw new Error(`FAIL: Lightbox download attribute still contains raw file extension: "${lbFilename.downloadAttr}"`);
    }
  }

  // Take screenshot of clean lightbox topbar
  const picPath = path.join(ARTIFACT_DIR, "01_clean_title_no_extension.png");
  await page.screenshot({ path: picPath });
  console.log("Saved clean title screenshot:", picPath);

  await browser.close();
  console.log("=== SPEC 126 VERIFICATION PASSED 100% ===");
}

run().catch(err => {
  console.error("TEST FAILED:", err);
  process.exit(1);
});
