/**
 * Panel Resizer Engine
 * Provides butter-smooth, 120fps draggable column resizing for the sidebar and right details panel.
 */

(function () {
  'use strict';

  let activeResizer = null;
  let resizerType = null;
  let startX = 0;
  let startWidth = 0;
  let currentWidth = 0;

  function onPointerDown(e) {
    const handle = e.target.closest('[data-resizer]');
    if (!handle) return;

    e.preventDefault();
    e.stopPropagation();

    activeResizer = handle;
    resizerType = handle.getAttribute('data-resizer');
    startX = e.clientX;

    // Determine current panel width
    if (resizerType === 'sidebar') {
      const sidebar = document.querySelector('.nav-sidebar');
      startWidth = sidebar ? sidebar.getBoundingClientRect().width : 272;
    } else if (resizerType === 'details') {
      const panel = document.querySelector('.details-panel, .partner-info-sidebar, .server-members-panel');
      startWidth = panel ? panel.getBoundingClientRect().width : 288;
    } else {
      return;
    }

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
    if (!activeResizer) return;

    const dx = e.clientX - startX;
    const maxSidebar = Math.min(500, Math.round(window.innerWidth * 0.45));
    const maxDetails = Math.min(540, Math.round(window.innerWidth * 0.48));

    if (resizerType === 'sidebar') {
      currentWidth = Math.max(180, Math.min(maxSidebar, Math.round(startWidth + dx)));
      document.documentElement.style.setProperty('--sidebar-width', currentWidth + 'px');
    } else if (resizerType === 'details') {
      currentWidth = Math.max(200, Math.min(maxDetails, Math.round(startWidth - dx)));
      document.documentElement.style.setProperty('--details-width', currentWidth + 'px');
    }
  }

  function onPointerUp(e) {
    if (!activeResizer) return;

    const handle = activeResizer;
    handle.removeEventListener('pointermove', onPointerMove);
    handle.removeEventListener('pointerup', onPointerUp);
    handle.removeEventListener('pointercancel', onPointerUp);

    try {
      handle.releasePointerCapture(e.pointerId);
    } catch (_) {}

    handle.classList.remove('is-dragging');
    document.body.classList.remove('resizing-panel');

    // Persist user preference
    try {
      if (resizerType === 'sidebar' && currentWidth > 0) {
        localStorage.setItem('chatw_sidebar_width', currentWidth + 'px');
      } else if (resizerType === 'details' && currentWidth > 0) {
        localStorage.setItem('chatw_details_width', currentWidth + 'px');
      }
    } catch (_) {}

    activeResizer = null;
    resizerType = null;
  }

  function onDblClick(e) {
    const handle = e.target.closest('[data-resizer]');
    if (!handle) return;

    const type = handle.getAttribute('data-resizer');
    if (type === 'sidebar') {
      document.documentElement.style.removeProperty('--sidebar-width');
      try { localStorage.removeItem('chatw_sidebar_width'); } catch (_) {}
    } else if (type === 'details') {
      document.documentElement.style.removeProperty('--details-width');
      try { localStorage.removeItem('chatw_details_width'); } catch (_) {}
    }
  }

  document.addEventListener('pointerdown', onPointerDown, { passive: false });
  document.addEventListener('dblclick', onDblClick, { passive: true });
})();
