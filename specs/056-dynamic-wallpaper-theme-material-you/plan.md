# Technical Implementation Plan: Spec 056

1. **`app/src/platform/mod.rs`**:
   - Add `pub fn wallpaper_seed_hex(app_wp: &str) -> Option<String>` helper shared with `settings_view.rs` and `settings_more.rs`.
   - In `CONTRAST_ENGINE_JS`:
     - Track `window.__chatwLastWpSig` and `window.__chatwLastAccentAttr` + `window.__chatwAccentManual` so:
       - Changing wallpaper (`wpSig !== lastWpSig`) resets `__chatwAccentManual = false` (wallpaper theme immediately drives the entire app!).
       - Explicitly changing `data-accent` (clicking a `Màu nhấn` swatch) sets `__chatwAccentManual = true` (user override honored immediately!).
     - Curate vibrant seed `domHex` for all presets (`sunset: #EC4882`, `ocean: #0EA5E9`, `mint: #10B981`, `aurora: #7C5CFF`, `paper: #C97A2B`, `bubbles: #D9468F`, `night: #3B82F6`).
     - Compute `useWallpaperTheme = !c.customAccent && !window.__chatwAccentManual && c.wpKind !== 'none' && c.wpKind !== 'preset-dots' && (wpChroma >= 0.08 || c.wpKind === 'image')`.
     - When `useWallpaperTheme` is active (or `customAccent` is set):
       - Derive full WCAG-checked tonal palette (`--accent`, `--accent-hover`, `--accent-text`, `--on-accent`, `--accent-soft`, `--accent-soft-strong`, `--bg-hover`, `--bg-active`, `--bg-surface-hover`, `--adaptive-btn-bg/hover/active`, `--adaptive-switch-bg`, `--bg-input`, `--input-border`).
       - Sync unset `<input type="color" data-sync-accent="true">` elements live so no stale blue swatch lingers on rose/mint/violet/photo themes.
       - Set `data-wp-theme-active="true"|"false"` on `<html>` for crisp observability and E2E verification.
2. **`app/src/components/settings_view.rs` & `app/src/components/settings_more.rs`**:
   - Use `platform::wallpaper_seed_hex(&pr.app_wallpaper)` for default unset color preview values of `Màu nhấn tùy chọn`, `Bong bóng của bạn`, and `Màu nền ứng dụng`, and mark them with `data-sync-accent: "true"` when unset so uploaded images also sync their swatch live!
   - When user clicks a wallpaper tile in `settings_more.rs`, clear `p.custom_accent` so picking a new wallpaper always cleanly re-themes the entire app without stale custom accent lock-in.
3. **`app/e2e/spec052-visual-and-contrast-audit.mjs`**:
   - Assert full-app dynamic wallpaper theme synchronization across `Cực quang` (Violet), `Hoàng hôn` (Sakura Rose — `--accent`, `--accent-soft`, `--bg-hover`, `.btn-secondary` all shift to Rose, zero cold blue / purple-grey!), and `Bạc hà` (Fresh Mint Emerald)!
   - Capture live verification screenshots of `Hoàng hôn 100%`, `Bạc hà 100%`, `Đại dương 100%`, `Cực quang 100%`, `Giấy 100%`, and `Đêm sao 100%` and inspect every screenshot visually.
