# Spec 066 — Replace All Segmented Button Bars With Modern Web Dropdown Menus

## 1. Problem Statement
Across Settings, Profile Editor, Draw Studio, Sticker/Emoji Studio, and Enterprise views, the app currently uses 19 legacy `.segmented` / `.ent-segmented` horizontal button-bar controls (e.g., `Alert tone`, `Who can send friend requests`, `Who can call you`, `Who can add you to groups`, `List avatar animations`, `Display density`, `Bubble`, `Avatar`, `Message width`, `Font`, `Spreadsheet paste`, `Lock after inactivity`, `When to reply`, plus tabs/toggles in Profile Editor, Library, Avatar Studio, Draw Studio, Studio Tween, HR, Tasks, and Calendar).

The user explicitly requested:
> *"xóa toàn bộ giao diện kiểu như ảnh ở toàn bộ app đi nhé thay bằng tất cả là menu thả của web hiện ại"*

## 2. Goals & Requirements
1. **100% Elimination of Legacy Segmented Controls**:
   - Remove **all** `.segmented` and `.ent-segmented` horizontal pill-bar controls across all 10 Rust UI component files (19 call sites total) and remove obsolete `.segmented` / `.ent-segmented` CSS rules from `app/assets/main.css`.
   - Zero `.segmented` or `.ent-segmented` elements may exist anywhere in the rendered DOM.
2. **Unified Modern Web Dropdown Menu System (`.cw-dropdown` + `custom_pickers.js`)**:
   - Replace every segmented control with a semantic `<select class="cw-dropdown">` (with explicit `selected` sync on `<option>` and accessible `aria_label`), automatically enhanced by `app/web/custom_pickers.js` into an accessible, theme-synchronized modern web popover menu (`.cw-picker-popover.cw-select-menu`) following Radix UI / Shadcn UI / W3C ARIA APG Listbox patterns:
     - **Crisp Modern Trigger**: `appearance: none`, subtle 1px hairline border, refined `10px` radius, clean light/dark/translucent wallpaper surfaces (`>= 4.5:1` WCAG contrast), smooth hover border tint, `var(--accent)` open/focus ring, custom crisp SVG chevron that flips upward while `.cw-picker-open`, and full RTL mirroring (`html[dir="rtl"]`).
     - **Modern Floating Popover (`.cw-picker-popover.cw-select-menu`)**: elevated card surface (`12px` radius, crisp hairline border, refined shadow, `backdrop-filter: blur(16px)`), smooth directional enter animation (`data-side="bottom"` / `data-side="top"`), RTL edge alignment, `translate="no"` + `notranslate` protection, low-opacity translucent `var(--accent-soft)` selected option (`14%–26%` alpha synced with `--accent`), and crisp accent checkmark icon.
     - **Full Keyboard Accessibility (W3C APG Listbox)**: `aria-expanded` on trigger, `ArrowDown` / `ArrowUp` / `Home` / `End` option highlighting (`data-highlighted="true"`), `Enter` / `Space` selection, `Escape` / `Tab` dismissal returning focus to trigger, and prefix typeahead search.
3. **Clean Horizontal Settings Layout**:
   - Convert all Settings option rows that previously stacked vertically with `.settings-row.column-layout` into sleek, compact horizontal `.settings-row` rows (label + optional description on the left, compact `.cw-dropdown` aligned on the right), matching modern web settings UIs (Linear, Apple Settings, Shadcn/Radix).
4. **100% i18n & Canonical English Source Code**:
   - Keep all Rust UI labels strictly in canonical English (`en`) so all 197 languages in `i18n_runtime.js` translate both closed `<select>` triggers and opened `.cw-select-menu` option lists seamlessly, including RTL scripts (`ar`, `he`, `fa`, `ur`).
   - Also clean up any stray untranslated string right next to replaced controls (e.g., `"Khung"` -> `"Frame"` in `studio.rs` if present, keeping existing `_keys.json` exact matches intact).

## 3. Acceptance Criteria
- [ ] `grep -rn "segmented\|ent-segmented" app/src/` returns **0** UI elements.
- [ ] `document.querySelectorAll('.segmented, .ent-segmented').length === 0` across Appearance, Privacy, Notifications, Chat, App Lock, Auto-reply, Sync, Library, and Profile Editor.
- [ ] Every replaced control renders a sleek `<select class="cw-dropdown">` with `appearance: none`, custom SVG chevron, `0.00px` layout shift across wallpaper transparency states, and `>= 4.5:1` contrast in light, dark, and 100% transparent colorful wallpaper modes.
- [ ] Clicking any `.cw-dropdown` opens `.cw-picker-popover.cw-select-menu` (`role="listbox"`) with low-opacity `var(--accent-soft)` selected option (instantly reflecting Blue `#2E7CF6` and Violet `#8B5CF6`), accent checkmark icon, keyboard navigation (`ArrowDown`/`ArrowUp`/`Enter`/`Escape`), and selecting an option immediately updates application state.
- [ ] Switching language (e.g., `vi`, `ar` RTL) translates all dropdown triggers and opened popover options cleanly and mirrors chevron & popover alignment in RTL.
- [ ] `cargo test -p chatw-app --lib` passes 100%, `./app/build-web.sh release` succeeds cleanly, and all Playwright E2E suites pass 100% with visual screenshot verification.
