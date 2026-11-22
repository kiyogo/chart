# Spec 060 — Disable Automatic Wallpaper-Driven Web Recoloring

## Problem Statement
Previously (Specs 056–057), selecting or uploading a wallpaper with translucent panels automatically extracted the wallpaper's dominant color and overrode `--accent`, `--accent-hover`, `--accent-soft`, `--accent-text`, sent/received chat bubbles, active nav pills, buttons, and the app brand logo across the entire web application. Furthermore, changing the wallpaper cleared `prefs.custom_accent`.

The user explicitly requested:
> **"không được đổi màu toàn bộ web theo hình nền nhé"**

## Acceptance Criteria
1. **Zero Wallpaper-Driven Accent/Theme Recoloring**:
   - Changing the app wallpaper (preset like Hoàng hôn / Cực quang / Bạc hà / Đại dương / Đêm sao, custom solid color, or uploaded photo) at ANY panel transparency (`0%` through `100%`) and with or without Glass Blur MUST NEVER change `--accent`, `--accent-hover`, `--accent-text`, `--accent-soft`, `--msg-sent`, `--msg-recv`, `.rail-brand`, or `.self-chat-avatar` to the wallpaper's color.
   - `data-wp-theme-active` is always `"false"`.
2. **Strict Preservation of User's Chosen Accent**:
   - Both preset accents (`blue`, `violet`, `green`, `orange`, `rose`, `cyan`) and custom accents picked via the 7th circular color picker persist untouched when switching wallpapers, picking solid wallpaper colors, or uploading custom wallpaper images (`custom_accent` is NEVER cleared on wallpaper change).
   - The active checkmark in `Màu nhấn` and the default preview color of `Bong bóng của bạn` always reflect the user's chosen accent, never a wallpaper seed.
3. **Preserved Readability & Contrast**:
   - Adaptive light/dark ink polarity (`data-surface-tone`, `data-rail-tone`, `data-text-halo`, `data-framed-controls`) continues to keep text and controls legible on bright or dark wallpapers without altering the user's accent palette.
4. **Verification**:
   - Live Chromium E2E proves that selecting Sunset (`Hoàng hôn`), Aurora (`Cực quang`), Mint (`Bạc hà`), and a custom pink/red uploaded photo at `0%`, `50%`, `72%`, and `100%` transparency keeps `--accent` locked to the user's chosen Blue (`#2E7CF6`) or custom accent, with zero wallpaper color hijacking.
