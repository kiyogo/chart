# Spec 122: Transparent Focus Reticle Icon with Dark Diffusion Shadow and Clean Zero-Grey Hover

## 1. Problem Statement & User Intent
The user requested:
> "xóa cho tôi cái hiệu ứng khi di trỏ chuột vô ,xámnền xám + xóa nền khi di tro chuột đi đổi thành icon "Focus" và màu trawgs + hiện mờ đen ở dưới icon ,... hiện đẹp cho ôi nhé"

Accompanied by an uploaded screenshot showing the chat image corner button (`media_1795531480019_d1738739.png`) with the cursor hovering over a circular blue sparkle badge, along with earlier screenshots showing muddy grey hover capsules on settings and channel navigation rows.

### Key Requirements
1. **Remove Background (Idle State - "xóa nền khi di trỏ chuột đi")**:
   - The corner enhancement button on image thumbnails (`.rich-image-enhance-corner`) must have NO solid circular or translucent badge background (`background: transparent !important; border: none; box-shadow: none;`).
2. **Remove Grey Hover Effect ("xóa cho tôi cái hiệu ứng khi di trỏ chuột vô ,xámnền xám")**:
   - When hovering over the corner button (`:hover`), it must NOT have any grey or blue background box (`background: transparent !important;`).
   - When hovering over general interactive items across the app (settings nav items, channel rows, etc.), eliminate the dark muddy grey slate rectangle (`rgba(15, 23, 42, 0.065)`), replacing it with a clean, subtle luminous white sheen or transparent frosted highlight that never appears as a dirty grey patch over wallpapers and themes.
3. **Change to "Focus" Icon ("đổi thành icon 'Focus'")**:
   - Replace the sparkle star icon (`EnhanceToggleIcon`) in `.rich-image-enhance-corner` with a standard Lucide-style "Focus" viewfinder / reticle icon (`FocusIcon`): 4 corner brackets (`⌜ ⌝ ⌞ ⌟`) and a center circle reticle.
4. **Pure White Icon Color ("màu trắng")**:
   - The Focus icon SVG must be rendered in crisp pure white (`#FFFFFF` / `color: #ffffff !important; stroke: #ffffff;`).
5. **Dark Blur / Shadow Under Icon ("hiện mờ đen ở dưới icon")**:
   - Apply a diffused dark drop-shadow directly under the icon lines (`filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.85)) drop-shadow(0 0 6px rgba(0, 0, 0, 0.75));`) to ensure pristine legibility, contrast, and visual elegance across white, light, colorful, or dark images.
6. **Smooth Tactile Micro-Interaction ("hiện đẹp cho tôi nhé")**:
   - On hover, provide a smooth, subtle scale animation (`transform: scale(1.15)`) with ease transition, retaining 100% transparent background and zero box clipping.

## 2. Requirements & Acceptance Criteria
- [x] Dedicated vector `FocusIcon` component created in `app/src/components/icons.rs` with `circle { cx: "12", cy: "12", r: "3" }` and 4 corner bracket paths.
- [x] `app/src/components/rich.rs` imports and renders `FocusIcon` inside `.rich-image-enhance-corner`.
- [x] CSS class `.rich-image-enhance-corner` has `background: transparent !important;`, `color: #ffffff !important;`, and dark drop-shadow filter.
- [x] CSS selector `.rich-image-enhance-corner:hover` maintains `background: transparent !important;`, `color: #ffffff !important;`, and `transform: scale(1.15);`.
- [x] Global `--bg-hover` in light mode updated to a clean white frosted sheen (`rgba(255, 255, 255, 0.18)` or theme accent sheen), avoiding muddy grey boxes on wallpaper surfaces.
- [x] E2E test suite `app/e2e/spec122-corner-focus-icon.mjs` validates transparency, white color, drop shadow, and hover behavior.
