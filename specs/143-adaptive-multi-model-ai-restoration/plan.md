# Implementation Plan: Spec 143 — Adaptive Multi-Model AI Restoration & AVIF Crosshatch Fix

## 1. Core Rust (`crates/chatw-media-core`)
- **`src/ai/hardware.rs`**: Add `WebGl2`, `NcnnVulkan`, `Cuda`, `Qnn` to `AccelerationBackend`; add `display_max_edge: Option<u32>` and multi-platform backend selection helpers.
- **`src/ai/registry.rs`**:
  - Add `ModelFormat` (`Cwsp1`, `Cwsr1`, `Onnx`, `Ncnn`, `CoreMl`, `Builtin`), `min_vram_mb`, `download_url`, `input_channels`, `output_channels` to `ModelDescriptor`.
  - Allow `"CC-BY-4.0"` in `PERMITTED_LICENSES` (permissive commercial attribution license used by `4xNomosUni_span_multijpg` and `4xLSDIRCompact3`).
  - Populate real SHA-256 digests for all 5 bundled models (`clearreality_span_x4`, `nomosuni_span_x4`, `lsdir_compact3_x4`, `realesr_animevideov3`, `realesr_general_x4v3`) plus lazy-download catalog entries (`realesrgan_x4plus`, `realesrgan_anime_6b`, `swinir-real-sr-x2`, `scunet-denoise-x1`, `gfpgan-v1.4-identity-safe`, `real-cugan-anime-x2`, `fsrcnn-x2-fast`, `light_safmnpp_x4` with explicit license audit note).
  - Add `ModelManifest` JSON loader + `verify_manifest_integrity()` for extensible remote/local model updates.
- **`src/ai/realesrgan.rs`**:
  - Add `RealEsrganModelKind::NomosUniSpanX4` (`"nomosuni_span_x4"`, `"NomosUni AVIF SPAN (4×)"`, `"/models/nomosuni_span_x4.cwsp"`, `"1.0-multijpg"`, `is_span() == true`).
  - Add `select_realesrgan_model_adaptive(rgba, width, height, src_bytes, orig_profile, prefer_fast_sw)`:
    - Computes exact `source_bpp = (src_bytes * 8.0) / (width * height)` when `src_bytes > 0`.
    - Protects crisp text / documents / screenshots / QR codes (`CrispTextUi` with high sharpness & low skin tone → returns `None` to prevent text/QR hallucination!).
    - Routes **heavily compressed / low-res AVIF photos** (`max(w,h) <= 720` OR `(src_bytes > 0 && source_bpp < 0.65)` OR `AlreadyHeavilyCompressed`) → **`NomosUniSpanX4`** (eliminating diagonal AVIF crosshatch!).
    - Routes **clean high-res photos/portraits** → **`ClearRealitySpanX4`** (razor-sharp 4K eyelashes & hair!).
    - Routes **pure flat anime/manga** → **`AnimeVideoV3`**.
    - Keep `select_realesrgan_model` delegating to `select_realesrgan_model_adaptive(..., 0, ...)` for backwards compatibility.
- **`src/ai/router.rs`**: Sync `route_image_restoration` preferred model chains with `nomosuni_span_x4`, `clearreality_span_x4`, `realesrgan_x4plus`, `realesr_general_x4v3`, `realesrgan_anime_6b`, `scunet-denoise-x1`, `fsrcnn-x2-fast`, and `display_max_edge` cap.

## 2. WebAssembly & Browser Engine (`app/`)
- **`app/public/models/nomosuni_span_x4.cwsp`** + `LICENSE-CC-BY-4.0-NOMOSUNI-SPAN.txt`.
- **`app/src/storage/wasm_bridge.rs`**:
  - Update `selectRealEsrganModel(rgba, w, h, origProfileJson, requestedModelId?, srcBytes?)` to pass `srcBytes` into `select_realesrgan_model_adaptive`!
  - Whitelist `nomosuni_span_x4` (and `realesrgan_x4plus`, `realesrgan_anime_6b`, `scunet_denoise`, `fsrcnn_x2`) in `isModelLicenseAllowed`.
  - Expose `getModelRegistryJson()` so the frontend/diagnostics can inspect the canonical Rust `ModelRegistry` directly (single source of truth!).
- **`app/src/storage/realesrgan_gpu.rs`**:
  - Handle `webglcontextlost` cleanly (invalidate cached `GL_ENGINE` so next call re-initializes cleanly instead of failing permanently).
- **`app/web/ai_restorer.js`**:
  - Register `nomosuni_span_x4` in `APPROVED_MODELS`.
  - Pass `srcBytes` to `wasm.selectRealEsrganModel(...)`.
  - Support `meta.signal` (`AbortSignal`) cancellation before/between stages + `cancelRestoration(mediaId)` helper.
  - Tune adaptive Post-AI Optical RL clarity (`0.65` for `nomosuni_span_x4` / `< 900px`, `0.40` for `clearreality_span_x4` / `>= 900px`).
  - Bump `PIPELINE_VERSION = 'adaptive-multi-span-avif-v20'` and `app/web/sw.js` `STATIC_CACHE_NAME = 'chatw-static-v20'`.

## 3. Real Benchmark & Visual Verification
- `cargo test -p chatw-media-core`
- `./app/build-web.sh`
- `node app/e2e/spec143-adaptive-ai-benchmark.mjs` (measures real AVIF bytes, ratio, PSNR, SSIM, routing, latency, text/QR protection, and saves 400%-zoom face crops for `read_file` visual verification).
- `git add .`, commit, `git push origin main`.
