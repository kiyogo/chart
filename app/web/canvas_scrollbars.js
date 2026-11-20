/**
 * Canvas Overlay Scrollbars Engine
 * High-performance, floating canvas-based scroll indicator with proximity & scroll detection.
 *
 * Behaviors:
 * 1. Invisible by default when idle.
 * 2. Reveals as a thin 3px indicator when scrolling or when cursor approaches the scroll edge.
 * 3. Smoothly expands from 3px to 8px with higher contrast when cursor hovers directly over the scrollbar or drags it.
 * 4. Interactive: draggable thumb with 1:1 scroll synchronization.
 */

(function () {
  'use strict';

  // State
  let overlayCanvas = null;
  let ctx = null;
  let dpr = window.devicePixelRatio || 1;

  // Map<HTMLElement, { opacity, targetOpacity, thumbW, isNearEdge, isThumbHovered, isDragging, lastScrollTime, thumbRect }>
  const activeEntries = new Map();

  let dragTarget = null;
  let dragStartY = 0;
  let dragStartScrollTop = 0;
  let dragTrackHeight = 0;
  let dragThumbHeight = 0;
  let animFrameId = null;
  let fadeTimerId = null;

  // Configuration constants
  const PROXIMITY_PX = 20;
  const HOVER_ZONE_PX = 10;
  const EDGE_OUTER_TOLERANCE_PX = 2;
  const FADE_TIMEOUT_MS = 850;
  const THUMB_NORMAL_WIDTH = 3;
  const THUMB_EXPANDED_WIDTH = 6;
  const THUMB_MIN_HEIGHT = 28;
  const THUMB_EDGE_MARGIN = 2;

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

  function findScrollableContainerAt(target, clientX, clientY) {
    // Check all elements stacked under (clientX, clientY) so overlays/resizers don't hide the scrollable container
    const stack = (typeof document.elementsFromPoint === 'function')
      ? document.elementsFromPoint(clientX, clientY)
      : (target ? [target] : []);

    const seen = new Set();
    let bestEl = null;

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

    // Also check active visible containers in case the cursor is within EDGE_OUTER_TOLERANCE_PX of their right edge
    for (const [el] of activeEntries.entries()) {
      if (!el.isConnected || !isScrollableY(el)) continue;
      const rect = getContainerRect(el);
      if (
        clientY >= rect.top &&
        clientY <= rect.bottom &&
        clientX >= rect.right - PROXIMITY_PX &&
        clientX <= rect.right + EDGE_OUTER_TOLERANCE_PX
      ) {
        if (!bestEl) bestEl = el;
      }
    }
    if (bestEl) return bestEl;

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
        entry.targetOpacity = 0.92;
      } else if (entry.isNearEdge || isScrollingRecently) {
        entry.targetOpacity = 0.75;
      } else {
        entry.targetOpacity = 0.0;
      }

      if (isScrollingRecently && !entry.isNearEdge && !entry.isThumbHovered && !entry.isDragging) {
        soonestFadeDelay = Math.min(soonestFadeDelay, Math.max(16, FADE_TIMEOUT_MS - scrollAge + 16));
      }

      // Smoothly animate opacity
      const opSpeed = entry.targetOpacity > entry.opacity ? 0.28 : 0.12;
      entry.opacity += (entry.targetOpacity - entry.opacity) * opSpeed;
      if (Math.abs(entry.targetOpacity - entry.opacity) < 0.01) {
        entry.opacity = entry.targetOpacity;
      } else {
        hasActiveAnimations = true;
      }

      // Smoothly animate thumb width (4px <-> 7px)
      const targetWidth = isExpanded ? THUMB_EXPANDED_WIDTH : THUMB_NORMAL_WIDTH;
      entry.thumbW += (targetWidth - entry.thumbW) * 0.28;
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

      const isDoc = (el === document.documentElement || el === document.body);
      const rect = getContainerRect(el);
      if (rect.bottom <= 0 || rect.top >= window.innerHeight || rect.height <= 10) continue;

      const clientH = isDoc ? window.innerHeight : el.clientHeight;
      const scrollH = isDoc ? document.documentElement.scrollHeight : el.scrollHeight;
      const scrollT = isDoc ? (window.scrollY || document.documentElement.scrollTop) : el.scrollTop;

      const maxScroll = Math.max(1, scrollH - clientH);
      const trackH = Math.min(rect.height, clientH);
      const thumbH = Math.max(THUMB_MIN_HEIGHT, Math.round((clientH / scrollH) * trackH));
      const thumbMaxY = Math.max(0, trackH - thumbH);
      const thumbY = rect.top + Math.round((scrollT / maxScroll) * thumbMaxY);

      const thumbW = entry.thumbW;
      const thumbX = rect.right - thumbW - THUMB_EDGE_MARGIN;

      entry.thumbRect = {
        x: rect.right - HOVER_ZONE_PX,
        y: thumbY - 2,
        w: HOVER_ZONE_PX + EDGE_OUTER_TOLERANCE_PX,
        h: thumbH + 4,
        visualY: thumbY,
        visualH: thumbH,
        elRect: rect
      };

      ctx.save();
      ctx.globalAlpha = entry.opacity;

      // Crisp Flat Minimalist fill without thick blurry shadows
      const hoverProgress = Math.max(0, Math.min(1, (thumbW - THUMB_NORMAL_WIDTH) / (THUMB_EXPANDED_WIDTH - THUMB_NORMAL_WIDTH)));
      if (dark) {
        const a = 0.22 + hoverProgress * 0.42;
        ctx.fillStyle = `rgba(255, 255, 255, ${a.toFixed(3)})`;
      } else {
        const r = Math.round(90 - hoverProgress * 45);
        const g = Math.round(105 - hoverProgress * 50);
        const b = Math.round(125 - hoverProgress * 55);
        const a = 0.28 + hoverProgress * 0.44;
        ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${a.toFixed(3)})`;
      }

      drawRoundedPill(ctx, thumbX, thumbY, thumbW, thumbH, thumbW / 2);
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
      scheduleRender();
    }
  }, { capture: true, passive: true });

  // Handle pointer proximity & direct scrollbar hover
  window.addEventListener('pointermove', function (e) {
    if (dragTarget) {
      handleDragMove(e);
      return;
    }

    initCanvas();
    const x = e.clientX;
    const y = e.clientY;

    const el = findScrollableContainerAt(e.target, x, y);
    let changed = false;

    for (const [targetEl, entry] of activeEntries.entries()) {
      if (targetEl !== el && (entry.isNearEdge || entry.isThumbHovered)) {
        entry.isNearEdge = false;
        entry.isThumbHovered = false;
        changed = true;
      }
    }

    if (el && isScrollableY(el)) {
      const rect = getContainerRect(el);
      const isDoc = (el === document.documentElement || el === document.body);
      const clientH = isDoc ? window.innerHeight : el.clientHeight;
      const scrollH = isDoc ? document.documentElement.scrollHeight : el.scrollHeight;
      const scrollT = isDoc ? (window.scrollY || document.documentElement.scrollTop) : el.scrollTop;
      const maxScroll = Math.max(1, scrollH - clientH);
      const trackH = Math.min(rect.height, clientH);
      const thumbH = Math.max(THUMB_MIN_HEIGHT, Math.round((clientH / scrollH) * trackH));
      const thumbMaxY = Math.max(0, trackH - thumbH);
      const thumbY = rect.top + Math.round((scrollT / maxScroll) * thumbMaxY);

      const withinThumbY = y >= thumbY - 3 && y <= thumbY + thumbH + 3;
      const distFromRight = rect.right - x;
      const isNearEdge = y >= rect.top && y <= rect.bottom && distFromRight >= -EDGE_OUTER_TOLERANCE_PX && distFromRight <= PROXIMITY_PX;
      const isThumbHovered = withinThumbY && distFromRight >= -EDGE_OUTER_TOLERANCE_PX && distFromRight <= HOVER_ZONE_PX;

      const entry = getEntry(el);
      if (isNearEdge || isThumbHovered || activeEntries.has(el)) {
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

        dragTarget = el;
        dragStartY = y;
        dragStartScrollTop = (el === document.documentElement || el === document.body) ?
          (window.scrollY || document.documentElement.scrollTop) : el.scrollTop;

        const isDoc = (el === document.documentElement || el === document.body);
        const clientH = isDoc ? window.innerHeight : el.clientHeight;

        dragTrackHeight = Math.min(tr.elRect.height, clientH);
        dragThumbHeight = tr.visualH;
        entry.isDragging = true;
        entry.isThumbHovered = true;
        entry.lastScrollTime = performance.now();
        document.body.classList.add('canvas-scrollbar-dragging');
        scheduleRender();
        return;
      }

      // Hit on track
      const distFromRight = tr.elRect.right - x;
      if (distFromRight >= -EDGE_OUTER_TOLERANCE_PX && distFromRight <= HOVER_ZONE_PX && y >= tr.elRect.top && y <= tr.elRect.bottom) {
        e.preventDefault();
        e.stopPropagation();

        const isDoc = (el === document.documentElement || el === document.body);
        const clientH = isDoc ? window.innerHeight : el.clientHeight;
        const scrollH = isDoc ? document.documentElement.scrollHeight : el.scrollHeight;
        const trackH = Math.min(tr.elRect.height, clientH);
        const maxScroll = Math.max(1, scrollH - clientH);

        const clickOffsetRatio = (y - tr.elRect.top - tr.visualH / 2) / Math.max(1, trackH - tr.visualH);
        const targetScroll = Math.max(0, Math.min(maxScroll, clickOffsetRatio * maxScroll));

        if (isDoc) {
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
    const newScrollTop = Math.max(0, Math.min(maxScroll, dragStartScrollTop + scrollDelta));

    if (isDoc) {
      window.scrollTo(0, newScrollTop);
    } else {
      dragTarget.scrollTop = newScrollTop;
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
    document.body.classList.remove('canvas-scrollbar-dragging');
    scheduleRender();
  }

  window.addEventListener('pointerup', endDrag, { passive: true });
  window.addEventListener('pointercancel', endDrag, { passive: true });

  // Initialize once DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initCanvas);
  } else {
    initCanvas();
  }
})();
