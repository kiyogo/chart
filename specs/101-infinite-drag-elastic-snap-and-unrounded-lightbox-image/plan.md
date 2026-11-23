# Spec 101 Technical Plan: Infinite Drag with Elastic Snap-Back and True Unrounded Lightbox Image

## Architectural Plan

1. **Update `app/src/components/rich.rs`**:
   - In `GlobalImageLightbox`:
     - Allow dragging to start at any zoom level (remove `if zoom() > 1.01` restriction in `onmousedown`).
     - Remove `rubber_band_axis` and its 40px cap. During `onmousemove`, set `pan_x = opx + dx` and `pan_y = opy + dy` directly for 100% fluid, infinite drag in any direction.
     - On `finish_drag_and_snap()`, calculate the target clamped position with `clamp_lightbox_pan(pan_x, pan_y, ...)` and set `pan_x` and `pan_y`. When `is-panning` is cleared, CSS `transition: transform 300ms cubic-bezier(0.22, 1, 0.36, 1)` smoothly springs/glides the image back to place.
     - Set `can_pan` to `true` by default so cursor is `grab` when idle, and `grabbing` when actively dragging.
     - Ensure clicking the backdrop still closes the lightbox if `did_drag` is false.

2. **Update `app/assets/main.css`**:
   - Update `.lightbox-img`:
     - `border-radius: 0 !important;` (remove `border-radius: 6px;`).
     - `transition: transform 300ms cubic-bezier(0.22, 1, 0.36, 1);` for smooth, physical snap-back.
     - `cursor: grab;` when idle.

3. **Recompile WASM & Verification**:
   - Run `cargo check -p chatw-app --target wasm32-unknown-unknown`.
   - Run `./app/build-web.sh release`.
   - Implement Playwright test `app/e2e/spec101-infinite-drag-and-unrounded-lightbox.mjs`.
   - Verify drag tracking, snap-back coordinates, and computed `border-radius: 0px`.
