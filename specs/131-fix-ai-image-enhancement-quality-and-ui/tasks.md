# Tasks 131: Fix AI Image Enhancement Quality and Interface Glitches

- [x] Task 1: Update `app/src/storage/realesrgan_gpu.rs` with true strict-clamped AMD FidelityFX CAS shader and balanced knobs <!-- id: 0 -->
- [x] Task 2: Update `crates/chatw-media-core/src/ai/realesrgan.rs` to route portraits and photos to `GeneralX4V3` <!-- id: 1 -->
- [x] Task 3: Bump `PIPELINE_VERSION` to `rust-realesrgan-upscayl-v9` in `app/web/ai_restorer.js` <!-- id: 2 -->
- [x] Task 4: Build Web Bundle with `./app/build-web.sh` and verify compilation <!-- id: 3 -->
- [x] Task 5: Create and run Playwright verification test `app/e2e/spec131-pristine-ai-enhancement.mjs` verifying portrait face quality <!-- id: 4 -->
- [x] Task 6: Mandatory auto-commit and git push to `origin main` (Rule 1) <!-- id: 5 -->
