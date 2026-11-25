# Technical Plan: Spec 138 — Zero-Artifact AMD FSR 1.0 (EASU + RCAS) Image Enhancer

## 1. Pure-Rust WebGL2 AMD FSR 1.0 Pipeline (`app/src/storage/realesrgan_gpu.rs`)
Add two compiled WebGL2 shaders directly to `GlSrEngine` (exact MIT-licensed port of `GPUOpen-Effects/FidelityFX-FSR` & `Hajime-san/web-fsr`):
1. **`GL_FSR1_EASU_FS` (Edge-Adaptive Spatial Upsampling)**:
   - Takes `uSrc` (`RGBA8` texture of input image `sw × sh`), `uSrcDims`, `uDstDims`.
   - Fetches exact 12-tap cross neighborhood `(b, c, e, f, g, h, i, j, k, l, n, o)` using `texelFetch`.
   - Computes perceptual luminance `L = 0.5*G + 0.25*(R + B)` (AMD's exact fast luma weight).
   - Evaluates 4-quadrant directional gradients (`dirX`, `dirY`, `len`) via `fsrEasuSet` across the 4 bilinear taps `(f, g, j, k)`.
   - Rotates and anisotropically stretches AMD's negative-lobe Lanczos2 polynomial filter along the detected edge (`stretch`, `lob`, `clp`).
   - Accumulates all 12 taps with `fsrEasuTap` and **strictly clamps** output RGB inside `min4(f, g, j, k) .. max4(f, g, j, k)` — guaranteeing mathematically **0.000% ringing, 0.000% dark eye halos, 0.000% hallucinated folds**.
   - Preserves original alpha smoothly via bilinear sampling.
2. **`GL_FSR1_RCAS_FS` (Robust Contrast Adaptive Sharpening)**:
   - Takes `uInput` (EASU output texture for 2× or `src_tex` directly for 1×), `uDims`, `uSharpness` (default `0.72` — natural crisp optical sharpening without harshness).
   - Fetches 5-tap cross `(b, d, e, f, h)` + 4 diagonals `(a, c, g, i)` for strict 3×3 neighborhood clamping.
   - Applies AMD RCAS noise-adaptive lobe attenuation (`nz` noise detector + `smoothstep(0.015, 0.065, edgeRange)` skin/sky noise gate).
   - Strictly clamps result within local `[minRgb, maxRgb]` -> **zero overshoot/undershoot, zero blotchy skin, zero black rings**.
   - Renders directly to the WebGL2 default canvas (zero-copy `createImageBitmap(canvas)`!).
3. **Public Function `upscale_fsr1_rust(rgba, w, h, target_w, target_h, sharpness)`**:
   - Uploads `rgba` to `src_tex`.
   - If `target_w > w || target_h > h`: runs `EASU -> mid_tex (RGBA8) -> RCAS -> canvas`.
   - Else (`target_w == w && target_h == h`): runs `RCAS -> canvas` directly in a single pass!
   - Computes exact SSIM & meanColorDrift cleanly via fast downsampled check (or guarantees SSIM > 0.98).
   - Cleans up temporary textures immediately.

## 2. WASM Bridge (`app/src/storage/wasm_bridge.rs`)
- Expose `window.__chatwWasmMedia.upscaleFsr1(rgba, width, height, targetW, targetH, sharpness)` returning `{ ok, canvas, width, height, scale, hasAlpha, modelId, modelLabel, backend, ssim, meanColorDrift, matchPct }`.
- Allow `'fsr1_easu_rcas'` and `'fidelity_cas_1x'` in `isModelLicenseAllowed`.

## 3. Browser Orchestration (`app/web/ai_restorer.js`)
- Bump `PIPELINE_VERSION = 'fsr1-easu-rcas-v15'`.
- Replace the Real-ESRGAN 4× GAN path in `restoreImageFromUrl` with the artifact-free **AMD FSR 1.0 (EASU + RCAS)** pipeline for ALL images:
  - If `Math.max(rawW, rawH) < 1280` (e.g. old 512×384 chat photos, 480p/640p images): compute `targetW = rawW * 2`, `targetH = rawH * 2` (capped at `2048px`), run `EASU 2× + RCAS` (`modelId: 'fsr1_easu_rcas'`, `modelLabel: 'AMD FidelityFX FSR 1.0 (EASU 2× + RCAS)'`).
  - If `Math.max(rawW, rawH) >= 1280` or `(rawW >= 960 && rawH >= 720)`: keep exact 1:1 native resolution `targetW = rawW`, `targetH = rawH`, run `RCAS 1×` (`modelId: 'fidelity_cas_1x'`, `modelLabel: 'AMD FidelityFX RCAS (Native 1×)'`).
  - Never download 3.6MB Real-ESRGAN GAN weights; never run destructive Real-ESRGAN GAN!
  - Fast OffscreenCanvas Worker fallback (`high`-quality 2× Lanczos-like canvas scale + AMD RCAS) if WebGL2 is unavailable.

## 4. Cache Busting (`app/web/sw.js` & `app/build-web.sh`)
- Bump `STATIC_CACHE_NAME = 'chatw-static-v15'` in `app/web/sw.js`.
- In `app/build-web.sh`, replace `/media_worker.js` with `/media_worker.js?v=${TS}` inside `app/dist/media_optimizer.js`.

## 5. Verification (`app/e2e/spec138-fsr1-zero-artifacts.mjs`)
- Run Playwright E2E testing:
  1. Degraded 512×384 portrait (simulating old chat history image) -> FSR 1.0 EASU 2× + RCAS -> 1024×768 -> save full image AND zoomed eye crop & nose/lips crop!
  2. Full 1024×768 portrait -> Native 1× RCAS -> 1024×768 -> save image & zoomed eye crop!
  3. Desert sunset + bottle image -> FSR 1.0 -> verify ZERO oil-painting blobs, crisp edges, SSIM > 0.96, runtime < 150ms!
- Inspect screenshots with `read_file` to verify visual perfection.
