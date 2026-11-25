# Feature Specification: Spec 141 — 2-Stage Progressive Pipeline (Algorithmic First → 4× Photo AI + Post-AI Optical Clarity)

## 1. Problem Statement
The user reported on Spec 140 (`media_1795589280823_d96d1772.jpg`):
- `"nó lại còn xấu hơn nữa rồi"` (It got even worse!)
- `"nó làm nét bằng thuận toán trước xong nó sẽ làm nét bằng AI"` (Sharpen with the algorithm first, and then sharpen with AI!)

### Root Cause & Product Requirement Analysis
1. **Why Spec 140 (`4xLSDIRCompact3` alone) looked soft/muddy on 10KB compressed photos**:
   - `4xLSDIRCompact3` (75% C3 compression-removal + 25% R3) excels at erasing 100% of 8×8 compression blocks and scaling 4× (`2048×1536` / `4K`) without alien eye distortions, **but** its heavy denoiser slightly softens fine hair strands, eyelashes, and irises when run alone with only a weak 1px clamped CAS pass.
2. **2-Stage Progressive Pipeline (Requested by User)**:
   - **Stage 1 — Instant Algorithmic Sharpening (`< 45 ms`)**: Immediately runs WebGL2 AMD FSR 1.0 EASU 2× + Regularized RL Deconvolution and emits `onStage1(stage1Url)` so both Chat and Lightbox immediately show a crisp, natural algorithmically sharpened image within 45 ms!
   - **Stage 2 — 4× Photo AI Super-Resolution (`4xLSDIRCompact3`) + Post-AI Optical RL Clarity Polish (`~450 ms`)**:
     - Runs `4xLSDIRCompact3` 4× on WebGL2 8-MRT FP16 GPU to completely eliminate 8×8 AVIF/JPEG compression blocks and generate `2048×1536` / `3840×2880` (4K).
     - Immediately chains **Post-AI Edge-Masked Luma RL Optical Clarity** (`wasm.upscaleFsr1` 1× native on the clean, block-free 4× AI output, `sharpness = 1.15`) to restore studio-grade razor-sharp hair strands, sparkling natural eyes, crisp eyelashes, and crisp collar/necklace details — ZERO compression blocks, ZERO muddy softness, ZERO alien eyes, EVEN AT 400% ZOOM!
     - Seamlessly upgrades the displayed image from Stage 1 (Algorithmic) → Stage 2 (4× AI + Optical Clarity) and caches the final result permanently in IndexedDB.

---

## 2. Goals & Acceptance Criteria
- [ ] **2-Stage Progressive Execution in `app/web/ai_restorer.js`**:
  - Emit `meta.onStage1(stage1Url)` immediately after Stage 1 Algorithmic GPU sharpening completes (`< 60 ms`).
  - Run Stage 2 `4xLSDIRCompact3` 4× AI + Post-AI Edge-Masked Luma RL Optical Clarity (`sharpness = 1.15`) on the block-free 4× AI output.
  - Bump `PIPELINE_VERSION = 'progressive-alg-then-lsdir3-rl-v18'`.
- [ ] **Progressive UI Updates in `app/src/components/rich.rs`**:
  - Wire `onStage1` in BOTH `RichImage` (Chat) and `GlobalLightbox` (Lightbox) so Stage 1 Algorithmic displays immediately, followed seamlessly by Stage 2 AI 4× + Optical Clarity!
- [ ] **Cache Invalidation in `app/web/sw.js`**:
  - Bump `STATIC_CACHE_NAME = 'chatw-static-v18'`.
- [ ] **E2E & Visual Proof Verification**:
  - Run Playwright E2E verifying BOTH `onStage1` fires first with a valid Stage 1 Algorithmic URL (`< 150ms`) AND Stage 2 finishes with 4× AI + Post-AI Optical Clarity (`2048×1536` / `3840×2880`), higher Laplacian sharpness than Spec 140 alone, `SSIM >= 0.98`, ZERO blocks & studio-sharp eyes at 400% zoom!
- [ ] **Mandatory Auto-Push (Rule 1)**:
  - Commit and push all changes immediately to `origin main`.
