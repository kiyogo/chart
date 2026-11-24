# Spec 121: Ultra-Sharp AI Restoration & Maximum Aggressive Image Compression

## 1. Problem Statement & User Intent
The user requested:
1. "làm nét ảnh bằng AI mạnh hơn nữa đi nhé" (Make AI image sharpening / super-resolution significantly stronger).
2. "và nén ảnh mạnh hơn nữa nhé" (Compress uploaded / transmitted images significantly stronger).

The user attached an anime artwork image opened in ChatW lightbox to illustrate the desired effect: crisp, vibrant, razor-sharp lines and vivid textures after AI enhancement, while saving maximum bandwidth and storage via ultra-aggressive compression during upload and transit.

## 2. Requirements & Acceptance Criteria
1. **Stronger AI Sharpening & Restoration**:
   - WebGL2 fragment shader `fs_color_lock`:
     - Increase AMD CAS peak weight from `-0.28` to `-0.38`, clamp bound from `-0.21` to `-0.28`.
     - Increase radius-2 isotropic deconvolution weight from `0.94` to `1.28`.
     - Increase radius-1 micro-detail boost from `0.34` to `0.46`.
     - Increase dark ink / fine-line stroke tightener from `0.22` to `0.34`, with tighter edgeRange sensitivity `smoothstep(0.10, 0.38, edgeRange5)` to make hair, eyes, eyelashes, and textile folds razor-sharp with zero white halos or ringing.
     - Increase base GPU sharpening multiplier `color1[2]` from `1.32` (anime) / `1.22` (general) to `1.68` (anime) / `1.55` (general).
   - Quality evaluation gate:
     - Update `max_sharpness_boost` in `crates/chatw-media-core/src/quality.rs` from `2.35` to `3.60` so ultra-sharp enhanced images pass the anti-degradation gate cleanly.
     - Ensure SSIM quality gate in `crates/chatw-media-core/src/ai/realesrgan.rs` accepts enhanced high-frequency detail.
2. **Maximum Aggressive Image Compression**:
   - In `app/web/media_worker.js` & `app/web/media_optimizer.js`:
     - In `adaptiveMaxEdge`: downscale images with `maxEdge > 900` by 0.5x, scale images with `maxEdge > 640` to 540-640px for default ultra-optimized transmission.
     - Reduce AVIF quality `q` from `0.48` to `0.38` (and data saver from `0.44` to `0.32`, crisp text from `0.54` to `0.44`). This achieves 35-50% smaller AVIF byte sizes while preserving structural integrity for the receiver's on-device AI 4x Super-Resolution.
   - In `crates/chatw-media-core/src/avif.rs`:
     - In `UltraAggressive` mode: reduce target quality from `58.0..62.0` down to `45.0..50.0`, with aggressive max edge downscaling at `> 900`.
3. **Compatibility & Zero Regression**:
   - Sender instant preview (< 20ms) continues to show the original untouched file locally.
   - E2E tests `spec121`, `spec120`, `spec116`, and `spec115` must pass 100%.
   - Full automated git commit and push to `origin main` according to Rule 1.
