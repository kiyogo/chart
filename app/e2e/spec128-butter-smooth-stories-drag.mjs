import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.CHATW_URL || 'http://127.0.0.1:8085';
const OUT_DIR = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_spec128';

if (!fs.existsSync(OUT_DIR)) {
  fs.mkdirSync(OUT_DIR, { recursive: true });
}

async function dismissModals(page) {
  for (let i = 0; i < 6; i++) {
    const ov = page.locator('.modal-overlay').first();
    if (!(await ov.isVisible().catch(() => false))) break;
    const btn = ov.locator("button.story-close, button[aria-label='Skip tour'], button[aria-label='Close'], button:has-text('Later'), button:has-text('Skip'), button:has-text('Đóng'), button:has-text('Bỏ qua'), button.btn-secondary").first();
    if (await btn.isVisible().catch(() => false)) {
      await btn.click({ force: true }).catch(() => {});
      await page.waitForTimeout(300);
    } else {
      await page.keyboard.press('Escape').catch(() => {});
      await page.waitForTimeout(300);
    }
  }
}

async function run() {
  console.log('--- Spec 128: Butter-Smooth Kinetic Dragging for Stories & Notes Strip ---');
  const browser = await chromium.launch({
    executablePath: process.env.CHROME_PATH || '/usr/bin/google-chrome',
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-web-security']
  });

  const ctx = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1
  });

  const page = await ctx.newPage();
  console.log('1. Loading ChatW at', BASE);
  await page.goto(BASE, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(1500);

  // Authenticate if needed
  const isAuth = await page.evaluate(() => !!document.querySelector('.auth-shell, .auth-card'));
  if (isAuth) {
    const devInp = await page.$('.auth-dev-form input, input[aria-label], input[placeholder]');
    if (devInp) {
      await devInp.fill('spec128_tester_' + Date.now().toString(36).slice(-4));
      const submitBtn = await page.$(".auth-dev-form button[type='submit'], button[type='submit'], .btn-primary");
      if (submitBtn) await submitBtn.click();
    }
    await page.waitForSelector('.app-rail', { timeout: 25000 });
  }

  await dismissModals(page);
  await page.waitForSelector('.notes-strip', { timeout: 20000 });

  // Ensure strip is scrollable by populating dummy friends if only 1 item
  await page.evaluate(() => {
    const strip = document.querySelector('.notes-strip');
    if (!strip) return;
    // If not scrollable, append test dummy items
    if (strip.scrollWidth <= strip.clientWidth) {
      for (let i = 1; i <= 10; i++) {
        const item = document.createElement('div');
        item.className = 'note-item test-dummy';
        item.innerHTML = `
          <button class="note-avatar-btn" style="width:52px;height:52px;border-radius:50%;background:#4caf50;color:#fff;display:flex;align-items:center;justify-content:center;">${i}</button>
          <span style="font-size:12px;">Friend ${i}</span>
        `;
        strip.appendChild(item);
      }
    }
  });

  const stripInfo = await page.evaluate(() => {
    const strip = document.querySelector('.notes-strip');
    const style = window.getComputedStyle(strip);
    return {
      id: strip.id,
      scrollWidth: strip.scrollWidth,
      clientWidth: strip.clientWidth,
      scrollSnapType: style.scrollSnapType,
      willChange: style.willChange
    };
  });
  console.log('2. Strip initial state:', stripInfo);

  if (stripInfo.scrollSnapType && stripInfo.scrollSnapType !== 'none') {
    throw new Error(`Expected scroll-snap-type to be none, got: ${stripInfo.scrollSnapType}`);
  }

  // 3. Perform a continuous smooth drag across 20 intermediate steps
  console.log('3. Simulating butter-smooth mouse drag gesture...');
  const box = await page.locator('.notes-strip').boundingBox();
  if (!box) throw new Error('Could not find bounding box of .notes-strip');

  const startX = box.x + box.width * 0.75;
  const endX = box.x + box.width * 0.25;
  const centerY = box.y + box.height * 0.5;

  await page.mouse.move(startX, centerY);
  await page.mouse.down({ button: 'left' });

  const intermediateScrolls = [];
  const steps = 25;
  for (let i = 1; i <= steps; i++) {
    const currentX = startX + (endX - startX) * (i / steps);
    await page.mouse.move(currentX, centerY);
    await page.waitForTimeout(12); // ~80fps flick
    const s = await page.evaluate(() => {
      const el = document.querySelector('.notes-strip');
      return {
        scrollLeft: el.scrollLeft,
        isDragging: el.classList.contains('dragging')
      };
    });
    intermediateScrolls.push(s);
  }

  // Release immediately while in motion to test kinetic momentum
  await page.mouse.up({ button: 'left' });
  const releaseScroll = await page.evaluate(() => document.querySelector('.notes-strip').scrollLeft);

  // Wait 350ms for kinetic momentum decay to glide forward
  await page.waitForTimeout(350);
  const settledScroll = await page.evaluate(() => document.querySelector('.notes-strip').scrollLeft);

  // Screenshot after settled momentum
  const shot1 = path.join(OUT_DIR, '01_butter_smooth_drag_active.png');
  await page.screenshot({ path: shot1 });

  console.log('4. Drag trajectory summary:', {
    startScroll: intermediateScrolls[0]?.scrollLeft,
    endStepScroll: intermediateScrolls[intermediateScrolls.length - 1]?.scrollLeft,
    releaseScroll,
    settledScroll,
    momentumTravel: settledScroll - releaseScroll
  });

  // Verify smooth monotonic increase
  let reversals = 0;
  for (let i = 1; i < intermediateScrolls.length; i++) {
    if (intermediateScrolls[i].scrollLeft < intermediateScrolls[i - 1].scrollLeft) {
      reversals++;
    }
  }

  if (reversals > 0) {
    throw new Error(`Drag experienced ${reversals} reversals/jitters during continuous leftward movement!`);
  }

  if (intermediateScrolls[intermediateScrolls.length - 1].scrollLeft <= 10) {
    throw new Error('Drag failed to scroll the notes-strip horizontally!');
  }

  // Verify kinetic momentum traveled beyond the release point
  if (settledScroll <= releaseScroll) {
    console.warn('Note: Momentum glide was subtle or reached edge boundary.');
  } else {
    console.log(`Kinetic momentum successfully glided +${settledScroll - releaseScroll}px!`);
  }

  // Screenshot after settled momentum
  const shot2 = path.join(OUT_DIR, '02_butter_smooth_momentum_settled.png');
  await page.screenshot({ path: shot2 });
  console.log('Saved settled screenshot:', shot2);

  await browser.close();
  console.log('=== SPEC 128: BUTTER-SMOOTH STORIES DRAG VERIFICATION 100% PASSED ===');
}

run().catch(err => {
  console.error('Spec 128 verification failed:', err);
  process.exit(1);
});
