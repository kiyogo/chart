# Spec 142: Pure-Rust SPAN 4× (`4x-ClearRealityV1`, Apache-2.0) Ultra-Sharp Photorealistic Neural Super-Resolution

## 1. Problem Statement
In Spec 140/141, we replaced `realesr-general-x4v3` (which distorted eyes and melted foliage) with `4xLSDIRCompact3` (`SRVGGNetCompact`, 16 flat convs, no skip connections, no attention) inside a 2-stage progressive pipeline (Algorithmic first → AI second). While `4xLSDIRCompact3` eliminated compression blocks and eye distortions, user visual review (`media_1795590223141_78718678.jpg`) confirmed that on compressed low-resolution portraits (e.g. `512×384`), its 400%-zoom output was **still visibly soft/blurry** on eyelashes, irises, eyebrows, and individual hair strands because `SRVGGNetCompact` lacks residual attention and multi-scale feature fusion.

## 2. Open-Source Research (Rule 3)
We investigated CVPR NTIRE 2024 winning architectures on GitHub (`hongyuanyu/SPAN`, `chaiNNer-org/spandrel`) and OpenModelDB / HuggingFace under strictly permissive licenses (`Apache-2.0`, `CC-BY-4.0`, `BSD-3-Clause`):
- **`SPAN` (Swift Parameter-free Attention Network)** (`hongyuanyu/SPAN`): 48 feature channels (`12` RGBA slices), 6 `SPAB` residual spatial-channel attention blocks (`SiLU` + parameter-free `(out3 + x) * (sigmoid(out3) - 0.5)` attention) + 4-way multi-scale feature concatenation (`conv_cat([feat0, conv_2(b6), b1, b6_c1_silu])`) + `PixelShuffle(4×)`.
- All 20 `Conv3XC` re-parameterized training blocks fuse mathematically into single 3×3 convolutions at inference (`426,288` parameters = **only `832.6 KB` FP16** — **30% smaller and ~28% fewer FLOPs than `4xLSDIRCompact3`**!).
- Visual A/B benchmark on the real AMD Radeon RX 590 GPU (`512×384 q=0.45` crushed portrait → `2048×1536`, inspected at 400% face zoom):
  - `4xLSDIRCompact3 + RL`: Soft/blurry eyes and smeared bangs.
  - `4xNomosUni_span_multijpg` (`CC-BY-4.0`): Cleaner skin, still minor smeared patches in dark bangs.
  - **`4x-ClearRealityV1` (`Kim2091/ClearRealityV1`, `Apache-2.0`)**: **Razor-sharp individual hair strands, sparkling crisp eyes, natural eyelids & eyelashes, realistic organic skin micro-texture, ZERO 8×8 compression blocks, ZERO alien eye folds, ZERO oil-painting blobs!**
  - Fused `CWSP1` FP16 weights verified across all 22 layers against Kim2091's official NCNN `4x-ClearRealityV1-fp16.bin` (`worst_w_diff = 0.006`, exact IEEE-754 FP16 precision).

## 3. Acceptance Criteria
1. **Bundle `clearreality_v1_x4.cwsp`** (`852,596` bytes, `Apache-2.0`, `CWSP1` FP16 format) at `app/public/models/clearreality_v1_x4.cwsp` (served at `/models/clearreality_v1_x4.cwsp`).
2. **Pure-Rust SPAN Core (`crates/chatw-media-core/src/ai/span.rs`)**:
   - `CWSP1` header & FP16 parser (`SpanGpuF16Model`, `SpanModel`), cache-line-contiguous RGBA16F GPU weight packers (`conv_in`, `conv48`, `conv_cat`), and Pure-Rust CPU tiled fallback (`run_span_tiled`).
   - Unit tests passing in `cargo test -p chatw-media-core`.
3. **Pure-Rust WebGL2 SPAN GPU Engine (`app/src/storage/realesrgan_gpu.rs`)**:
   - FP16 6-MRT (`24ch/pass` = **exactly 2 uniform draw calls per 48ch layer** when `MAX_DRAW_BUFFERS >= 6`) with automatic 4-MRT fallback (`3 draw calls/layer`), chunked async `gl.fenceSync` execution (60–120fps UI), seamless halo tiling for high-res images, Wavelet Color Anchor (`GL_FIDELITY_CAS_FS`), and 0.1ms async PBO quality gate.
4. **2-Stage Progressive Pipeline Preserved (`app/web/ai_restorer.js`, `app/src/components/rich.rs`)**:
   - **Stage 1 (Instant Algorithmic `< 150ms`)**: AMD FSR 1.0 EASU 2× + Regularized RL Deconvolution displays immediately.
   - **Stage 2 (Ultra-Sharp Neural 4×)**: `4x-ClearRealityV1` SPAN 4× upgrades all Photos/Portraits/Mixed images + adaptive Post-AI Optical RL Clarity (`0.85` for low-res `< 900px`, `0.45` for high-res `>= 900px`). Pure flat cel-shaded Anime/Manga continues using `realesr_animevideov3`.
   - Bump `PIPELINE_VERSION` (`v19`) and `STATIC_CACHE_NAME` (`chatw-static-v19`).
5. **Visual & E2E Verification**:
   - Playwright E2E test verifies BOTH crushed `512×384` AND pristine `1024×768` portraits through the real compiled Rust WASM WebGL2 engine, inspecting 400%-zoom face crops with `read_file` to guarantee razor-sharp eyelashes/hair/eyes and zero artifacts.
6. **Mandatory Git Auto-Push (Rule 1)**: Clean commit and push to `origin main`.
