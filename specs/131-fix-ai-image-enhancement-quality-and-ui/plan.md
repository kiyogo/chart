# Plan 131: Fix AI Image Enhancement Quality and Interface Glitches

## 1. Architecture & Design

### A. Pristine AMD FidelityFX CAS Shader in `app/src/storage/realesrgan_gpu.rs`
Replace the unconstrained Laplacian overshooting filter with mathematically bounded AMD FidelityFX Contrast Adaptive Sharpening:
- Sample center `e` and 8 neighbors: 4 cross (`b, d, f, h`) and 4 diagonal (`a, c, g, i`).
- Compute scalar luminance `eL`, `bL, dL, fL, hL, aL, cL, gL, iL`.
- Local min/max:
  - `float mnCrossL = min(min(min(dL, eL), min(fL, bL)), hL);`
  - `float mxCrossL = max(max(max(dL, eL), max(fL, bL)), hL);`
  - `float edgeRange = mxCrossL - mnCrossL;`
- Skin & Flat Tone Protection Gate:
  - `float gate = smoothstep(0.015, 0.065, edgeRange);`
  - Smooth skin and subtle eyelid shadows (`edgeRange < 0.015`) receive zero post-sharpening distortion, keeping them soft, natural, and artifact-free.
- CAS Adaptive Weight:
  - `float rcpM = 1.0 / max(mxCrossL, 1e-4);`
  - `float amp = sqrt(clamp(min(mnCrossL, 2.0 - mxCrossL) * rcpM, 0.0, 1.0));`
  - `float peak = -0.15 * clamp(uColor1.z, 0.0, 1.2) * gate;`
  - `float w = max(amp * peak, -0.18);`
- 8-Tap CAS Cross + Diagonal Formula:
  - `vec3 crossSum = b + d + f + h;`
  - `vec3 diagSum = a + c + g + i;`
  - `vec3 cas = clamp((e + crossSum * w + diagSum * (w * 0.45)) / (1.0 + (4.0 + 1.8) * w), 0.0, 1.0);`
- STRICT NEIGHBORHOOD BOUNDS (No Overshoot / Zero Darkening):
  - `vec3 mnRgb = min(min(min(min(a, b), min(c, d)), min(e, f)), min(g, min(h, i)));`
  - `vec3 mxRgb = max(max(max(max(a, b), max(c, d)), max(e, f)), max(g, max(h, i)));`
  - `vec3 sharp = clamp(cas, mnRgb, mxRgb);`
  - Because `sharp` is clamped to `[mnRgb, mxRgb]`, it is mathematically impossible for any pixel to become darker than its darkest neighbor! This completely eliminates black rings around eyes and skin craters!

### B. Intelligent Model Selection in `crates/chatw-media-core/src/ai/realesrgan.rs`
Update `select_realesrgan_model`:
- Check for portraits / real-world photos:
  ```rust
  let is_photo_or_portrait = matches!(
      analysis.class,
      ImageClass::Portrait
          | ImageClass::Photo
          | ImageClass::LowLightNoisy
          | ImageClass::HdrWideGamut
          | ImageClass::AlreadyHeavilyCompressed
  ) || analysis.skin_tone_ratio > 0.04 || analysis.center_skin_ratio > 0.08;

  if is_photo_or_portrait {
      return Some(RealEsrganModelKind::GeneralX4V3);
  }
  ```
- Anime model (`RealEsrganModelKind::AnimeVideoV3`) is strictly reserved for:
  ```rust
  let is_anime = matches!(analysis.class, ImageClass::AnimeManga | ImageClass::Meme | ImageClass::FlatGraphic | ImageClass::AlphaGraphic | ImageClass::CrispTextUi)
      || (analysis.strong_outline_ratio > 0.015 && analysis.palette_sparsity > 0.50);
  ```
- Remove the clause that forced `AnimeVideoV3` when `prefer_fast_sw && (w * h) > 220_000`.

### C. Bump Pipeline Version in `app/web/ai_restorer.js`
- Change `const PIPELINE_VERSION = 'rust-realesrgan-upscayl-v9';`
- In `app/dist/ai_restorer.js` as well.

---

## 2. Files to Modify
1. `app/src/storage/realesrgan_gpu.rs`: Replace `GL_FIDELITY_CAS_FS` with strict-clamped AMD FidelityFX CAS; set balanced sharpening knobs (`1.0` anime, `0.85` photo).
2. `crates/chatw-media-core/src/ai/realesrgan.rs`: Fix `select_realesrgan_model` so portraits and real photos are routed to `GeneralX4V3`.
3. `app/web/ai_restorer.js`: Bump `PIPELINE_VERSION` to `rust-realesrgan-upscayl-v9`.
4. `app/dist/ai_restorer.js`: Keep in sync.
5. `app/build-web.sh`: Rebuild frontend bundle.
6. `app/e2e/spec131-pristine-ai-enhancement.mjs`: Create Playwright verification test verifying portrait restoration without blackened eye artifacts.
