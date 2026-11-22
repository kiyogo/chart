# Feature Specification: Translucent-Only Dynamic Wallpaper Theming & Inline Circular Custom Accent Swatch (Spec 057)

**Feature Branch**: `main`
**Created**: 2026-11-22
**Status**: Completed

## 1. User Requirements (Images 1 & 2)

1. **Dynamic Wallpaper Color Adaptation ONLY When Panels Are Actually Translucent (Image 1)**:
   - When `Độ trong suốt khung trắng` is `0%` (or `< 25%`, i.e., `panel_opacity > 75%` — panels are solid/opaque white, ash, dark, or OLED and the wallpaper does not bleed into the panel surface), the app MUST keep the user's chosen `Màu nhấn` (`blue`, `violet`, `green`, `orange`, `rose`, `cyan`, or custom color) 100% as before.
   - Dynamic Material You wallpaper theming (`useWallpaperTheme`) activates ONLY when `panel_opacity <= 75%` (`transparency >= 25%`) over a chromatic/photographic wallpaper where the wallpaper actually shines through the translucent/blurred panels.
   - Dragging transparency back down below `25%` immediately restores the user's chosen `Màu nhấn` across all variables and color inputs.

2. **Inline Circular Custom Color Swatch at Right End of `Màu nhấn` (Image 2)**:
   - Remove the `"Màu nhấn tùy chọn"` text label and separate row completely from Settings -> Appearance.
   - Place the custom accent color picker directly at the right end of `.accent-swatches` as a 7th identical borderless circle (`2.125rem × 2.125rem`, `border-radius: 50%`), matching the exact size and geometry of the 6 preset circles.
   - When unset (`custom_accent.is_empty()`): renders a clean conic rainbow gradient circle; clicking opens the theme-synced custom color picker popover right below the circle.
   - When set (`!custom_accent.is_empty()`): fills solidly with the selected custom hex color and shows the same centered white checkmark (`CheckIcon`).

## 2. Acceptance Criteria

1. With an uploaded image or `Hoàng hôn` wallpaper at `Độ trong suốt khung trắng: 0%`, `data-wp-theme-active="false"` and `--accent` / active pills remain the user's chosen `Màu nhấn` (e.g., Blue `#2E7CF6`).
2. Increasing `Độ trong suốt khung trắng` to `>= 25%` (e.g., `52%`, `72%`, `100%`) over `Hoàng hôn` / `Bạc hà` / `Cực quang` / uploaded photo activates `data-wp-theme-active="true"` and harmonizes the translucent UI to the wallpaper theme.
3. No `"Màu nhấn tùy chọn"` text renders anywhere in Appearance; `.accent-swatches` contains 7 equal-size borderless circles in one row, with the custom color picker `<input type="color">` integrated into the 7th rightmost circle.
4. All 4 Playwright E2E suites (`spec048`, `spec050`, `spec052`, `spec053`) pass 100% with 0 page errors.
