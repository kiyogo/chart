# Plan 066 — Architecture & File-by-File Implementation Plan

## Open-Source Design Reference
Following **Radix UI Select**, **Shadcn UI Select**, and **W3C ARIA APG Listbox** patterns:
- Trigger acts as accessible control with `aria-expanded`, `appearance: none`, custom chevron SVG rotating/flipping on open, subtle 1px border, and focus halo (`var(--accent)`).
- Popover portal attached to `document.body` (`role="listbox"`, `data-side="bottom" | "top"`, `dir` mirrored from `<html>`, `translate="no"`, `.notranslate`) with keyboard highlighting (`data-highlighted="true"`), `ArrowDown`/`ArrowUp`/`Home`/`End`/`Enter`/`Space`/`Escape`/typeahead support, smooth 110ms scale+translate entrance, low-opacity `var(--accent-soft)` selected background, and accent checkmark indicator.

## Files to Modify
1. **`app/web/custom_pickers.js`**:
   - Upgrade `createPopover`, `positionPopover`, `closePopover`, `openSelect`, and global `keydown` handler:
     - Set `translate="no"`, `.notranslate`, and `dir` (inherited from `document.documentElement.dir || 'ltr'`) on `.cw-picker-popover`.
     - Set `aria-expanded="true"` / `"false"` on the anchor `<select>`.
     - Compute `data-side="top"` vs `data-side="bottom"` and RTL-aware horizontal positioning (`r.right - pr.width` when RTL).
     - Add full keyboard navigation (`ArrowDown`, `ArrowUp`, `Home`, `End`, `Enter`, `Space`, `Tab`, `Escape`) and 500ms typeahead buffer on open `.cw-select-menu` popovers, highlighting options with `data-highlighted="true"` and keeping them scrolled into view.
     - On `ArrowDown`/`ArrowUp` when a closed `<select>` has focus, open the custom menu cleanly instead of showing native OS chrome.
2. **Rust UI Components (10 files, 19 sites — ALL `.segmented` & `.ent-segmented` replaced with `<select class="cw-dropdown">`)**:
   - `app/src/components/settings_more.rs`:
     - Replace `Segmented` component with `SelectMenu` (keeping props `(options, value, on_change, label)` cleanly) rendering `<select class="cw-dropdown" aria_label: "{label}", value: "{value}", onchange: ..., for (v, l) in options { option { key: "{v}", value: "{v}", selected: value == v, "{l}" } }>`.
     - Convert all 12 Settings rows (`Spreadsheet paste`, `Alert tone`, `Who can send friend requests`, `Who can call you`, `Who can add you to groups`, `Lock after inactivity`, `When to reply`, `Bubble`, `Avatar`, `Message width`, `Font`, `List avatar animations`) from stacked `.settings-row.column-layout` to crisp horizontal `.settings-row` (and move `AutoReplySettings`'s conditional `From`/`To` `inline-form` right below the horizontal row).
   - `app/src/components/settings_view.rs`:
     - Replace `Display density` `.segmented` (line 493) with `<select class="cw-dropdown" aria_label: "Display density", ...>`.
   - `app/src/components/profile_modal.rs`:
     - Replace Edit Profile `.segmented` tab bar (line 192) with `<div class="cw-dropdown-bar"><select class="cw-dropdown" aria_label: "Section", ...></div>`.
   - `app/src/components/settings_extra.rs`:
     - Replace Library `.segmented` tab bar (line 774) with `<select class="cw-dropdown" aria_label: "Library", ...>`.
   - `app/src/components/avatar_motion.rs`:
     - Replace `Direction` (line 286) and `Activate` (line 295) `.segmented` controls with `<select class="cw-dropdown">` (and add `class: "cw-dropdown"` to `Movement type` right below for 100% visual unity!).
   - `app/src/components/studio.rs`:
     - Replace tween mode `.segmented` (line 689) with `<select class="cw-dropdown">` (and keep `"Motion Curve"` select unified).
   - `app/src/components/draw_studio.rs`:
     - Replace drawing tool `.segmented` (`Pen`, `Eraser`, `Star sign`, line 171) with `<select class="cw-dropdown" aria_label: "Pen", ...>`.
   - `app/src/components/enterprise/hr_dept.rs`:
     - Replace `ent-segmented` view toggle (line 606) with `<select class="cw-dropdown">`.
   - `app/src/components/enterprise/tasks_projects.rs`:
     - Replace `ent-segmented` view mode (line 162) and business operation action selector (line 737) with `<select class="cw-dropdown">`.
   - `app/src/components/enterprise/time_leave.rs`:
     - Replace both `ent-segmented` horizon and filter bars (lines 617, 622) with `<select class="cw-dropdown">`.
3. **`app/assets/main.css` & `app/web/adaptive_surface.js` & `app/src/platform/mod.rs`**:
   - Remove ALL obsolete `.segmented` and `.ent-segmented` CSS rules and stale comments.
   - Add a refined, unified **Spec 066 Modern Web Dropdown System** for `select`, `.cw-dropdown`, `.settings-select`, `.cw-picker-popover.cw-select-menu`, `.cw-select-option`:
     - Light opaque, dark opaque, light translucent wallpaper, dark translucent wallpaper trigger surfaces with `>= 4.5:1` contrast and 0.00px size stability.
     - Crisp inline SVG chevron (down when closed, flipped up when `.cw-picker-open`; dark/light stroke polarity matched; RTL mirrored padding & background-position).
     - Sleek `.settings-row > select.cw-dropdown` right-aligned sizing (`min-width: 10.25rem; max-width: 16rem; width: auto; flex: 0 0 auto;`).
     - Modern elevated `.cw-picker-popover.cw-select-menu` with smooth `data-side="bottom"`/`data-side="top"` entrance animation, `var(--accent-soft)` low-opacity selected highlight, `data-highlighted="true"` keyboard hover tint, and `var(--accent)` checkmark icon.
4. **E2E Verification Suites**:
   - Update `app/e2e/spec052-visual-and-contrast-audit.mjs` and `app/e2e/spec062-i18n-all-33-languages-and-unified-ui.mjs` to assert:
     - Zero `.segmented` / `.ent-segmented` nodes anywhere in Settings tabs.
     - Modern `.cw-dropdown` triggers have 0.00px size stability across wallpaper transparency, `>= 4.5:1` contrast, custom chevron (`appearance: none`).
     - Clicking `.cw-dropdown` opens `.cw-picker-popover.cw-select-menu` with `role="listbox"`, low-opacity translucent `var(--accent-soft)` selected option (Blue & Violet accent sync!), keyboard navigation (`ArrowDown` + `Enter`) updates value cleanly, and Vietnamese + RTL Arabic render translated dropdowns & mirrored chevrons cleanly.
