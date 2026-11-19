/**
 * Panel Resizer Engine
 * Provides butter-smooth, 120fps draggable column resizing for the sidebar, settings nav, and right details panel.
 */

(function () {
  'use strict';

  const CONFIGS = {
    sidebar: {
      selector: '.nav-sidebar',
      cssVar: '--sidebar-width',
      storageKey: 'chatw_sidebar_width',
      fallbackWidth: 272,
      minWidth: 180,
      maxWidth: () => Math.min(500, Math.round(window.innerWidth * 0.45)),
      direction: 1
    },
    'settings-nav': {
      selector: '.settings-nav',
      cssVar: '--settings-nav-width',
      storageKey: 'chatw_settings_nav_width',
      fallbackWidth: 240,
      minWidth: 180,
      maxWidth: () => Math.min(400, Math.round(window.innerWidth * 0.4)),
      direction: 1
    },
    details: {
      selector: '.details-panel, .partner-info-sidebar, .server-members-panel',
      cssVar: '--details-width',
      storageKey: 'chatw_details_width',
      fallbackWidth: 288,
      minWidth: 200,
      maxWidth: () => Math.min(540, Math.round(window.innerWidth * 0.48)),
      direction: -1
    }
  };

  let activeResizer = null;
  let activeConfig = null;
  let startX = 0;
  let startWidth = 0;
  let currentWidth = 0;

  function onPointerDown(e) {
    const handle = e.target.closest('[data-resizer]');
    if (!handle) return;

    const cfg = CONFIGS[handle.getAttribute('data-resizer')];
    if (!cfg) return;

    e.preventDefault();
    e.stopPropagation();

    activeResizer = handle;
    activeConfig = cfg;
    startX = e.clientX;

    const panel = document.querySelector(cfg.selector);
    startWidth = panel ? panel.getBoundingClientRect().width : cfg.fallbackWidth;
    currentWidth = startWidth;

    handle.classList.add('is-dragging');
    document.body.classList.add('resizing-panel');

    try {
      handle.setPointerCapture(e.pointerId);
    } catch (_) {}

    handle.addEventListener('pointermove', onPointerMove);
    handle.addEventListener('pointerup', onPointerUp);
    handle.addEventListener('pointercancel', onPointerUp);
  }

  function onPointerMove(e) {
    if (!activeResizer || !activeConfig) return;

    const dx = (e.clientX - startX) * activeConfig.direction;
    const maxW = activeConfig.maxWidth();
    currentWidth = Math.max(activeConfig.minWidth, Math.min(maxW, Math.round(startWidth + dx)));
    document.documentElement.style.setProperty(activeConfig.cssVar, currentWidth + 'px');
  }

  function onPointerUp(e) {
    if (!activeResizer || !activeConfig) return;

    const handle = activeResizer;
    handle.removeEventListener('pointermove', onPointerMove);
    handle.removeEventListener('pointerup', onPointerUp);
    handle.removeEventListener('pointercancel', onPointerUp);

    try {
      handle.releasePointerCapture(e.pointerId);
    } catch (_) {}

    handle.classList.remove('is-dragging');
    document.body.classList.remove('resizing-panel');

    try {
      if (currentWidth > 0) {
        localStorage.setItem(activeConfig.storageKey, currentWidth + 'px');
      }
    } catch (_) {}

    activeResizer = null;
    activeConfig = null;
  }

  function onDblClick(e) {
    const handle = e.target.closest('[data-resizer]');
    if (!handle) return;

    const cfg = CONFIGS[handle.getAttribute('data-resizer')];
    if (!cfg) return;

    document.documentElement.style.removeProperty(cfg.cssVar);
    try { localStorage.removeItem(cfg.storageKey); } catch (_) {}
  }

  document.addEventListener('pointerdown', onPointerDown, { passive: false });
  document.addEventListener('dblclick', onDblClick, { passive: true });
})();
