# Tasks 060 — Disable Automatic Wallpaper-Driven Web Recoloring

- [x] **T1**: Disable wallpaper-driven accent/button/hover/bubble recoloring in `app/src/platform/mod.rs` (`useWallpaperTheme = false`, `effAccentHex` strictly follows user preset/custom accent, clean neutral surfaces, keep adaptive text readability).
- [x] **T2**: Remove wallpaper accent override logic in `app/src/components/settings_view.rs` (`manual_accent`, `wp_matched_accent`, `wp_overrides_swatch`).
- [x] **T3**: Remove `wallpaper_seed_hex` default and stop clearing `p.custom_accent` on wallpaper changes in `app/src/components/settings_more.rs`.
- [x] **T4**: Update `app/e2e/spec052-visual-and-contrast-audit.mjs` to assert that switching wallpapers (Sunset, Aurora, Mint, custom photo) at any transparency (`0%`–`100%`) NEVER alters user accent color or custom accent.
- [x] **T5**: Rebuild `./app/build-web.sh release`, run `spec052` (12/12 PASS), `spec053` (4/4 PASS), `spec059` (12/12 PASS), `modal-overlay-check` (7/7 PASS), and visually verify live screenshots via `read_file`.
- [x] **T6**: Mark tasks complete, commit, and push to `origin main`.
