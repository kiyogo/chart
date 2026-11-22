// Automated Live Playwright E2E Verification for Spec 052:
// 1. Chat item exact 8px left/right inset, larger avatar (>=50px), and crisp 1px border on hover/selected for .has-nameplate
// 2. Unified 8px left/right/top insets across Messages sidebar, Settings nav, Settings body, Feed, Meet, etc.
// 3. Large & bold .settings-group-title matching .sidebar-title ("Tin nhắn")
// 4. Left Rail (.app-rail) 100% synchronized with panel transparency & glass blur (Image 10)
// 5. Adaptive Contrast & Image Analysis Engine (data-text-halo, darker ink & multi-layer halo over transparent/busy backgrounds - Image 9)
// 6. Full-bleed <input type="color"> swatch (zero inner padding/border), "Mặc định" button on the LEFT, and framed .btn-ghost buttons (Images 5, 11, 12)
// 7. Zero cloud 3-step guide banner & zero verbose .settings-row-desc descriptions + concise labels (Image 13)
// 8. Official web SVG brand logos + all 33/33 language flags rendered as real SVGs (zero grey text code fallbacks - Image 6 & 13)
// 9. Date separator non-overlapping (position: relative, never sticky stacking) & date format ("20 tháng 11" / "22 tháng 11 năm 2025" - Image 14)
// 10. Self-chat inline device pills to the right of "Chính mình" without "Thiết bị:", floating .chat-header without white bar, top unblurred fade (Image 15)
// 11. Meet view (.meet-column) rounded bottom corners & bottom gap matching .app-rail (never touching bottom of browser viewport)

import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve('web/frontend/node_modules/playwright'));

const BASE_URL = process.env.CHATW_URL || 'http://127.0.0.1:8085';

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
    const btn = ov
      .locator(
        'button[aria-label="Bỏ qua giới thiệu"], button[aria-label="Đóng"], button:has-text("Để sau"), button:has-text("Bỏ qua"), button:has-text("Đóng"), button.btn-secondary'
      )
      .first();
    if (await btn.isVisible().catch(() => false)) {
      await btn.click({ force: true }).catch(() => {});
      await page.waitForTimeout(250);
    } else {
      await page.keyboard.press('Escape').catch(() => {});
      await page.waitForTimeout(250);
    }
  }
}

