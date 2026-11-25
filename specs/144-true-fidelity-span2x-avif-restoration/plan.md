# Implementation Plan: Spec 144 — Native SPAN 2× (`2xNomosUni_span_multijpg`) & True Ground-Truth Pipeline

## 1. Model Weights & Registry
- Copy `scratch/nomosuni_span_x2.cwsp` (`821,420` bytes) → `app/public/models/nomosuni_span_x2.cwsp`.
- Compute real SHA-256 and register `nomosuni_span_x2` (`CC-BY-4.0`, `scale_factor: 2`, `built_in: true`) in `crates/chatw-media-core/src/ai/registry.rs`.

## 2. Pure-Rust SPAN 2× & 4× Core (`crates/chatw-media-core/src/ai/span.rs`)
- Generalize `SpanGpuF16Model::from_cwsp_bytes` and `SpanModel::from_cwsp_bytes` to accept `scale == 2 || scale == 4`:
  - `up_out_ch = 3 * scale * scale` (`12` for `2×`, `48` for `4×`).
  - `pack_span_conv_up_weights(w_u16, out_ch)` pads width to `ceil(out_ch / 4) * 4` (`16` for 4-MRT compatibility when `out_ch == 12`, `48` when `out_ch == 48`).
  - Generalize `PixelShuffle(scale)` in `infer_span_tile_cpu` (`sub = dy * scale + dx`, `s2 = scale * scale`).
- Add unit test verifying bundled `nomosuni_span_x2.cwsp` parses cleanly with `scale == 2` and runs CPU forward pass to exact `2×` dimensions.

## 3. Adaptive Multi-Scale Router & Perceptual AVIF (`realesrgan.rs`, `avif.rs`)
- Add `RealEsrganModelKind::NomosUniSpanX2` (`"nomosuni_span_x2"`, `"NomosUni AVIF SPAN (2×)"`, `"/models/nomosuni_span_x2.cwsp"`, `scale() -> 2`, `is_span() -> true`).
- Update `select_realesrgan_model_adaptive`:
  - Photos/Portraits with `max_edge >= 640` → `NomosUniSpanX2` (2×, crisp eyelashes/necklace/skin, zero crosshatch, 4× less VRAM).
  - Tiny/crushed Photos/Portraits with `max_edge < 640` → `NomosUniSpanX4` (4×, zero crosshatch).
  - Pure flat Anime → `AnimeVideoV3` (4×).
  - Crisp Text/QR → `None`.
- Update `crates/chatw-media-core/src/avif.rs`: Portrait `UltraAdaptive` quality candidates `[76.0, 82.0, 88.0]` with `min_ssim_floor = 0.96` so delicate eyelashes & jewelry survive AVIF encoding cleanly.

## 4. WebGL2 GPU Engine & Browser Pipeline (`realesrgan_gpu.rs`, `wasm_bridge.rs`, `ai_restorer.js`, `sw.js`)
- `realesrgan_gpu.rs`:
  - Add `uniform int uScale;` to `GL_SHUFFLE4X_FS` (`s2 = uScale * uScale`, `sub = (op.y % uScale) * uScale + (op.x % uScale)`).
  - Store `scale: u32` in `GlUploadedSpanModel` and `ActiveSpanTileGl`; allocate `sr_tex` at `(tw * scale, th * scale)`; run `upsampler` for `3` slices (`1` 4-MRT pass) when `scale == 2` or `12` slices when `scale == 4`.
  - Use `let scale = kind.scale() as usize;` in `restore_image_realesrgan_rust` (for both GPU tiling and CPU fallback!).
  - Use gentle `color1[2] = 0.25` CAS for `NomosUniSpanX2` (preserving 100% natural texture while locking exact sender `OriginalQualityProfile` color balance!).
- `wasm_bridge.rs`: Whitelist `nomosuni_span_x2` in `selectRealEsrganModel` and `isModelLicenseAllowed`.
- `ai_restorer.js`:
  - Register `nomosuni_span_x2` in `APPROVED_MODELS`.
  - **Skip Post-AI Optical RL (`upscaleFsr1`) for `nomosuni_span_x2`** (pure SPAN 2× + exact sender color lock is already razor-sharp and avoids extra blur/ringing passes!). Keep gentle `0.55` RL only for `4×` models on tiny `< 640px` inputs.
  - Bump `PIPELINE_VERSION = 'adaptive-span2x-avif-v21'` and `sw.js` `STATIC_CACHE_NAME = 'chatw-static-v21'`.

## 5. Verification
- `cargo test -p chatw-media-core`
- `./app/build-web.sh`
- `node app/e2e/spec144-true-ground-truth-e2e.mjs` on BOTH user test portraits (`media_1795549648014_4417d69b.jpg` & `media_1795545206648_e1cddb86.jpg`), inspecting lossless 400%-zoom PNG crops + error-diff maps via `read_file`.
- Commit & `git push origin main`.
