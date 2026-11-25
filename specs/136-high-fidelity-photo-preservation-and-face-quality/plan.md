# Technical Plan: Spec 136 — High-Fidelity Photo Preservation & Face Quality

## 1. Architecture Overview
To solve the degraded photo quality and face distortion while adhering to Rule 3 (Open Source research on `dinhkhactientl2/upscale-pro` and `ncoevoet/facet`):

1. **Resolution & Compression Pipeline (`app/web/media_worker.js` & `app/web/media_optimizer.js`)**:
   - Update `adaptiveMaxEdge(w, h, isCrisp, qualityMode)`:
     - If `maxEdge <= 1920`, return `null` (zero downscaling, full 1:1 original resolution preserved).
     - If `maxEdge > 2048`, cap to `1920` (preserving aspect ratio).
     - If `qualityMode === 'data_saver'`, cap to `1280` only when `maxEdge > 1280`.
   - Update quality factor:
     - For photos/portraits (`isCrisp == false`), change `q` from `0.38` to `0.80` (80% quality).
     - For crisp graphics, change `q` to `0.82`.
     - In `data_saver` mode, change `q` to `0.65`.
   - Set `superResRecommended = Boolean(tw < origW)`. For images uploaded at 1024x768 (`tw == origW`), `superResRecommended` will be `false`.

2. **Core Rust AVIF Encoder (`crates/chatw-media-core/src/avif.rs`)**:
   - In `AvifCompressionMode::UltraAggressive`:
     - Do not downscale standard photos below 1920px. Only downscale when `max_dim > 2048` to `1920`.
   - In candidate configs for `ImageClass::Portrait | ImageClass::Photo`:
     - Increase qualities from `48.0 / 62.0 / 74.0` to `78.0 / 82.0 / 86.0`.
     - 10-bit YCbCr ensures smooth skin gradients without contouring/banding.

3. **Smart Face & High-Res Preservation in AI Restorer (`app/web/ai_restorer.js` & `crates/chatw-media-core/src/ai/realesrgan.rs`)**:
   - In `app/web/ai_restorer.js`:
     - Detect when an image is already high resolution (`procW >= 960 || procH >= 720`) and was not downscaled (`meta.superResRecommended === false` or `meta.targetWidth <= procW`):
     - When enhancing a high-resolution image, avoid destructive 4× Real-ESRGAN upscaling to 4K that alters facial geometry.
     - Instead, perform 1× native AMD FidelityFX CAS sharpening pass on the high-res pixels, yielding crisp hair, eyelashes, and eyes while leaving skin textures and facial anatomy 100% natural and unwarped.
     - For genuinely low-resolution images (`procW < 720 && procH < 720` or `meta.superResRecommended === true`), execute Real-ESRGAN 4× (`GeneralX4V3`).
     - Bump `PIPELINE_VERSION = 'rust-realesrgan-facet-v14'`.

4. **Service Worker Cache Invalidation (`app/web/sw.js`)**:
   - Bump `STATIC_CACHE_NAME` to `'chatw-static-v14'` so installed Service Workers fetch the new scripts.

5. **Verification**:
   - Cargo tests in `chatw-media-core`.
   - Build web bundle `./app/build-web.sh`.
   - Playwright test uploading Image 2 (`media_1795549648014_4417d69b.jpg`) to verify 1024x768 transmission and pristine visual fidelity in Lightbox.
