# Spec 131: Fix AI Image Enhancement Quality and Interface Glitches (Pristine AMD CAS, Portrait Model Routing, Zero Artifacts)

## 1. Problem Statement
The user reported:
> *"và lỗi giao diện nó bị lỗi hết làm nét bằng AI rồi*
> *ảnh lỗi hết rồi*
> *làm nét ảnh bằng AI ngu quá"*

With attachments:
- `media_1795540788011_0a8f0da3.jpg`: Severe necrotic / blackened crater rings around human eyelids and eyes.
- `media_1795540816506_d908dfd3.jpg`: Real-life photograph of a girl in a sailor uniform where human skin and facial features were disfigured by AI enhancement.

### Root Causes Identified:
1. **Unconstrained Laplacian Sharpening in `GL_FIDELITY_CAS_FS` (from Spec 127)**:
   - Spec 127 introduced an 8-tap Laplacian high-frequency boost on top of neural network output with `boundMinL = clamp(mnAllL - edgeRange * 0.35, 0.0, 1.0)` and `scalarRatio = clamp(targetL / eL, 0.65, 1.45)`.
   - The low noise gate threshold (`0.008`) treated natural subtle eyelid shadows and skin variations as edges, and multiplied dark shadow pixels by `0.65` (35% darkening) and highlights by `1.45`.
   - This turned natural eyelid shadows and soft eye gradients into blackened, demonic, pitted crater rings.

2. **Mismatched Model Routing (`AnimeVideoV3` forced on Real Portraits)**:
   - In `crates/chatw-media-core/src/ai/realesrgan.rs`, `select_realesrgan_model` contained:
     `analysis.flat_pixel_ratio > 0.22 && analysis.color_saturation_mean > 0.16` or `prefer_fast_sw && (w * h) > 220_000`.
   - Real-world portraits with smooth skin and blurred bokeh backgrounds matched this condition, forcing the anime cartoon model `realesr_animevideov3` on human faces!
   - Anime models draw hard cartoon ink outlines around eyes and cel-shade skin, completely destroying real human faces. Real photos and portraits must use `realesr_general_x4v3` (DNI-35).

3. **Stale Corrupted Cache in IndexedDB**:
   - Because previous outputs were stored under `PIPELINE_VERSION = 'rust-realesrgan-upscayl-v8'`, the corrupted images persist in browser storage.
   - Bumping `PIPELINE_VERSION` to `rust-realesrgan-upscayl-v9` invalidates the old corrupted outputs and regenerates clean, natural 4K images.

---

## 2. Requirements & Goals

1. **Restore True AMD FidelityFX CAS (Zero Black Craters / Zero Halos)**:
   - Eliminate unconstrained Laplacian overshooting (`boundMinL - edgeRange * 0.35` and `0.65` darkening).
   - Re-implement mathematically proven AMD FidelityFX Contrast Adaptive Sharpening where the output is strictly bounded: `clamp(cas, mnRgb, mxRgb)`.
   - No pixel can ever be darkened below its darkest local neighbor or brightened above its brightest neighbor.
   - Gentle noise gate (`smoothstep(0.015, 0.065, edgeRange)`) protecting skin tones, eyelids, and smooth gradients while keeping eyelashes, pupils, hair contours, and fine textures crisp.

2. **Accurate Portrait & Photo Model Routing**:
   - Detect real portraits and photos (`ImageClass::Portrait`, `ImageClass::Photo`, or `skin_tone_ratio > 0.05` or `center_skin_ratio > 0.10`).
   - Route all portraits and real-life photos strictly to `RealEsrganModelKind::GeneralX4V3` (`realesr_general_x4v3.cwsr`).
   - Reserve `AnimeVideoV3` strictly for genuine anime/manga art, 2D digital illustrations, and memes.
   - Do NOT allow software WebGL or image size to force `AnimeVideoV3` onto a human portrait.

3. **Cache Invalidation (`v9`)**:
   - Bump `PIPELINE_VERSION = 'rust-realesrgan-upscayl-v9'` in `app/web/ai_restorer.js` so all previously corrupted cached images are refreshed with pristine neural quality.

4. **Verification**:
   - Test portrait super-resolution using `realesr_general_x4v3` + AMD CAS on real portrait photos.
   - Verify that eye regions and skin have ZERO blackened rings, zero craters, and natural, beautiful clarity.
   - Run E2E test verifying Lightbox AI enhancement and wand toggle functionality.
