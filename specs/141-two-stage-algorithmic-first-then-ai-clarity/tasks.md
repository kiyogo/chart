# Implementation Tasks: Spec 141 — 2-Stage Progressive Pipeline (Algorithmic First → 4× Photo AI + Post-AI Optical Clarity)

- [x] **Task 1: Implement 2-Stage Progressive Enhancement (Stage 1 Algorithmic → Stage 2 `4xLSDIRCompact3` 4× AI + Post-AI Optical RL Clarity) in `app/web/ai_restorer.js`**
  - Emits `meta.onStage1(stage1Url)` immediately after Stage 1 WebGL2 EASU + RL Deconvolution (`~157ms` total including decode/encode).
  - Chains Stage 2 `4xLSDIRCompact3` 4× AI → Post-AI Edge-Masked Luma RL Optical Clarity (`sharpness = 1.15`) directly on the clean block-free 4× AI output.
  - Bumped `PIPELINE_VERSION = 'progressive-alg-then-lsdir3-rl-v18'`.

- [x] **Task 2: Wire progressive `onStage1` UI swaps in `app/src/components/rich.rs`**
  - Updated BOTH `RichImage` (Chat) and `GlobalLightbox` (Lightbox) to immediately display Stage 1 Algorithmic preview upon `"stage1"`, then seamlessly upgrade to Stage 2 AI 4× + Optical RL Clarity on `"done"`.

- [x] **Task 3: Bump Service Worker cache in `app/web/sw.js`**
  - Bumped `STATIC_CACHE_NAME = 'chatw-static-v18'`.

- [x] **Task 4: Build, E2E + Visual Inspection Verification, and Auto-Push (Rule 1)**
  - Built web bundle `./app/build-web.sh` cleanly.
  - Executed Playwright E2E `app/e2e/spec141-two-stage-progressive-alg-then-ai.mjs` on real AMD Radeon WebGL2 GPU:
    - Stage 1 (Algorithmic First): fired in `157 ms` (`1024×768`).
    - Stage 2 (`LSDIR Photo Compact v3 (4×) + Optical RL`): `2048×1536`, `SSIM = 0.993`, `elapsedMs = 1010 ms`.
  - Visually verified `verify_spec141/03_stage2_ai_plus_rl_face_400pct.png` via `read_file`.
  - Commit and push to `origin main`.
