import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));
const { execSync } = require('node:child_process');

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_two_users';
fs.mkdirSync(OUT, { recursive: true });

const browser = await chromium.launch({
  executablePath: '/usr/bin/google-chrome',
  headless: true,
  args: ['--no-sandbox', '--disable-gpu'],
});

try {
  const ctxA = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const pageA = await ctxA.newPage();

  const ctxB = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const pageB = await ctxB.newPage();

  const suffix = `${Date.now().toString(36).slice(-4)}`;
  const userA = `u1_${suffix}`;
  const userB = `u2_${suffix}`;

  // Login user A
  await pageA.goto(`${BASE}/`);
  await pageA.waitForSelector('.auth-card');
  await pageA.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(userA);
  await pageA.locator('.auth-dev-form button[type="submit"]').click();
  await pageA.waitForSelector('.workspace-shell, .app-shell');
  await pageA.waitForTimeout(500);
  const skipA = pageA.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skipA.isVisible().catch(() => false)) await skipA.click();

  // Login user B
  await pageB.goto(`${BASE}/`);
  await pageB.waitForSelector('.auth-card');
  await pageB.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(userB);
  await pageB.locator('.auth-dev-form button[type="submit"]').click();
  await pageB.waitForSelector('.workspace-shell, .app-shell');
  await pageB.waitForTimeout(500);
  const skipB = pageB.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skipB.isVisible().catch(() => false)) await skipB.click();

  // Create mutual friendship in PostgreSQL directly:
  const sql = `
    INSERT INTO friendships (user_a, user_b)
    SELECT LEAST(u1.id, u2.id), GREATEST(u1.id, u2.id)
    FROM users u1, users u2
    WHERE u1.username = '${userA}' AND u2.username = '${userB}'
    ON CONFLICT DO NOTHING;
  `;
  execSync(`psql -h /tmp/kiyo-pg-run -U chatw -d chatw -c "${sql}"`);
  console.log('Friendship created in DB between', userA, 'and', userB);

  // User A sets a thought note
  await pageA.waitForSelector('.notes-strip', { timeout: 10000 });
  const bubbleA = pageA.locator('.notes-strip .note-item').first().locator('.note-bubble');
  await bubbleA.click();
  await pageA.waitForTimeout(400);
  await pageA.locator('.note-emoji-chip:has-text("🚀")').click();
  await pageA.locator('input[aria-label="Nội dung ghi chú"]').fill('Dự án đã sẵn sàng!');
  await pageA.getByRole('button', { name: 'Chia sẻ', exact: true }).click();
  await pageA.waitForTimeout(600);

  // User B reloads page to fetch friend list and friend status
  await pageB.reload();
  await pageB.waitForSelector('.notes-strip', { timeout: 10000 });
  await pageB.waitForTimeout(500);
  await pageB.screenshot({ path: path.join(OUT, 'userB_sees_userA_tile.png') });

  // Locate User A's tile in User B's notes strip
  const friendTileInB = pageB.locator(`.notes-strip .note-item:has-text("${userA}")`).first();
  const friendBubbleInB = friendTileInB.locator('.note-bubble');
  const friendBubbleText = (await friendBubbleInB.textContent()).trim();
  console.log('Friend bubble text seen by User B:', friendBubbleText);

  // Click on friend's bubble
  await friendBubbleInB.click();
  await pageB.waitForTimeout(400);
  await pageB.screenshot({ path: path.join(OUT, 'userB_friend_modal.png') });

  const fullNoteInModal = (await pageB.locator('.note-view-card').textContent()).trim();
  console.log('Full note in modal:', fullNoteInModal);

  const replyInput = pageB.locator('input[placeholder*="Trả lời ghi chú"]');
  if (await replyInput.isVisible()) {
    await replyInput.fill('Tuyệt vời!');
    await pageB.locator('.note-modal .btn-primary').first().click();
    await pageB.waitForTimeout(600);
    await pageB.screenshot({ path: path.join(OUT, 'userB_chat_opened.png') });

    const composerText = await pageB.locator('.composer-input').inputValue();
    console.log('Composer input text after reply click:', composerText);
  }

  console.log('Two-user sync test completed successfully!');

} catch (err) {
  console.error(err);
} finally {
  await browser.close();
}
