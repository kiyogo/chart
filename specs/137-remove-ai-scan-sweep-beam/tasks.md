# Implementation Tasks: Spec 137 — Remove AI Scan Sweep Beam

- [x] **Task 1: Remove scan sweep beam from RSX templates in `app/src/components/rich.rs`**
  - Removed `div { class: "ai-gen-dots-wave" }` from `RichImage` loaded stage.
  - Removed `div { class: "ai-gen-dots-wave", aria_hidden: "true" }` from `RichImage` placeholder stage.
  - Removed `div { class: "ai-gen-dots-wave" }` from `Lightbox` enhancement overlay.

- [x] **Task 2: Clean up CSS rules in `app/assets/main.css`**
  - Removed laser scan beam gradient, pseudo-element `::after`, and `@keyframes chatwAiLaserScan`.
  - Added safety `.ai-gen-dots-wave { display: none !important; }`.
  - Cleaned up reduced motion media queries.

- [x] **Task 3: Build & E2E verification**
  - Ran `./app/build-web.sh` (compiled in 52s, packed CWRT1 `core.bin`).
  - Ran Playwright E2E verification test `app/e2e/spec137-remove-ai-scan-sweep.mjs` (verified `ai-gen-dots-wave` is absent from DOM and CSS is `display: none !important`).
  - Captured visual proof screenshot `verify_spec137/01_lightbox_loading_no_sweep_beam.png`.

- [x] **Task 4: Auto-Push to GitHub (Rule 1)**
  - `git add .`
  - `git commit -m "style(spec-137): remove ai scan sweep beam from media overlays"`
  - `git push origin main`
