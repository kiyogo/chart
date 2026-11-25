# Feature Specification: Spec 138 — Zero-Artifact AMD FSR 1.0 (EASU + RCAS) Image Enhancer

## 1. Problem Statement
The user reported persistent, severe AI enhancement artifacts across both human portraits and general scenes:
- `"nó vẫn lỗi AI làm nét quá nhiều ,.... xấu lôi ,... quá nhiều"` (AI sharpening still has way too many errors, ugly/broken...)
- Attached evidence (4 screenshots):
  1. `media_1795584852872_9824a87d.jpg`: Smeared, plastic-wax skin on nose and lips with melted hair edges.
  2. `media_1795584859417_1451db8f.jpg`: Distorted alien irises and grotesque squiggly folded lower eyelids.
  3. `media_1795584865795_2f477adc.jpg`: Full portrait degraded by Real-ESRGAN 4× GAN hallucination.
  4. `media_1795584991819_f8861795.jpg`: Sunset desert and green bottle melted into ugly oil-painting / watercolor blobs with dark cartoon outlines.

### Root Cause Analysis
1. **Inherent GAN Hallucination in `realesr_general_x4v3` / `realesr_animevideov3`**:
   - Even after Spec 136 preserved new uploads at native resolution, ALL existing chat history images (previously stored at 512×384 or ≤ 800px), all low-res uploads, and any image routed to `realesr_animevideov3` STILL executed Real-ESRGAN 4× (`SRVGGNetCompact` GAN).
   - GAN-trained compact super-resolution models are mathematically prone to inventing fake squiggly eye creases, airbrushing skin pores into plastic wax, and melting natural textures (rocks, foliage, bottles) into smeared oil-painting blobs.
2. **Open Source Research (Rule 3)**:
   - Evaluated `GPUOpen-Effects/FidelityFX-FSR` (MIT), `Hajime-san/web-fsr` (MIT WebGL2 port of `FsrEasuF` + `FsrRcasF`), `Artoriuz/ArtCNN`, and `haasn/libplacebo`.
   - **AMD FidelityFX Super Resolution 1.0 (EASU + RCAS)** is the industry-standard zero-hallucination spatial upscaling & contrast-adaptive sharpening pipeline:
     - **EASU (Edge-Adaptive Spatial Upsampling)**: 12-tap directional Lanczos2-derived elliptical filter aligned along local edges, strictly clamped within local `[min4, max4]`. Zero ringing, zero alien eyes, zero oil-painting blobs.
     - **RCAS (Robust Contrast Adaptive Sharpening)**: Noise-gated adaptive sharpening strictly bounded inside local `[min, max]`, preserving smooth skin gradients and bokeh while crispening eyelashes, hair, rocks, and edges.
     - Runs in **< 5 ms** on WebGL2 GPU with **zero 3.6MB model downloads**.

---

## 2. Goals & Acceptance Criteria
- [ ] **Completely Eliminate Destructive Real-ESRGAN GAN Hallucination**:
  - Replace the default Real-ESRGAN 4× GAN path in `app/web/ai_restorer.js` with **AMD FidelityFX Super Resolution 1.0 (`EASU` + `RCAS`)** for ALL images (portraits, faces, landscapes, anime, memes, old 512×384 chat history photos, and new 1024px+ photos).
- [ ] **Pure-Rust WebGL2 AMD FSR 1.0 Engine (`app/src/storage/realesrgan_gpu.rs` & `wasm_bridge.rs`)**:
  - Implement exact MIT-licensed `GL_FSR1_EASU_FS` (12-tap directional edge-adaptive Lanczos2 with strict `[min4, max4]` anti-ringing clamp) and `GL_FSR1_RCAS_FS` (noise-gated RCAS with strict neighborhood clamp) in WebGL2.
  - Expose `window.__chatwWasmMedia.upscaleFsr1(rgba, w, h, scale, sharpness)` returning a zero-copy GPU canvas in < 5ms.
  - Scale policy:
    - Low-resolution / previously downscaled images (`max(w, h) < 1280`): **2× EASU + RCAS** (e.g. `512×384 -> 1024×768`), capped cleanly at max 2048px.
    - Already high-resolution images (`max(w, h) >= 1280` or `w >= 960 && h >= 720`): **1× Native RCAS** (zero geometric warping, crisp natural clarity).
- [ ] **Zero-Artifact Worker Fallback**:
  - Provide matching fast OffscreenCanvas Worker fallback in `app/web/ai_restorer.js` when WebGL2 is unavailable.
- [ ] **Instant Cache Invalidation for All Broken Historical AI Outputs**:
  - Bump `PIPELINE_VERSION` to `'fsr1-easu-rcas-v15'` in `app/web/ai_restorer.js`.
  - Bump `STATIC_CACHE_NAME` to `'chatw-static-v15'` in `app/web/sw.js`.
  - Cache-bust `/media_worker.js?v=${TS}` inside `app/build-web.sh`.
- [ ] **Visual Proof & E2E Verification**:
  - Run Playwright E2E verifying BOTH a degraded 512×384 portrait (old chat history scenario) AND a 1024×768 portrait AND the desert bottle scene (`media_1795584991819_f8861795.jpg`).
  - Visually inspect zoomed-in eye crops to confirm ZERO alien eyes, ZERO plastic skin, ZERO oil-painting blobs, and sub-50ms execution!
- [ ] **Mandatory Auto-Push (Rule 1)**:
  - Commit and push all changes immediately to `origin main`.
