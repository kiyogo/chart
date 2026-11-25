# Tasks: Spec 143 — Adaptive Multi-Model AI Restoration & AVIF Crosshatch Fix

- [x] **T1**: Bundle `app/public/models/nomosuni_span_x4.cwsp` (`4xNomosUni_span_multijpg`, CC-BY-4.0, 833 KB) + `LICENSE-CC-BY-4.0-NOMOSUNI-SPAN.txt` and record real SHA-256 digests.
- [x] **T2**: Upgrade `crates/chatw-media-core/src/ai/hardware.rs` (multi-platform backends: WebGPU, WebGL2, Vulkan, NCNN-Vulkan, DirectML, CUDA, Metal, CoreML, NNAPI, QNN, CPU SIMD) and `registry.rs` (real SHA-256s, full model catalog, extensible `ModelManifest`, strict commercial license gate blocking CodeFormer).
- [x] **T3**: Add `RealEsrganModelKind::NomosUniSpanX4` and `select_realesrgan_model_adaptive` (AVIF `source_bpp` + resolution + text/QR zero-hallucination guard) in `crates/chatw-media-core/src/ai/realesrgan.rs`, and sync `router.rs`.
- [x] **T4**: Update `app/src/storage/realesrgan_gpu.rs` (`webglcontextlost` resilience), `app/src/storage/wasm_bridge.rs` (`srcBytes` pass-through, `nomosuni_span_x4` whitelist, `getModelRegistryJson`), `app/web/ai_restorer.js` (`v20`, `AbortSignal` cancellation, adaptive RL `0.65`/`0.40`), and `app/web/sw.js` (`chatw-static-v20`).
- [x] **T5**: Run `cargo test -p chatw-media-core` and `./app/build-web.sh`.
- [x] **T6**: Run real end-to-end AVIF + Adaptive AI benchmark (`app/e2e/spec143-adaptive-ai-benchmark.mjs`) across crushed `512×384` AVIF portrait, `1024×768` AVIF portrait, crisp text/QR image, and flat anime; visually inspect 400%-zoom face crops with `read_file` to confirm ZERO crosshatch and razor-sharp clarity.
- [x] **T7**: Mark all tasks `[x]`, `git add .`, commit, and `git push origin main` (Rule 1).
