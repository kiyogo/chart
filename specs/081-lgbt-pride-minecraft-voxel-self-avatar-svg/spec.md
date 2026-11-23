# Spec 081 — Pure Vector SVG Pride Minecraft-Voxel Avatar for "Note to self" ("Chính mình")

## Problem Statement
The "Note to self" ("Chính mình") avatar used a plain solid-accent circle with a generic monochrome `DevicesIcon`. The user requested redesigning the "Chính mình" avatar as a crisp pure-vector SVG celebrating LGBTQ+ Pride inspired by the iconic r/Minecraft Reddit avatar (isometric pastel ice-blue Minecraft voxel bunny cube over the 6-stripe Pride rainbow flag).

## Acceptance Criteria
1. **100% Pure Vector SVG (`SelfChatPrideAvatar`)**:
   - Crisp inline SVG component rendered at full bleed inside the circular `.self-chat-avatar` in both the sidebar conversation list (`conversations.rs`) and the chat header (`chat.rs`).
   - Also published as a standalone vector asset `app/public/self-chat-avatar.svg` (matching `AGENTS.md` SVG vector graphic standards).
   - Zero raster images, zero `data:image/`, zero extra network requests during chat boot.
2. **Authentic Pride Rainbow + Isometric Minecraft Voxel Bunny Design**:
   - Full-bleed 6-stripe LGBTQ+ Pride Rainbow flag (`#E40303`, `#FF8C00`, `#FFED00`, `#008026`, `#004DFF`, `#750787`) clipped cleanly inside a circle.
   - Hand-crafted isometric 3D Minecraft-style icy-blue voxel bunny cube with multi-tone pixel-block facet shading, two 3D upright voxel ears, kawaii pixel eyes + nose, 3 visible stubby voxel legs, and subtle soft shadow for high visual pop at 38px, 46px, and large sizes.
   - Zero CSS `color` / `fill-opacity` override interference from `.self-chat-avatar .cw-icon` or dark-mode filters.
3. **Verified Visually & Automated**:
   - Live Playwright test (`app/e2e/spec081-self-chat-pride-svg-avatar.mjs`) asserts pure inline `<svg>` inside both sidebar `.chat-item .self-chat-avatar` and `.chat-header .self-chat-avatar`, verifies all 6 Pride stripe hex colors, confirms 0 network/image leaks, and inspects screenshots at both normal & zoomed detail.
