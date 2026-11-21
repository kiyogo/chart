# Implementation Plan — Spec 040

1. **Preferences & Runtime Application (`app/src/prefs.rs`, `app/src/platform/mod.rs`)**:
   - Add `pub panel_opacity: u8` (default `100`, clamped `0..=100`) and `pub panel_blur: u8` (default `12`, clamped `0..=32`) to `Prefs`. Change default `app_wallpaper_dim` to `0`.
   - In `platform::apply_customization(p: &Prefs)`, export CSS vars `--panel-opacity: "{}%"`, `--panel-blur: "{}px"`, and attribute `data-panel-glass: (p.panel_opacity < 100).to_string()`.
2. **Global Range Slider Fix & Panel Glass CSS (`app/assets/main.css`)**:
   - Fix `input[type="range"]` (`min-height: 0 !important; height: 6px; padding: 0;`) with active progress track gradient (`--range-pct`) and always-visible circular accent thumb (`opacity: 1; transform: scale(1);`).
   - Fix `.workspace-shell::before` (`inset: -28px;`) and remove the `preset-` override that disabled `--app-wp-dim`.
   - Add `html[data-panel-glass="true"]` translucent background & modal-safe `:not(:has(.modal-overlay, .settings-modal-backdrop, .lightbox))` backdrop blur styles for all major panels, headers, cards, and composer.
3. **Unify Appearance Settings (`app/src/components/settings_view.rs`, `app/src/components/settings_more.rs`, `app/src/components/settings_extra.rs`)**:
   - Move `Hình nền trò chuyện` out of `CustomizationSettings` into `AdvancedLookSettings` (rendered inside `SettingsTab::Appearance`).
   - Mount `AdvancedLookSettings` inside `SettingsTab::Appearance` ("Giao diện") and remove it from `SettingsTab::Customize` ("Tùy chỉnh").
   - Add `Độ mờ đục khung trắng` (`0%`–`100%`) and `Làm mờ kính khung` (`0px`–`32px`) sliders with `--range-pct` live fill into `AdvancedLookSettings`, plus clean unified reset button resetting both `Appearance` and visual `Prefs`, and clear `custom_accent` when selecting a preset accent swatch.
   - Update `tab_keywords(SettingsTab::Appearance)` and `tab_keywords(SettingsTab::Customize)` for search discoverability.
4. **Verification & Delivery**:
   - Create & run `app/e2e/spec040-appearance-panel-glass.mjs`, inspect screenshots visually, run full Playwright regression suite + Rust unit tests, rebuild Web WASM & Linux Desktop release + `.deb`, commit, and push to `origin/main`.
