# Feature Specification: Clean Chat Scroll Mask & Zero Header-Message Collision (Spec 065)

**Feature Branch**: `main`
**Created**: 2026-11-23
**Status**: Active
**Input**: User uploaded screenshot `media_1795366697209_2bd53038.png` showing a scrolled blue message bubble colliding directly behind the floating chat header icons (`phone`, `video`, `details`, `more`) and `12:09` timestamp while `.chat-header::before` painted a 66px milky white gradient fade across the top half of the blue bubble, washing out the white message text into an unreadable smudge.

## Root Cause
1. `.chat-header::before` extended `calc(var(--header-height) + 0.65rem)` (~66px) with a slow linear mask (`0.98` at `0%` -> `0.80` at `54%` -> `0` at `100%`). Because it was only `80% -> 0%` across the lower half of the header, scrolled message bubbles bled directly through behind the header icons and were washed out in milky white haze.
2. `.composer::before` similarly started `top: -1.25rem` (`-20px`) with a `0.99 -> 0.84 -> 0` milky fade that smeared across message bubbles above the composer.
3. `.chat-messages` had no viewport mask separating the message scroll canvas from the header and composer zones.

## Requirements & Acceptance Criteria
- **FR-1**: `.chat-messages` applies a crisp viewport `mask-image` that masks `0px .. calc(var(--header-height) - 2px)` to `transparent (0%)`, feathers cleanly over `12px` strictly BELOW the header (`calc(var(--header-height) - 2px) -> calc(var(--header-height) + 10px)`), stays `100%` solid `#000` across the entire readable message area, and feathers cleanly over `14px` right above `.composer-box` (`calc(100% - 68px) -> calc(100% - 54px)`).
- **FR-2**: When `.pinned-bar` or `.chat-banner` is present above `.chat-messages`, the top mask switches cleanly to `transparent 0px -> #000 10px` so messages below the pinned bar are never clipped.
- **FR-3**: Whenever `.lightbox` or `.modal-overlay` is open inside `.chat-messages`, the mask is lifted (`:not(:has(.lightbox, .modal-overlay))`) so full-viewport photo viewers render unclipped.
- **FR-4**: Tighten `.chat-header::before` to exact `height: var(--header-height)` (`0%..84%` solid `#000` -> `100%` transparent) and `.composer::before` to `top: -6px` (`0%..86%` solid `#000` -> `100%` transparent), and align `hdrH` / `compH` in `adaptive_surface.js` to exact pixel heights so wallpaper-matched strips stay 1px seamless with zero milky washout.
- **FR-5**: Visual screenshot inspection proves ZERO blue-bubble bleed behind header icons (`📞`, `📹`, `◫`, `...`) and ZERO milky washout over scrolled bubbles on light, dark, preset wallpaper, and translucent photo wallpaper.
