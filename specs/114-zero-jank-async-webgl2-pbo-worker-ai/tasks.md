# Spec 114 Tasks

- [x] T001: Enable `WebGlSync` and `WebGlBuffer` in `app/Cargo.toml`.
- [x] T002: Implement non-blocking `gl.fenceSync` polling, 2-phase Async `PIXEL_PACK_BUFFER` (PBO) quality-gate readback, chunked FP16 weight uploads, and `warmup_gpu()` in `app/src/storage/realesrgan_gpu.rs` + export `warmupRealEsrganGpu` in `app/src/storage/wasm_bridge.rs`.
- [x] T003: Offload blob-to-RGBA decoding and 4K `ImageBitmap`-to-JPEG encoding to a persistent `OffscreenCanvas` Web Worker + idle GPU warmup in `app/web/ai_restorer.js`.
- [x] T004: Yield 2 `rAF` frames on Lightbox auto-enhance start in `app/src/components/rich.rs` and remove heavy full-screen `backdrop-filter` fade hitch on `.lightbox` in `app/assets/main.css`.
- [x] T005: Build release WASM (`./app/build-web.sh release`), verify live `rAF` stall profile drops from `500ms` -> `52ms` via `app/e2e/spec114-zero-jank-lightbox-ai-profile.mjs`, and pass all regression E2E suites (`spec113`, `spec112`, `spec101`, `verify_clean_ai_ui_and_flows`).
- [x] T006: Update `tasks.md` and commit/push to `origin main` per `AGENTS.md`.
