# Spec 080 — Borderless Unified Top-Right Lightbox Toolbar, 100% Min-Zoom Floor & Corner-Clamped Elastic Pan Snap-Back

## Problem Statement
1. **Unwanted Borders & Split Controls**: Top-right buttons (`Tải bản gốc`, `×`) and the separate bottom zoom dock still had visible borders/pill frames instead of a single 100% unified, borderless top-right menu (Telegram Web K style).
2. **Shrinking Too Small**: Allowing zoom down to `25%` let users shrink photos into tiny unreadable thumbnails.
3. **Dragging Zoomed Photos Off-Screen**: Unbounded panning allowed dragging a zoomed photo completely out of the viewport into empty dark void instead of clamping at the image corners and springing back into position.

## Acceptance Criteria
1. **Zero Borders & 100% Unified Top-Right Toolbar**:
   - Remove the bottom `.lightbox-dock` completely.
   - Place all controls (`Zoom out`, `100%`, `Zoom in`, `Rotate`, `Download`, `Close`) in one single borderless top-right `.lightbox-actions` toolbar where every button shares the exact same borderless circular ghost icon styling (`border: none`, `box-shadow: none`, transparent idle -> soft translucent hover fill, crisp white icons).
2. **Minimum Zoom = `100%` (`1.0x`), Maximum Zoom = `400%` (`4.0x`)**:
   - Never shrinks below `100%` via button, mouse wheel, or keyboard `-`.
   - `Zoom out` button is disabled at `100%`.
3. **Exact Corner-Clamped Panning + Automatic Spring Snap-Back**:
   - Pure Rust math helper `clamp_lightbox_pan(px, py, zoom, rotation, base_w, base_h, vw, vh)`:
     - Computes exact effective visual size `(eff_w, eff_h)` (swapping `base_w`/`base_h` on 90°/270° rotation).
     - `max_pan_x = ((eff_w - vw) / 2.0).max(0.0)`, `max_pan_y = ((eff_h - vh) / 2.0).max(0.0)`.
   - During drag: soft rubber-band resistance (`excess * 0.22`, capped at `40px`) prevents losing the photo off-screen even mid-drag.
   - On release (`onmouseup`/`onmouseleave`) AND on zoom/rotate changes: immediately clamps `(pan_x, pan_y)` to `[-max_pan_x..=max_pan_x, -max_pan_y..=max_pan_y]` so CSS smoothly springs the image back so its edges sit flush at the viewport corner (or dead-center when `eff <= viewport`).
4. **Verified via Rust Unit Tests + Live Playwright E2E**:
   - Rust unit tests verify `clamp_lightbox_pan` at 100%, 200%, 90° rotation, portrait/landscape, and extreme overscroll.
   - Live Playwright verifies: zero borders (`borderWidth === '0px'`), zero `.lightbox-dock`, all 6 controls unified in top-right `.lightbox-actions`, zooming out at `100%` stays locked at `100%`, dragging `+2000px, +2000px` at `250%` NEVER loses the image and automatically snaps back flush to the exact top-left viewport corner (`imgLeft <= 2`, `imgTop <= 2`), plus visual screenshot verification.
