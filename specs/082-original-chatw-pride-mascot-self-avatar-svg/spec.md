# Spec 082 — Original ChatW Mascot SVG on LGBTQ+ Pride Flag for "Note to self" ("Chính mình")

## Problem Statement
The previous Spec 081 voxel bunny closely mirrored the r/Minecraft Reddit icon. The user requested keeping the LGBTQ+ Pride rainbow flag underneath while replacing the foreground subject with an **original, distinct ChatW visual style** (not copying Minecraft's blocky style).

## Design Direction
- **Background**: Crisp full-bleed 6-stripe LGBTQ+ Pride Rainbow flag (`#E40303`, `#FF8C00`, `#FFED00`, `#008026`, `#004DFF`, `#750787`) clipped cleanly inside a circle.
- **Original Foreground Subject — ChatW Bubble-Spirit Mascot**:
  - Smooth, plump, organic rounded **ChatW speech-bubble companion** with soft rounded ears, gentle pearlescent white-to-ice-blue gradient shading (`#FFFFFF` → `#EDF3FF` → `#D9E6FF`), bottom-right speech-bubble tail, and soft drop shadow for strong figure-ground separation over the rainbow.
  - Friendly kawaii expression: glossy deep-slate eyes (`#1E293B`) with dual catchlights, soft rose blush cheeks (`#FB7185`), cute happy curved smile, soft pastel pink inner ears (`#F9A8D4`), and a crisp golden-white floating 4-point sparkle star representing personal notes/ideas in "Chính mình".
  - 100% pure vector SVG with unique atomic gradient & clipPath IDs per instance (zero ID collisions, zero raster images, zero external network fetches).

## Acceptance Criteria
1. Standalone `app/public/self-chat-avatar.svg` and inline `SelfChatPrideAvatar` in `app/src/components/icons.rs` match 100%, contain all 6 Pride rainbow stripes underneath, and render the original smooth ChatW speech-bubble mascot (zero Minecraft voxel blocks).
2. Crisp, joyful, and instantly legible at `220px`, `51px` (sidebar), and `36px` (header), verified via live Playwright E2E (`spec081-self-chat-pride-svg-avatar.mjs` updated for Spec 082) and visual screenshot inspection.
3. All Rust unit tests pass and changes are committed & pushed to `origin main`.
