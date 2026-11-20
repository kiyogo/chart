/**
 * Canvas Overlay Scrollbars Engine
 * High-performance, floating canvas-based scroll indicator with proximity & scroll detection.
 *
 * Behaviors:
 * 1. Invisible by default when idle.
 * 2. Reveals as a slender 3px indicator when scrolling or when cursor approaches the scroll edge.
 * 3. Smoothly expands from 3px to 8px with higher contrast when cursor hovers over the scrollbar thumb or drags it.
 * 4. Supports both standard containers and flex-direction: column-reverse (chat messages).
 * 5. Interactive: draggable thumb with 1:1 scroll synchronization.
 */

(function () {
  'use strict';

  let overlayCanvas = null;
  let ctx = null;
  let dpr = window.devicePixelRatio || 1;

  // Map<HTMLElement, { opacity, targetOpacity, thumbW, isNearEdge, isThumbHovered, isDragging, lastScrollTime, thumbRect }>
  const activeEntries = new Map();

  let dragTarget = null;
  let dragIsColReverse = false;
  let dragStartY = 0;
  let dragStartScrollTop = 0;
  let dragTrackHeight = 0;
  let dragThumbHeight = 0;
  let animFrameId = null;
  let fadeTimerId = null;

  let lastPointerX = 0;
  let lastPointerY = 0;
  let hasPointer = false;

  // Configuration constants
  const PROXIMITY_PX = 48;
  const HOVER_ENTER_PX = 7;
  const HOVER_EXIT_PX = 11;
  const EDGE_OUTER_TOLERANCE_PX = 4;
  const THUMB_Y_TOLERANCE_PX = 4;
  const FADE_TIMEOUT_MS = 850;
  const THUMB_NORMAL_WIDTH = 3;
  const THUMB_EXPANDED_WIDTH = 9;
  const THUMB_MIN_HEIGHT = 28;
  const THUMB_EDGE_MARGIN = 2;

  const SCROLLABLE_FALLBACK_SELECTOR =
    '.chat-messages, .settings-body, .settings-nav, .sidebar-scrollable, .feed-scroll, .ent-main, .partner-info-sidebar, .server-members-panel, .quick-results, .modal-dialog, .forum-view';

  function initCanvas() {
    if (overlayCanvas) return;
    overlayCanvas = document.createElement('canvas');
    overlayCanvas.id = 'canvas-scrollbar-overlay';
    overlayCanvas.setAttribute('aria-hidden', 'true');
    overlayCanvas.style.position = 'fixed';
    overlayCanvas.style.top = '0';
    overlayCanvas.style.left = '0';
    overlayCanvas.style.width = '100vw';
    overlayCanvas.style.height = '100vh';
    overlayCanvas.style.pointerEvents = 'none';
    overlayCanvas.style.zIndex = '99999';
    overlayCanvas.style.display = 'block';

    ctx = overlayCanvas.getContext('2d', { alpha: true });
    resizeCanvas();

    window.addEventListener('resize', resizeCanvas, { passive: true });
    document.addEventListener('DOMContentLoaded', () => {
      if (!document.body.contains(overlayCanvas)) {
        document.body.appendChild(overlayCanvas);
      }
    });

    if (document.body && !document.body.contains(overlayCanvas)) {
      document.body.appendChild(overlayCanvas);
    }
  }

  function resizeCanvas() {
    if (!overlayCanvas || !ctx) return;
    dpr = window.devicePixelRatio || 1;
    const w = window.innerWidth;
    const h = window.innerHeight;
    overlayCanvas.width = Math.round(w * dpr);
    overlayCanvas.height = Math.round(h * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    scheduleRender();
  }

  function isScrollableY(el) {
    if (!el || el === document.body || el === document.documentElement) {
      const doc = document.documentElement;
      return (doc.scrollHeight - doc.clientHeight) > 4;
    }
    if (el.scrollHeight <= el.clientHeight + 4) return false;
    const style = window.getComputedStyle(el);
    const overflowY = style.overflowY;
    return overflowY === 'auto' || overflowY === 'scroll';
  }

  function getContainerRect(el) {
    if (el === document.documentElement || el === document.body) {
      return {
        left: 0,
        top: 0,
        right: window.innerWidth,
        bottom: window.innerHeight,
        width: window.innerWidth,
        height: window.innerHeight
      };
    }
    return el.getBoundingClientRect();
  }

  function getScrollMetrics(el) {
    const isDoc = (el === document.documentElement || el === document.body);
    const rect = getContainerRect(el);
    const clientH = isDoc ? window.innerHeight : el.clientHeight;
    const scrollH = isDoc ? document.documentElement.scrollHeight : el.scrollHeight;
    const scrollT = isDoc ? (window.scrollY || document.documentElement.scrollTop) : el.scrollTop;
    const isColReverse = !isDoc && window.getComputedStyle(el).flexDirection === 'column-reverse';

    const maxScroll = Math.max(1, scrollH - clientH);
    const trackH = Math.min(rect.height, clientH);
    const thumbH = Math.max(THUMB_MIN_HEIGHT, Math.round((clientH / scrollH) * trackH));
    const thumbMaxY = Math.max(0, trackH - thumbH);

    // For column-reverse, scrollTop is 0 at bottom and negative down to -maxScroll at top.
    const ratio = isColReverse
      ? Math.max(0, Math.min(1, 1 + (scrollT / maxScroll)))
      : Math.max(0, Math.min(1, scrollT / maxScroll));

    const thumbY = rect.top + Math.round(ratio * thumbMaxY);
    return { isDoc, isColReverse, rect, clientH, scrollH, scrollT, maxScroll, trackH, thumbH, thumbMaxY, thumbY };
  }

  function findScrollableContainerAt(target, clientX, clientY) {
    const stack = (typeof document.elementsFromPoint === 'function')
      ? document.elementsFromPoint(clientX, clientY)
      : (target ? [target] : []);

    const seen = new Set();

    for (let i = 0; i < stack.length; i++) {
      let curr = stack[i];
      while (curr && curr !== document.documentElement) {
        if (!seen.has(curr)) {
          seen.add(curr);
          if (isScrollableY(curr)) {
            const rect = curr.getBoundingClientRect();
            if (
              clientY >= rect.top &&
              clientY <= rect.bottom &&
              clientX >= rect.left &&
              clientX <= rect.right + EDGE_OUTER_TOLERANCE_PX
            ) {
              return curr;
            }
          }
        }
        curr = curr.parentElement;
      }
    }

    // Check active entries + known scrollable containers when pointer sits in the outer margin/resizer gutter
    for (const [el] of activeEntries.entries()) {
      if (!el.isConnected || !isScrollableY(el)) continue;
      const rect = getContainerRect(el);
      if (
        clientY >= rect.top &&
        clientY <= rect.bottom &&
        clientX >= rect.right - PROXIMITY_PX &&
        clientX <= rect.right + EDGE_OUTER_TOLERANCE_PX
      ) {
        return el;
      }
    }

    const candidates = document.querySelectorAll(SCROLLABLE_FALLBACK_SELECTOR);
    for (let i = 0; i < candidates.length; i++) {
      const el = candidates[i];
      if (!isScrollableY(el)) continue;
      const rect = el.getBoundingClientRect();
      if (
        rect.width > 0 &&
        rect.height > 0 &&
        clientY >= rect.top &&
        clientY <= rect.bottom &&
        clientX >= rect.right - PROXIMITY_PX &&
        clientX <= rect.right + EDGE_OUTER_TOLERANCE_PX
      ) {
        return el;
      }
    }

    if (isScrollableY(document.documentElement)) {
      return document.documentElement;
    }
    return null;
  }

  function getEntry(el) {
    let entry = activeEntries.get(el);
    if (!entry) {
      entry = {
        opacity: 0,
        targetOpacity: 0,
        thumbW: THUMB_NORMAL_WIDTH,
        isNearEdge: false,
        isThumbHovered: false,
        isDragging: false,
        lastScrollTime: 0,
        thumbRect: null
      };
      activeEntries.set(el, entry);
    }
    return entry;
  }

  function updateHoverStates(x, y) {
    const el = findScrollableContainerAt(null, x, y);
    let changed = false;

    for (const [targetEl, entry] of activeEntries.entries()) {
      if (targetEl !== el && (entry.isNearEdge || entry.isThumbHovered)) {
        entry.isNearEdge = false;
        entry.isThumbHovered = false;
        changed = true;
      }
    }

    if (el && isScrollableY(el)) {
      const m = getScrollMetrics(el);
      const distFromRight = m.rect.right - x;
      const withinY = y >= m.rect.top && y <= m.rect.bottom;
      const isNearEdge = withinY && distFromRight >= -EDGE_OUTER_TOLERANCE_PX && distFromRight <= PROXIMITY_PX;
      const existing = activeEntries.get(el);
      const wasHovered = existing ? existing.isThumbHovered : false;
      const xLimit = wasHovered ? HOVER_EXIT_PX : HOVER_ENTER_PX;
      const withinThumbY = y >= m.thumbY - THUMB_Y_TOLERANCE_PX && y <= m.thumbY + m.thumbH + THUMB_Y_TOLERANCE_PX;
      const isThumbHovered = withinThumbY && distFromRight >= -EDGE_OUTER_TOLERANCE_PX && distFromRight <= xLimit;

      if (isNearEdge || isThumbHovered || activeEntries.has(el)) {
        const entry = getEntry(el);
        if (entry.isNearEdge !== isNearEdge || entry.isThumbHovered !== isThumbHovered) {
          entry.isNearEdge = isNearEdge;
          entry.isThumbHovered = isThumbHovered;
          changed = true;
        }
      }
    }

    if (changed) {
      scheduleRender();
    }
  }

  function isDarkTheme() {
    return document.documentElement.getAttribute('data-theme') === 'dark' ||
      (document.documentElement.getAttribute('data-theme-pref') === 'system' &&
        window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches);
  }

  function drawRoundedPill(c, x, y, w, h, r) {
    r = Math.min(r, w / 2, h / 2);
    c.beginPath();
    c.moveTo(x + r, y);
    c.lineTo(x + w - r, y);
    c.arcTo(x + w, y, x + w, y + r, r);
    c.lineTo(x + w, y + h - r);
    c.arcTo(x + w, y + h, x + w - r, y + h, r);
    c.lineTo(x + r, y + h);
    c.arcTo(x, y + h, x, y + h - r, r);
    c.lineTo(x, y + r);
    c.arcTo(x, y, x + r, y, r);
    c.closePath();
    c.fill();
  }

  function scheduleFadeCheck(delayMs) {
    if (fadeTimerId) clearTimeout(fadeTimerId);
    fadeTimerId = setTimeout(() => {
      fadeTimerId = null;
      scheduleRender();
    }, delayMs);
  }

  function render() {
    animFrameId = null;
    if (!ctx || !overlayCanvas) return;

    const currentDpr = window.devicePixelRatio || 1;
    if (
      currentDpr !== dpr ||
      Math.round(window.innerWidth * currentDpr) !== overlayCanvas.width ||
      Math.round(window.innerHeight * currentDpr) !== overlayCanvas.height
    ) {
      resizeCanvas();
      return;
    }

    ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);

    const now = performance.now();
    const dark = isDarkTheme();
    let hasActiveAnimations = false;
    let soonestFadeDelay = Infinity;

    for (const [el, entry] of activeEntries.entries()) {
      if (!el.isConnected || !isScrollableY(el)) {
        activeEntries.delete(el);
        continue;
      }

      const scrollAge = now - entry.lastScrollTime;
      const isScrollingRecently = entry.lastScrollTime > 0 && scrollAge < FADE_TIMEOUT_MS;
      const isExpanded = entry.isThumbHovered || entry.isDragging;

      if (entry.isDragging) {
        entry.targetOpacity = 1.0;
      } else if (entry.isThumbHovered) {
        entry.targetOpacity = 0.94;
      } else if (entry.isNearEdge || isScrollingRecently) {
        entry.targetOpacity = 0.72;
      } else {
        entry.targetOpacity = 0.0;
      }

      if (isScrollingRecently && !entry.isNearEdge && !entry.isThumbHovered && !entry.isDragging) {
        soonestFadeDelay = Math.min(soonestFadeDelay, Math.max(16, FADE_TIMEOUT_MS - scrollAge + 16));
      }

      // Smoothly animate opacity
      const opSpeed = entry.targetOpacity > entry.opacity ? 0.3 : 0.14;
      entry.opacity += (entry.targetOpacity - entry.opacity) * opSpeed;
      if (Math.abs(entry.targetOpacity - entry.opacity) < 0.01) {
        entry.opacity = entry.targetOpacity;
      } else {
        hasActiveAnimations = true;
      }

      // Smoothly animate thumb width (3px <-> 9px)
      const targetWidth = isExpanded ? THUMB_EXPANDED_WIDTH : THUMB_NORMAL_WIDTH;
      entry.thumbW += (targetWidth - entry.thumbW) * 0.24;
      if (Math.abs(targetWidth - entry.thumbW) < 0.05) {
        entry.thumbW = targetWidth;
      } else {
        hasActiveAnimations = true;
      }

      if (entry.opacity <= 0.005) {
        entry.opacity = 0;
        entry.thumbRect = null;
        if (!entry.isNearEdge && !entry.isThumbHovered && !entry.isDragging && !isScrollingRecently) {
          activeEntries.delete(el);
        }
        continue;
      }

      const m = getScrollMetrics(el);
      if (m.rect.bottom <= 0 || m.rect.top >= window.innerHeight || m.rect.height <= 10) continue;

      const thumbW = entry.thumbW;
      const thumbX = m.rect.right - thumbW - THUMB_EDGE_MARGIN;

      entry.thumbRect = {
        x: m.rect.right - HOVER_EXIT_PX,
        y: m.thumbY - THUMB_Y_TOLERANCE_PX,
        w: HOVER_EXIT_PX + EDGE_OUTER_TOLERANCE_PX,
        h: m.thumbH + THUMB_Y_TOLERANCE_PX * 2,
        visualY: m.thumbY,
        visualH: m.thumbH,
        elRect: m.rect
      };

      ctx.save();
      ctx.globalAlpha = entry.opacity;

      const hoverProgress = Math.max(0, Math.min(1, (thumbW - THUMB_NORMAL_WIDTH) / (THUMB_EXPANDED_WIDTH - THUMB_NORMAL_WIDTH)));
      if (dark) {
        const a = 0.22 + hoverProgress * 0.44;
        ctx.fillStyle = `rgba(255, 255, 255, ${a.toFixed(3)})`;
      } else {
        const r = Math.round(90 - hoverProgress * 45);
        const g = Math.round(105 - hoverProgress * 50);
        const b = Math.round(125 - hoverProgress * 55);
        const a = 0.28 + hoverProgress * 0.46;
        ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${a.toFixed(3)})`;
      }

      drawRoundedPill(ctx, thumbX, m.thumbY, thumbW, m.thumbH, thumbW / 2);
      ctx.restore();
    }

    if (hasActiveAnimations) {
      animFrameId = requestAnimationFrame(render);
    } else if (soonestFadeDelay < Infinity) {
      scheduleFadeCheck(soonestFadeDelay);
    }
  }

  function scheduleRender() {
    if (!animFrameId) {
      animFrameId = requestAnimationFrame(render);
    }
  }

  function clearAllHoverStates() {
    hasPointer = false;
    let changed = false;
    for (const [, entry] of activeEntries.entries()) {
      if (entry.isNearEdge || entry.isThumbHovered) {
        entry.isNearEdge = false;
        entry.isThumbHovered = false;
        changed = true;
      }
    }
    if (changed) scheduleRender();
  }

  // Handle scroll events anywhere in the window
  window.addEventListener('scroll', function (e) {
    initCanvas();
    const target = e.target;
    const el = (target === document || target === window) ? document.documentElement : target;
    if (el && isScrollableY(el)) {
      const entry = getEntry(el);
      entry.lastScrollTime = performance.now();
      if (hasPointer) {
        updateHoverStates(lastPointerX, lastPointerY);
      }
      scheduleRender();
    }
  }, { capture: true, passive: true });

  // Handle pointer proximity & direct scrollbar hover
  window.addEventListener('pointermove', function (e) {
    lastPointerX = e.clientX;
    lastPointerY = e.clientY;
    hasPointer = true;

    if (dragTarget) {
      handleDragMove(e);
      return;
    }

    initCanvas();
    updateHoverStates(e.clientX, e.clientY);
  }, { passive: true });

  window.addEventListener('pointerleave', clearAllHoverStates, { passive: true });
  window.addEventListener('blur', clearAllHoverStates, { passive: true });

  // Handle pointer down on thumb or track
  window.addEventListener('pointerdown', function (e) {
    if (e.button !== 0) return;
    if (e.target && e.target.closest && e.target.closest('[data-resizer]')) return;

    const x = e.clientX;
    const y = e.clientY;

    for (const [el, entry] of activeEntries.entries()) {
      if (!entry.thumbRect || entry.opacity < 0.05) continue;

      const tr = entry.thumbRect;
      // Hit on thumb
      if (x >= tr.x && x <= tr.x + tr.w && y >= tr.y && y <= tr.y + tr.h) {
        e.preventDefault();
        e.stopPropagation();

        const m = getScrollMetrics(el);
        dragTarget = el;
        dragIsColReverse = m.isColReverse;
        dragStartY = y;
        dragStartScrollTop = m.scrollT;
        dragTrackHeight = m.trackH;
        dragThumbHeight = m.thumbH;

        entry.isDragging = true;
        entry.isThumbHovered = true;
        entry.lastScrollTime = performance.now();
        document.body.classList.add('canvas-scrollbar-dragging');
        scheduleRender();
        return;
      }

      // Hit on track
      const distFromRight = tr.elRect.right - x;
      if (distFromRight >= -EDGE_OUTER_TOLERANCE_PX && distFromRight <= HOVER_EXIT_PX && y >= tr.elRect.top && y <= tr.elRect.bottom) {
        e.preventDefault();
        e.stopPropagation();

        const m = getScrollMetrics(el);
        const ratio = Math.max(0, Math.min(1, (y - m.rect.top - m.thumbH / 2) / Math.max(1, m.trackH - m.thumbH)));
        const targetScroll = m.isColReverse ? (ratio - 1) * m.maxScroll : ratio * m.maxScroll;

        if (m.isDoc) {
          window.scrollTo({ top: targetScroll, behavior: 'smooth' });
        } else {
          el.scrollTo({ top: targetScroll, behavior: 'smooth' });
        }

        entry.lastScrollTime = performance.now();
        scheduleRender();
        return;
      }
    }
  }, { capture: true, passive: false });

  function handleDragMove(e) {
    if (!dragTarget) return;
    const dy = e.clientY - dragStartY;
    const isDoc = (dragTarget === document.documentElement || dragTarget === document.body);
    const clientH = isDoc ? window.innerHeight : dragTarget.clientHeight;
    const scrollH = isDoc ? document.documentElement.scrollHeight : dragTarget.scrollHeight;
    const maxScroll = Math.max(1, scrollH - clientH);
    const usableTrackH = Math.max(1, dragTrackHeight - dragThumbHeight);

    const scrollDelta = (dy / usableTrackH) * maxScroll;
    if (dragIsColReverse) {
      const newScrollTop = Math.max(-maxScroll, Math.min(0, dragStartScrollTop + scrollDelta));
      dragTarget.scrollTop = newScrollTop;
    } else {
      const newScrollTop = Math.max(0, Math.min(maxScroll, dragStartScrollTop + scrollDelta));
      if (isDoc) {
        window.scrollTo(0, newScrollTop);
      } else {
        dragTarget.scrollTop = newScrollTop;
      }
    }

    const entry = getEntry(dragTarget);
    entry.lastScrollTime = performance.now();
    scheduleRender();
  }

  function endDrag() {
    if (!dragTarget) return;
    const entry = getEntry(dragTarget);
    entry.isDragging = false;
    entry.lastScrollTime = performance.now();
    dragTarget = null;
    dragIsColReverse = false;
    document.body.classList.remove('canvas-scrollbar-dragging');
    scheduleRender();
  }

  window.addEventListener('pointerup', endDrag, { passive: true });
  window.addEventListener('pointercancel', endDrag, { passive: true });

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initCanvas);
  } else {
    initCanvas();
  }
})();
