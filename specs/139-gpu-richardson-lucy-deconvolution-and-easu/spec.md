# Feature Specification: Spec 139 — Pure-Rust WebGL2 EASU + Regularized Richardson-Lucy Deconvolution Sharpener

## 1. Problem Statement
Following Spec 138 (which eliminated Real-ESRGAN GAN's alien eyes, plastic wax skin, and oil-painting blobs), the user reported:
- `"nó không làm nét gì luô á ,nó vẫn quá mờ"` (It doesn't sharpen anything at all, it's still way too blurry!)
- Attached screenshot: `media_1795586448624_7dd9652b.jpg` (512×384 old chat portrait upscaled to 1024×768 via EASU + 1px strictly clamped RCAS).

### Root Cause Analysis
1. **EASU Alone Cannot Invert Optical / Downscale Blur**:
   - EASU is a 12-tap Lanczos2-derived directional interpolator. While it eliminates stair-stepping with zero artifacts, downscaling 1024×768 → 512×384 acts as a low-pass Gaussian/Lanczos Point Spread Function (PSF, $\sigma \approx 1.15\text{ px}$). Interpolation alone leaves eyelashes, hair strands, and iris rims 2–3 pixels wide and soft.
2. **1-Pixel Strictly Clamped RCAS Cannot Narrow Multi-Pixel Blurred Edges**:
   - Because RCAS clamps every output pixel strictly within its immediate 1-pixel 3×3 neighborhood `[mnRgb, mxRgb]` with a `-0.1875` lobe cap, it physically cannot tighten a 3-pixel blurred hair strand or eyelash back to crisp 1-pixel definition.
3. **Open-Source Solution (Rule 3 — RawTherapee / Darktable / NASA RL Deconvolution + AMD FSR 1.0 EASU)**:
   - **Regularized, Edge-Masked Luma Richardson-Lucy (RL) Deconvolution** mathematically inverts the downscale blur PSF directly on luminance (`Y`), narrowing blurred edges back to crisp camera-grade sharpness while keeping 100% natural anatomy and exact original color:
     - **Tikhonov/tanh ratio damping**: Eliminates ringing halos around high-contrast boundaries.
     - **Gradient Edge-Confidence Mask** (`smoothstep(0.012, 0.065, grad)`): Protects smooth skin, bokeh, and sunset skies from noise amplification while restoring full crispness to eyelashes, irises, hair strands, collar stripes, necklaces, and rocks.
     - **Luma-only delta injection** (`clamp(dy, -0.18, 0.18)`): Preserves 100% original hue and saturation with zero chromatic artifacts or dark eye craters.
     - **Dual-scale micro-clarity pass**: Restores crisp local micro-contrast without GAN hallucination.

---

## 2. Goals & Acceptance Criteria
- [ ] **Pure-Rust WebGL2 GPU Pipeline (`app/src/storage/realesrgan_gpu.rs`)**:
  - Chain **Pass 1: AMD FSR 1.0 EASU** (directional 2× upscaling when `target > src`) → **Pass 2: Regularized Edge-Masked Luma Richardson-Lucy Deconvolution** (`6 iterations`, $\sigma = 1.15$ for 2× upscaled images; `3 iterations`, $\sigma = 0.85$ for 1× high-res images) → **Pass 3: Edge-Gated Micro-Clarity & Luma Recombination**.
  - Runs entirely on GPU in **< 25 ms** with **zero model downloads** and **zero main-thread stalls**.
- [ ] **Matching Fast OffscreenCanvas Worker Fallback (`app/web/ai_restorer.js`)**:
  - Implement matching regularized RL deconvolution + clarity fallback in the background worker when WebGL2 is unavailable.
- [ ] **Immediate Cache Invalidation**:
  - Bump `PIPELINE_VERSION = 'fsr1-rl-deconv-v16'` in `app/web/ai_restorer.js`.
  - Bump `STATIC_CACHE_NAME = 'chatw-static-v16'` in `app/web/sw.js`.
- [ ] **Quantitative & Visual Verification**:
  - Playwright E2E test verifying Laplacian sharpness gain (**> 2.0× sharper** than Spec 138 EASU-only on 512×384 portrait!), `SSIM >= 0.94`, GPU runtime `< 80 ms`, ZERO `.cwsr` downloads, and visually verified razor-sharp hair, eyelashes, eyes, collar, and natural skin!
- [ ] **Mandatory Auto-Push (Rule 1)**:
  - Automatically commit and push to `origin main`.
