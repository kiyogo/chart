# Spec 086 — Vibrant Fresh LGBT 7-Color Theme (All Text + Icons + Bright Pride Avatar) & Tight 1/3 Flush Sidebars with Centered Scrollbar

## Problem Statement
1. **LGBT font 7-color palette looked dark/muddy** (mustard `#D99B00` / dark green `#009E49` / dark pill stops) and left many UI texts (theme labels, dropdowns, previews, inputs, tiles, timestamps) and **all UI SVG icons** plain grey/black instead of bright vibrant rainbow.
2. **Pride rainbow flag inside the "Chính mình" avatar** (`self-chat-avatar.svg` & `SelfChatPrideAvatar`) used darker traditional colors (`#E40303`, `#008026`, `#004DFF`, `#750787` + dark veil) instead of vibrant bright Pride colors.
3. **"Nhà phát triển" (`Developer`)** in Settings nav had an extra `margin-top: 6px` gap above it instead of matching 100% the exact spacing of all other Settings nav items.
4. **Sidebar list items** in Messages, Friends, Communities, Channels, and Settings had vertical gaps (`gap: 0.3rem` / `gap: 2px`) between items instead of touching flush (`0px`), and wide left/right panel insets (`8px`) instead of **1/3 of current (`3px`)**.
5. **Right-side canvas scrollbar** (`app/web/canvas_scrollbars.js`) fell back to `edgeMargin = 4px` when `padding-right < 5px` instead of staying mathematically centered between the item's right edge and the panel's right border inside the narrow `3px` right gutter.

## Acceptance Criteria
1. **Radiant Fresh 7-Color LGBT Spectrum**: Bright high-chroma rainbow stops (`#FF2E63`, `#FF7B00`, `#FFBE00`, `#00CE67`, `#00B4FF`, `#3B82F6`, `#B833FF`) applied across **ALL text AND ALL monochrome UI SVG icons (`svg.cw-icon`)** in the entire app when `data-font="lgbt"` is selected — animated smoothly when `Reduce motion` is OFF, frozen statically on the vibrant 7-color gradient when `Reduce motion` is ON.
2. **Bright Fresh Pride Flag on "Chính mình" Avatar**: Vibrant 6-stripe Pride colors (`#FF2E55`, `#FF8C1A`, `#FFE119`, `#00D166`, `#1A8CFF`, `#A838FF`), zero dark tint veil, preserving Spec 085's star-free +1px right-centered mascot.
3. **100% Uniform "Nhà phát triển" Spacing**: `.settings-nav-item.settings-nav-dev` has `margin-top: 0` and sits flush (`0px` gap) identically to all other Settings nav items.
4. **Flush Sidebar Lists & Exact 1/3 Left/Right Inset (`3px`)**: `--panel-inset: 3px` across all left sidebars (Messages, Friends, Communities, Channels, Settings, Server Settings) and `gap: 0` strictly on those sidebar list containers so list items touch flush without disturbing chat bubbles or right-side content panels.
5. **Centered Right Scrollbar**: `canvas_scrollbars.js` dynamically scales `thumbW` (`2px` inside `3px` gutter) and centers horizontally (`(padRight - thumbW) / 2 = 0.5px` on both left and right sides) in every sidebar gutter.
6. All Rust unit tests + live Playwright E2E + visual screenshots pass, and changes are committed & pushed to `origin main`.
