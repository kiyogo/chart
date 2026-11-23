# Spec 102: Move LGBT Font Option to the End of Font Selection Menu

## Problem Statement
In Settings -> Appearance / More settings (Giao diện / Khác), the font family dropdown (`SelectMenu` with `font_picker: true`) placed the "LGBT" font at the 2nd position, immediately following "Theo hệ thống" (System). The user requested to move "LGBT" to the very end of the list ("lgbt cho xuống cuối").

## Acceptance Criteria
- **AC1**: In the font picker dropdown (`SelectMenu` in `settings_more.rs`), `("lgbt", "LGBT")` is repositioned from the 2nd item to the final item at the bottom of the list (after `dyslexic` / "Readable").
- **AC2**: "Theo hệ thống" (System) remains the first/default item at the top.
- **AC3**: Selecting "LGBT" still works identically, applying `data-font="lgbt"` and styling properly.
- **AC4**: Automated Playwright E2E verification confirms "LGBT" is the last option in the dropdown and can be selected properly.
