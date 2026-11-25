# Tasks 147: Zero-Hang Non-Blocking Hybrid AI Image Restoration Pipeline

- [x] T1: Reproduce live `90%` / `3%` stall in real Chrome with WebGPU enabled (`AMD Radeon RX 590`) and pinpoint exact bottlenecks (`ai_restorer.js`, `face_restorer.js`, `rich.rs`).
- [x] T2: Harden `app/web/face_restorer.js` with `loadOrt` timeout, `detectFaces` timeout, and single-flight `gfpganInitPromise`.
- [x] T3: Decouple Stage 2B (`GFPGANv1.4`) into idle-yielding background `faceBgQueue` + `onRefined` + live `<img>` swap in `app/web/ai_restorer.js`, add strict timeouts to `callAiMediaWorker`, `fetch`, `detectHardware`, `predecodeImageUrl`, and `enqueueExclusive`, and bump `PIPELINE_VERSION = 'hybrid-span-gfpgan-v24'`.
- [x] T4: Add `try/catch` and guaranteed post-loop cleanup (`ai_loading = false` / `upscaling_ai = false`) in `app/src/components/rich.rs`.
- [x] T5: Explicitly bypass `/models/` & `/ort/` in `app/web/sw.js` and bump `STATIC_CACHE_NAME = 'chatw-static-v24'`.
- [x] T6: Rebuild web bundle (`./app/build-web.sh`), run `cargo test -p chatw-media-core` (42/42 passed), and verify in real Chrome CDP with WebGPU enabled that 3 concurrent images ALL hit `100%` without stalling at `90%`/`3%` AND background `GFPGANv1.4` refines seamlessly.
- [x] T7: Commit and push to `origin/main` (`https://github.com/kiyogo/chart`).
