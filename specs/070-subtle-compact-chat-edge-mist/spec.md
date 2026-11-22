# Feature Specification: Subtle Compact Top & Bottom Chat Edge Mist (Spec 070)

**Feature Branch**: `070-subtle-compact-chat-edge-mist`  
**Created**: 2026-11-23  
**Status**: In Progress  
**Input**: User feedback + 2 screenshots showing (1) top chat mist (`.chat-header::before` + `.chat-messages` top mask) is too tall (`+30px` / `84px`) and too opaque (`0.86`), washing out the E2EE notice card and scrolled messages too heavily; (2) bottom chat mist (`.composer::before` + `.chat-messages` bottom mask) extends too high (`-30px` / `92px`) and is far too white (`0.86`), visibly bleaching bubbles situated directly above the floating composer pill.

## Acceptance Criteria
1. **Compact Higher Top Mist**: `.chat-header::before` sits tighter and higher (`calc(var(--header-height) + 12px)` ≈ 64px vs 82.5px) with a lighter translucent mist curve (peak `0.56`, fading smoothly to `0`). `.chat-messages` top dissolve mask starts higher and finishes at `56px` (instead of `84px`) so notices and messages immediately below the header stay crisp and legible.
2. **Non-Bleaching Sheer Bottom Mist**: `.composer::before` hugs the composer tightly (`top: -6px` instead of `-30px`) with a gentle sheer mist (peak `0.48`, fading rapidly to `0`). `.chat-messages` bottom dissolve finishes at `calc(100% - 56px)` (instead of `calc(100% - 92px)`), guaranteeing bubbles directly above the composer pill stay 100% vivid and unbleached while bubbles tucked behind the pill fade subtly without turning stark white.
3. **1:1 Wallpaper Synchronization**: `adaptive_surface.js` matches `topMistH = hdrH + 12` and `botMistH = compRectH + 6` for pixel-accurate `wp-image` cover projection and `data-chat-edge-strip="true"` 2D per-pixel canvas strips across light, dark, Ocean, and multi-color wallpapers.
4. **Zero Regressions**: All Playwright E2E suites (`spec052`, `spec053`, `spec059`, `spec061`, `spec062`, `spec069`) pass 100%, and live visual screenshots confirm the exact refinement.
