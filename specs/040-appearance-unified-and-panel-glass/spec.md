# Feature Specification — Spec 040: Fix Broken Appearance Sliders, Unify Look Controls into Giao diện & Add White Panel Opacity/Glass Control

## Problem Statement
1. **Broken range sliders**: In Settings, `input[type="range"]` inherited `min-height: var(--control-height)` (`2.5rem`) and hid its thumb (`opacity: 0`), causing sliders like `Độ tối phủ lên ảnh` and `Làm mờ` to render as giant blank gray pills with invisible knobs.
2. **Fragmented visual settings**: All visual look & wallpaper controls (`Màu nhấn tùy chọn`, `Bong bóng tin nhắn`, `Hình dạng và bố cục`, `Phông chữ`, `Giảm chuyển động`, `Hình nền trò chuyện`, `Hình nền toàn ứng dụng`, `Độ tối phủ lên ảnh`, `Làm mờ`) were separated into the `Tùy chỉnh` tab instead of living in `Giao diện`.
3. **Missing panel opacity / translucency control**: Users setting an app wallpaper had no way to adjust the opacity/translucency (and frosted glass blur) of the white panels (`.nav-sidebar`, `.chat-column`, `.settings-nav`, `.settings-body`, `.details-panel`, cards, composer) so the background could shine through cleanly.
4. **Preset wallpaper dim ignored & blur edge halo**: `html[data-app-wp^="preset-"] .workspace-shell::after { background: transparent; }` ignored `--app-wp-dim`, and `inset: 0` on blurred fixed wallpaper caused faded edge halos. Clicking built-in accent swatches in `Giao diện` did not clear `custom_accent`.

## Acceptance Criteria
1. **Crisp, always-visible sliders globally**: `input[type="range"]` uses `min-height: 0 !important; height: 6px; padding: 0;` with an always-visible circular accent thumb (`opacity: 1`) and dynamic active progress fill (`--range-pct`), eliminating giant empty gray pills across the entire app.
2. **All visual appearance controls unified inside `Giao diện` (`SettingsTab::Appearance`)**:
   - Mode (Sáng/Tối/Hệ thống), Accent swatches (+ clearing `custom_accent` on swatch pick), Font size & Density, Live chat preview, Colors (`Màu nhấn tùy chọn`, `Bong bóng tin nhắn của bạn`, `Bong bóng tin nhắn người khác`), Shapes & Layout (`Bong bóng`, `Ảnh đại diện`, `Độ rộng tin nhắn`, `Phông chữ`, `Giảm chuyển động`), `Hình nền trò chuyện`, `Hình nền toàn ứng dụng` + `Khung giao diện`, and unified `Khôi phục giao diện mặc định` (resets both `Appearance` and visual `Prefs`).
   - `Tùy chỉnh` (`SettingsTab::Customize`) cleanly retains behavioral workspace settings (`Đồng bộ`, `Cột điều hướng bên trái` pin toggles, `Thông báo và hiệu ứng`) without duplicate appearance clutter.
3. **White panel translucency & glass blur controls (`panel_opacity`, `panel_blur`)**:
   - Add `panel_opacity: u8` (`0..=100`, default `100`) and `panel_blur: u8` (`0..=32`, default `12`) to `Prefs`, persisted & synced.
   - Always show `Độ mờ đục khung trắng` (`0%`–`100%`) and `Làm mờ kính khung` (`0px`–`32px`) inside `Giao diện` -> `Khung giao diện & Hình nền toàn ứng dụng`.
   - When `panel_opacity < 100`, `html[data-panel-glass="true"]` smoothly applies translucent `color-mix(..., var(--panel-opacity), transparent)` + modal-safe `backdrop-filter: blur(var(--panel-blur))` across `.nav-sidebar`, `.chat-column`, `.settings-nav`, `.settings-body`, `.details-panel`, `.partner-info-sidebar`, `.server-members-panel`, `.notifications-page`, `.download-page`, `.search-page`, `.meet-column`, `.feed-column`, `.composer-box`, and nested cards.
   - At default `panel_opacity = 100`, zero translucency overrides apply (100% solid pure white, zero regressions).
4. **Wallpaper fixes**:
   - `Độ tối phủ lên ảnh` works consistently on images, custom colors, AND preset wallpapers (`app_wallpaper_dim` defaults to `0`).
   - `Làm mờ hình nền` uses `inset: -28px` on `.workspace-shell::before` so blurred backgrounds never show edge halos.
5. **Verification**: Add Playwright test `app/e2e/spec040-appearance-panel-glass.mjs`, visually inspect Light & Dark + Glass + Wallpaper screenshots, pass all unit tests & existing regression suites, rebuild Web & Linux Desktop + `.deb`, commit, and push to `origin/main`.
