# Implementation Plan: Spec 142 — Pure-Rust SPAN 4× (`4x-ClearRealityV1`) Engine

## 1. Binary Weight Format (`CWSP1`)
- Header (20 bytes little-endian):
  - `magic: [u8; 4]` = `b"CWSP"`
  - `version: u32` = `1`
  - `scale: u32` = `4`
  - `feat_ch: u32` = `48`
  - `flags: u32` (`bit 0`: `1` = `norm=True` `(rgb - (0.4488, 0.4371, 0.4040)) * 255.0`, `0` = `no_norm`)
- Payload (22 layers, each `weights_f16` followed by `bias_f16`):
  - Layer 0 (`conv_1`): `48 × 3 × 9` + `48`
  - Layers 1..=18 (`block_1..6` × `[c1_r, c2_r, c3_r]`): `48 × 48 × 9` + `48` each
  - Layer 19 (`conv_2`): `48 × 48 × 9` + `48`
  - Layer 20 (`conv_cat`): `48 × 192` + `48`
  - Layer 21 (`upsampler.0`): `48 × 48 × 9` + `48`
- Total size: `852,596` bytes (`832.6 KB`).
- Source: `/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/scratch/clearreality_v1_x4.cwsp` → copy to `app/public/models/clearreality_v1_x4.cwsp`.

## 2. File Changes

### A. `crates/chatw-media-core/src/ai/span.rs` (New)
- `CWSP_MAGIC = b"CWSP"`
- `SpanGpuF16Layer { weights_u16: Vec<u16>, bias: Vec<f32> }`
- `SpanGpuF16Model { scale: u32, feat_ch: usize, use_norm: bool, conv_1, blocks: [[SpanGpuF16Layer; 3]; 6], conv_2, conv_cat, upsampler }`
- GPU cache-line-contiguous RGBA16F texture packers:
  - `pack_span_conv_in_weights` → width `48`, height `9` (`RGBA16F`)
  - `pack_span_conv48_weights` → width `48`, height `108` (`RGBA16F`)
  - `pack_span_conv_cat_weights` → width `48`, height `48` (`RGBA16F`)
- `SpanModel` (f32 CPU model) + `run_span_tiled(...)` Pure-Rust CPU fallback for non-WebGL2 environments.
- Unit tests for header parsing, weight packing invariants, and small-tile CPU forward pass.

### B. `crates/chatw-media-core/src/ai/mod.rs` & `realesrgan.rs` & `registry.rs`
- Export `pub mod span;` in `crates/chatw-media-core/src/ai/mod.rs`.
- Add `RealEsrganModelKind::ClearRealitySpanX4`:
  - `model_id()` → `"clearreality_span_x4"`
  - `model_label()` → `"ClearReality SPAN (4×)"`
  - `cwsr_url()` → `"/models/clearreality_v1_x4.cwsp"`
  - `version()` → `"1.0-span"`
  - `is_span()` → `matches!(self, Self::ClearRealitySpanX4)`
- Update `select_realesrgan_model()` to route all non-flat-anime images (Photos, Portraits, Mixed, General) to `RealEsrganModelKind::ClearRealitySpanX4`.
- Register `clearreality_span_x4` (`Apache-2.0`) in `ModelRegistry` / license whitelist.

### C. `app/src/storage/realesrgan_gpu.rs`
- Add 6-MRT & 4-MRT WebGL2 FP16 shaders for SPAN:
  - `GL_SPAN_CONV_IN_6MRT_FS` / `GL_SPAN_CONV_IN_4MRT_FS` (input `(rgb - mean) * 255.0` when `uNorm == 1`, `3 -> 48`)
  - `GL_SPAN_CONV48_6MRT_FS` / `GL_SPAN_CONV48_4MRT_FS` (`uMode`: `0` linear, `1` SiLU, `2` SPAB `(t3 + skip) * (sigmoid(t3) - 0.5)`)
  - `GL_SPAN_CONV_CAT_6MRT_FS` / `GL_SPAN_CONV_CAT_4MRT_FS` (4-way 1×1 conv over `[feat0, b6_conv2, b1_out, b6_c1_act]`)
  - `GL_SPAN_PS4_FS` (`PixelShuffle(4×)` → `clamp(rgb, 0.0, 1.0)` into `tile.sr_tex`!)
- Add `GlUploadedSpanModel`, `GlActiveSpanTile` (6× `RGBA16F` 12-layer `TEXTURE_2D_ARRAY`: `feat0`, `b1_out`, `b6_c1_act`, `scratch_a`, `scratch_b`, `ping_c`, `pong_d` — zero read/write collision!), `ensure_span_model_uploaded_async`, `run_span_tile_gl_async` (executes 1 SPAB block per non-blocking GPU fence for 60–120fps UI responsiveness!).
- Dispatch `kind.is_span()` inside `restore_image_realesrgan_rust` (reusing the exact same tiling, `GL_FIDELITY_CAS_FS` Wavelet Color Anchor, zero-copy WebGL2 canvas, and 0.1ms async PBO quality gate, plus `run_span_tiled` CPU fallback!).

### D. `app/src/storage/wasm_bridge.rs`, `app/web/ai_restorer.js`, `app/web/sw.js`
- Allow `"clearreality_span_x4"` & `"lsdir_compact3_x4"` in `wasm_bridge.rs` explicit `req_str` override and `isModelLicenseAllowed`.
- Register `clearreality_span_x4` in `APPROVED_MODELS` in `app/web/ai_restorer.js`.
- Tune adaptive Post-AI Optical RL strength: `maxDim < 900 ? 0.85 : 0.45` (razor-sharp on low-res 512×384 chat photos, ultra-clean natural on high-res 1024×768+ originals!).
- Bump `PIPELINE_VERSION = 'progressive-alg-then-clearreality-span-v19'` and `STATIC_CACHE_NAME = 'chatw-static-v19'`.

## 3. Verification Strategy
1. `cargo test -p chatw-media-core` (all existing + new SPAN unit tests pass).
2. `./app/build-web.sh` (Rust WASM release build + frontend bundle compiles cleanly with zero errors).
3. Playwright E2E verification (`app/e2e/spec142-clearreality-span.mjs`) testing BOTH `512×384 q=0.45` crushed portrait AND `1024×768` pristine portrait through `window.__chatwAiRestorer.restoreImageFromUrl` (2-stage progressive flow: Stage 1 Algorithmic → Stage 2 ClearReality SPAN 4× + Optical RL), visually inspecting 400%-zoom face crops with `read_file`.
4. Commit & `git push origin main`.
