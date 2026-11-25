# Implementation Tasks: Spec 139 — Pure-Rust WebGL2 EASU + Regularized Richardson-Lucy Deconvolution

- [x] **Task 1: Add WebGL2 RL Deconvolution shaders (`GL_RL_INIT_FS`, `GL_RL_BLUR_FS`, `GL_RL_RATIO_FS`, `GL_RL_UPDATE_FS`, `GL_RL_FINAL_FS`) in `app/src/storage/realesrgan_gpu.rs`**
  - Compiled and wired all 5 FP16/FP32 separable deconvolution shaders into `RealEsrganGlEngine`.
  - Upgraded `run_fsr1_gpu` to execute EASU (when 2×) → Regularized Edge-Masked Luma RL Deconvolution (`6 iters, σ=1.15, damping=2.1, clarity=0.28` for 2×; `3 iters, σ=0.85, damping=2.6, clarity=0.20` for 1×) → Dual-Scale Micro-Clarity & Luma Recombination.
  - Properly deleted all temporary GPU textures after draw.

- [x] **Task 2: Update `app/web/ai_restorer.js` and `app/web/sw.js`**
  - Bumped `PIPELINE_VERSION = 'fsr1-rl-deconv-v16'` and set `sharpness = 1.0`.
  - Bumped `STATIC_CACHE_NAME = 'chatw-static-v16'` in `app/web/sw.js`.

- [x] **Task 3: Build, Quantitative & Visual E2E Verification, and Auto-Push (Rule 1)**
  - Built web bundle with `./app/build-web.sh` (compiled cleanly in 48.8s with zero warnings).
  - Executed Playwright E2E `app/e2e/spec139-gpu-rl-deconv-sharpness.mjs` on real AMD Radeon WebGL2 GPU:
    - Measured **2.05× sharpness gain** (Laplacian RMS `7.54 → 15.48`) over Spec 138 EASU-only.
    - `512×384 → 1024×768`: `SSIM = 0.9921`, `gpuMs = 36 ms`.
    - `1024×768 → 1024×768`: `SSIM = 0.9966`, `gpuMs = 63 ms`.
    - Zero `.cwsr` model downloads.
  - Visually verified `verify_spec139/01..04.png` via `read_file`: razor-sharp hair, eyelashes, irises, collar, and natural skin.
  - Commit and push to `origin main`.
