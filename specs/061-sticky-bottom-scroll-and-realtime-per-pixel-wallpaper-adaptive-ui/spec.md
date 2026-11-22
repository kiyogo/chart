# Spec 061 — Sticky-Bottom Chat Loading, 1px-Accurate Top/Bottom Chat Edge Fades & Real-Time Per-Position Adaptive Surface Engine

## Problem Statement
1. **Chat fails to open anchored at the bottom (`Image 1` vs `Image 2`)**:
   - When opening a conversation containing encrypted images (`EncryptedImage`), stickers, or poll cards, `scroll_chat_to_bottom(true)` fires once before Dioxus finishes patching and long before async blobs decrypt and `<img>` elements decode/expand. As images grow from a 40px spinner to 300px+, `scrollHeight` increases and pushes the newest messages (e.g., `"21 tháng 11"` + `"Bình chọn 123"`) below the viewport, leaving the user stuck halfway up the history.
2. **Bottom (`.composer::before`) and top (`.chat-header::before`) chat edge fades do not match the exact wallpaper pixels (`Image 1` & `Image 2`)**:
   - `.composer::before` still paints a hardcoded white `var(--bg-surface)` + `blur(14px)` gradient, creating a jarring white band at the bottom of `Đại dương` (`wp-ocean`) and all other chat/app wallpapers.
   - On multi-color image wallpapers (`wp-image`) and translucent app wallpapers (`wp-none` + `data-app-wp != "none"`), a single flat average color cannot match horizontal/vertical color transitions per pixel.
3. **No real-time per-position adaptive surface/contrast adjustment across UI items (`Image 3`)**:
   - Over complex multi-tone wallpapers at high transparency, different rows in `.settings-nav`, `.nav-sidebar`, `.settings-body`, and `.app-rail` sit over completely different local wallpaper colors and brightnesses (e.g., bright white vs dark crimson ribbon vs pale pink). Without changing the global `--accent` (Spec 060), each element needs real-time per-position pixel sampling as it scrolls/resizes so ink polarity, local hover glass wash, and local split-contrast backing adapt continuously to the exact pixels behind that item.

## Acceptance Criteria
1. **Bulletproof Bottom-Anchored Chat Loading**:
   - Opening any conversation (DM, Self-Chat, Group, Channel) — including conversations with async-loading images/media/polls — immediately scrolls to the bottom AND stays pinned to the bottom (`scrollHeight - scrollTop - clientHeight <= 4`) as async images decrypt, decode, and expand.
   - Any explicit user upward scroll (`wheel` up, touch drag, scrollbar drag, `ArrowUp`/`PageUp`/`Home`) immediately releases bottom-pinning with zero scroll fighting, and loading older history upward (`snapshot_chat_scroll_height` / `restore_chat_scroll_after_prepend`) preserves reading position without snapping back to bottom.
   - Scrolling back down to the bottom re-engages bottom-pinning for incoming messages.
2. **1px-Accurate Top & Bottom Chat Edge Fades**:
   - Both `.chat-header::before` (top) and `.composer::before` (bottom) match the exact underlying chat wallpaper pixels with ZERO mismatched white band:
     - Preset/gradient chat wallpapers (`wp-ocean`, `wp-sunset`, `wp-mint`, `wp-aurora`, `wp-night`, `wp-paper`, `wp-dots`, `wp-bubbles`, `wp-custom`): exact full-column gradient/pattern projection anchored at `0 0` (top) and `0 100%` (bottom) via live `--chat-col-h`.
     - Uploaded chat image wallpapers (`wp-image`): exact native-resolution `cover` projection via live `--chat-bg-w`, `--chat-bg-h`, `--chat-bg-x`, `--chat-bg-top-y`, `--chat-bg-bot-y` at both top and bottom.
     - Translucent `wp-none` chat over app wallpaper (`data-app-wp != "none"`): real-time cropped 2D canvas strips (`--chat-top-fade-strip`, `--chat-bot-fade-strip`) sampled from the composited blurred/dimmed/tinted app wallpaper at the exact top & bottom screen rects.
3. **Real-Time Per-Position Adaptive Surface & Contrast (`adaptive_surface.js`)**:
   - Renders an offscreen composited backdrop model (`160×90`) of the active wallpaper (image `cover` geometry, gradients, solid color, blur, dim, panel opacity) and samples each visible UI item's exact bounding rect in real time on scroll, resize, resizer drag, and appearance updates (rAF-throttled).
   - Dynamically sets `data-local-tone="light" | "dark"`, wallpaper-matched `--local-hover-bg`, and (only on genuinely split high-contrast dark+light patches at high transparency) `--local-scrim-bg` / `data-local-scrim="true"` per element — updating instantaneously as items scroll across bright and dark wallpaper regions.
   - Strictly preserves Spec 060 (`--accent`, brand logo, sent bubbles, and user's chosen `Màu nhấn` are NEVER changed by the wallpaper), Spec 059 (zero nested `backdrop-filter`, zero stacking regressions), and Spec 052 (`0.00px` layout shift).
4. **Verification**:
   - Live Chromium E2E (`app/e2e/spec061-sticky-bottom-and-per-pixel-adaptive-ui.mjs`) verifies async image expansion keeps chat pinned to bottom, user wheel-up unlocks smoothly, Ocean/Image/App-Wallpaper top & bottom fades match exact pixels with zero white bar, and scrolling a split dark+light wallpaper dynamically flips individual `.settings-nav-item` `data-local-tone` (`light` <-> `dark`) in real time while preserving user Blue accent.
   - All existing suites (`spec052`, `spec053`, `spec059`, `modal-overlay-check`) pass 100%, verified by visual screenshot inspection.
