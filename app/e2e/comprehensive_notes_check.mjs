import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_notes_full';
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
  page.on('console', (msg) => {
    if (msg.type() === 'error' && !msg.text().includes('401 (Unauthorized)')) errors.push(msg.text());
  });

  const suffix = `${Date.now().toString(36).slice(-5)}`;
  await page.goto(`${BASE}/`);
  await page.waitForSelector('.auth-card', { timeout: 30000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(`note_${suffix}`);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await page.waitForTimeout(800);
  const skip = page.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skip.isVisible().catch(() => false)) await skip.click();

  // 1. Verify Notes Strip and thought bubble
  await page.waitForSelector('.notes-strip', { timeout: 10000 });
  const noteTile = page.locator('.notes-strip .note-item').first();
  const bubble = noteTile.locator('.note-bubble');
  const initialBubbleText = (await bubble.textContent()).trim();
  record('Initial bubble text contains "Chia sẻ" and "suy nghĩ"', initialBubbleText.includes('Chia sẻ') && initialBubbleText.includes('suy nghĩ'), initialBubbleText);
  await page.screenshot({ path: path.join(OUT, '01_notes_strip_initial.png') });

  // 2. Click the bubble with real mouse click
  await bubble.click();
  await page.waitForTimeout(400);
  const modal = page.locator('.note-modal');
  const modalVisible = await modal.isVisible();
  record('Clicking thought bubble opens modal directly in "Chia sẻ suy nghĩ" tab', modalVisible && (await page.locator('.note-modal input[aria-label="Nội dung ghi chú"]').isVisible()));
  await page.screenshot({ path: path.join(OUT, '02_modal_opened.png') });

  // 3. Check tabs in modal
  const thoughtTab = page.locator('.note-modal button:has-text("Chia sẻ suy nghĩ")');
  const storyTab = page.locator('.note-modal button:has-text("Tạo tin ảnh")');
  record('Modal contains "Chia sẻ suy nghĩ" and "Tạo tin ảnh" tabs', (await thoughtTab.isVisible()) && (await storyTab.isVisible()));

  // 4. Fill thought note and save
  await page.locator('.note-emoji-chip:has-text("☕")').click();
  await page.locator('input[aria-label="Nội dung ghi chú"]').fill('Cà phê sáng cùng đồng nghiệp');
  await page.screenshot({ path: path.join(OUT, '03_modal_filled.png') });

  const shareBtn = page.getByRole('button', { name: 'Chia sẻ', exact: true });
  await shareBtn.click();
  await page.waitForTimeout(600);

  const modalClosedAfterSave = !(await modal.isVisible());
  record('Modal closes after sharing thought note', modalClosedAfterSave);

  const updatedBubbleText = (await bubble.textContent()).trim();
  record('Bubble updates immediately in real-time', updatedBubbleText.includes('☕') && updatedBubbleText.includes('Cà phê sáng'), updatedBubbleText);
  await page.screenshot({ path: path.join(OUT, '04_bubble_updated.png') });

  // 5. Reload page to verify backend persistence
  await page.reload();
  await page.waitForSelector('.notes-strip', { timeout: 10000 });
  const reloadedBubbleText = (await page.locator('.notes-strip .note-bubble').first().textContent()).trim();
  record('Thought note persists after page reload', reloadedBubbleText.includes('☕') && reloadedBubbleText.includes('Cà phê sáng'), reloadedBubbleText);
  await page.screenshot({ path: path.join(OUT, '05_bubble_persisted.png') });

  // 6. Click bubble again to verify edit & delete
  await page.locator('.notes-strip .note-bubble').first().click();
  await page.waitForTimeout(400);
  const deleteBtn = page.locator('.note-modal button:has-text("Xóa tin")');
  record('"Xóa tin" button visible when note exists', await deleteBtn.isVisible());

  // Click delete
  await deleteBtn.click();
  await page.waitForTimeout(600);
  const clearedBubbleText = (await page.locator('.notes-strip .note-bubble').first().textContent()).trim();
  record('Bubble reverts back to placeholder after deleting', clearedBubbleText.includes('Chia sẻ') && clearedBubbleText.includes('suy nghĩ'), clearedBubbleText);
  await page.screenshot({ path: path.join(OUT, '06_bubble_cleared.png') });

  // 7. Test "Tạo tin" avatar click directly opening "Tạo tin ảnh" tab & posting + viewing a story
  await page.locator('.notes-strip .note-avatar-btn').first().click();
  await page.waitForTimeout(400);
  const uploadLabel = page.locator('label[for="story-media-upload"]');
  const captionInput = page.locator('input[aria-label="Chú thích tin"]');
  const postStoryBtn = page.locator('.note-modal button:has-text("Đăng tin")');
  record('Clicking avatar / "Tạo tin" opens modal directly in "Tạo tin ảnh" tab', (await uploadLabel.isVisible()) && (await captionInput.isVisible()) && (await postStoryBtn.isVisible()));
  await captionInput.fill('Tin buổi chiều mát mẻ');
  await page.screenshot({ path: path.join(OUT, '07_story_tab.png') });
  await postStoryBtn.click();
  await page.waitForTimeout(700);
  const hasStoryRing = await page.locator('.notes-strip .my-note-tile .note-avatar-wrap.has-story-ring').isVisible();
  record('Avatar displays Story Ring immediately after posting a story', hasStoryRing);

  // Open story modal & click "Xem tin của bạn"
  await page.locator('.notes-strip .note-avatar-btn').first().click();
  await page.waitForTimeout(400);
  const viewMineBtn = page.locator('.note-modal button:has-text("Xem tin của bạn")');
  record('"Xem tin của bạn" button appears when user has active story', await viewMineBtn.isVisible());
  await viewMineBtn.click();
  await page.waitForSelector('.story-viewer-modal', { timeout: 5000 });
  await page.waitForTimeout(300);
  const viewerText = await page.locator('.story-viewer-modal').innerText();
  record('Story Viewer Modal renders active story caption', viewerText.includes('Tin buổi chiều mát mẻ'), viewerText);
  await page.screenshot({ path: path.join(OUT, '07b_story_viewer.png') });
  await page.locator('.story-viewer-modal button:has-text("Đóng")').click();
  await page.waitForTimeout(300);

  // 8. Verify Toggle switch equidistant centering across app
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav button', { hasText: 'Quyền riêng tư' }).click();
  await page.waitForSelector('input[type="checkbox"].switch', { timeout: 10000 });
  await page.waitForTimeout(300);

  const switchMetrics = await page.evaluate(() => {
    const sw = document.querySelector('input[type="checkbox"].switch');
    if (!sw) return null;
    const r = sw.getBoundingClientRect();
    const cs = getComputedStyle(sw);
    const after = getComputedStyle(sw, '::after');
    return {
      w: Math.round(r.width),
      h: Math.round(r.height),
      knobW: parseInt(after.width, 10),
      knobH: parseInt(after.height, 10),
      top: parseInt(after.top, 10),
      left: parseInt(after.left, 10),
    };
  });
  console.log('Switch metrics:', switchMetrics);
  if (switchMetrics) {
    const isEquidistant = switchMetrics.w === 42 && switchMetrics.h === 24 && switchMetrics.knobW === 20 && switchMetrics.knobH === 20 && switchMetrics.top === 2 && switchMetrics.left === 2;
    record('Toggle switch thumb is 100% equidistant (2px on all sides)', isEquidistant, JSON.stringify(switchMetrics));
  }
  await page.screenshot({ path: path.join(OUT, '08_toggle_switches.png') });

  // 9. Verify Scrollbar dimensions
  const scrollbarStats = await page.evaluate(() => {
    const overlay = document.getElementById('canvas-scrollbar-overlay');
    return {
      exists: !!overlay,
      tagName: overlay ? overlay.tagName : null,
    };
  });
  record('Canvas scrollbar overlay active', scrollbarStats.exists);

  fs.writeFileSync(path.join(OUT, 'report.json'), JSON.stringify({ checks, errors }, null, 2));

} catch (err) {
  console.error('Test error:', err);
  errors.push(err.message);
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSummary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0 || errors.length > 0) process.exit(1);
