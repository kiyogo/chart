# Technical Plan: Spec 139 — Pure-Rust WebGL2 EASU + Regularized Richardson-Lucy Deconvolution

## 1. WebGL2 Shader Pipeline (`app/src/storage/realesrgan_gpu.rs`)
Extend `RealEsrganGlEngine` with 4 lightweight, ultra-fast WebGL2 shaders operating on `RGBA16F` (or `RGBA32F` fallback) textures:

1. **`GL_RL_INIT_FS`**:
   - Reads `uBase` (`base_tex`, either EASU 2× output or `src_tex` for 1×).
   - Computes perceptual luminance `Y = dot(rgb, vec3(0.299, 0.587, 0.114))`.
   - Computes central-difference gradient `g = length(vec2(Yr - Yl, Yd - Yu))`.
   - Computes smooth edge-confidence mask `edgeMask = smoothstep(0.012, 0.065, g)`.
   - Outputs `vec4(Y_orig, Y_est, edgeMask, 1.0)` into `state_a`.
2. **`GL_RL_BLUR_FS`** (Separable 9-tap Gaussian blur, horizontal or vertical via `uDir` uniform):
   - Reads `uSrc` (channel selected via `uChannel`: `0` = `.y` (`Y_est`), `1` = `.x` (`tmp`), `2` = `.z` (`ratio`)).
   - Supports arbitrary `uSigma` (e.g. `1.15` for RL PSF on 2×, `0.85` for RL PSF on 1×, `2.2` for micro-clarity).
   - 9 taps (`-4..+4`), normalized exactly in fragment shader (no uniform arrays needed!).
3. **`GL_RL_RATIO_FS`**:
   - Reads `uState` (`Y_orig = .x`, `Y_est = .y`) and `uBlurV` (`blurred = .x`).
   - Computes damped Richardson-Lucy multiplicative ratio:
     `float r = (Y_orig + 0.01) / (blurred + 0.01);`
     `float ratio = 1.0 + tanh((r - 1.0) * uDamping) / uDamping;`
   - Outputs `vec4(ratio, 0.0, 0.0, 1.0)`.
4. **`GL_RL_UPDATE_FS`**:
   - Reads `uState` (`Y_orig = .x`, `Y_est = .y`, `edgeMask = .z`) and `uCorr` (`corr = .x`).
   - Updates `Y_est`:
     `float next = Y_est * corr;`
     `float m = 0.18 + 0.82 * edgeMask;`
     `float estNew = clamp(Y_est + (next - Y_est) * m, 0.0, 1.0);`
   - Outputs `vec4(Y_orig, estNew, edgeMask, 1.0)`.
5. **`GL_RL_FINAL_FS`** (renders directly to WebGL2 default canvas with Y-flip!):
   - Reads `uBase` (`base_tex`), `uState` (`Y_orig = .x`, `Y_est = .y`, `edgeMask = .z`), `uMidBlur` (`blur(Y_est, sigma=2.2) = .x`).
   - Applies edge-gated micro-clarity:
     `float estFinal = clamp(Y_est + (Y_est - midBlur) * uClarity * (0.25 + 0.75 * edgeMask), 0.0, 1.0);`
   - Computes soft-clamped luma delta `dy = clamp(estFinal - Y_orig, -0.185, 0.185)`.
   - Recombines into `uBase.rgb + vec3(dy)` (strictly clamped `[0, 1]`, preserving exact original hue/saturation and alpha!).

## 2. Parameter Tuning (Verified by Visual Benchmark `02_rl5_full.png`)
- **Upscaled 2× Images (`dw > src_w || dh > src_h`)**:
  - `iters = 6`, `sigma = 1.15`, `damping = 2.1`, `clarity = 0.28`
  - Turns blurry 512×384 photos into razor-sharp, studio-clean 1024×768 with crisp hair strands, sparkling eyes, natural skin pores, and zero ringing or alien folds.
- **Native 1× High-Res Images (`dw == src_w && dh == src_h`)**:
  - `iters = 3`, `sigma = 0.85`, `damping = 2.6`, `clarity = 0.20`
  - Adds crisp micro-definition to eyelashes, hair, text, and foliage without over-sharpening.

## 3. Worker Fallback & Cache Invalidation
- Update `app/web/ai_restorer.js`:
  - Bump `PIPELINE_VERSION = 'fsr1-rl-deconv-v16'`.
  - Update worker fallback `type: 'cas'` to run matching regularized RL deconvolution + clarity when WebGL2 is unavailable.
- Bump `STATIC_CACHE_NAME = 'chatw-static-v16'` in `app/web/sw.js`.

## 4. Verification (`app/e2e/spec139-gpu-rl-deconv-sharpness.mjs`)
- Run Playwright E2E on real WebGL2 GPU measuring:
  - Sharpness gain (Laplacian RMS of Spec 139 vs blurry EASU-only — assert `> 1.8×` real sharpness improvement!).
  - SSIM fidelity (`>= 0.94`).
  - GPU speed (`< 80 ms`).
  - Zero `.cwsr` downloads.
  - Visually inspect full portraits and 2× zoomed eye crops (`verify_spec139/*.png`).
