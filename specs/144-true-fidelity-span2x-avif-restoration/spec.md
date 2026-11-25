# Spec 144: True Ground-Truth AI Restoration Audit, Native SPAN 2× (`2xNomosUni_span_multijpg`) Engine & Adaptive Perceptual AVIF

## 1. Root Cause Findings from Full-Pipeline Audit
Auditing the full pipeline against the **uncompressed original `1024×768` image** (instead of the compressed AVIF input) revealed 3 critical findings:
1. **Misleading SSIM Gate**: Comparing 4× AI output downsampled back to the *compressed AVIF* reported `SSIM = 0.9965` even when fine eyelashes, silver necklace links, and hair strands were degraded. True full-reference evaluation against the uncompressed original exposed the real gaps (`PSNR`, `trueSsim`, `edgeCorr`, `colorMae`).
2. **4× Over-Upscaling & Post-AI RL Overhead on `>= 640px` Photos**: Upscaling a `1024×768` AVIF by `4×` (`4096×3072`) forced the model to hallucinate 15/16 pixels (higher artifact risk, 4× VRAM) and then running 5-pass Post-AI RL Deconvolution added unnecessary blur/ringing risk.
3. **Multi-Model Ground-Truth Benchmark Results (Real GPU & FP32 Inference)**:
   - `2xNomosUni_span_multijpg` (`CC-BY-4.0`, 802 KB FP16, SPAN **2×**): **#1 overall** — `trueSsim = 0.9771`, `edgeCorr = 0.9832` on `1024×768` 32 KB AVIF, AND `trueSsim = 0.9496`, `edgeCorr = 0.9519` (+0.0327 over un-restored AVIF) on crushed `512×384` 13.7 KB AVIF! Visual 400% inspection confirmed **100% preservation of silver necklace chain, lower & upper eyelashes, individual hair strands, and natural skin with ZERO crosshatch**.
   - `4xNomosUni_span_multijpg` (`CC-BY-4.0`, 833 KB FP16, SPAN **4×**): Best for tiny `< 640px` heavily compressed chat photos needing 4× scaling (`2048×1536`) with zero crosshatch.
   - `Light SAFMN++` (`129 KB` FP16, AIS 2024 AVIF 4K SR): Verified with real FP32 inference — zero artifacts and ultra-tiny, but visually softer on hair/eyelashes than `2xNomosUni SPAN` (ideal as a lightweight constrained fallback, not primary).
   - `2xHFA2kSPAN` (`802 KB`): Lower PSNR (`26.77 dB`) and higher color shift (`colorMae = 10.26`) — rejected.

## 2. Acceptance Criteria
1. **Bundle `nomosuni_span_x2.cwsp`** (`821,420` bytes = `802.2 KB`, `CC-BY-4.0`, `CWSP1` `scale=2`) at `app/public/models/nomosuni_span_x2.cwsp` with verified SHA-256 in `ModelRegistry`.
2. **Native SPAN 2× + 4× Support in Pure-Rust Core & WebGL2 Engine**:
   - Extend `crates/chatw-media-core/src/ai/span.rs` (`SpanGpuF16Model`, `SpanModel`, `run_span_tiled`) to support both `scale = 2` (`upsampler: 48 -> 12`, `PixelShuffle(2×)`) and `scale = 4` (`upsampler: 48 -> 48`, `PixelShuffle(4×)`).
   - Extend `app/src/storage/realesrgan_gpu.rs` (`GL_SHUFFLE4X_FS` with `uniform int uScale;` + dynamic `scale = kind.scale()`) so `NomosUniSpanX2` runs on the 6-MRT/4-MRT FP16 WebGL2 GPU engine with exact `OriginalQualityProfile` color lock (`GL_FIDELITY_CAS_FS` gentle `0.25` CAS, eliminating the `+4/255` global brightness offset!).
3. **Adaptive Multi-Scale Routing (`select_realesrgan_model_adaptive`)**:
   - Photos/Portraits with `max_edge >= 640` → **`NomosUniSpanX2`** (`2×`, zero over-hallucination, zero Post-AI RL needed, 4× less VRAM!).
   - Tiny/crushed Photos/Portraits with `max_edge < 640` → **`NomosUniSpanX4`** (`4×`, zero crosshatch!).
   - Pure flat Anime/Manga → **`AnimeVideoV3`** (`4×`).
   - Crisp Text/Documents/QR → **`None`** (zero text/QR hallucination!).
4. **Adaptive Perceptual AVIF Compression (`crates/chatw-media-core/src/avif.rs`)**:
   - Raise portrait `UltraAdaptive` minimum quality candidate to `76..86` (`min_ssim_floor = 0.96`) to protect eyelashes, hair, and delicate jewelry during AVIF encoding.
5. **True Full-Reference Ground-Truth Benchmark (`app/e2e/spec144-true-ground-truth-e2e.mjs`)**:
   - Compare final production pipeline output **directly against uncompressed originals** on BOTH user test portraits (`media_1795549648014_4417d69b.jpg` and `media_1795545206648_e1cddb86.jpg`) at 100%, 200%, and 400% zoom, measuring true PSNR, true windowed SSIM, edge correlation, color MAE, and exporting lossless PNGs + diff heatmaps for visual inspection via `read_file`.
6. **Mandatory Auto-Push (Rule 1)**: Commit and push cleanly to `origin main`.
