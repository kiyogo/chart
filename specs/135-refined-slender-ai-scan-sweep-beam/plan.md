# Technical Plan: Spec 135 — Refined Slender AI Scan Sweep Beam

## Architecture & Visual Design

### 1. Refactored CSS Architecture (`app/assets/main.css`)
- **Container (`.ai-gen-dots-wave`)**:
  - `position: absolute; inset: 0; z-index: 5; pointer-events: none; overflow: hidden; mix-blend-mode: screen; border-radius: inherit;`
  - Zero background color or linear-gradient on the container itself (eliminating any screen-wide washouts).
  - Zero heavy `filter: drop-shadow(...)`.
- **Slender Laser Sheen (`.ai-gen-dots-wave::after`)**:
  - `position: absolute; top: -20%; bottom: -20%; width: 100%; left: 0; pointer-events: none;`
  - Subtle, narrow 105deg multi-stop gradient (only 7% effective width):
    ```css
    linear-gradient(
      105deg,
      transparent 0%,
      transparent 46.5%,
      rgba(56, 189, 248, 0.16) 48.2%,
      rgba(255, 255, 255, 0.45) 49.4%,
      rgba(255, 255, 255, 0.92) 50.0%,
      rgba(255, 255, 255, 0.45) 50.6%,
      rgba(168, 85, 247, 0.18) 51.8%,
      transparent 53.5%,
      transparent 100%
    )
    ```
  - Subtle, focused ray glow: `filter: drop-shadow(0 0 4px rgba(255, 255, 255, 0.65)) drop-shadow(0 0 10px rgba(56, 189, 248, 0.35));`
- **Animation (`@keyframes chatwAiLaserScan`)**:
  - `0%`: `transform: translate3d(-105%, 0, 0); opacity: 0;`
  - `6%`: `opacity: 1;`
  - `70%`: `transform: translate3d(105%, 0, 0); opacity: 1;`
  - `74%, 100%`: `transform: translate3d(105%, 0, 0); opacity: 0;` (0.6s breathing interval between sweeps)
  - Timing: `2.4s cubic-bezier(0.22, 1, 0.36, 1) infinite`.

### 2. Verification Plan
- Build web application with `./app/build-web.sh`.
- Run Playwright E2E verification test capturing high-resolution screenshots of the new slender laser beam in chat thumbnails and lightbox modes.
- Inspect rendered screenshots to ensure zero milky floodlights, zero edge clippings, and elegant illumination.
