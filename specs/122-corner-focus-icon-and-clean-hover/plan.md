# Implementation Plan - Spec 122: Transparent Focus Reticle Icon & Clean Zero-Grey Hover

## 1. Architectural Strategy
The enhancement corner button on chat image thumbnails currently renders a circular frosted dark badge with a 3-star sparkle SVG (`EnhanceToggleIcon`).
To satisfy the user's explicit aesthetic requirements:
1. **Vector Icon**: Add `FocusIcon` to `app/src/components/icons.rs` implementing the classic camera viewfinder / reticle motif: 4 corner alignment brackets (`M3 7V5a2 2 0 0 1 2-2h2`, `M17 3h2a2 2 0 0 1 2 2v2`, `M21 17v2a2 2 0 0 1-2 2h-2`, `M7 21H5a2 2 0 0 1-2-2v-2`) with an inner center reticle circle (`circle { cx: "12", cy: "12", r: "3" }`).
2. **Component Integration**: In `app/src/components/rich.rs`, replace `EnhanceToggleIcon { size: 15 }` with `FocusIcon { size: 18 }`.
3. **Pure Transparent Button with Dark Diffusion Scrim**:
   - Strip all background colors, borders, and backdrop-filters from `.rich-image-enhance-corner`.
   - Set `background: transparent !important;`.
   - Set `color: #ffffff !important;`.
   - Apply layered SVG drop-shadow filter: `drop-shadow(0 2px 4px rgba(0, 0, 0, 0.85)) drop-shadow(0 0 6px rgba(0, 0, 0, 0.75))`.
   - On `:hover`: maintain `background: transparent !important;`, scale up to 1.15x.
4. **Zero-Grey Hover Across App Surfaces**:
   - In `app/assets/main.css`, update light mode `--bg-hover` and translucent surface hovers to use luminous white sheen (`rgba(255, 255, 255, 0.18)`) or low-opacity accent tint instead of muddy grey slate (`rgba(15, 23, 42, 0.065)`).
5. **E2E Automation**: Create `app/e2e/spec122-corner-focus-icon.mjs` with Playwright to verify DOM properties, SVG attributes, and hover styles.

## 2. File Change Plan
| File Path | Action | Description |
|---|---|---|
| `app/src/components/icons.rs` | Edit | Add `FocusIcon` component with reticle and 4 corner brackets. |
| `app/src/components/rich.rs` | Edit | Import `FocusIcon` and render inside `span.rich-image-enhance-corner`. |
| `app/assets/main.css` | Edit | Update `.rich-image-enhance-corner` styles and `--bg-hover` light mode. |
| `app/e2e/spec122-corner-focus-icon.mjs` | Create | E2E validation script. |
| `specs/122-corner-focus-icon-and-clean-hover/tasks.md` | Create | SDD task checklist. |
