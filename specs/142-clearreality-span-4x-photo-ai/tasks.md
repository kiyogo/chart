# Tasks: Spec 142 — Pure-Rust SPAN 4× (`4x-ClearRealityV1`) Engine

- [x] **T1**: Copy verified `clearreality_v1_x4.cwsp` (`852,596` bytes, Apache-2.0) into `app/public/models/clearreality_v1_x4.cwsp` and confirm `.gitignore` tracks it.
- [x] **T2**: Implement `crates/chatw-media-core/src/ai/span.rs` (`CWSP1` parser, FP16 GPU weight packers, Pure-Rust CPU tiled fallback, and unit tests) and export in `crates/chatw-media-core/src/ai/mod.rs`.
- [x] **T3**: Add `RealEsrganModelKind::ClearRealitySpanX4` to `crates/chatw-media-core/src/ai/realesrgan.rs`, route all Photo/Portrait/Mixed images to `ClearRealitySpanX4`, and register `clearreality_span_x4` (`Apache-2.0`) in `registry.rs`.
- [x] **T4**: Implement Pure-Rust WebGL2 FP16 6-MRT / 4-MRT SPAN 4× engine in `app/src/storage/realesrgan_gpu.rs` and dispatch `kind.is_span()` inside `restore_image_realesrgan_rust`.
- [x] **T5**: Update `app/src/storage/wasm_bridge.rs`, `app/web/ai_restorer.js` (register model, adaptive Post-AI RL `0.85`/`0.45`, bump `PIPELINE_VERSION` to `v19`), and `app/web/sw.js` (`chatw-static-v19`).
- [x] **T6**: Run `cargo test -p chatw-media-core` and `./app/build-web.sh`.
- [x] **T7**: Run Playwright E2E test (`app/e2e/spec142-clearreality-span.mjs`) on both `512×384` crushed portrait and `1024×768` pristine portrait; visually inspect 400%-zoom face crops via `read_file` to verify razor-sharp eyelashes/eyes/hair and zero artifacts.
- [x] **T8**: Check off all tasks, `git add .`, commit (Conventional Commit), and `git push origin main` (Rule 1).
