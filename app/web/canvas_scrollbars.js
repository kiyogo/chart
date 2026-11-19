/**
 * Canvas Overlay Scrollbars Engine
 * High-performance, floating canvas-based scroll indicator with proximity & scroll detection.
 * 
 * Behaviors:
 * 1. Invisible by default when idle.
 * 2. Reveals when mouse moves close to the scroll edge (within ~24px).
 * 3. Instantly reveals when scrolling (wheel, touch, keyboard, drag) and fades after 800ms.
 * 4. Interactive: draggable thumb with 1:1 scroll synchronization.
 */

(function () {
  'use strict';

  // State
  let overlayCanvas = null;
  let ctx = null;
  let dpr = window.devicePixelRatio || 1;

  // Track active scrollable elements and their animation state
  // Map<HTMLElement, { opacity: number, targetOpacity: number, lastScrollTime: number, isHovered: number, thumbRect: {x,y,w,h} }>
  const activeEntries = new Map();

  let dragTarget = null;
  let dragStartY = 0;
  let dragStartScrollTop = 0;
  let dragTrackHeight = 0;
  let dragThumbHeight = 0;
  let animFrameId = null;

  // Configuration constants
  const PROXIMITY_PX = 24;
  const FADE_TIMEOUT_MS = 800;
  const THUMB_NORMAL_WIDTH = 6;
  const THUMB_EXPANDED_WIDTH = 8;
  const THUMB_MIN_HEIGHT = 28;
  const THUMB_EDGE_MARGIN = 3;

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
    ctx.scale(dpr, dpr);
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

  function findScrollableContainer(target, clientX, clientY) {
    let curr = target;
    while (curr && curr !== document.documentElement) {
      if (isScrollableY(curr)) {
        const rect = curr.getBoundingClientRect();
        if (
          clientY >= rect.top &&
          clientY <= rect.bottom &&
          clientX >= rect.left &&
          clientX <= rect.right
        ) {
          return curr;
        }
      }
      curr = curr.parentElement;
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
        isHovered: false,
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

  function render() {
    animFrameId = null;
    if (!ctx || !overlayCanvas) return;

    ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);

    const now = performance.now();
    const dark = isDarkTheme();
    let hasActiveAnimations = false;

    for (const [el, entry] of activeEntries.entries()) {
      // Check if element is still connected and scrollable
      if (!el.isConnected || !isScrollableY(el)) {
        activeEntries.delete(el);
        continue;
      }

      // Check scroll activity timeout
      const scrollAge = now - entry.lastScrollTime;
      if (entry.isDragging) {
        entry.targetOpacity = 1.0;
      } else if (entry.isHovered) {
        entry.targetOpacity = 0.85;
      } else if (scrollAge < FADE_TIMEOUT_MS) {
        entry.targetOpacity = 0.95;
      } else {
        entry.targetOpacity = 0.0;
      }

      // Animate opacity smoothly
      const speed = entry.targetOpacity > entry.opacity ? 0.22 : 0.08;
      entry.opacity += (entry.targetOpacity - entry.opacity) * speed;
      if (Math.abs(entry.targetOpacity - entry.opacity) < 0.01) {
        entry.opacity = entry.targetOpacity;
      }

      if (entry.opacity <= 0.005) {
        entry.opacity = 0;
        entry.thumbRect = null;
        if (!entry.isHovered && !entry.isDragging && scrollAge >= FADE_TIMEOUT_MS) {
          activeEntries.delete(el);
        }
        continue;
      }

      hasActiveAnimations = true;

      // Compute bounding box and thumb geometry
      const isDoc = (el === document.documentElement || el === document.body);
      const rect = isDoc ?
        { top: 0, right: window.innerWidth, bottom: window.innerHeight, height: window.innerHeight } :
        el.getBoundingClientRect();

      // Skip offscreen elements
      if (rect.bottom <= 0 || rect.top >= window.innerHeight || rect.height <= 10) continue;

      const clientH = isDoc ? window.innerHeight : el.clientHeight;
      const scrollH = isDoc ? document.documentElement.scrollHeight : el.scrollHeight;
      const scrollT = isDoc ? (window.scrollY || document.documentElement.scrollTop) : el.scrollTop;

      const maxScroll = Math.max(1, scrollH - clientH);
      const trackH = Math.min(rect.height, clientH);
      const thumbH = Math.max(THUMB_MIN_HEIGHT, Math.round((clientH / scrollH) * trackH));
      const thumbMaxY = trackH - thumbH;
      const thumbY = rect.top + Math.round((scrollT / maxScroll) * thumbMaxY);

      const isExpanded = entry.isHovered || entry.isDragging;
      const thumbW = isExpanded ? THUMB_EXPANDED_WIDTH : THUMB_NORMAL_WIDTH;
      const thumbX = rect.right - thumbW - THUMB_EDGE_MARGIN;

      // Store thumb rectangle for click and drag hit-testing
      entry.thumbRect = {
        x: thumbX - 4, // Hit-box padding
        y: thumbY,
        w: thumbW + 8,
        h: thumbH,
        elRect: rect
      };

      // Draw subtle shadow for contrast over various backgrounds
      ctx.save();
      ctx.globalAlpha = entry.opacity;

      ctx.shadowColor = dark ? 'rgba(0, 0, 0, 0.45)' : 'rgba(0, 0, 0, 0.15)';
      ctx.shadowBlur = 4;
      ctx.shadowOffsetX = 0;
      ctx.shadowOffsetY = 1;

      // Color selection
      if (dark) {
        ctx.fillStyle = isExpanded ? 'rgba(255, 255, 255, 0.65)' : 'rgba(255, 255, 255, 0.32)';
      } else {
        ctx.fillStyle = isExpanded ? 'rgba(30, 41, 59, 0.65)' : 'rgba(100, 116, 139, 0.38)';
      }

      drawRoundedPill(ctx, thumbX, thumbY, thumbW, thumbH, thumbW / 2);
      ctx.restore();
    }

    if (hasActiveAnimations) {
      animFrameId = requestAnimationFrame(render);
    }
  }

  function scheduleRender() {
    if (!animFrameId) {
      animFrameId = requestAnimationFrame(render);
    }
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

  // Handle pointer proximity ("di trỏ chuột gần vô mới hiện")
  window.addEventListener('pointermove', function (e) {
    if (dragTarget) {
      // Actively dragging
      handleDragMove(e);
      return;
    }

    initCanvas();
    const x = e.clientX;
    const y = e.clientY;

    const el = findScrollableContainer(e.target, x, y);
    let changed = false;

    // Reset hover status for all entries not under cursor
    for (const [targetEl, entry] of activeEntries.entries()) {
      if (targetEl !== el && entry.isHovered) {
        entry.isHovered = false;
        changed = true;
      }
    }

    if (el && isScrollableY(el)) {
      const isDoc = (el === document.documentElement || el === document.body);
      const rect = isDoc ?
        { right: window.innerWidth, top: 0, bottom: window.innerHeight } :
        el.getBoundingClientRect();

      const distFromRight = rect.right - x;
      const isNearEdge = distFromRight >= 0 && distFromRight <= PROXIMITY_PX && y >= rect.top && y <= rect.bottom;

      const entry = getEntry(el);
      if (entry.isHovered !== isNearEdge) {
        entry.isHovered = isNearEdge;
        changed = true;
      }
    }

    if (changed) {
      scheduleRender();
    }
  }, { passive: false });

  // Handle pointer down on thumb (start drag or track click)
  window.addEventListener('pointerdown', function (e) {
    const x = e.clientX;
    const y = e.clientY;

    for (const [el, entry] of activeEntries.entries()) {
      if (!entry.thumbRect || entry.opacity < 0.1) continue;

      const tr = entry.thumbRect;
      // Check hit on thumb
      if (x >= tr.x && x <= tr.x + tr.w && y >= tr.y && y <= tr.y + tr.h) {
        e.preventDefault();
        e.stopPropagation();

        dragTarget = el;
        dragStartY = y;
        dragStartScrollTop = (el === document.documentElement || el === document.body) ?
          (window.scrollY || document.documentElement.scrollTop) : el.scrollTop;

        const isDoc = (el === document.documentElement || el === document.body);
        const clientH = isDoc ? window.innerHeight : el.clientHeight;
        const scrollH = isDoc ? document.documentElement.scrollHeight : el.scrollHeight;

        dragTrackHeight = Math.min(tr.elRect.height, clientH);
        dragThumbHeight = tr.h;
        entry.isDragging = true;
        entry.lastScrollTime = performance.now();
        scheduleRender();
        return;
      }

      // Check hit on track (near edge, above or below thumb)
      const distFromRight = tr.elRect.right - x;
      if (distFromRight >= 0 && distFromRight <= PROXIMITY_PX && y >= tr.elRect.top && y <= tr.elRect.bottom) {
        e.preventDefault();
        e.stopPropagation();

        const isDoc = (el === document.documentElement || el === document.body);
        const clientH = isDoc ? window.innerHeight : el.clientHeight;
        const scrollH = isDoc ? document.documentElement.scrollHeight : el.scrollHeight;
        const trackH = Math.min(tr.elRect.height, clientH);
        const maxScroll = Math.max(1, scrollH - clientH);

        const clickOffsetRatio = (y - tr.elRect.top - tr.h / 2) / Math.max(1, trackH - tr.h);
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
