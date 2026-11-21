// Live Chromium verification for Spec 042:
// - Inverted & intuitive slider: "Độ trong suốt khung trắng" (0% = solid, 100% = fully transparent)
// - No blur by default: "Làm mờ kính khung (Frosted glass)" switch defaults to OFF
// - Turning ON the switch reveals blur slider and sets visible transparency if currently 0%
// - Adjusting blur slider immediately live-updates --panel-blur
// - Reset restores solid white panels and turns OFF glass blur
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_042';
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
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`s042_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(700);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // Open Settings > Giao diện
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  await page.waitForTimeout(300);

  // Scroll to wallpaper & panel settings
  await page.evaluate(() => {
    const el = document.querySelector('.settings-body');
    if (el) el.scrollTop = el.scrollHeight;
  });
  await page.waitForTimeout(200);

  // 1. Verify initial default state:
  // - Label: "Độ trong suốt khung trắng: 0%"
  // - Glass blur switch is unchecked (false)
  // - Blur slider is NOT visible
  // - Root attributes: data-panel-translucent="false", data-panel-glass="false", --panel-blur="0px"
  const bodyText = await page.locator('.settings-body').innerText();
  record(
    'Initial transparency slider displays "Độ trong suốt khung trắng: 0%"',
    bodyText.includes('Độ trong suốt khung trắng: 0%'),
    `hasLabel=${bodyText.includes('Độ trong suốt khung trắng: 0%')}`,
  );

  const glassSwitch = page.locator('.toggle-row', { hasText: 'Làm mờ kính khung (Frosted glass)' }).locator('input.switch');
  const isCheckedInitial = await glassSwitch.isChecked();
  record(
    'Glass blur switch defaults to OFF (no blur by default)',
    isCheckedInitial === false,
    `isChecked=${isCheckedInitial}`,
  );

  const blurSliderVisible = await page.locator('input[aria-label="Mức độ làm mờ kính"]').isVisible().catch(() => false);
  record(
    'Blur slider is hidden when glass blur switch is OFF',
    blurSliderVisible === false,
    `visible=${blurSliderVisible}`,
  );

  const initialRoot = await page.evaluate(() => {
    const r = document.documentElement;
    return {
      glassAttr: r.getAttribute('data-panel-glass'),
      translucentAttr: r.getAttribute('data-panel-translucent'),
      panelBlur: r.style.getPropertyValue('--panel-blur'),
      panelOpacity: r.style.getPropertyValue('--panel-opacity'),
    };
  });
  record(
    'DOM attributes and vars reflect zero blur by default',
    initialRoot.glassAttr === 'false' && initialRoot.panelBlur === '0px',
    JSON.stringify(initialRoot),
  );

  await page.screenshot({ path: path.join(OUT, '01_default_state.png'), fullPage: true });

  // 2. Test transparency slider: set transparency to 50%
  const transSlider = page.locator('input[aria-label="Độ trong suốt khung trắng"]');
  await transSlider.fill('50');
  await transSlider.dispatchEvent('input');
  await page.waitForTimeout(300);

  const stateAfter50 = await page.evaluate(() => {
    const r = document.documentElement;
    return {
      glassAttr: r.getAttribute('data-panel-glass'),
      translucentAttr: r.getAttribute('data-panel-translucent'),
      panelBlur: r.style.getPropertyValue('--panel-blur'),
      panelOpacity: r.style.getPropertyValue('--panel-opacity'),
    };
  });
  record(
    'Moving transparency to 50% sets panel-opacity to 50% and translucent="true", but glass="false" (no blur)',
    stateAfter50.panelOpacity === '50%' && stateAfter50.translucentAttr === 'true' && stateAfter50.glassAttr === 'false' && stateAfter50.panelBlur === '0px',
    JSON.stringify(stateAfter50),
  );

  await page.screenshot({ path: path.join(OUT, '02_transparency_50_pct_no_blur.png'), fullPage: true });

  // 3. Turn ON glass blur switch
  await glassSwitch.click();
  await page.waitForTimeout(300);

  const isCheckedAfterClick = await glassSwitch.isChecked();
  const blurSliderNowVisible = await page.locator('input[aria-label="Mức độ làm mờ kính"]').isVisible();
  record(
    'Ticking glass blur switch turns it ON and reveals blur slider',
    isCheckedAfterClick === true && blurSliderNowVisible === true,
    `checked=${isCheckedAfterClick}, sliderVisible=${blurSliderNowVisible}`,
  );

  const stateWithGlass = await page.evaluate(() => {
    const r = document.documentElement;
    return {
      glassAttr: r.getAttribute('data-panel-glass'),
      panelBlur: r.style.getPropertyValue('--panel-blur'),
    };
  });
  record(
    'Enabling glass blur activates data-panel-glass="true" and positive --panel-blur',
    stateWithGlass.glassAttr === 'true' && stateWithGlass.panelBlur !== '0px',
    JSON.stringify(stateWithGlass),
  );

  await page.screenshot({ path: path.join(OUT, '03_glass_blur_enabled.png'), fullPage: true });

  // 4. Test live blur slider adjustment: change blur to 28px
  const blurSlider = page.locator('input[aria-label="Mức độ làm mờ kính"]');
  await blurSlider.fill('28');
  await blurSlider.dispatchEvent('input');
  await page.waitForTimeout(300);

  const stateBlur28 = await page.evaluate(() => {
    return document.documentElement.style.getPropertyValue('--panel-blur');
  });
  record(
    'Adjusting blur slider to 28px immediately live-updates --panel-blur to 28px',
    stateBlur28 === '28px',
    `--panel-blur=${stateBlur28}`,
  );

  await page.screenshot({ path: path.join(OUT, '04_blur_slider_28px.png'), fullPage: true });

  // 5. Test "Khôi phục giao diện mặc định"
  await page.locator('button', { hasText: 'Khôi phục giao diện mặc định' }).click();
  await page.waitForTimeout(400);

  const stateReset = await page.evaluate(() => {
    const r = document.documentElement;
    return {
      glassAttr: r.getAttribute('data-panel-glass'),
      translucentAttr: r.getAttribute('data-panel-translucent'),
      panelBlur: r.style.getPropertyValue('--panel-blur'),
      panelOpacity: r.style.getPropertyValue('--panel-opacity'),
    };
  });
  const glassSwitchReset = await glassSwitch.isChecked();
  record(
    'Reset restores default solid white panels and turns OFF glass blur',
    stateReset.glassAttr === 'false' && stateReset.panelBlur === '0px' && glassSwitchReset === false,
    JSON.stringify({ ...stateReset, switchChecked: glassSwitchReset }),
  );

  await page.screenshot({ path: path.join(OUT, '05_reset_restored.png'), fullPage: true });

  record('No unexpected runtime page errors', errors.length === 0, errors.join('; '));

} catch (err) {
  console.error('Fatal test error:', err);
  process.exitCode = 1;
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 042 Summary: ${checks.length - failed.length}/${checks.length} checks passed.`);
if (failed.length > 0) {
  process.exitCode = 1;
}
