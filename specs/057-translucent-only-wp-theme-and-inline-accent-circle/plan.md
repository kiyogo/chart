# Technical Implementation Plan: Spec 057

1. **`app/src/platform/mod.rs` (`CONTRAST_ENGINE_JS`)**:
   - Gate `useWallpaperTheme` strictly on translucent panels (`alpha <= 0.75`, i.e., `transparency >= 25%`):
     `const useWallpaperTheme = hasWpSeed && alpha <= 0.75 && !hasCustomAccent && !window.__chatwAccentManual;`
   - Ensure `isTranslucentWp` controls button/input/bubble adaptation cleanly so at `0%` transparency (`alpha === 1.0`), ALL dynamic overrides are cleanly removed and the user's chosen `Màu nhấn` renders identically to default.
   - Sync `input[type="color"][data-sync-accent="true"]` back to the user's preset accent whenever `useWallpaperTheme` flips back to `false`.
2. **`app/src/components/settings_view.rs`**:
   - Remove the entire `.accent-custom-row` (`"Màu nhấn tùy chọn"` text + separate row).
   - Add the 7th borderless circular custom color swatch (`span.accent-swatch.accent-swatch-custom` wrapping transparent full-bleed `input[type="color"].accent-swatch-color-input` + `CheckIcon` when active) at the right end of `.accent-swatches`.
   - Gate preset swatch checkmark sync (`wp_overrides_swatch`) on `pr.panel_opacity <= 75` so at `0%` transparency the checkmark accurately stays on the user's chosen preset swatch.
3. **`app/src/components/settings_more.rs`**:
   - Gate `Bong bóng của bạn` default preview wallpaper-seed sync on `pr.panel_opacity <= 75` so at `0%` transparency it stays synced to the user's chosen `Màu nhấn`.
4. **`app/assets/main.css`**:
   - Style `.accent-swatch-custom` and `.accent-swatch-color-input` to render an exact `2.125rem × 2.125rem` borderless circle matching the 6 preset swatches.
5. **E2E Verification (`spec052`, `spec053`, `spec048`, `spec050`)**:
   - Update `spec052` check #4 (use `Màu bong bóng của bạn` for the left `"Mặc định"` check; verify `Hoàng hôn @ 0%` stays Blue `data-wp-theme-active="false"` while `Hoàng hôn @ 72%` switches to Sakura Rose `data-wp-theme-active="true"`).
   - Update `spec053` check #2 to verify 7 borderless circles in `.accent-swatches`, zero `"Màu nhấn tùy chọn"` text, and rightmost circle custom color picker geometry (`width === height`, `border-radius: 50%`, inline right of 6th circle).
