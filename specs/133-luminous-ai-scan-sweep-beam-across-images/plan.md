# Plan 133: Luminous AI Laser Scan Sweep Beam Across Images

## Technical Architecture & Changes

### 1. Upgrade Sweep Wave in `app/assets/main.css`
- Refactor `.ai-gen-dots-wave`:
  ```css
  .ai-gen-dots-wave {
    position: absolute;
    inset: -10% -25%;
    z-index: 5;
    background: linear-gradient(
      108deg,
      transparent 32%,
      rgba(56, 189, 248, 0.22) 42%,
      rgba(96, 165, 250, 0.60) 47%,
      rgba(255, 255, 255, 0.95) 50%,
      rgba(192, 132, 252, 0.60) 53%,
      rgba(168, 85, 247, 0.22) 58%,
      transparent 68%
    );
    filter: drop-shadow(0 0 16px rgba(56, 189, 248, 0.85)) drop-shadow(0 0 6px rgba(255, 255, 255, 0.95));
    will-change: transform;
    transform: translate3d(-60%, 0, 0);
    animation: chatwAiLaserGlide 1.8s cubic-bezier(0.25, 1, 0.5, 1) infinite;
    pointer-events: none;
  }
  .ai-gen-dots-wave::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 3px;
    background: #FFFFFF;
    box-shadow: 0 0 10px #FFFFFF, 0 0 20px #38BDF8, 0 0 36px #818CF8;
    transform: skewX(-18deg);
    opacity: 0.95;
    pointer-events: none;
  }
  @keyframes chatwAiLaserGlide {
    0% { transform: translate3d(-62%, 0, 0); }
    100% { transform: translate3d(62%, 0, 0); }
  }
  ```
- Support reduced-motion preferences:
  ```css
  html[data-reduce-motion="true"] .ai-gen-dots-wave,
  html[data-reduce-motion="true"] .ai-gen-dots-wave::after,
  @media (prefers-reduced-motion: reduce) {
    .ai-gen-dots-wave,
    .ai-gen-dots-wave::after {
      animation: none !important;
    }
  }
  ```

### 2. Add Missing Wave to Initial Image Loading in `app/src/components/rich.rs`
- In `RichImage` component (around line 1042-1046):
  ```rust
  div { class: "ai-gen-frost", aria_hidden: "true" }
  div { class: "ai-gen-aurora", aria_hidden: "true" }
  div { class: "ai-gen-dots-base", aria_hidden: "true" }
  div { class: "ai-gen-dots-wave", aria_hidden: "true" }
  ```

### 3. Build & E2E Verification
- Compile web bundle: `./app/build-web.sh`.
- Create Playwright E2E test `app/e2e/spec133-ai-scan-sweep-beam.mjs` verifying:
  - `.ai-gen-dots-wave` and its laser line `::after` exist and animate.
  - Presence in Lightbox and in chat message loading stages.
  - Capture verification screenshots in artifact directory.
- Commit & Push to GitHub `origin main`.
