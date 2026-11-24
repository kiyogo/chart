# Tasks: Spec 134 — Fix AI High-Detail Portrait Model Routing and Eye Artifacts

- [x] Task 1: Fix `extract_original_quality_profile` and `select_realesrgan_model` in `crates/chatw-media-core/src/ai/realesrgan.rs`
- [x] Task 2: Bump `PIPELINE_VERSION` to `v10` in `app/web/ai_restorer.js`
- [x] Task 3: Add automated unit tests in `crates/chatw-media-core/tests/` to lock portrait/photo vs anime model selection
- [x] Task 4: Run `cargo test -p chatw-media-core` and build with `./app/build-web.sh`
- [x] Task 5: Verify visual output and run Playwright E2E test
- [x] Task 6: Clean up temporary test files and execute mandatory git commit & push (Rule 1)