try {
  const tag = Math.random().toString(36).slice(2, 7);
  const uname = `s52_${tag}`;
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => errors.push(e.message));

  await page.goto(BASE_URL, { waitUntil: 'networkidle' });
  await page.waitForSelector('.auth-card', { timeout: 25000 });
  await page.locator('input[aria-label="Tên tài khoản thử nghiệm"]').fill(uname);
  await page.locator('.auth-dev-form button[type="submit"]').click();
  await page.waitForSelector('.app-rail', { timeout: 25000 });
  await page.waitForTimeout(800);
  await dismissModals(page);

  // 1. Check Chat Item left/right inset (8px), larger avatar (>=50px), and crisp 1px nameplate hover/active border
  const chatItemAudit = await page.evaluate(() => {
    const sb = document.querySelector('.nav-sidebar');
    const item = document.querySelector('.nav-sidebar .chat-item');
    const av = item ? item.querySelector('.self-chat-avatar, .user-avatar') : null;
    if (!sb || !item) return null;
    const sbR = sb.getBoundingClientRect();
    const itR = item.getBoundingClientRect();
    const avR = av ? av.getBoundingClientRect() : null;
    const hdr = sb.querySelector('.sidebar-top-header');
    const hdrR = hdr ? hdr.getBoundingClientRect() : null;
    const title = sb.querySelector('.sidebar-title');
    const titleCS = title ? getComputedStyle(title) : null;

    // Test .has-nameplate hover & selected 1px crisp border
    item.classList.add('has-nameplate', 'np-ocean', 'selected');
    const npSelectedShadow = getComputedStyle(item).boxShadow;
    const npSelectedBg = getComputedStyle(item).backgroundColor;
    item.classList.remove('selected', 'has-nameplate', 'np-ocean');

    return {
      leftGap: Math.round((itR.left - sbR.left) * 10) / 10,
      rightGap: Math.round((sbR.right - itR.right) * 10) / 10,
      topGap: hdrR ? Math.round((hdrR.top - sbR.top) * 10) / 10 : null,
      avatarWidth: avR ? Math.round(avR.width * 10) / 10 : 0,
      sidebarTitleSize: titleCS ? titleCS.fontSize : '',
      sidebarTitleWeight: titleCS ? titleCS.fontWeight : '',
      npSelectedShadow,
      npSelectedBg,
    };
  });
  record(
    'Chat item has symmetrical 8px left/right inset, enlarged avatar (>=50px), and crisp 1px border on .has-nameplate',
    chatItemAudit &&
      Math.abs(chatItemAudit.leftGap - chatItemAudit.rightGap) <= 0.5 &&
      chatItemAudit.leftGap === 8 &&
      chatItemAudit.avatarWidth >= 50 &&
      chatItemAudit.npSelectedShadow.includes('1px') &&
      chatItemAudit.npSelectedBg === 'rgba(0, 0, 0, 0)',
    JSON.stringify(chatItemAudit)
  );

  // 2. Self-chat header: no "Thiết bị:" label, device pills inline to right of "Chính mình", floating .chat-header with top unblurred gradient fade, non-overlapping .date-separator
  await page.locator('.nav-sidebar .chat-item').first().click();
  await page.waitForSelector('.chat-column .chat-header', { timeout: 10000 });
  const composer = page.locator('textarea.composer-input').first();
  await composer.fill('Tin nhắn kiểm thử Spec 052');
  await composer.press('Enter');
  await page.waitForSelector('.date-separator', { timeout: 10000 });

  const selfChatAudit = await page.evaluate(() => {
    const col = document.querySelector('.chat-column');
    const hdr = document.querySelector('.chat-column > .chat-header');
    const msgs = document.querySelector('.chat-column > .chat-messages');
    const hdrCS = getComputedStyle(hdr);
    const hdrBeforeCS = getComputedStyle(hdr, '::before');
    const hdrAfterCS = getComputedStyle(hdr, '::after');
    const idEl = hdr.querySelector('.chat-header-identity');
    const actEl = hdr.querySelector('.chat-header-actions');
    const devStrip = hdr.querySelector('.header-inline-devices');
    const hasThietBiLabel = document.body.innerText.includes('Thiết bị:');
    const dateSep = document.querySelector('.date-separator');
    const dateSepCS = dateSep ? getComputedStyle(dateSep) : null;
    const dateText = dateSep ? dateSep.textContent.trim() : '';
    const colR = col.getBoundingClientRect();
    const msgsR = msgs ? msgs.getBoundingClientRect() : null;
    const idR = idEl ? idEl.getBoundingClientRect() : null;
    const devR = devStrip ? devStrip.getBoundingClientRect() : null;
    const idCS = idEl ? getComputedStyle(idEl) : null;
    const actCS = actEl ? getComputedStyle(actEl) : null;
    return {
      hdrPos: hdrCS.position,
      hdrBg: hdrCS.backgroundColor,
      hdrPointer: hdrCS.pointerEvents,
      beforeDisplay: hdrBeforeCS.display,
      beforeBgImg: hdrBeforeCS.backgroundImage,
      beforeBlur: hdrBeforeCS.backdropFilter || hdrBeforeCS.webkitBackdropFilter || 'none',
      afterDisplay: hdrAfterCS.display,
      msgsStartsAtTop: msgsR ? Math.abs(msgsR.top - colR.top) <= 1 : false,
      idRadius: idCS ? parseFloat(idCS.borderTopLeftRadius) : 0,
      idBg: idCS ? idCS.backgroundColor : '',
      actRadius: actCS ? parseFloat(actCS.borderTopLeftRadius) : 0,
      hasThietBiLabel,
      devicesRightOfTitle: idR && devR ? devR.left >= idR.right + 4 : false,
      datePos: dateSepCS ? dateSepCS.position : '',
      dateText,
    };
  });
  record(
    'Self-chat header floats Avatar+Name & Actions as frosted capsules with progressive top frosted mist mirroring composer, places device pills inline right of title, and fixes date pill format',
    selfChatAudit.hdrPos === 'absolute' &&
      selfChatAudit.hdrBg === 'rgba(0, 0, 0, 0)' &&
      selfChatAudit.hdrPointer === 'none' &&
      selfChatAudit.beforeDisplay === 'block' &&
      selfChatAudit.beforeBgImg.includes('linear-gradient') &&
      selfChatAudit.beforeBlur.includes('blur') &&
      selfChatAudit.afterDisplay === 'none' &&
      selfChatAudit.msgsStartsAtTop &&
      selfChatAudit.idRadius >= 20 &&
      selfChatAudit.idBg !== 'rgba(0, 0, 0, 0)' &&
      selfChatAudit.actRadius >= 20 &&
      !selfChatAudit.hasThietBiLabel &&
      selfChatAudit.devicesRightOfTitle &&
      selfChatAudit.datePos === 'relative' &&
      /^\d{1,2} tháng \d{1,2}$/.test(selfChatAudit.dateText),
    JSON.stringify(selfChatAudit)
  );

  // 2b. Feed View ("Mới"): verify centered .feed-inner layout (max-width <= 740px, centered horizontally)
  await page.locator('.app-rail button[aria-label="Mới"]').click();
  await page.waitForSelector('.feed-column .feed-inner', { timeout: 10000 });
  const feedAudit = await page.evaluate(() => {
    const col = document.querySelector('.feed-column');
    const inner = document.querySelector('.feed-column .feed-inner');
    const colR = col.getBoundingClientRect();
    const inR = inner.getBoundingClientRect();
    const leftMargin = Math.round(inR.left - colR.left);
    const rightMargin = Math.round(colR.right - inR.right);
    return {
      innerWidth: Math.round(inR.width),
      colWidth: Math.round(colR.width),
      leftMargin,
      rightMargin,
      centered: Math.abs(leftMargin - rightMargin) <= 16 && leftMargin >= 40,
    };
  });
  record(
    'Feed view ("Mới") preserves its clean centered layout (not stretched full-width to the left edge)',
    feedAudit.centered,
    JSON.stringify(feedAudit)
  );

  // 3. Meet View (.meet-column): rounded bottom corners and bottom gap matching .app-rail (never touching bottom of viewport)
  await page.locator('.app-rail button[aria-label="Họp nhóm"]').click();
  await page.waitForSelector('.meet-column', { timeout: 10000 });
  const meetAudit = await page.evaluate(() => {
    const rail = document.querySelector('.app-rail');
    const meet = document.querySelector('.meet-column');
    const railR = rail.getBoundingClientRect();
    const meetR = meet.getBoundingClientRect();
    const meetCS = getComputedStyle(meet);
    return {
      railBottomGap: Math.round((window.innerHeight - railR.bottom) * 10) / 10,
      meetBottomGap: Math.round((window.innerHeight - meetR.bottom) * 10) / 10,
      bottomLeftRadius: meetCS.borderBottomLeftRadius,
      bottomRightRadius: meetCS.borderBottomRightRadius,
    };
  });
  record(
    'Meet view (.meet-column) has rounded bottom corners and stands spaced away from bottom of browser identical to .app-rail',
    meetAudit.meetBottomGap >= 7 &&
      Math.abs(meetAudit.meetBottomGap - meetAudit.railBottomGap) <= 1 &&
      parseFloat(meetAudit.bottomLeftRadius) >= 12 &&
      parseFloat(meetAudit.bottomRightRadius) >= 12,
    JSON.stringify(meetAudit)
  );

  // 4. Settings: unified 8px left/right/top insets, large bold .settings-group-title matching "Tin nhắn", full-bleed color inputs, left "Mặc định" button, and .app-rail opacity sync + adaptive halo
  await page.locator('.app-rail button[aria-label="Cài đặt"]').click();
  await page.waitForSelector('.settings-nav', { timeout: 10000 });
  await page.locator('.settings-nav-item', { hasText: 'Giao diện' }).click();
  await page.waitForTimeout(300);

  const appearanceAudit = await page.evaluate(() => {
    const nav = document.querySelector('.settings-nav');
    const body = document.querySelector('.settings-body');
    const navCS = getComputedStyle(nav);
    const bodyCS = getComputedStyle(body);
    const grpTitle = body.querySelector('.settings-group-title');
    const grpCS = grpTitle ? getComputedStyle(grpTitle) : null;
    const colorInput = body.querySelector('input[type="color"]');
    const colorCS = colorInput ? getComputedStyle(colorInput) : null;
    return {
      navPadTop: navCS.paddingTop,
      navPadLeft: navCS.paddingLeft,
      navPadRight: navCS.paddingRight,
      bodyPadTop: bodyCS.paddingTop,
      bodyPadLeft: bodyCS.paddingLeft,
      bodyPadRight: bodyCS.paddingRight,
      grpTitleSize: grpCS ? grpCS.fontSize : '',
      grpTitleWeight: grpCS ? grpCS.fontWeight : '',
      colorPadding: colorCS ? colorCS.paddingTop : '',
      colorAppearance: colorCS ? colorCS.appearance : '',
    };
  });
  record(
    'Settings nav has 8px pill inset, Settings body has balanced breathing room (>=14px top/left so text never touches rounded corners), large bold .settings-group-title (matching "Tin nhắn"), and full-bleed color input',
    appearanceAudit.navPadLeft === '8px' &&
      appearanceAudit.navPadTop === '8px' &&
      parseFloat(appearanceAudit.bodyPadLeft) >= 14 &&
      parseFloat(appearanceAudit.bodyPadTop) >= 13 &&
      appearanceAudit.grpTitleSize === chatItemAudit.sidebarTitleSize &&
      Number(appearanceAudit.grpTitleWeight) >= 700 &&
      appearanceAudit.colorPadding === '0px' &&
      appearanceAudit.colorAppearance === 'none',
    JSON.stringify(appearanceAudit)
  );

  // Trigger custom accent color change and verify "Mặc định" button appears to the LEFT of <input type="color"> and has a button frame
  const colorRowAudit = await page.evaluate(() => {
    const colorInput = document.querySelector('.settings-body input[type="color"]');
    const proto = window.HTMLInputElement.prototype;
    const desc = Object.getOwnPropertyDescriptor(proto, 'value');
    desc.set.call(colorInput, '#E11D48');
    colorInput.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  });
  await page.waitForTimeout(250);
  const macDinhAudit = await page.evaluate(() => {
    const colorInput = document.querySelector('.settings-body input[type="color"]');
    const row = colorInput.closest('.settings-row');
    const resetBtn = Array.from(row.querySelectorAll('button')).find((b) => b.textContent.includes('Mặc định'));
    if (!resetBtn) return null;
    const cR = colorInput.getBoundingClientRect();
    const bR = resetBtn.getBoundingClientRect();
    const bCS = getComputedStyle(resetBtn);
    return {
      isLeftOfColor: bR.right <= cR.left + 2,
      btnBg: bCS.backgroundColor,
      btnRadius: bCS.borderRadius,
    };
  });
  record(
    'When a custom color is set, framed "Mặc định" button appears on the LEFT of the color swatch button',
    macDinhAudit && macDinhAudit.isLeftOfColor && macDinhAudit.btnBg !== 'rgba(0, 0, 0, 0)',
    JSON.stringify(macDinhAudit)
  );

  // Set panel transparency to 25% over Ocean wallpaper -> verify that when panel is ALREADY easy to read (75% opaque), NO text-shadow is applied!
  await page.locator('.wallpaper-tile', { hasText: 'Đại dương' }).last().click();
  await page.evaluate(() => {
    const slider = document.querySelector('input[aria-label="Độ trong suốt khung trắng"]');
    const desc = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
    desc.set.call(slider, '25');
    slider.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await page.waitForTimeout(250);
  const easyReadAudit = await page.evaluate(() => {
    const body = document.querySelector('.settings-body');
    return {
      textHaloAttr: document.documentElement.getAttribute('data-text-halo'),
      bodyTextShadow: getComputedStyle(body).textShadow,
    };
  });

  // Now push panel transparency to 80% over Ocean wallpaper -> verify .app-rail background matches .settings-nav, text darkens to #000, tight dark glow activates on transparent text, while buttons/inputs stay text-shadow: none!
  await page.evaluate(() => {
    const slider = document.querySelector('input[aria-label="Độ trong suốt khung trắng"]');
    const desc = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
    desc.set.call(slider, '80');
    slider.dispatchEvent(new Event('input', { bubbles: true }));
  });
  await page.waitForTimeout(300);

  const railAndHaloAudit = await page.evaluate((easyReadAudit) => {
    const rail = document.querySelector('.app-rail');
    const nav = document.querySelector('.settings-nav');
    const body = document.querySelector('.settings-body');
    const btn = body.querySelector('.btn-secondary');
    const railCS = getComputedStyle(rail);
    const navCS = getComputedStyle(nav);
    const bodyCS = getComputedStyle(body);
    return {
      easyReadNoShadow: easyReadAudit.textHaloAttr === 'false' && easyReadAudit.bodyTextShadow === 'none',
      railBg: railCS.backgroundColor,
      navBg: navCS.backgroundColor,
      textHaloAttr: document.documentElement.getAttribute('data-text-halo'),
      bodyTextShadow: bodyCS.textShadow,
      btnTextShadow: btn ? getComputedStyle(btn).textShadow : 'none',
      textPrimaryColor: bodyCS.color,
    };
  }, easyReadAudit);
  record(
    'Left Rail (.app-rail) opacity syncs 100%, easy-to-read panels & pills/buttons have ZERO text-shadow, and extreme transparency activates deep black ink + tight dark glow',
    railAndHaloAudit.easyReadNoShadow &&
      railAndHaloAudit.railBg === railAndHaloAudit.navBg &&
      railAndHaloAudit.textHaloAttr === 'true' &&
      railAndHaloAudit.textPrimaryColor === 'rgb(0, 0, 0)' &&
      railAndHaloAudit.bodyTextShadow.includes('rgba(0, 0, 0') &&
      !railAndHaloAudit.bodyTextShadow.includes('255, 255, 255') &&
      railAndHaloAudit.btnTextShadow === 'none',
    JSON.stringify(railAndHaloAudit)
  );

  // 4b. Zero Size Shift (0.00px) from Frameless (52%) -> Framed (72%) + Non-Muddy Luminous Active & Secondary Button Colors over Sunset ("Hoàng hôn")
  await page.evaluate(() => {
    const colorInput = document.querySelector('.settings-body input[type="color"]');
    const row = colorInput?.closest('.settings-row');
    const resetBtn = row && Array.from(row.querySelectorAll('button')).find((b) => b.textContent.includes('Mặc định'));
    if (resetBtn) resetBtn.click();
  });
  await page.waitForTimeout(200);
  await page.locator('.wallpaper-tile', { hasText: 'Hoàng hôn' }).last().click();
  await page.waitForTimeout(200);

  const setTransparency = async (pct) => {
    await page.evaluate((val) => {
      const slider = document.querySelector('input[aria-label="Độ trong suốt khung trắng"]');
      const desc = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
      desc.set.call(slider, String(val));
      slider.dispatchEvent(new Event('input', { bubbles: true }));
    }, pct);
    await page.waitForTimeout(260);
  };

  const measureButtons = () =>
    page.evaluate(() => {
      const segs = Array.from(document.querySelectorAll('.settings-body .segmented button')).map((el) => {
        const r = el.getBoundingClientRect();
        return {
          text: el.textContent.trim(),
          active: el.classList.contains('active'),
          w: Math.round(r.width * 100) / 100,
          h: Math.round(r.height * 100) / 100,
          bg: getComputedStyle(el).backgroundColor,
          borderTop: getComputedStyle(el).borderTopWidth,
          shadow: getComputedStyle(el).boxShadow,
        };
      });
      const sec = document.querySelector('.settings-body .btn-secondary');
      const secR = sec ? sec.getBoundingClientRect() : null;
      const navActive = document.querySelector('.settings-nav-item.active');
      return {
        textHalo: document.documentElement.getAttribute('data-text-halo'),
        luminous: document.documentElement.getAttribute('data-luminous-controls'),
        segs,
        sec: sec
          ? {
              w: Math.round(secR.width * 100) / 100,
              h: Math.round(secR.height * 100) / 100,
              bg: getComputedStyle(sec).backgroundColor,
              borderTop: getComputedStyle(sec).borderTopWidth,
              shadow: getComputedStyle(sec).boxShadow,
            }
          : null,
        navActiveBg: navActive ? getComputedStyle(navActive).backgroundColor : '',
        navActiveShadow: navActive ? getComputedStyle(navActive).boxShadow : '',
      };
    });

  await setTransparency(52);
  const m52 = await measureButtons();
  if (process.env.SPEC052_SHOT_DIR) {
    await page.screenshot({ path: path.join(process.env.SPEC052_SHOT_DIR, 'sunset_52.png') });
  }

  await setTransparency(72);
  const m72 = await measureButtons();
  if (process.env.SPEC052_SHOT_DIR) {
    await page.screenshot({ path: path.join(process.env.SPEC052_SHOT_DIR, 'sunset_72.png') });
  }

  const parseRgba = (s) => {
    const m = (s || '').match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\s*\)/i);
    if (!m) return null;
    return { r: +m[1], g: +m[2], b: +m[3], a: m[4] === undefined ? 1 : +m[4] };
  };
  let maxSegDW = 0;
  let maxSegDH = 0;
  for (let i = 0; i < Math.min(m52.segs.length, m72.segs.length); i++) {
    maxSegDW = Math.max(maxSegDW, Math.abs(m72.segs[i].w - m52.segs[i].w));
    maxSegDH = Math.max(maxSegDH, Math.abs(m72.segs[i].h - m52.segs[i].h));
  }
  const secDW = m52.sec && m72.sec ? Math.abs(m72.sec.w - m52.sec.w) : 99;
  const secDH = m52.sec && m72.sec ? Math.abs(m72.sec.h - m52.sec.h) : 99;
  const activeSeg72 = m72.segs.find((s) => s.active);
  const idleSeg72 = m72.segs.find((s) => !s.active);
  const activeIdleHeightDiff = activeSeg72 && idleSeg72 ? Math.abs(activeSeg72.h - idleSeg72.h) : 99;
  const navActiveRgba = parseRgba(m72.navActiveBg);
  const sec52Rgba = parseRgba(m52.sec?.bg);
  const sec72Rgba = parseRgba(m72.sec?.bg);

  record(
    'Frameless (52%) -> Framed (72%) button transition has EXACT 0.00px width/height shift and airy soft luminous active/secondary colors over Sunset wallpaper',
    m52.textHalo === 'false' &&
      m72.textHalo === 'true' &&
      m72.luminous === 'true' &&
      maxSegDW === 0 &&
      maxSegDH === 0 &&
      secDW === 0 &&
      secDH === 0 &&
      activeIdleHeightDiff === 0 &&
      idleSeg72?.borderTop === '0px' &&
      idleSeg72?.shadow.includes('inset') &&
      Boolean(
        navActiveRgba &&
          navActiveRgba.r >= 228 &&
          navActiveRgba.g >= 234 &&
          navActiveRgba.b >= 245 &&
          navActiveRgba.b - navActiveRgba.r >= 15 &&
          navActiveRgba.a >= 0.76
      ) &&
      Boolean(sec52Rgba && sec52Rgba.r === 255 && sec52Rgba.g === 255 && sec52Rgba.b === 255 && sec52Rgba.a >= 0.55) &&
      Boolean(sec72Rgba && sec72Rgba.r === 255 && sec72Rgba.g === 255 && sec72Rgba.b === 255 && sec72Rgba.a >= 0.65),
    JSON.stringify({
      maxSegDW,
      maxSegDH,
      secDW,
      secDH,
      activeIdleHeightDiff,
      navActiveBg: m72.navActiveBg,
      navActiveShadow: m72.navActiveShadow,
      sec52Bg: m52.sec?.bg,
      sec72Bg: m72.sec?.bg,
      idleSeg72: idleSeg72 ? { bg: idleSeg72.bg, borderTop: idleSeg72.borderTop, shadow: idleSeg72.shadow } : null,
    })
  );

  // 5. Cloud Settings: 3-step guide banner removed, verbose descriptions removed, concise labels, official SVG brand logos present
  await page.locator('.settings-nav-item', { hasText: 'Đám mây' }).click();
  await page.waitForTimeout(300);
  const cloudAudit = await page.evaluate(() => {
    const guide = document.querySelector('.cw-cloud-guide');
    const guideVisible = guide ? getComputedStyle(guide).display !== 'none' : false;
    const brands = Array.from(document.querySelectorAll('.settings-body [data-brand] svg'));
    const bodyText = document.querySelector('.settings-body').innerText;
    return {
      guideVisible,
      brandSvgCount: brands.length,
      hasLongDesc: bodyText.includes('Dữ liệu được khóa bằng mật khẩu này trước khi gửi lên đám mây'),
    };
  });
  record(
    'Cloud Settings removes 3-step guide banner & verbose descriptions while rendering all 10 official SVG cloud brand logos',
    !cloudAudit.guideVisible && !cloudAudit.hasLongDesc && cloudAudit.brandSvgCount >= 10,
    JSON.stringify(cloudAudit)
  );

  // 6. Language Settings: all 33/33 language tiles render official 4x3 SVG flags (zero grey text code fallbacks)
  await page.locator('.settings-nav-item', { hasText: 'Ngôn ngữ' }).click();
  await page.waitForTimeout(300);
  const langAudit = await page.evaluate(() => {
    const tiles = document.querySelectorAll('.lang-tile');
    const flagSvgs = document.querySelectorAll('.lang-tile .lang-flag-badge svg');
    const textPills = document.querySelectorAll('.lang-tile .lang-code-pill');
    return {
      tiles: tiles.length,
      flagSvgs: flagSvgs.length,
      textPills: textPills.length,
    };
  });
  record(
    'All 33/33 language tiles render official SVG national flags with zero grey text code fallbacks',
    langAudit.tiles === 33 && langAudit.flagSvgs === 33 && langAudit.textPills === 0,
    JSON.stringify(langAudit)
  );

  record('Zero unhandled page errors during Spec 052 verification', errors.length === 0, errors.join(' | '));
} finally {
  await browser.close();
}

const failed = checks.filter((c) => !c.pass);
console.log(`\nSpec 052 Summary: ${checks.length - failed.length}/${checks.length} passed`);
if (failed.length > 0) process.exit(1);
