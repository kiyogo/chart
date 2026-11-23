# Spec 075: Full-Viewport Image Lightbox Containment Fix & Action Bar Polish

## Problem Statement
Clicking an image in chat opened `.lightbox` trapped inside the single `.message-row` rectangle instead of covering the full viewport (`100vw × 100vh`), and `.lightbox-close` (`position: absolute`) overlapped the `"Download original"` button inside `.lightbox-actions`.

## Root Causes
1. `.message-row { content-visibility: auto; }` and `.message-stream { contain: layout style; }` establish a CSS containment block that traps `position: fixed` descendants (`.lightbox`, `.modal-overlay`).
2. `.lightbox-close` had `position: absolute; top: var(--space-4); right: var(--space-4);` even though it lives inside `.lightbox-actions` (which is already `position: absolute; top: 1rem; right: 1rem;`), causing the close icon and download button to overlap with poor contrast on the dark overlay.

## Acceptance Criteria
- AC1: Clicking any chat image opens `.lightbox` covering the full browser viewport (`top === 0, left === 0, width === innerWidth, height === innerHeight`).
- AC2: Virtual scrolling CSS (`content-visibility: auto` on `.message-row` and `contain: layout style` on `.message-stream`) remains active whenever no lightbox/modal is open.
- AC3: `.lightbox-actions` renders a crisp, high-contrast Download pill and Close (`×`) circular button side-by-side with zero overlap, and clicking backdrop, `×`, or pressing `Escape` closes the lightbox cleanly.
