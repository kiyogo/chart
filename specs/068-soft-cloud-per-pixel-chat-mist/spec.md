# Spec 068 — Soft Per-Pixel Cloud Mist Edge Fade (Remove Solid White Top/Bottom Chat Bars)

## 1. Problem Statement
In Spec 065, `.chat-header::before` (`mask: #000 0%..84% -> transparent 100%`), `.composer::before` (`mask: #000 0%..86% -> transparent 100%`), and `.chat-messages` (`mask: transparent 0..50.5px -> #000 62.5px`) created an opaque solid bar ("thanh trắng") at the top and bottom of the chat viewport with a harsh 12px linear cutoff slicing across scrolled message bubbles.

The user requested:
> *"không được hiện cái thanh trắng ở trên và dưới chứ chỉ hiện mây trắng và mày tùy theo màu từng vị trí của hình nền ,... thôi nhé hiện đẹp nhất có thể"*

## 2. Solution Design (Cubic-Eased Per-Pixel Cloud Mist)
1. **Zero Solid Bar Plateaus on `.chat-header::before` & `.composer::before`**:
   - Eliminate all `0%..84% #000` and `0%..86% #000` solid bar masks.
   - Extend the top mist zone smoothly to `calc(var(--header-height) + 30px)` (`~83px`) and the bottom mist zone smoothly to `top: -30px` (`~94px`), applying a smooth 6-stop eased cubic cloud alpha mask (`0.86 -> 0.66 -> 0.40 -> 0.18 -> 0.05 -> 0`) with zero `backdrop-filter` haze and zero hard rectangular edge.
   - Paint the cloud mist with the **exact per-pixel color of the active background at every `(x, y)` coordinate**:
     - White / Dark canvas -> silky white/dark cloud mist (`mây trắng` / dark mist) seamlessly dissolving into the canvas.
     - Gradient & preset chat wallpapers (`wp-ocean`, `wp-sunset`, `wp-mint`, `wp-aurora`, `wp-night`, `wp-paper`, `wp-dots`, `wp-bubbles`, `wp-custom`) -> full-height aligned gradient/pattern cloud mist.
     - Uploaded chat photo wallpaper (`wp-image`) -> 1:1 pixel-aligned photo cloud mist matched to extended mist height.
     - Translucent chat over multi-color app wallpapers (`data-chat-edge-strip="true"`) -> 2D offscreen canvas crops the exact `(x, y)` pixels across the full extended top (`hdrH + 30`) and bottom (`compH + 30`) mist zones.
2. **Silky 8-Stop Cubic-Eased Viewport Dissolve on `.chat-messages`**:
   - Replace the abrupt 12px linear cliff with a velvety 8-stop S-curve alpha dissolve at both top (`0px -> 84px`) and bottom (`calc(100% - 92px) -> 100%`) so scrolled messages drift gently into atmospheric cloud mist matched natively to every background pixel beneath them, while keeping header icons 100% legible and latest bottom messages 100% crisp.

## 3. Acceptance Criteria
- [ ] Neither `.chat-header::before` nor `.composer::before` has a flat solid plateau (`#000 84%` / `#000 86%` removed); both use smooth multi-stop cubic cloud alpha fades (`<= 0.88` peak easing smoothly to `0`).
- [ ] `.chat-messages` uses a smooth multi-stop cubic-eased cloud dissolve at top and bottom (no 12px razor cutoff).
- [ ] `adaptive_surface.js` crops exact 1:1 per-pixel 2D strips matching the full extended top (`hdrH + 30`) and bottom (`compH + 30`) cloud mist heights for both `wp-image` and translucent multi-color app wallpapers.
- [ ] Visual screenshot inspection confirms silky cloud mist (`mây trắng` on white chat, sky-blue mist on Ocean, and exact per-pixel multi-color mist on split dark/light & colorful wallpapers) with zero solid white bar at top or bottom.
- [ ] All Playwright E2E suites and Rust unit tests pass 100%.
