// Live 2-user Chromium E2E verification for Spec 029: Rust str0m SFU + Screen Share Relay
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8085';
const OUT = process.argv[2] || '/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/verify_029';
fs.mkdirSync(OUT, { recursive: true });

const checks = [];
function record(name, pass, detail = '') {
  checks.push({ name, pass, detail });
  console.log(`${pass ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

const browser = await chromium.launch({
  executablePath: '/usr/bin/google-chrome',
  headless: true,
  args: [
    '--no-sandbox',
    '--disable-gpu',
    '--use-fake-ui-for-media-stream',
    '--use-fake-device-for-media-stream',
    '--enable-usermedia-screen-capturing',
    '--auto-select-desktop-capture-source=Entire screen',
  ],
});

const errors = [];
try {
  // Helper to provide an animated real MediaStream fallback ONLY if headless lacks X11 desktop capture
  const installHeadlessScreenFallback = async (context) => {
    await context.addInitScript(() => {
      const orig = navigator.mediaDevices && navigator.mediaDevices.getDisplayMedia
        ? navigator.mediaDevices.getDisplayMedia.bind(navigator.mediaDevices)
        : null;
      if (!navigator.mediaDevices) return;
      navigator.mediaDevices.getDisplayMedia = async (constraints) => {
        if (orig) {
          try {
            const s = await orig(constraints);
            if (s && s.getVideoTracks().length > 0) return s;
          } catch (err) {
            console.warn('[E2E] native headless getDisplayMedia failed, using animated canvas MediaStream for real WebRTC->SFU transport:', err);
          }
        }
        const canvas = document.createElement('canvas');
        canvas.width = 960;
        canvas.height = 540;
        const ctx = canvas.getContext('2d');
        let frame = 0;
        const draw = () => {
          frame++;
          ctx.fillStyle = '#0F172A';
          ctx.fillRect(0, 0, canvas.width, canvas.height);
          ctx.fillStyle = '#2563EB';
          ctx.fillRect(40, 40, 880, 72);
          ctx.fillStyle = '#FFFFFF';
          ctx.font = 'bold 28px sans-serif';
          ctx.fillText(`KIYO SFU LIVE SCREEN SHARE #${frame}`, 64, 86);
          ctx.fillStyle = '#10B981';
          ctx.fillRect(40 + ((frame * 12) % 700), 180, 180, 140);
          ctx.fillStyle = '#F59E0B';
          ctx.fillRect(40, 360, 880, 120);
          ctx.fillStyle = '#111827';
          ctx.font = '22px monospace';
          ctx.fillText('Realtime VP8 -> Rust str0m SFU -> Remote Browser', 64, 428);
        };
        draw();
        const interval = setInterval(draw, 80);
        const stream = canvas.captureStream(15);
        const track = stream.getVideoTracks()[0];
        const origStop = track.stop.bind(track);
        track.stop = () => {
          clearInterval(interval);
          origStop();
        };
        return stream;
      };
    });
  };

  const ctxA = await browser.newContext({ viewport: { width: 1366, height: 860 }, permissions: ['camera', 'microphone'] });
  const ctxB = await browser.newContext({ viewport: { width: 1366, height: 860 }, permissions: ['camera', 'microphone'] });
  await installHeadlessScreenFallback(ctxA);
  await installHeadlessScreenFallback(ctxB);

  const pageA = await ctxA.newPage();
  const pageB = await ctxB.newPage();
  pageA.on('pageerror', (e) => errors.push(`A: ${e.message}`));
  pageB.on('pageerror', (e) => errors.push(`B: ${e.message}`));

  const suffix = `${Date.now().toString(36).slice(-5)}${Math.floor(Math.random() * 90 + 10)}`;
  const userA = `host_${suffix}`;
  const userB = `guest_${suffix}`;

  // Login A
  await pageA.goto(`${BASE}/`);
  await pageA.waitForSelector('.auth-card', { timeout: 30000 });
  await pageA.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(userA);
  await pageA.locator('.auth-dev-form button[type="submit"]').click();
  await pageA.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await pageA.waitForTimeout(600);
  const skipA = pageA.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skipA.isVisible().catch(() => false)) await skipA.click();

  // Login B
  await pageB.goto(`${BASE}/`);
  await pageB.waitForSelector('.auth-card', { timeout: 30000 });
  await pageB.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(userB);
  await pageB.locator('.auth-dev-form button[type="submit"]').click();
  await pageB.waitForSelector('.workspace-shell, .app-shell', { timeout: 30000 });
  await pageB.waitForTimeout(600);
  const skipB = pageB.locator('button[aria-label="Bỏ qua giới thiệu"]');
  if (await skipB.isVisible().catch(() => false)) await skipB.click();

  // User A opens MeetView & starts meeting immediately
  await pageA.locator('.app-rail button[aria-label="Họp nhóm"]').click();
  await pageA.waitForSelector('.meet-hero-card', { timeout: 10000 });
  await pageA.locator('input[aria-label="Tên cuộc họp"]').fill(`SFU Meet ${suffix}`);
  await pageA.locator('button', { hasText: 'Bắt đầu họp ngay' }).click();
  await pageA.waitForSelector('.active-call-overlay', { timeout: 15000 });

  const meetCodeText = await pageA.locator('.call-meet-code').textContent();
  const meetCode = (meetCodeText || '').replace(/^Mã:\s*/i, '').trim();
  record('User A created and joined Meet room', meetCode.length > 0, `code=${meetCode}`);

  // User B joins same meeting room
  await pageB.locator('.app-rail button[aria-label="Họp nhóm"]').click();
  await pageB.waitForSelector('.meet-hero-card', { timeout: 10000 });
  await pageB.locator('input[placeholder*="abc-defg-hij"]').fill(meetCode);
  await pageB.locator('button', { hasText: 'Tham gia ngay' }).click();
  await pageB.waitForSelector('.active-call-overlay', { timeout: 15000 });

  // Wait for BOTH A and B to reach ICE connected to Rust str0m SFU
  const waitIce = async (page) => {
    for (let i = 0; i < 40; i++) {
      const st = await page.evaluate(() => window.__chatwWebRTC ? window.__chatwWebRTC.iceState : 'none');
      if (st === 'connected' || st === 'completed') return st;
      await page.waitForTimeout(250);
    }
    return await page.evaluate(() => window.__chatwWebRTC ? window.__chatwWebRTC.iceState : 'none');
  };
  const [iceA, iceB] = await Promise.all([waitIce(pageA), waitIce(pageB)]);
  record('Both clients connected via DTLS-SRTP/UDP to Rust str0m SFU', (iceA === 'connected' || iceA === 'completed') && (iceB === 'connected' || iceB === 'completed'), `iceA=${iceA}, iceB=${iceB}`);

  // User A shares screen
  await pageA.locator('button[aria-label="Chia sẻ màn hình"]').click();
  await pageA.waitForSelector('#webrtc-local-screen-video', { timeout: 10000 });

  // Wait for User A local preview AND User B remote SFU screen video to have real frames (>0x0)
  let statsA = null;
  let statsB = null;
  for (let i = 0; i < 40; i++) {
    statsA = await pageA.evaluate(async () => {
      const v = document.getElementById('webrtc-local-screen-video');
      let outBytes = 0;
      if (window.__chatwWebRTC && window.__chatwWebRTC.pc) {
        const rep = await window.__chatwWebRTC.pc.getStats();
        rep.forEach((r) => {
          if (r.type === 'outbound-rtp' && r.kind === 'video') outBytes += (r.bytesSent || 0);
        });
      }
      return v ? { w: v.videoWidth, h: v.videoHeight, ready: v.readyState, paused: v.paused, outBytes } : null;
    });
    statsB = await pageB.evaluate(async () => {
      const v = document.getElementById('webrtc-remote-screen-video');
      let inBytes = 0;
      let framesDecoded = 0;
      if (window.__chatwWebRTC && window.__chatwWebRTC.pc) {
        const rep = await window.__chatwWebRTC.pc.getStats();
        rep.forEach((r) => {
          if (r.type === 'inbound-rtp' && r.kind === 'video') {
            inBytes += (r.bytesReceived || 0);
            framesDecoded += (r.framesDecoded || 0);
          }
        });
      }
      let nonBlack = false;
      if (v && v.videoWidth > 0 && v.videoHeight > 0) {
        const c = document.createElement('canvas');
        c.width = 64;
        c.height = 36;
        const ctx = c.getContext('2d');
        ctx.drawImage(v, 0, 0, 64, 36);
        const px = ctx.getImageData(0, 0, 64, 36).data;
        let bright = 0;
        for (let k = 0; k < px.length; k += 4) {
          if (px[k] + px[k + 1] + px[k + 2] > 80) bright++;
        }
        nonBlack = bright > 50;
      }
      return {
        hasEl: Boolean(v),
        w: v ? v.videoWidth : 0,
        h: v ? v.videoHeight : 0,
        ready: v ? v.readyState : 0,
        paused: v ? v.paused : true,
        inBytes,
        framesDecoded,
        nonBlack,
      };
    });
    if (statsA && statsA.w > 0 && statsB.hasEl && statsB.w > 0 && statsB.framesDecoded > 0 && statsB.nonBlack) {
      break;
    }
    await pageB.waitForTimeout(250);
  }

  record(
    'User A local screen share stage renders live video & sends RTP up to SFU',
    Boolean(statsA && statsA.w > 0 && statsA.h > 0 && statsA.outBytes > 0),
    JSON.stringify(statsA)
  );
  record(
    'User B receives live decoded screen-share video frames from Rust SFU server',
    Boolean(statsB && statsB.hasEl && statsB.w > 0 && statsB.h > 0 && statsB.inBytes > 0 && statsB.framesDecoded > 0 && statsB.nonBlack),
    JSON.stringify(statsB)
  );

  await pageA.screenshot({ path: path.join(OUT, '01_userA_presenting.png') });
  await pageB.screenshot({ path: path.join(OUT, '02_userB_receiving.png') });

  // User A stops screen share -> User B presentation stage disappears cleanly
  await pageA.locator('button[aria-label="Dừng chia sẻ màn hình"]').click();
  let stoppedB = false;
  for (let i = 0; i < 25; i++) {
    const visibleB = await pageB.locator('#webrtc-remote-screen-video').isVisible().catch(() => false);
    const visibleA = await pageA.locator('#webrtc-local-screen-video').isVisible().catch(() => false);
    if (!visibleB && !visibleA) {
      stoppedB = true;
      break;
    }
    await pageB.waitForTimeout(200);
  }
  record('Stopping screen share removes stage on both presenter & remote viewer', stoppedB);
  await pageB.screenshot({ path: path.join(OUT, '03_userB_after_stop.png') });

  record('Zero runtime pageerror events', errors.length === 0, errors.join('; '));
} finally {
  await browser.close();
}

const passed = checks.filter((c) => c.pass).length;
console.log(`\nSummary: ${passed}/${checks.length} passed`);
if (passed !== checks.length) process.exit(1);
