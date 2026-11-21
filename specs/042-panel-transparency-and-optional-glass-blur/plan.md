# Technical Plan — Spec 042: Intuitive Panel Transparency Scale, Optional Glass Blur Toggle & Live Blur Responsiveness

## Architectural Overview
In Spec 040, adjustable panel translucency and backdrop blur were introduced. However:
1. The slider was labeled "Độ mờ đục khung trắng: 100%" (100% = solid, 0% = transparent), which is counter-intuitive for users who expect 100% to mean 100% transparent.
2. Glass blur (`backdrop-filter: blur(14px)`) was automatically activated on any translucent panel, whereas users want no blur by default and prefer a dedicated checkbox/switch to turn on frosted glass blur only if desired.
3. The blur slider had no effect when the panel was solid white (0% transparency), leading to confusion.

## Changes Required

### 1. Preference Model (`app/src/prefs.rs`)
- Add `#[serde(default)] pub panel_glass: bool` to `Prefs` struct.
- In `Prefs::default()`: set `panel_glass: false`.
- In `Prefs::clean_and_clamp()` / `sanitize()`: retain `panel_glass` (bool, default `false`).
- Ensure full serialization & deserialization compatibility with existing stored preferences.

### 2. Platform CSS Properties & DOM Attributes (`app/src/platform/mod.rs`)
- In `apply_theme_and_prefs(p: &Prefs)`:
  - Set `--panel-opacity: format!("{}%", p.panel_opacity)`
  - Set `--panel-blur: format!("{}px", if p.panel_glass { p.panel_blur } else { 0 })`
  - Set attribute `data-panel-glass: (p.panel_glass && p.panel_opacity < 100).to_string()`
  - Set attribute `data-panel-translucent: (p.panel_opacity < 100).to_string()`

### 3. Styling (`app/assets/main.css`)
- Support `html[data-panel-translucent="true"]` (and backward-compatible `html[data-panel-glass="true"]`) for semi-transparent backgrounds of panels (`.nav-sidebar`, `.chat-column`, `.settings-nav`, `.settings-body`, `.details-panel`, etc.).
- Ensure `backdrop-filter: blur(var(--panel-blur, 12px))` is ONLY applied when `html[data-panel-glass="true"]`.
- When `data-panel-glass="false"`, `backdrop-filter` is `none`, providing crystal-clear transparency without smudge or blur by default.

### 4. Settings UI (`app/src/components/settings_more.rs`)
- Compute `panel_transparency = 100u8.saturating_sub(pr.panel_opacity)`.
- Replace "Độ mờ đục khung trắng" slider with:
  - Label: `Độ trong suốt khung trắng: {panel_transparency}%`
  - Range: `min="0"`, `max="100"`
  - Description: `Tăng lên để các khung danh sách, chat và cài đặt trong suốt, hiện hình nền phía sau`
  - `oninput`: `let t: u8 = e.value().parse().unwrap_or(0); c.update(move |p| p.panel_opacity = 100u8.saturating_sub(t))`
- Add dedicated `Toggle` switch:
  - Label: `Làm mờ kính khung (Frosted glass)`
  - Description: `Bật hiệu ứng kính mờ phía sau các khung khi trong suốt`
  - Checked: `pr.panel_glass`
  - `on_change`: toggles `p.panel_glass`. When enabling and `p.panel_opacity == 100` (transparency 0%), automatically set `p.panel_opacity = 60` (40% transparency) so the frosted glass blur effect is immediately visible.
- Conditionally render blur slider only when `pr.panel_glass` is `true`:
  - Label: `Mức độ làm mờ kính: {pr.panel_blur}px`
  - Description: `Điều chỉnh độ nhòe mờ đục của lớp kính (0px đến 32px)`
  - Range: `0` to `32`
- In `Khôi phục giao diện mặc định`: reset `p.panel_glass = d.panel_glass` (false) and `p.panel_opacity = d.panel_opacity` (100).

### 5. Automated Verification (`app/e2e/spec042-transparency-toggle-blur.mjs`)
- Test Playwright end-to-end:
  - Verify default state (0% transparency, glass blur toggle is OFF, zero blur).
  - Verify sliding transparency to 40% updates `--panel-opacity: 60%`, `data-panel-translucent="true"`, but `data-panel-glass="false"` (no blur).
  - Verify turning ON the glass blur toggle activates `data-panel-glass="true"` and displays the blur slider.
  - Verify sliding blur slider updates `--panel-blur` in real-time.
  - Verify resetting default restores solid panels and toggle OFF.
