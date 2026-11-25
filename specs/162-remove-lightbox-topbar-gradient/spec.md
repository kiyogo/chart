# Spec 162 — Remove Lightbox Topbar Horizontal Gradient Seam

## Problem
When opening and zooming an image in the Lightbox (`.lightbox`), `.lightbox-topbar` renders a `76px` tall `linear-gradient(...)` backdrop (`rgba(8, 10, 14, 0.72)` -> `0.34` -> `0`). Over bright areas of a zoomed photo, the abrupt gradient termination at `76px` creates a visible horizontal straight bright edge/bar ("thanh thẳng sáng") cutting across the image.

## Acceptance Criteria
1. `.lightbox-topbar` uses `background: transparent !important;` with no `linear-gradient`, no border, and no box-shadow, so zero horizontal bar or brightness seam appears across zoomed images.
2. Filename text, zoom percentage readout, and toolbar SVG icons remain legible over bright photos via subtle text/drop shadows only.
3. Web WASM bundle builds cleanly (`./app/build-web.sh`), daemon serves the updated bundle, and changes are committed & pushed to `origin main`.
