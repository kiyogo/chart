// ChatW Real-Time Adaptive Surface, 1px Edge Fade Projection & Sticky-Bottom Scroll Engine (Spec 061)
// 1. window.__chatwStickToBottom: ResizeObserver + MutationObserver + capture image-load bottom pinning
//    so conversations with async encrypted images/polls ALWAYS open anchored at the newest message,
//    while instantly releasing on any genuine user upward scroll or prepend-history load.
// 2. Exact 1px-accurate top (.chat-header::before) & bottom (.composer::before) wallpaper edge fades
//    across preset gradients, custom colors, uploaded chat images (cover projection), and translucent app wallpapers.
// 3. 60fps real-time per-position pixel sampling across visible UI rows/items as they scroll over multi-tone
//    wallpapers (per-element ink polarity, local harmonized hover tint, split-contrast local scrim) WITHOUT ever
//    overriding the user's chosen Màu nhấn (--accent).
(function () {
  if (window.__chatwAdaptiveSurface) return;

  // =========================================================================
  // PART 1: Bulletproof Sticky-Bottom Chat Scroller (use-stick-to-bottom model)
  // =========================================================================
  const sticky = {
    el: null,
    stream: null,
    pinned: true,
    pinLockUntil: 0,
    lastProgAt: 0,
    touchStartY: 0,
    ro: null,
    mo: null,
    rafId: 0,
    retryRaf: 0,
  };

  function scrollNow(el, behavior) {
    if (!el || window.__chatwPrependSnap) return;
    sticky.lastProgAt = performance.now();
    const target = el.scrollHeight;
    if (behavior === 'smooth') {
      try {
        el.scrollTo({ top: target, behavior: 'smooth' });
      } catch (_) {
        el.scrollTop = target;
      }
    } else {
      el.scrollTop = target;
    }
  }

  function schedulePinFlush() {
    if (sticky.rafId) return;
    sticky.rafId = requestAnimationFrame(() => {
      sticky.rafId = 0;
      if (!sticky.pinned || window.__chatwPrependSnap) return;
      const el = sticky.el && sticky.el.isConnected ? sticky.el : document.getElementById('chat-messages-scroll');
      if (el) {
        if (el !== sticky.el) attachScroller(el, false);
        scrollNow(el, 'instant');
      }
    });
  }

  function unpinScroller() {
    sticky.pinned = false;
    sticky.pinLockUntil = 0;
  }

  function onWheel(e) {
    if (e.deltaY < -1) unpinScroller();
  }

  function onTouchStart(e) {
    if (e.touches && e.touches.length) sticky.touchStartY = e.touches[0].clientY;
  }

  function onTouchMove(e) {
    if (e.touches && e.touches.length && e.touches[0].clientY - sticky.touchStartY > 8) {
      unpinScroller();
    }
  }

  function onPointerDown(e) {
    const el = sticky.el;
    if (!el) return;
    const r = el.getBoundingClientRect();
    if (e.clientX >= r.right - 20) unpinScroller();
  }

  function onKeyDown(e) {
    if (!sticky.el) return;
    const tag = e.target && e.target.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || (e.target && e.target.isContentEditable)) return;
    if (e.key === 'PageUp' || e.key === 'Home' || e.key === 'ArrowUp') {
      unpinScroller();
    }
  }

  function onScroll() {
    const el = sticky.el;
    if (!el || window.__chatwPrependSnap) return;
    if (performance.now() - sticky.lastProgAt < 60) return;
    const dist = el.scrollHeight - el.scrollTop - el.clientHeight;
    if (dist <= 36) {
      sticky.pinned = true;
    } else if (performance.now() > sticky.pinLockUntil && dist > 64) {
      sticky.pinned = false;
    }
  }

  function onMediaLoad() {
    if (sticky.pinned && !window.__chatwPrependSnap) {
      if (sticky.el) scrollNow(sticky.el, 'instant');
      schedulePinFlush();
    }
  }

  function detachScroller() {
    if (sticky.el) {
      sticky.el.removeEventListener('wheel', onWheel);
      sticky.el.removeEventListener('touchstart', onTouchStart);
      sticky.el.removeEventListener('touchmove', onTouchMove);
      sticky.el.removeEventListener('pointerdown', onPointerDown);
      sticky.el.removeEventListener('scroll', onScroll);
      sticky.el.removeEventListener('load', onMediaLoad, true);
    }
    if (sticky.ro) {
      sticky.ro.disconnect();
      sticky.ro = null;
    }
    if (sticky.mo) {
      sticky.mo.disconnect();
      sticky.mo = null;
    }
    sticky.el = null;
    sticky.stream = null;
  }

  function attachScroller(el, resetPin) {
    if (!el) return;
    if (sticky.el === el) {
      const curStream = el.querySelector('.message-stream');
      if (curStream && curStream !== sticky.stream && sticky.ro) {
        if (sticky.stream) sticky.ro.unobserve(sticky.stream);
        sticky.stream = curStream;
        sticky.ro.observe(curStream);
      }
      return;
    }
    detachScroller();
    sticky.el = el;
    sticky.stream = el.querySelector('.message-stream');
    if (resetPin) {
      sticky.pinned = true;
      sticky.pinLockUntil = performance.now() + 2800;
    }

    el.addEventListener('wheel', onWheel, { passive: true });
    el.addEventListener('touchstart', onTouchStart, { passive: true });
    el.addEventListener('touchmove', onTouchMove, { passive: true });
    el.addEventListener('pointerdown', onPointerDown, { passive: true });
    el.addEventListener('scroll', onScroll, { passive: true });
    el.addEventListener('load', onMediaLoad, true);

    if (typeof ResizeObserver !== 'undefined') {
      sticky.ro = new ResizeObserver(() => {
        if (sticky.pinned && !window.__chatwPrependSnap) {
          scrollNow(el, 'instant');
        }
      });
      sticky.ro.observe(el);
      if (sticky.stream) sticky.ro.observe(sticky.stream);
    }

    if (typeof MutationObserver !== 'undefined') {
      sticky.mo = new MutationObserver(() => {
        const curStream = el.querySelector('.message-stream');
        if (curStream && curStream !== sticky.stream && sticky.ro) {
          if (sticky.stream) sticky.ro.unobserve(sticky.stream);
          sticky.stream = curStream;
          sticky.ro.observe(curStream);
        }
        if (sticky.pinned && !window.__chatwPrependSnap) {
          schedulePinFlush();
        }
      });
      sticky.mo.observe(el, { childList: true, subtree: true });
    }

    if (sticky.pinned) {
      scrollNow(el, 'instant');
      schedulePinFlush();
    }
  }

  window.addEventListener('keydown', onKeyDown, { passive: true });

  window.__chatwStickToBottom = {
    pin(instant) {
      sticky.pinned = true;
      sticky.pinLockUntil = performance.now() + 2800;
      const run = () => {
        const el = document.getElementById('chat-messages-scroll');
        if (!el) return false;
        attachScroller(el, false);
        scrollNow(el, instant ? 'instant' : 'smooth');
        return true;
      };
      const found = run();
      queueMicrotask(run);
      if (sticky.retryRaf) cancelAnimationFrame(sticky.retryRaf);
      sticky.retryRaf = requestAnimationFrame(() => {
        sticky.retryRaf = 0;
        run();
        requestAnimationFrame(() => {
          if (sticky.pinned && !window.__chatwPrependSnap) {
            const el = document.getElementById('chat-messages-scroll');
            if (el) scrollNow(el, 'instant');
          }
        });
      });
      if (!found) {
        setTimeout(() => {
          if (sticky.pinned) run();
        }, 60);
      }
    },
    unpin() {
      unpinScroller();
    },
    isPinning() {
      return sticky.pinned && performance.now() < sticky.pinLockUntil;
    },
    isPinned() {
      return sticky.pinned;
    },
  };

  // =========================================================================
  // PART 2: Offscreen Composited Backdrop Canvas & Per-Pixel Samplers
  // =========================================================================
  const BW = 160;
  const BH = 90;
  const rawCanvas = document.createElement('canvas');
  rawCanvas.width = BW;
  rawCanvas.height = BH;
  const rawCtx = rawCanvas.getContext('2d', { willReadFrequently: true });

  const panelCanvas = document.createElement('canvas');
  panelCanvas.width = BW;
  panelCanvas.height = BH;
  const panelCtx = panelCanvas.getContext('2d', { willReadFrequently: true });

  const chatImgCanvas = document.createElement('canvas');
  chatImgCanvas.width = 128;
  chatImgCanvas.height = 72;
  const chatImgCtx = chatImgCanvas.getContext('2d', { willReadFrequently: true });

  const imgCache = new Map();
  let panelPixels = null;
  let lastBackdropKey = '';
  let lastEdgeKey = '';
  let chatPixels = null;
  let lastChatWpUrl = '';
  let taggedElements = new Set();
  let localRaf = 0;
  let fullRaf = 0;

  function parseHex(hex) {
    if (!hex || typeof hex !== 'string') return null;
    const h = hex.trim().replace(/^#/, '');
    if (!/^[0-9a-fA-F]{6}$/.test(h)) return null;
    return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
  }

  function apcaY(r, g, b) {
    let y =
      0.2126729 * Math.pow(r / 255, 2.4) +
      0.7151522 * Math.pow(g / 255, 2.4) +
      0.072175 * Math.pow(b / 255, 2.4);
    if (y < 0.022) y += Math.pow(0.022 - y, 1.414);
    return y;
  }

  function calcAPCA(fg, bg) {
    const yfg = apcaY(fg[0], fg[1], fg[2]);
    const ybg = apcaY(bg[0], bg[1], bg[2]);
    let c = 0;
    if (ybg > yfg) {
      c = 1.14 * (Math.pow(ybg, 0.56) - Math.pow(yfg, 0.57));
      if (Math.abs(c) < 0.1) return 0;
      c -= 0.027;
    } else {
      c = 1.14 * (Math.pow(ybg, 0.65) - Math.pow(yfg, 0.62));
      if (Math.abs(c) < 0.1) return 0;
      c += 0.027;
    }
    return c * 100;
  }

  function getLoadedImage(url, onReady) {
    if (!url) return null;
    const entry = imgCache.get(url);
    if (entry) {
      if (entry.loaded) return entry.img;
      return null;
    }
    const img = new Image();
    img.crossOrigin = 'anonymous';
    const rec = { img, loaded: false, failed: false };
    imgCache.set(url, rec);
    img.onload = () => {
      rec.loaded = true;
      if (onReady) onReady();
    };
    img.onerror = () => {
      rec.failed = true;
    };
    // Never trigger a direct HTTP fetch for protected /uploads/ or /wallpapers/ URLs —
    // reuse the single deduplicated SharedWorker blob: URL!
    const isProtected =
      typeof url === 'string' &&
      !url.startsWith('blob:') &&
      !url.startsWith('data:') &&
      (url.startsWith('/uploads/') || url.startsWith('/api/attachments/') || url.startsWith('/wallpapers/'));
    if (isProtected && window.__chatwCore && typeof window.__chatwCore.downloadMediaBlobUrl === 'function') {
      const cached = window.__chatwCore.getCachedBlobUrlSync && window.__chatwCore.getCachedBlobUrlSync(url);
      if (cached) {
        img.src = cached;
      } else {
        window.__chatwCore.downloadMediaBlobUrl(url, { priority: 'HIGH' }).then((burl) => {
          if (burl) img.src = burl;
          else rec.failed = true;
        }).catch(() => { rec.failed = true; });
      }
      return null;
    }
    img.src = url;
    return null;
  }

  function drawCover(ctx, img, destW, destH, insetPx, vw, vh) {
    const nw = img.naturalWidth || img.width || 1;
    const nh = img.naturalHeight || img.height || 1;
    const fullW = vw + insetPx * 2;
    const fullH = vh + insetPx * 2;
    const scale = Math.max(fullW / nw, fullH / nh);
    const drawW = nw * scale;
    const drawH = nh * scale;
    const offX = (fullW - drawW) / 2 - insetPx;
    const offY = (fullH - drawH) / 2 - insetPx;
    const sx = destW / vw;
    const sy = destH / vh;
    ctx.drawImage(img, offX * sx, offY * sy, drawW * sx, drawH * sy);
  }

  function getPanelBaseHex(root) {
    const baseTheme = root.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
    const variant = root.getAttribute('data-theme-variant') || baseTheme;
    const surfaceTone = root.getAttribute('data-surface-tone') || baseTheme;
    if (baseTheme === 'light' && surfaceTone === 'dark') return '#141821';
    if (baseTheme === 'dark' && surfaceTone === 'light') return '#FFFFFF';
    if (variant === 'ash') return '#313338';
    if (variant === 'oled') return '#090A0E';
    return baseTheme === 'dark' ? '#1C1F26' : '#FFFFFF';
  }

  function drawPresetAppWallpaper(ctx, wpKind, baseTheme, w, h) {
    const dark = baseTheme === 'dark';
    ctx.clearRect(0, 0, w, h);
    if (wpKind === 'preset-sunset') {
      ctx.fillStyle = dark ? '#111318' : '#FFF7F4';
      ctx.fillRect(0, 0, w, h);
      const g = ctx.createLinearGradient(0, 0, w * 0.55, h);
      if (dark) {
        g.addColorStop(0, 'rgba(249, 115, 22, 0.32)');
        g.addColorStop(1, 'rgba(219, 39, 119, 0.28)');
      } else {
        g.addColorStop(0, 'rgba(255, 168, 128, 0.42)');
        g.addColorStop(0.55, 'rgba(251, 146, 178, 0.38)');
        g.addColorStop(1, 'rgba(232, 162, 226, 0.32)');
      }
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, w, h);
    } else if (wpKind === 'preset-ocean') {
      ctx.fillStyle = dark ? '#111318' : '#F3F9FF';
      ctx.fillRect(0, 0, w, h);
      const g = ctx.createLinearGradient(0, 0, w * 0.36, h);
      if (dark) {
        g.addColorStop(0, 'rgba(14, 165, 233, 0.32)');
        g.addColorStop(1, 'rgba(37, 99, 235, 0.28)');
      } else {
        g.addColorStop(0, 'rgba(125, 211, 252, 0.40)');
        g.addColorStop(1, 'rgba(96, 165, 250, 0.30)');
      }
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, w, h);
    } else if (wpKind === 'preset-mint') {
      ctx.fillStyle = dark ? '#111318' : '#F3FDF8';
      ctx.fillRect(0, 0, w, h);
      const g = ctx.createLinearGradient(0, 0, w * 0.45, h);
      if (dark) {
        g.addColorStop(0, 'rgba(16, 185, 129, 0.28)');
        g.addColorStop(1, 'rgba(163, 230, 53, 0.22)');
      } else {
        g.addColorStop(0, 'rgba(110, 231, 183, 0.36)');
        g.addColorStop(1, 'rgba(190, 242, 100, 0.26)');
      }
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, w, h);
    } else if (wpKind === 'preset-aurora') {
      ctx.fillStyle = dark ? '#111318' : '#F7F9FF';
      ctx.fillRect(0, 0, w, h);
      const r1 = ctx.createRadialGradient(w * 0.1, 0, 0, w * 0.1, 0, w * 0.6);
      r1.addColorStop(0, dark ? 'rgba(34, 211, 238, 0.28)' : 'rgba(56, 189, 248, 0.30)');
      r1.addColorStop(0.7, 'rgba(56, 189, 248, 0)');
      ctx.fillStyle = r1;
      ctx.fillRect(0, 0, w, h);
      const r2 = ctx.createRadialGradient(w * 0.9, h * 0.1, 0, w * 0.9, h * 0.1, w * 0.6);
      r2.addColorStop(0, dark ? 'rgba(167, 139, 250, 0.28)' : 'rgba(192, 132, 252, 0.30)');
      r2.addColorStop(0.7, 'rgba(192, 132, 252, 0)');
      ctx.fillStyle = r2;
      ctx.fillRect(0, 0, w, h);
    } else if (wpKind === 'preset-night') {
      const g = ctx.createLinearGradient(0, 0, w * 0.28, h);
      g.addColorStop(0, '#101936');
      g.addColorStop(1, '#090E1F');
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, w, h);
    } else if (wpKind === 'preset-paper') {
      ctx.fillStyle = dark ? '#2F2C27' : '#EFEBE3';
      ctx.fillRect(0, 0, w, h);
    } else if (wpKind === 'preset-bubbles') {
      ctx.fillStyle = dark ? '#111318' : '#E9EDF3';
      ctx.fillRect(0, 0, w, h);
      const r1 = ctx.createRadialGradient(w * 0.12, h * 0.18, 0, w * 0.12, h * 0.18, w * 0.14);
      r1.addColorStop(0, 'rgba(46, 124, 246, 0.22)');
      r1.addColorStop(1, 'rgba(46, 124, 246, 0)');
      ctx.fillStyle = r1;
      ctx.fillRect(0, 0, w, h);
      const r2 = ctx.createRadialGradient(w * 0.82, h * 0.7, 0, w * 0.82, h * 0.7, w * 0.2);
      r2.addColorStop(0, 'rgba(236, 72, 153, 0.18)');
      r2.addColorStop(1, 'rgba(236, 72, 153, 0)');
      ctx.fillStyle = r2;
      ctx.fillRect(0, 0, w, h);
    } else {
      ctx.fillStyle = dark ? '#111318' : '#E9EDF3';
      ctx.fillRect(0, 0, w, h);
    }
  }

  function rebuildBackdropCanvas() {
    const root = document.documentElement;
    const c = window.__chatwCustom || {};
    const wpKind = root.getAttribute('data-app-wp') || c.wpKind || 'none';
    const baseTheme = root.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
    const panelOpacity = Math.max(0, Math.min(100, c.panelOpacity == null ? 100 : Number(c.panelOpacity)));
    const panelGlass = root.getAttribute('data-panel-glass') === 'true';
    const panelBlur = panelGlass ? parseFloat(root.style.getPropertyValue('--panel-blur')) || 12 : 0;
    const wpBlur = parseFloat(root.style.getPropertyValue('--app-wp-blur')) || 0;
    const wpDim = Math.max(0, Math.min(80, Number(c.dim) || 0)) / 100;
    const wpColor = c.wpColor || root.style.getPropertyValue('--app-wp-color').trim() || '';
    const wpImgUrl = c.wpImageUrl || '';
    const panelHex = getPanelBaseHex(root);
    const vw = Math.max(1, window.innerWidth || 1440);
    const vh = Math.max(1, window.innerHeight || 900);

    if (wpKind === 'none' || panelOpacity >= 99) {
      panelPixels = null;
      lastBackdropKey = '';
      return false;
    }

    const key = `${wpKind}|${wpColor}|${wpImgUrl}|${baseTheme}|${panelHex}|${panelOpacity}|${panelBlur}|${wpBlur}|${wpDim}|${Math.round(vw / 16)}x${Math.round(vh / 16)}`;
    if (key === lastBackdropKey && panelPixels) return true;

    let imgReady = true;
    rawCtx.save();
    rawCtx.filter = 'none';
    rawCtx.clearRect(0, 0, BW, BH);
    if (wpKind === 'image') {
      const img = getLoadedImage(wpImgUrl, () => {
        lastBackdropKey = '';
        lastEdgeKey = '';
        scheduleFullRefresh();
      });
      if (img) {
        if (wpBlur > 0) {
          rawCtx.filter = `blur(${Math.max(0.5, wpBlur * (BW / vw)).toFixed(2)}px)`;
        }
        drawCover(rawCtx, img, BW, BH, 28, vw, vh);
      } else {
        imgReady = false;
        rawCtx.fillStyle = baseTheme === 'dark' ? '#111318' : '#E9EDF3';
        rawCtx.fillRect(0, 0, BW, BH);
      }
    } else if (wpKind === 'color') {
      rawCtx.fillStyle = parseHex(wpColor) ? wpColor : (baseTheme === 'dark' ? '#111318' : '#E9EDF3');
      rawCtx.fillRect(0, 0, BW, BH);
    } else {
      drawPresetAppWallpaper(rawCtx, wpKind, baseTheme, BW, BH);
    }
    rawCtx.restore();

    if (wpDim > 0) {
      rawCtx.fillStyle = `rgba(0, 0, 0, ${wpDim.toFixed(3)})`;
      rawCtx.fillRect(0, 0, BW, BH);
    }

    panelCtx.save();
    panelCtx.clearRect(0, 0, BW, BH);
    panelCtx.filter = panelBlur > 0 ? `blur(${Math.max(0.6, panelBlur * (BW / vw)).toFixed(2)}px)` : 'none';
    panelCtx.drawImage(rawCanvas, 0, 0);
    panelCtx.restore();

    const alpha = panelOpacity / 100;
    if (alpha > 0) {
      const rgb = parseHex(panelHex) || [255, 255, 255];
      panelCtx.fillStyle = `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${alpha.toFixed(3)})`;
      panelCtx.fillRect(0, 0, BW, BH);
    }

    try {
      panelPixels = panelCtx.getImageData(0, 0, BW, BH).data;
      if (imgReady) lastBackdropKey = key;
    } catch (_) {
      panelPixels = null;
      return false;
    }
    return true;
  }

  // =========================================================================
  // PART 3: Exact 1px-Accurate Top & Bottom Chat Edge Fades
  // =========================================================================
  function hasExplicitChatWallpaper(chatCol) {
    if (!chatCol) return false;
    const cl = chatCol.classList;
    return (
      cl.contains('wp-ocean') ||
      cl.contains('wp-sunset') ||
      cl.contains('wp-mint') ||
      cl.contains('wp-aurora') ||
      cl.contains('wp-night') ||
      cl.contains('wp-paper') ||
      cl.contains('wp-dots') ||
      cl.contains('wp-bubbles') ||
      cl.contains('wp-custom') ||
      cl.contains('wp-image')
    );
  }

  function syncChatEdgeFades() {
    const chatCol = document.querySelector('.chat-column');
    if (!chatCol) return;
    const colRect = chatCol.getBoundingClientRect();
    if (colRect.width < 10 || colRect.height < 10) return;

    const colW = Math.round(colRect.width * 10) / 10;
    const colH = Math.round(colRect.height * 10) / 10;
    chatCol.style.setProperty('--chat-col-w', `${colW}px`);
    chatCol.style.setProperty('--chat-col-h', `${colH}px`);

    const hdr = chatCol.querySelector(':scope > .chat-header');
    const comp = chatCol.querySelector(':scope > .composer');
    const hdrH = hdr ? Math.max(48, hdr.getBoundingClientRect().height) : 56;
    const compRectH = comp ? Math.max(48, comp.getBoundingClientRect().height) : 62;
    const botMistH = compRectH + 4;

    if (chatCol.classList.contains('wp-image')) {
      chatCol.removeAttribute('data-chat-edge-strip');
      chatCol.style.removeProperty('--chat-top-fade-strip');
      chatCol.style.removeProperty('--chat-bot-fade-strip');
      const st = chatCol.getAttribute('style') || '';
      const mUrl = st.match(/--chat-wp-image\s*:\s*url\(['"]?([^'")]+)['"]?\)/);
      const url = mUrl && mUrl[1] ? mUrl[1] : '';
      if (url) {
        const img = getLoadedImage(url, () => {
          lastEdgeKey = '';
          syncChatEdgeFades();
          updateChatHeaderLocalTone(chatCol);
        });
        if (img) {
          const nw = img.naturalWidth || img.width || 1;
          const nh = img.naturalHeight || img.height || 1;
          const scale = Math.max(colRect.width / nw, colRect.height / nh);
          const drawW = nw * scale;
          const drawH = nh * scale;
          const offX = (colRect.width - drawW) / 2;
          const offY = (colRect.height - drawH) / 2;
          const botOffY = offY - (colRect.height - botMistH);
          chatCol.style.setProperty('--chat-bg-w', `${drawW.toFixed(2)}px`);
          chatCol.style.setProperty('--chat-bg-h', `${drawH.toFixed(2)}px`);
          chatCol.style.setProperty('--chat-bg-x', `${offX.toFixed(2)}px`);
          chatCol.style.setProperty('--chat-bg-top-y', `${offY.toFixed(2)}px`);
          chatCol.style.setProperty('--chat-bg-bot-y', `${botOffY.toFixed(2)}px`);
          if (lastChatWpUrl !== url || !chatPixels) {
            try {
              chatImgCtx.clearRect(0, 0, chatImgCanvas.width, chatImgCanvas.height);
              drawCover(chatImgCtx, img, chatImgCanvas.width, chatImgCanvas.height, 0, colRect.width, colRect.height);
              chatPixels = chatImgCtx.getImageData(0, 0, chatImgCanvas.width, chatImgCanvas.height).data;
              lastChatWpUrl = url;
            } catch (_) {
              chatPixels = null;
            }
          }
          updateChatHeaderLocalTone(chatCol);
        }
      }
      return;
    }

    chatCol.style.removeProperty('--chat-bg-w');
    chatCol.style.removeProperty('--chat-bg-h');
    chatCol.style.removeProperty('--chat-bg-x');
    chatCol.style.removeProperty('--chat-bg-top-y');
    chatCol.style.removeProperty('--chat-bg-bot-y');
    chatPixels = null;
    lastChatWpUrl = '';

    chatCol.removeAttribute('data-chat-edge-strip');
    chatCol.style.removeProperty('--chat-top-fade-strip');
    chatCol.style.removeProperty('--chat-bot-fade-strip');
    lastEdgeKey = '';
  }
  }

  // =========================================================================
  // PART 4: Real-Time Per-Position Adaptive Surface & Contrast (60fps)
  // =========================================================================
  function sampleRectInBuffer(buf, bw, bh, rx, ry, rw, rh, refX, refY, refW, refH) {
    const cols = 5;
    const rows = 2;
    let rSum = 0, gSum = 0, bSum = 0;
    let lumSum = 0, lumSq = 0;
    let minB = 1, maxB = 0;
    let n = 0;
    for (let j = 0; j < rows; j++) {
      const py = ry + rh * ((j + 0.5) / rows);
      const by = Math.max(0, Math.min(bh - 1, Math.floor(((py - refY) / refH) * bh)));
      for (let i = 0; i < cols; i++) {
        const px = rx + rw * ((i + 0.5) / cols);
        const bx = Math.max(0, Math.min(bw - 1, Math.floor(((px - refX) / refW) * bw)));
        const idx = (by * bw + bx) * 4;
        const r = buf[idx];
        const g = buf[idx + 1];
        const b = buf[idx + 2];
        const lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
        rSum += r;
        gSum += g;
        bSum += b;
        lumSum += lum;
        lumSq += lum * lum;
        if (lum < minB) minB = lum;
        if (lum > maxB) maxB = lum;
        n++;
      }
    }
    if (!n) return null;
    const avgR = rSum / n;
    const avgG = gSum / n;
    const avgB = bSum / n;
    const meanB = lumSum / n;
    const stdB = Math.sqrt(Math.max(0, lumSq / n - meanB * meanB));
    return { avgR, avgG, avgB, meanB, minB, maxB, stdB };
  }

  function applyLocalMetrics(el, m, alpha, isRowOrBtn) {
    if (!m) return;
    const lcBlack = Math.abs(calcAPCA([0, 0, 0], [m.avgR, m.avgG, m.avgB]));
    const lcWhite = Math.abs(calcAPCA([255, 255, 255], [m.avgR, m.avgG, m.avgB]));
    const tone = lcWhite > lcBlack + 4 || m.meanB < 0.44 ? 'dark' : 'light';

    if (el.getAttribute('data-local-tone') !== tone) {
      el.setAttribute('data-local-tone', tone);
    }

    if (isRowOrBtn) {
      let hoverBg;
      if (tone === 'dark') {
        const hr = Math.round(m.avgR * 0.24 + 255 * 0.76);
        const hg = Math.round(m.avgG * 0.24 + 255 * 0.76);
        const hb = Math.round(m.avgB * 0.24 + 255 * 0.76);
        hoverBg = `rgba(${hr}, ${hg}, ${hb}, ${alpha < 0.5 ? '0.18' : '0.11'})`;
      } else if (alpha < 0.55) {
        const hr = Math.round(m.avgR * 0.20 + 255 * 0.80);
        const hg = Math.round(m.avgG * 0.20 + 255 * 0.80);
        const hb = Math.round(m.avgB * 0.20 + 255 * 0.80);
        hoverBg = `rgba(${hr}, ${hg}, ${hb}, 0.50)`;
      } else {
        const hr = Math.round(m.avgR * 0.25);
        const hg = Math.round(m.avgG * 0.25);
        const hb = Math.round(m.avgB * 0.25);
        hoverBg = `rgba(${hr}, ${hg}, ${hb}, 0.065)`;
      }
      if (el.style.getPropertyValue('--local-hover-bg') !== hoverBg) {
        el.style.setProperty('--local-hover-bg', hoverBg);
      }

      // Spec 062: Never apply random patchy per-button white frosted scrim boxes (data-local-scrim).
      if (el.hasAttribute('data-local-scrim')) {
        el.removeAttribute('data-local-scrim');
        el.style.removeProperty('--local-scrim-bg');
      }
    }
    taggedElements.add(el);
  }

  function updateChatHeaderLocalTone(chatCol) {
    if (!chatCol || !chatPixels) return;
    const colRect = chatCol.getBoundingClientRect();
    if (colRect.width < 10 || colRect.height < 10) return;
    const targets = chatCol.querySelectorAll(
      ':scope > .chat-header .chat-header-identity, :scope > .chat-header .chat-header-actions .icon-btn'
    );
    for (const el of targets) {
      const r = el.getBoundingClientRect();
      if (r.width < 2 || r.height < 2) continue;
      const m = sampleRectInBuffer(
        chatPixels,
        chatImgCanvas.width,
        chatImgCanvas.height,
        r.left,
        r.top,
        r.width,
        r.height,
        colRect.left,
        colRect.top,
        colRect.width,
        colRect.height
      );
      applyLocalMetrics(el, m, 0.0, true);
    }
  }

  function clearLocalTags() {
    if (!taggedElements.size) return;
    for (const el of taggedElements) {
      if (!el) continue;
      el.removeAttribute('data-local-tone');
      el.removeAttribute('data-local-scrim');
      el.style.removeProperty('--local-hover-bg');
      el.style.removeProperty('--local-scrim-bg');
    }
    taggedElements.clear();
  }

  // Spec 062, 063 & 066: Sample transparent rows/tiles (including low-opacity translucent active nav/rail/chat pills
  // so their ink adapts over split light/dark wallpapers), while NEVER overriding filled .btn-secondary/.btn-ghost/.cw-dropdown.
  const LOCAL_ROW_SELECTOR = [
    '.app-rail .rail-item:not(.has-logo)',
    '.settings-nav .settings-nav-item',
    '.nav-sidebar .chat-item:not(.has-nameplate)',
    '.nav-sidebar .filter-chip',
    '.nav-sidebar .channel-row',
    '.settings-body .wallpaper-tile',
    '.settings-body .theme-card',
  ].join(', ');

  const LOCAL_TEXT_SELECTOR = [
    '.settings-nav .settings-nav-title',
    '.nav-sidebar .sidebar-title',
    '.nav-sidebar .sidebar-section-header',
    '.nav-sidebar .category-row',
    '.settings-body .settings-page-title',
    '.settings-body .settings-group-title',
    '.settings-body .settings-label',
    '.settings-body .settings-subtab',
    '.settings-body .settings-profile-name',
    '.settings-body .settings-profile-sub',
    '.settings-body .range-row label',
    '.settings-body .range-row > span',
  ].join(', ');

  function updateLocalSurfaces() {
    const root = document.documentElement;
    const wpKind = root.getAttribute('data-app-wp') || 'none';
    const c = window.__chatwCustom || {};
    const panelOpacity = c.panelOpacity == null ? 100 : Number(c.panelOpacity);
    const alpha = Math.max(0, Math.min(100, panelOpacity)) / 100;

    if (wpKind === 'none' || panelOpacity >= 96 || !panelPixels) {
      clearLocalTags();
      const chatCol = document.querySelector('.chat-column.wp-image');
      if (chatCol && chatPixels) updateChatHeaderLocalTone(chatCol);
      return;
    }

    const vw = Math.max(1, window.innerWidth || 1440);
    const vh = Math.max(1, window.innerHeight || 900);
    const seen = new Set();

    const process = (selector, isRow) => {
      const nodes = document.querySelectorAll(selector);
      for (let i = 0; i < nodes.length; i++) {
        const el = nodes[i];
        const r = el.getBoundingClientRect();
        if (r.width < 2 || r.height < 2 || r.bottom <= 0 || r.top >= vh || r.right <= 0 || r.left >= vw) {
          continue;
        }
        // For full-width flex labels/headings, sample strictly behind the actual text glyph span on the left
        let sampleW = r.width;
        let sampleY = r.top;
        let sampleH = r.height;
        if (!isRow) {
          const textLen = (el.textContent || '').trim().length;
          if (textLen > 0) {
            sampleW = Math.min(r.width, Math.max(48, textLen * 9.2));
          }
        } else if (el.classList.contains('wallpaper-tile') || el.classList.contains('theme-card')) {
          // Sample behind the bottom caption text of wallpaper/theme tiles
          sampleY = r.bottom - Math.min(26, r.height * 0.32);
          sampleH = Math.min(26, r.height * 0.32);
        }
        const m = sampleRectInBuffer(panelPixels, BW, BH, r.left, sampleY, sampleW, sampleH, 0, 0, vw, vh);
        applyLocalMetrics(el, m, alpha, isRow);
        seen.add(el);
      }
    };

    process(LOCAL_ROW_SELECTOR, true);
    process(LOCAL_TEXT_SELECTOR, false);

    // Sample floating .composer-box at its exact bottom screen position so it matches bright/dark bottom halves!
    const compBox = document.querySelector('.chat-column:not(.wp-night):not([data-chat-tone]) .composer-box');
    if (compBox) {
      const cr = compBox.getBoundingClientRect();
      if (cr.width > 10 && cr.height > 10) {
        const cm = sampleRectInBuffer(panelPixels, BW, BH, cr.left, cr.top, cr.width, cr.height, 0, 0, vw, vh);
        applyLocalMetrics(compBox, cm, alpha, false);
        seen.add(compBox);
      }
    }

    // Clean elements that scrolled off-screen or were detached
    for (const el of taggedElements) {
      if (!seen.has(el) && !el.closest('.chat-column.wp-image')) {
        el.removeAttribute('data-local-tone');
        el.removeAttribute('data-local-scrim');
        el.style.removeProperty('--local-hover-bg');
        el.style.removeProperty('--local-scrim-bg');
        taggedElements.delete(el);
      }
    }

    const chatCol = document.querySelector('.chat-column');
    if (chatCol) {
      if (chatCol.classList.contains('wp-image') && chatPixels) {
        updateChatHeaderLocalTone(chatCol);
      } else if (!hasExplicitChatWallpaper(chatCol)) {
        const hdrTargets = chatCol.querySelectorAll(
          ':scope > .chat-header .chat-header-identity, :scope > .chat-header .chat-header-actions .icon-btn'
        );
        for (const el of hdrTargets) {
          const r = el.getBoundingClientRect();
          if (r.width < 2 || r.height < 2) continue;
          const m = sampleRectInBuffer(panelPixels, BW, BH, r.left, r.top, r.width, r.height, 0, 0, vw, vh);
          applyLocalMetrics(el, m, alpha, true);
        }
      }
    }
  }

  function scheduleLocalPass() {
    if (localRaf) return;
    localRaf = requestAnimationFrame(() => {
      localRaf = 0;
      updateLocalSurfaces();
    });
  }

  function refreshAll() {
    rebuildBackdropCanvas();
    syncChatEdgeFades();
    updateLocalSurfaces();
  }

  function scheduleFullRefresh() {
    if (fullRaf) return;
    fullRaf = requestAnimationFrame(() => {
      fullRaf = 0;
      refreshAll();
    });
  }

  // Real-time passive capture scroll listener: catches scrolling in .settings-nav, .settings-body, .nav-sidebar, etc.
  document.addEventListener(
    'scroll',
    (e) => {
      const t = e.target;
      if (t && t.id === 'chat-messages-scroll') return;
      if (panelPixels) scheduleLocalPass();
    },
    { capture: true, passive: true }
  );

  window.addEventListener(
    'resize',
    () => {
      lastBackdropKey = '';
      lastEdgeKey = '';
      scheduleFullRefresh();
    },
    { passive: true }
  );

  // Observe DOM additions/class changes so new panels, chat switches, and resizer drags stay synced in real time
  let domObsTimer = 0;
  function startObservers() {
    if (!document.body) return;
    const scroller = document.getElementById('chat-messages-scroll');
    if (scroller) attachScroller(scroller, true);

    if (typeof MutationObserver !== 'undefined') {
      const bodyObs = new MutationObserver((mutations) => {
        let scrollerCheck = false;
        let surfaceCheck = false;
        for (let i = 0; i < mutations.length; i++) {
          const m = mutations[i];
          if (m.type === 'childList') {
            if (m.addedNodes.length || m.removedNodes.length) {
              scrollerCheck = true;
              surfaceCheck = true;
            }
          } else if (m.type === 'attributes') {
            const el = m.target;
            if (el && el.classList && el.classList.contains('chat-column')) {
              surfaceCheck = true;
            }
          }
        }
        if (scrollerCheck) {
          const cur = document.getElementById('chat-messages-scroll');
          if (cur && cur !== sticky.el) {
            attachScroller(cur, true);
          } else if (!cur && sticky.el) {
            detachScroller();
          }
        }
        if (surfaceCheck) {
          clearTimeout(domObsTimer);
          domObsTimer = setTimeout(scheduleFullRefresh, 32);
        }
      });
      bodyObs.observe(document.body, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: ['class', 'style'],
      });
    }

    if (typeof ResizeObserver !== 'undefined') {
      let observedChat = null;
      const panelRo = new ResizeObserver(() => {
        lastEdgeKey = '';
        scheduleFullRefresh();
      });
      const ensureChatRo = () => {
        const cc = document.querySelector('.chat-column');
        if (cc && cc !== observedChat) {
          if (observedChat) panelRo.unobserve(observedChat);
          observedChat = cc;
          panelRo.observe(cc);
        }
      };
      ensureChatRo();
      setInterval(ensureChatRo, 800);
    }

    scheduleFullRefresh();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', startObservers, { once: true });
  } else {
    startObservers();
  }

  window.__chatwAdaptiveSurface = {
    refresh: scheduleFullRefresh,
    refreshSync: refreshAll,
    updateLocal: updateLocalSurfaces,
    syncChatEdgeFades,
    // Expose internal test helper for deterministic E2E pixel verification
    getSampleAt(x, y) {
      if (!panelPixels) return null;
      const vw = Math.max(1, window.innerWidth || 1440);
      const vh = Math.max(1, window.innerHeight || 900);
      const bx = Math.max(0, Math.min(BW - 1, Math.floor((x / vw) * BW)));
      const by = Math.max(0, Math.min(BH - 1, Math.floor((y / vh) * BH)));
      const idx = (by * BW + bx) * 4;
      return { r: panelPixels[idx], g: panelPixels[idx + 1], b: panelPixels[idx + 2] };
    },
  };
})();
