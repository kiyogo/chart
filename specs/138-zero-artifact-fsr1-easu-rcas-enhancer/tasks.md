# Implementation Tasks: Spec 138 — Zero-Artifact AMD FSR 1.0 (EASU + RCAS) Image Enhancer

- [x] **Task 1: Implement Pure-Rust WebGL2 AMD FSR 1.0 (`GL_FSR1_EASU_FS` + `GL_FSR1_RCAS_FS`) in `app/src/storage/realesrgan_gpu.rs`**
  - Added exact MIT-licensed 12-tap EASU directional edge-adaptive upscaler shader and noise-gated RCAS sharpening shader.
  - Added `Fsr1EasuProg` and `Fsr1RcasProg` to `RealEsrganGlEngine`.
  - Implemented `run_fsr1_gpu` and `upscale_fsr1_rust(rgba, width, height, target_w, target_h, sharpness, on_progress)`.

- [x] **Task 2: Expose `upscaleFsr1` in `app/src/storage/wasm_bridge.rs`**
  - Registered `window.__chatwWasmMedia.upscaleFsr1`.
  - Whitelisted `fsr1_easu_rcas` and `fidelity_cas_1x` in `isModelLicenseAllowed`.

- [x] **Task 3: Replace destructive Real-ESRGAN GAN with AMD FSR 1.0 (EASU + RCAS) in `app/web/ai_restorer.js`**
  - Bumped `PIPELINE_VERSION = 'fsr1-easu-rcas-v15'` (invalidating all broken historical Real-ESRGAN outputs in IndexedDB).
  - Routed ALL images through AMD FSR 1.0 (2× EASU + RCAS for low-res `< 1280px`, 1× native RCAS for high-res).
  - Updated `maxInEdge = 2560` so Full HD / 2K images never lose native resolution.
  - Added matching fast 2×/1× OffscreenCanvas Worker fallback when WebGL2 is unavailable.
  - Eliminated 3.6MB Real-ESRGAN `.cwsr` model downloads and GAN hallucination on all user images.

- [x] **Task 4: Cache-bust Service Worker and `media_worker.js`**
  - Bumped `STATIC_CACHE_NAME = 'chatw-static-v15'` in `app/web/sw.js`.
  - Cache-busted `/media_worker.js?v=${TS}` inside `app/build-web.sh`.

- [x] **Task 5: Build, Visual Inspection E2E, and Auto-Push (Rule 1)**
  - Built web bundle with `./app/build-web.sh` (compiled cleanly with zero warnings).
  - Executed Playwright E2E `app/e2e/spec138-fsr1-zero-artifacts.mjs` on real AMD Radeon WebGL2 GPU:
    - 512×384 old chat portrait -> `fsr1_easu_rcas` -> 1024×768 in 41ms GPU, SSIM `0.9932`.
    - 1024×768 portrait -> `fidelity_cas_1x` -> 1024×768 in 33ms GPU, SSIM `0.9964`.
    - Zero `.cwsr` model downloads.
  - Visually verified full outputs and 2× zoomed eye crops (`verify_spec138/01..04.png`).
  - Commit and push to `origin main`.
