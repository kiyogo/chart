# Implementation Plan: Spec 053

## Files to Modify
1. **`app/web/boot.js`**:
   - Resolve `chatw_theme` across `light | ash | dark | oled | system` before WASM boots, setting `data-theme="dark"` for `ash | dark | oled` (or system dark) and `data-theme-variant` to the exact resolved variant (`light | ash | dark | oled`).
2. **`app/src/platform/mod.rs`**:
   - Allow `theme` values `["light", "ash", "dark", "oled", "system"]` and `accent` values `["blue", "cyan", "violet", "green", "orange", "rose"]` in `load_appearance()`.
   - Update `apply_appearance()` to set `data-theme`, `data-theme-variant`, `data-theme-pref`, and `meta[name="theme-color"]` cleanly for all 5 themes, and update `PRESET_ACCENTS`, `panelBaseB` (`ash`: `0.22`, `dark`: `0.13`, `oled`: `0.03`), and `--accent-soft` (`12%` light, `18%` dark, `13%` translucent light) in `CONTRAST_ENGINE_JS`.
3. **`app/src/components/settings_view.rs` & `app/src/components/settings_more.rs`**:
   - Expand `Chủ đề` cards to all 5 themes (`Sáng`, `Xám tro`, `Tối`, `Đen huyền`, `Theo hệ thống`) with authentic preview swatches.
   - Upgrade `Màu nhấn` circular swatches to the 6 vibrant fresh presets (`#2E7CF6`, `#06B6D4`, `#8B5CF6`, `#10B981`, `#F97316`, `#F43F7E`) and place the custom accent color picker + left-aligned `"Mặc định"` button directly beneath the swatches inside the `Màu nhấn` group (removing the duplicate row from `AppearanceColorsShapes`).
4. **`app/assets/theme.css`**:
   - Refine `[data-theme="dark"]` (`Tối` — comfortable `#1C1F26`), add `[data-theme-variant="ash"]` (`Xám tro` — Discord Ash `#313338`) and `[data-theme-variant="oled"]` (`Đen huyền` — OLED `#090A0E` / `#000000`).
   - Update all vibrant accent palettes (`--accent`, `--accent-hover`, `--accent-text`, `--accent-soft: 12%` light / `18%` dark, `--accent-soft-strong: 18%` light / `26%` dark).
5. **`app/assets/main.css`**:
   - Adjust `.theme-preview` grid to accommodate 5 theme cards cleanly.
   - Remove `.accent-swatch.active` black outer ring (`box-shadow: none !important`).
   - Remove all borders from `input[type="color"]` and `.cw-cp-swatch`.
   - Remove outer capsule backgrounds/borders/shadows from `.chat-header-identity`, `.chat-header-actions`, and `.back-btn`, and calibrate the wallpaper-matched unblurred `.chat-header::before` top fade (`height: calc(var(--header-height) + 0.65rem)`, `backdrop-filter: none`, smooth `mask-image`).
   - Balance `.chat-item` padding (`padding: 6px 8px 6px 6px !important;`) so the avatar has equal top, bottom, and left breathing room (`~6px`) inside the pill.
6. **`app/e2e/spec052-visual-and-contrast-audit.mjs` & `app/e2e/spec053-themes-accents-frameless-header.mjs`**:
   - Update Spec 052 header assertions to match frameless floating header + balanced avatar padding, and add Spec 053 E2E test verifying all 5 themes (`Sáng`, `Xám tro`, `Tối`, `Đen huyền`, `Theo hệ thống`), zero black ring on `.accent-swatch.active`, frameless `<input type="color">`, inline custom accent picker under `Màu nhấn`, vibrant 12% accent tint, frameless floating chat header across DM & Community channels, and equal top/bottom/left avatar gap in `.chat-item`.
