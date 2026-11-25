# Tasks: Spec 144 — Native SPAN 2× (`2xNomosUni_span_multijpg`) & True Ground-Truth Pipeline

- [x] **T1**: Bundle `app/public/models/nomosuni_span_x2.cwsp` (`821,420` bytes, CC-BY-4.0), compute SHA-256, and register in `crates/chatw-media-core/src/ai/registry.rs`.
- [x] **T2**: Extend `crates/chatw-media-core/src/ai/span.rs` to support both `scale = 2` and `scale = 4` (GPU FP16 packing + CPU reference + unit tests).
- [x] **T3**: Add `RealEsrganModelKind::NomosUniSpanX2` (`scale() -> 2`), update `select_realesrgan_model_adaptive` in `realesrgan.rs`, and tune Portrait AVIF perceptual quality floor in `avif.rs`.
- [x] **T4**: Extend `app/src/storage/realesrgan_gpu.rs` (`uScale` in `GL_SHUFFLE4X_FS`, dynamic `scale` in SPAN & `restore_image_realesrgan_rust`), `wasm_bridge.rs`, `ai_restorer.js` (`v21`, skip redundant Post-AI RL on `nomosuni_span_x2`), and `sw.js` (`chatw-static-v21`).
- [x] **T5**: Run `cargo test -p chatw-media-core` and `./app/build-web.sh`.
- [x] **T6**: Run `app/e2e/spec144-true-ground-truth-e2e.mjs` comparing production output directly against uncompressed originals on BOTH user portraits + inspecting lossless 400% face/necklace/eye crops via `read_file`.
- [x] **T7**: Check off all tasks, `git add .`, commit, and `git push origin main` (Rule 1).
