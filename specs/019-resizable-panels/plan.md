# Technical Plan - Resizable UI Panels

## Architecture & Design

### 1. CSS & Layout Architecture (`app/assets/main.css`)
- Root variables:
  ```css
  :root {
    --sidebar-width: 17rem;
    --details-width: 18rem;
  }
  ```
- Resizer Handle Styling (`.panel-resizer`):
  - `.panel-resizer`: `width: 6px; margin: 0 -3px; cursor: col-resize; position: relative; z-index: 20; flex-shrink: 0; user-select: none; touch-action: none;`
  - Inner visual indicator: pseudo-element `::after` with `width: 2px; height: 100%; background: transparent; transition: background 0.15s ease;`
  - On `:hover` or `.is-dragging`: `::after` turns `var(--accent)`.
  - While dragging: body gets `cursor: col-resize` and `user-select: none`.
- Update `.details-panel.server-members-panel`:
  - Set `width: var(--details-width);` so server member panel resizes identically to DM details panel.

### 2. Panel Resizer Engine (`app/web/panel_resizer.js`)
- Universal, event-delegated resizer listener on `document`:
  - Listens to `pointerdown` on `[data-resizer]`.
  - Captures pointer with `setPointerCapture(e.pointerId)`.
  - Tracks delta from start position:
    - Left resizer (`data-resizer="sidebar"`): calculates `newWidth = startWidth + (e.clientX - startX)`. Clamps between `180px` and `min(480px, window.innerWidth * 0.45)`.
    - Right resizer (`data-resizer="details"`): calculates `newWidth = startWidth - (e.clientX - startX)`. Clamps between `200px` and `min(520px, window.innerWidth * 0.45)`.
  - Updates `--sidebar-width` or `--details-width` on `document.documentElement.style`.
  - On `pointerup`: persists width in `localStorage.setItem('chatw_sidebar_width', width + 'px')` and `localStorage.setItem('chatw_details_width', width + 'px')`.
  - On `dblclick`: resets to default `17rem` / `18rem` and clears localStorage entry.

### 3. Bootstrap Persistence (`app/web/boot.js`)
- In `boot.js`, immediately load saved widths before WASM boots:
  ```javascript
  const sw = get('chatw_sidebar_width', '');
  if (sw) root.style.setProperty('--sidebar-width', sw);
  const dw = get('chatw_details_width', '');
  if (dw) root.style.setProperty('--details-width', dw);
  ```

### 4. Dioxus Template Integration (`app/src/pages/chat.rs`)
- Insert `<div class="panel-resizer resizer-left" data-resizer="sidebar" title="Kéo để thay đổi độ rộng danh sách"></div>` right after the navigation sidebar.
- Insert `<div class="panel-resizer resizer-right" data-resizer="details" title="Kéo để thay đổi độ rộng chi tiết"></div>` right before the right panel (`DmDetailsPanel` and `ServerMembersPanel`).

### 5. Packaging & Verification
- Include `panel_resizer.js` in `app/build-web.sh` and `index.html`.
- Run tests and compile release bundle.
- Verify in browser with browser subagent.
- Git commit and push to `origin main`.
