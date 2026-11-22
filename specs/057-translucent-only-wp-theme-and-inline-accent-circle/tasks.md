# Task List: Spec 057

- [x] T001: Gate `useWallpaperTheme` on `alpha <= 0.75` (`transparency >= 25%`) and restore preset accent cleanly at `0%` transparency in `app/src/platform/mod.rs`.
- [x] T002: Remove `"Màu nhấn tùy chọn"` row and integrate the 7th circular custom accent swatch at the right end of `.accent-swatches` in `app/src/components/settings_view.rs`, `app/src/components/settings_more.rs`, and `app/assets/main.css`.
- [x] T003: Update `app/e2e/spec052-visual-and-contrast-audit.mjs` and `app/e2e/spec053-themes-accents-frameless-header.mjs` to assert translucent-only wallpaper theming and the inline 7th circular custom accent swatch.
- [x] T004: Build web release (`./app/build-web.sh release`), run all 4 Playwright E2E suites (`spec052`, `spec053`, `spec048`, `spec050`), and visually inspect live screenshots of `0%` transparency over `Hoàng hôn` + inline 7-circle `Màu nhấn` row + `72%` transparency over `Hoàng hôn`.
- [x] T005: Mark complete, commit, and push to `origin main`.
