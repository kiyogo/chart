# Feature Specification: Spec 135 — Refined Slender AI Scan Sweep Beam

## 1. Problem Statement
The user reported that the AI scan sweep beam added in Spec 133 is way too oversized, clunky, visually buggy, and harsh:
- `"cai thanh quét nó quá to + quá là lỗi + xấu rồi"` (The scanning bar is too big + too buggy/broken + ugly!)
- Image uploaded by user (`media_1795549576745_2c456307.jpg`) clearly demonstrates the issue:
  1. **Excessive Width**: The scan sheen spanned 36% of a 150%-width container, creating an opaque milky-white/lavender floodlight over 54% of the entire image width.
  2. **Severe Visual Blinding**: Triple heavy drop shadows (`filter: drop-shadow(0 0 16px ...) drop-shadow(0 0 6px ...)` and `box-shadow: 0 0 36px #818CF8`) completely washed out and hid the image preview during restoration.
  3. **Skewing & Clipping Defects**: Negative container insets (`inset: -10% -25%`) combined with `transform: skewX(-18deg)` produced jagged, disconnected border cutoffs across different aspect ratios.

## 2. Goals & Acceptance Criteria
- [x] **Slender & Compact Light Ray**:
  - Replace the 54%-wide opaque floodlight with a slender, refined light ray (~7% total width, ~24px - 48px depending on image size) that sweeps across cleanly.
  - The core is a crisp 1.2% (~1.5px - 2px) luminous ray with a gentle, translucent cyan/violet aura feather.
- [x] **Translucent Blending & Non-Destructive Illumination**:
  - Use `mix-blend-mode: screen` on the sweep container with zero heavy opaque floodlights. The underlying image remains clearly visible and naturally illuminated as the ray passes.
  - Remove destructive double-stacked drop shadows and massive 36px glow bleeds.
- [x] **Clean Geometry & Edge Clipping**:
  - Set container to `inset: 0` with `overflow: hidden` and `border-radius: inherit`.
  - The beam glides seamlessly from off-screen left (`translate3d(-105%, 0, 0)`) to off-screen right (`translate3d(105%, 0, 0)`), with smooth entry/exit opacity gating and an intentional breathing pause between cycles.
- [x] **Cross-Platform & Motion Accessibility**:
  - Retain `prefers-reduced-motion` and `html[data-reduce-motion="true"]` overrides.
- [x] **Mandatory Auto-Push (Rule 1)**:
  - Commit and push all changes immediately upon turn completion.
