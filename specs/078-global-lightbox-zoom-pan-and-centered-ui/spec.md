# Spec 078 — Global Root-Portal Image Lightbox Viewer (Pixel-Centered, Telegram/Preview UI, Zoom, Pan & Rotate)

## Problem Statement
1. **Off-Center Image Placement**: `.lightbox` was rendered inside `.rich-image` inside `.message-row` inside `.chat-messages`, causing CSS cascade collisions (`.rich-image img`), asymmetric top padding (`4.5rem`), and layout/ancestor interference that pushed images visibly off-center.
2. **Dated Lightbox UI**: Chunky bordered buttons floating over a flat gray backdrop with no clean header, no image metadata, and no modern floating control dock.
3. **Missing Zoom, Pan & Rotate**: Users could not zoom in, zoom out, scroll-wheel zoom, double-click zoom, drag-to-pan zoomed photos, or rotate images.

## Architecture & Acceptance Criteria
1. **Root-Portal Global Lightbox (`GLOBAL_LIGHTBOX` mounted at root of `ChatPage`)**:
   - Completely decoupled from `.rich-image`, `.message-row`, `.message-stream`, and `.chat-column`.
   - Full-viewport backdrop (`position: fixed; inset: 0; width: 100vw; height: 100vh; z-index: 9999`) with deep cinema frosted glass (`rgba(8, 10, 15, 0.90)` + `backdrop-filter: blur(22px)`).
   - Stage uses pure symmetric `display: flex; align-items: center; justify-content: center; padding: 0;` — mathematical horizontal & vertical center error **≤ 2 px** (`|imgCenterX - vw/2| <= 2` and `|imgCenterY - vh/2| <= 2`).
2. **Full Interactive Zoom, Pan & Rotate**:
   - Zoom range `25%` – `600%` (`0.25x` – `6.0x`):
     - Bottom frosted-glass floating dock with **Zoom out (`−`)**, clickable live percentage reset pill (**`100%`**), **Zoom in (`+`)**, divider, and **Rotate 90° (`↻`)**.
     - Smooth **Mouse Wheel zoom** directly over the stage (prevents background chat scrolling).
     - **Double-click** toggles smoothly between `100%` and `200%`.
     - **Pointer Drag-to-Pan** when `zoom > 1.0` (60fps zero-lag `translate3d` with `grab` / `grabbing` cursor).
     - **Keyboard shortcuts**: `+`/`=` (zoom in), `-` (zoom out), `0` (reset 100%), `r`/`R` (rotate 90°), `Escape` (close).
3. **Modern Minimalist Floating Topbar**:
   - Subtle top gradient fade with clean filename & dimensions/size metadata on the left and borderless translucent glass **Download** + circular **Close** buttons on the right (non-overlapping, keeping `.lightbox-actions .btn-secondary` & `.lightbox-close` selectors intact).
4. **100% i18n Across All 197 World Languages**:
   - All lightbox tooltips and aria-labels (`"Zoom in"`, `"Zoom out"`, `"Reset zoom"`, `"Rotate image"`) translated across all 197 language packs.
