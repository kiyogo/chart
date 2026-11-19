# Technical Plan - Canvas Overlay Scrollbars

## Architecture & Design

### 1. Canvas Scrollbar Engine (`app/web/canvas_scrollbars.js`)
A lightweight, zero-dependency, high-performance module loaded during page bootstrap:
- **Element Observation**:
  - Automatically identifies scrollable containers: checks elements with `scrollHeight > clientHeight + 2` (or `scrollWidth > clientWidth + 2`), or elements with CSS `overflow-y: auto / scroll`.
  - Uses `MutationObserver` and global delegated listeners so dynamically mounted components (channels, modals, chat messages) automatically receive canvas scrollbars without manual component wrapping.
- **Canvas Overlay Architecture**:
  - Dynamically creates or maintains a high-DPI `<canvas class="canvas-scrollbar-overlay">` aligned to the right edge (for vertical scroll) or bottom edge (for horizontal scroll) of the container.
  - Position: `position: absolute; right: 2px; top: 0; pointer-events: auto; z-index: 99;`.
  - Ensures parent container has `position: relative` (or calculates overlay positioning relative to the bounding client rect if parent cannot be relative).
- **Proximity & Scroll Logic**:
  - **Proximity (`mousemove` / `pointermove`)**:
    - Calculates distance between pointer `clientX` and container's right bounding edge `rect.right`.
    - If `distance <= 24px` and within vertical bounds `[rect.top, rect.bottom]`: triggers fade-in to ~0.75 opacity.
    - If mouse moves away (> 24px): triggers fade-out after short delay (~250ms).
  - **Scroll (`scroll` capture listener)**:
    - On scroll event: instantly animates opacity to 1.0.
    - Resets debounce timer (800ms) to fade out back to 0.0.
  - **Dragging (`pointerdown`, `pointermove`, `pointerup`)**:
    - Hit-tests if pointer clicked on thumb rectangle.
    - When dragging: sets `element.scrollTop = (targetThumbTop / trackHeight) * (scrollHeight - clientHeight)`.
    - Retains 1.0 opacity while pointer is down.
- **Rendering & Animation**:
  - Renders smooth pill with `ctx.roundRect` or rounded caps.
  - Renders smooth opacity fade via `requestAnimationFrame` loop.
  - Respects dark and light theme colors: detects `data-theme="dark"` or computes CSS color variable `--accent` / `--text-secondary`.

### 2. Global CSS Adjustments (`app/assets/main.css`)
- Remove overly broad `*:hover::-webkit-scrollbar-thumb`.
- Hide native webkit/firefox scrollbars or make them completely transparent so the canvas scrollbar is the sole visual indicator.
- Maintain native scroll mechanics (`overflow-y: auto`) so touchpad pinch/kinetic scroll and mouse wheels work with 100% native performance.

### 3. Build & Packaging Integration
- Add `app/web/canvas_scrollbars.js` to `app/build-web.sh` (copy to `app/dist/canvas_scrollbars.js`).
- Reference `<script src="/canvas_scrollbars.js?v=..."></script>` in `app/web/index.html`.

### 4. Verification
- Verify that hovering in the center of content does NOT reveal the scrollbar.
- Verify that hovering within 24px of the scroll edge reveals the canvas scrollbar.
- Verify that scrolling reveals the canvas scrollbar, and stopping hides it after 800ms.
- Verify that clicking/dragging the canvas scrollbar scrolls the list.
- Automated tests and git push.
