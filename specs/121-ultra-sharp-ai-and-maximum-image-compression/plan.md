# Technical Plan - Spec 121: Ultra-Sharp AI Restoration & Maximum Aggressive Image Compression

## 1. Architecture & Component Changes

### 1.1 GPU WebGL2 Shader & Sharpening Multiplier
- File: `app/src/storage/realesrgan_gpu.rs`
  - In `fs_color_lock` shader:
    - `peak = -0.38 * uColor1.z * gate1;`
    - `w = max(amp * peak, vec3(-0.28));`
    - Deconvolution boost: `+ (cas1 - avg2) * (1.28 * uColor1.z * gate2) + (cas1 - avg1) * (0.46 * uColor1.z * gate1)`
    - Dark ink outline stroke tightener: `darkLine = clamp((lAvg2 - lDeconv) * 2.8, 0.0, 1.0) * smoothstep(0.10, 0.38, edgeRange5);` and `deconv = mix(deconv, mn5, darkLine * (0.34 * min(uColor1.z, 1.70)));`
  - In `restore_image_realesrgan_rust`:
    - `color1[2] = if kind == RealEsrganModelKind::AnimeVideoV3 { 1.68 } else { 1.55 };`

### 1.2 Quality Evaluation & Anti-Degradation Gate
- File: `crates/chatw-media-core/src/quality.rs`
  - Increase `max_sharpness_boost` to `3.60` (allowing high-frequency Laplacian edge boost without tripping the guard).
- File: `crates/chatw-media-core/src/ai/realesrgan.rs`
  - In `evaluate_sr_quality_gate`: ensure SSIM tolerance accounts for ultra-crisp micro-edge sharpening (`ssim >= 0.70`).

### 1.3 Client Image Compression Pipeline
- File: `app/web/media_worker.js`:
  - Update `adaptiveMaxEdge` to downscale `maxEdge > 900` to `Math.round(maxEdge * 0.5)` and `maxEdge > 640` to `540`.
  - Update AVIF quality `q`: `isCrisp ? 0.44 : (opts.qualityMode === 'data_saver' ? 0.32 : 0.38)`.
- File: `app/web/media_optimizer.js`:
  - Sync `adaptiveMaxEdge` and `q`: `isCrisp ? 0.44 : (qualityMode === 'data_saver' ? 0.32 : 0.38)`.
- File: `crates/chatw-media-core/src/avif.rs`:
  - In `UltraAggressive` mode: tune quality targets down to `45.0..50.0`.
  - In `aggressive_max_edge`: downscale at `max_dim > 900`.

### 1.4 E2E Test Suite
- File: `app/e2e/spec121-ultra-sharp-ai-and-max-compression.mjs`:
  - Verify that image compression achieves significant byte reduction (comparing original PNG vs compressed AVIF).
  - Verify that AI restoration produces higher Laplacian edge sharpness and crisp outlines.
  - Verify lightbox displays ultra-sharp output without artifacts or scrollbars.
