# Technical Plan (024)

## 1. Canvas Scrollbars (`app/web/canvas_scrollbars.js` & `app/assets/main.css`)
- Adjust constants:
  - `THUMB_NORMAL_WIDTH = 3` (sleek thin pill)
  - `THUMB_EXPANDED_WIDTH = 8` (clear hover/drag expansion)
  - `THUMB_EDGE_MARGIN = 3`
  - `PROXIMITY_PX = 28` (reveal thin 3px bar near edge)
  - `HOVER_ZONE_PX = 14` (expand to 8px when cursor is directly on the scrollbar strip or over `thumbRect`)
- Add `thumbW` to entry state and smoothly interpolate toward `targetWidth` (`8` when `isThumbHovered || isDragging`, else `3`).
- Remove `shadowBlur = 4` halo that blurred 3px pills into thick bands; use crisp Flat Minimalist colors for light and dark themes.
- Replace single-target walk-up with `findScrollableContainerAt(target, x, y)` using `document.elementsFromPoint(x, y)` + active entries bounding-box check (`distFromRight >= -3 && distFromRight <= PROXIMITY_PX`) so overlapping `.panel-resizer` or sibling elements at panel edges never block scrollbar hover.
- Add `pointerleave`, `blur`, and `canvas-scrollbar-dragging` body class cleanup, plus timeout-driven fade scheduling instead of spinning RAF while idle.

## 2. Toggle Switches (`app/assets/main.css`)
- Update `input[type="checkbox"].switch`:
  - Lock `--sw-w: 44px; --sw-h: 24px; --sw-pad: 3px; --sw-knob: 18px;`
  - Set `width`, `min-width`, `max-width`, `height`, `min-height`, `max-height`, `flex: 0 0 var(--sw-w)`, `box-sizing: border-box`, `padding: 0`, `margin: 0`, `border: none`.
- Update `input[type="checkbox"].switch::after`:
  - Use `top: 50%; left: var(--sw-pad); width: var(--sw-knob); height: var(--sw-knob); transform: translate3d(0, -50%, 0);`
  - Checked state: `transform: translate3d(calc(var(--sw-w) - var(--sw-knob) - var(--sw-pad) * 2), -50%, 0);`
  - Balanced subtle shadow (`0 0 1px rgba(0, 0, 0, 0.14), 0 1px 2px rgba(0, 0, 0, 0.14)`) so visual center matches geometric center.
- Guard `.ent-toolbar > input:not([type="checkbox"])` and `.ent-btn-group > input:not([type="checkbox"])` so checkboxes/switches inside enterprise containers are never stretched.
