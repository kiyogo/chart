# Feature Specification: Full-App Dynamic Wallpaper Color Theming (Material You / HCT-Inspired — Spec 056)

**Feature Branch**: `main`
**Created**: 2026-11-22
**Status**: Completed

## 1. User Requirement

> *"tất cả toàn bộ nó phải đổi màu theo đúng chủ đề của màu nền đi chứ"*
> (Uploaded screenshot: pink `Hoàng hôn` wallpaper with default cold-blue active pills turning muddy lavender-grey instead of adapting the entire interface to the pink wallpaper theme.)

## 2. Functional Specification & Acceptance Criteria

1. **Automatic Full-App Material You Dynamic Theme Sync from Wallpaper**:
   - Whenever an app wallpaper with chromatic hue is active (`preset-sunset`, `preset-ocean`, `preset-mint`, `preset-aurora`, `preset-paper`, `preset-bubbles`, `preset-night`, custom solid `color:#RRGGBB`, or any uploaded `image`), `CONTRAST_ENGINE_JS` automatically derives a complete, WCAG-verified tonal theme palette from the wallpaper's dominant seed color (`domHex`) and applies it across the **entire application**:
     - `--accent`, `--accent-hover`, `--accent-text`, `--on-accent` (primary buttons, checked switches, sliders, checkboxes, unread badges, ChatW logo, theme preview accent bars, sent message bubbles)
     - `--accent-soft`, `--accent-soft-strong` (active nav item `Giao diện`, active segmented buttons `Tròn`/`Vừa`/`Hệ thống`, active left-rail icon, active theme card, active wallpaper tile, selected chat/channel)
     - `--bg-hover`, `--bg-active`, `--bg-surface-hover` (soft wallpaper-harmonized hover/press veil — zero cold grey smudge!)
     - `--adaptive-btn-bg`, `--adaptive-btn-hover`, `--adaptive-btn-active` (harmonized flat borderless `.btn-secondary` / `.btn-ghost`)
     - `--adaptive-switch-bg` (harmonized OFF switch track)
     - `--bg-input` / `--input-border` (subtle frosted wallpaper-harmonized search/input fields)
     - Unset default `<input type="color" data-sync-accent="true">` swatches (`Màu nhấn tùy chọn`, `Bong bóng của bạn`) synchronize live to the active wallpaper theme color so zero mismatched blue squares remain.
2. **Curated Per-Wallpaper Seed Mapping + Live Image Color Extraction**:
   - `Hoàng hôn` (`preset-sunset`): Radiant Sakura Rose (`#EC4882`)
   - `Đại dương` (`preset-ocean`): Crystal Ocean Sky (`#0EA5E9`)
   - `Bạc hà` (`preset-mint`): Fresh Mint Emerald (`#10B981`)
   - `Cực quang` (`preset-aurora`): Luminous Aurora Violet (`#7C5CFF`)
   - `Giấy` (`preset-paper`): Warm Artisan Amber (`#C97A2B`)
   - `Bong bóng` (`preset-bubbles`): Soft Orchid Rose (`#D9468F`)
   - `Đêm sao` (`preset-night`): Starlight Sapphire (`#3B82F6`)
   - Solid Color (`color:#RRGGBB`): Exact chosen color (if chromatic; monochrome falls back to user accent)
   - Uploaded Image (`image`): Real-time 36-bin perceptual hue-scored `domHex` extracted from the image pixels.
   - Neutral Wallpapers (`Không`, `Chấm bi`): Cleanly honor user's chosen `Màu nhấn`.
3. **Seamless Manual Override & Re-Sync Semantics**:
   - Changing the app wallpaper always re-enables dynamic wallpaper theming (`__chatwAccentManual = false`) so picking any new wallpaper immediately recolors the entire UI.
   - Explicitly clicking a `Màu nhấn` swatch or selecting a `Màu nhấn tùy chọn` overrides the wallpaper theme immediately (`__chatwAccentManual = true`) until the user changes wallpaper or clicks `Mặc định`.
4. **Zero Regressions**:
   - Crisp unclipped Vietnamese `g` descenders, zero blurry text-shadow on smooth presets, frameless idle segmented buttons on smooth presets, 0.00px size stability, and all 4 Playwright E2E suites (`spec048`, `spec050`, `spec052`, `spec053`) pass 100% with 0 page errors.
