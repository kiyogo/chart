# Plan 060 — Remove Wallpaper Auto-Recoloring While Keeping Adaptive Text Readability

## Target Files
1. `app/src/platform/mod.rs`:
   - Disable `useWallpaperTheme` permanently (`const useWallpaperTheme = false;` and remove wallpaper-seed accent override, wallpaper-tinted hover/btn/switch/received-bubble branches).
   - Ensure `effAccentHex` ALWAYS resolves strictly from `c.customAccent` or the user's chosen `data-accent` preset (`blue`/`violet`/`green`/`orange`/`rose`/`cyan`) — never `domHex`.
   - Keep adaptive text luminance (`--text-primary`, `data-surface-tone`, `data-text-halo`, `data-framed-controls`) and clean neutral translucent button/hover surfaces intact.
   - Remove unused `wallpaper_seed_hex` and `mark_manual_accent` helpers (or replace with `accent_preset_hex`).
2. `app/src/components/settings_view.rs`:
   - Remove `manual_accent`, `last_wp_seen`, `wp_matched_accent`, and `wp_overrides_swatch` logic so the 6 preset swatches + 7th circular custom swatch strictly reflect `appearance().accent` and `prefs.custom_accent`.
3. `app/src/components/settings_more.rs`:
   - In `AppearanceColorsShapes`, derive default `self_val` from the user's chosen accent (`custom_accent` or `platform::load_appearance().accent`), never `wallpaper_seed_hex`.
   - In `AppearanceWallpapersGlass`, stop clearing `p.custom_accent` when selecting `"Không"`, any preset wallpaper, solid wallpaper color, or uploading a wallpaper image; default `wp_color` to `pr.app_wallpaper.strip_prefix("color:").unwrap_or("#60A5FA")`.
4. `app/e2e/spec052-visual-and-contrast-audit.mjs` & `app/e2e/spec059-glass-stacking-popovers.mjs`:
   - Update Spec 052's assertion from "Material You wallpaper recoloring" to verify the new Spec 060 invariant: wallpapers NEVER change `--accent` / active pills / bubbles at `0%`, `52%`, `72%`, or `100%` transparency, AND user custom accent persists across wallpaper switches!
   - Rebuild `./app/build-web.sh release`, run E2E suites + inspect screenshots visually, commit & push to `origin main`.
