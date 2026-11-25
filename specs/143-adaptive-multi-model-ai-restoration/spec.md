# Spec 143: Complete Adaptive Multi-Model AI Image Restoration System & AVIF Crosshatch Fix

## 1. Root Cause Analysis of User-Reported Artifact (`media_1795592221974_c4e78127.jpg`)
Reproducing the user's real AVIF chat pipeline (`encodeAvifAdaptive` → `restoreImageFromUrl`) proved:
- On **clean/high-res AVIF (`1024×768`, 32 KB, 8.21× smaller)**: `4x-ClearRealityV1` (`clearreality_span_x4`, Apache-2.0, 833 KB) produces studio-grade 4K output (`SSIM = 0.9965`, crisp individual eyelashes, zero artifacts).
- On **heavily compressed low-res AVIF (`512×384`, ~13 KB)**: `4x-ClearRealityV1` (trained on clean/mild degradations) amplifies AV1 directional intra-prediction ringing around eyes and bangs into **diagonal crosshatch mesh and cracked eye rings** (matching `media_1795592221974_c4e78127.jpg` 1:1).
- On the **exact same `512×384` AVIF**, **`4xNomosUni_span_multijpg`** (`nomosuni_span_x4`, CC-BY-4.0, 833 KB, trained specifically on multi-scale heavy compression) produces **ZERO crosshatch, ZERO eye cracking, clean sparkling eyes, crisp hair strands, and higher SSIM (`0.9921` vs `0.9918`)**!

## 2. Requirements
1. **Adaptive Multi-Model Routing (Fix Crosshatch + Full Content/Compression/Hardware Awareness)**:
   - Bundle **`nomosuni_span_x4.cwsp`** (`4xNomosUni_span_multijpg`, CC-BY-4.0, 832.6 KB FP16) alongside `clearreality_v1_x4.cwsp` (`Apache-2.0`, 832.6 KB), `lsdir_compact3_x4.cwsr`, `realesr_animevideov3.cwsr`, and `realesr_general_x4v3.cwsr`.
   - Route automatically in Pure-Rust (`select_realesrgan_model` & `route_image_restoration`) using:
     - **Heavily compressed / low-res AVIF photos** (`max_dim <= 720` OR `source_bpp < 0.65` OR `AlreadyHeavilyCompressed`) → **`nomosuni_span_x4`** (zero AVIF ringing amplification, zero crosshatch!).
     - **Clean / high-res AVIF photos** (`max_dim > 720` AND `source_bpp >= 0.65`) → **`clearreality_span_x4`** (razor-sharp 4K eyelashes, pores, hair strands!).
     - **Pure flat Anime / Manga / Illustrations** → **`realesr_animevideov3`** (or cached `RealESRGAN_x4plus_anime_6B` / `Real-CUGAN`).
     - **Crisp Text / Documents / Screenshots / QR Codes** → **Zero-Hallucination Text/QR Guard** (skip generative neural SR; preserve exact text & QR fidelity!).
     - **Constrained / Low-Power / Low-Battery / Thermal-Throttled** → skip heavy AI or use lightweight fast fallback.
2. **Extensible Manifest-Driven Multi-Platform Model Registry (`crates/chatw-media-core/src/ai/registry.rs`)**:
   - Full metadata for every model: `model_id`, `version`, `sha256_hex` (real SHA-256 for all bundled weights!), `spdx_license`, `commercial_allowed`, `format` (`cwsp1`, `cwsr1`, `onnx`, `ncnn`, `coreml`), `download_url`, `source_repo`, `domain`, `scale_factor`, `input_channels`, `output_channels`, `model_size_bytes`, `min_ram_mb`, `min_vram_mb`, `recommended_ram_mb`, `min_device_tier`, `supported_backends`, `quantization`, `built_in`.
   - Register the full open-source catalog: `clearreality_span_x4` (Apache-2.0), `nomosuni_span_x4` (CC-BY-4.0), `lsdir_compact3_x4` (CC-BY-4.0), `realesr_animevideov3` (BSD-3-Clause), `realesr_general_x4v3` (BSD-3-Clause), `realesrgan_x4plus` (BSD-3-Clause), `realesrgan_anime_6b` (BSD-3-Clause), `light_safmnpp_x4` (AVIF RTSR research/optional — license audited), `swinir-real-sr-x2` (Apache-2.0), `scunet-denoise-x1` (Apache-2.0), `gfpgan-v1.4-identity-safe` (Apache-2.0), `real-cugan-anime-x2` (MIT), `fsrcnn-x2-fast` (MIT).
   - **Strict Commercial License Gate**: `CodeFormer` (`NTU S-Lab 1.0 Non-Commercial`), `RVRT` (`CC-BY-NC-4.0`), `APISR` (`GPL-3.0`), `Upscayl` (`AGPL-3.0`) remain hard-blocked with explicit license reason.
   - Add `ModelManifest` JSON serialization & BLAKE3/SHA-256 verification so models can be added, updated, or lazy-downloaded without modifying app code.
3. **Multi-Platform Acceleration & Resource Guards (`hardware.rs`, `realesrgan_gpu.rs`, `ai_restorer.js`)**:
   - Support all platform backend descriptors: `WebGpu`, `WebGl2`, `Vulkan`, `NcnnVulkan`, `DirectMl`, `Cuda`, `Metal`, `CoreMl`, `NnApi`, `Qnn`, `OnnxCpuSimd`, `SpabLiteCpuSimd`.
   - Web client (`ai_restorer.js`):
     - Pass `srcBytes` (real decoded file byte size) into `selectRealEsrganModel` so Rust computes **exact `source_bpp`** for AVIF compression-aware routing!
     - Add `AbortSignal` / task cancellation support (`cancelTask(mediaId)`) so off-screen/closed images stop immediately.
     - Add `webglcontextlost` recovery & display-size cap (`maxDisplayEdge`) so we never upscale beyond display needs.
     - Preserve zero-copy sender-original display, E2EE client-side-only decryption, private local IndexedDB cache, and zero server-side GPU usage.
4. **Honest Real Benchmark Suite (`app/e2e/spec143-adaptive-ai-benchmark.mjs`)**:
   - Benchmark real ChatW AVIF compression + Adaptive AI restoration on: (a) crushed `512×384` AVIF portrait, (b) `1024×768` AVIF portrait, (c) crisp text/document image, (d) flat anime illustration.
   - Measure real file sizes, AVIF ratio, PSNR, SSIM, routing decisions, and GPU latency; inspect 400%-zoom crops via `read_file`.
   - Report honestly which platforms were verified (Linux x86_64 + Chrome WebGL2/WebGPU AMD RX 590) vs architecture-ready untested native targets (Windows/macOS/Android/iOS/iPadOS).
