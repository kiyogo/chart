# Spec 115 Tasks

- [x] T001: Remove all borders/shadows/outlines on Lightbox, AI overlay, progress pill, and chat image surfaces in `app/assets/main.css`, and add CSS hide rule for `#canvas-scrollbar-overlay` and `.panel-resizer` when Lightbox is open.
- [x] T002: Fix `app/web/canvas_scrollbars.js` (`zIndex: 8000` + `isLightboxOpen()` suppression across `render`, `scroll`, `pointermove`, `pointerdown`).
- [x] T003: Implement strict backend `.svg`/`.svgz`/XML-SVG blocking (`is_blocked_svg_upload` + unit tests) in `web/backend/src/storage/router.rs`, `dedup_engine.rs`, `references.rs`, `users/mod.rs`, and `library/mod.rs`.
- [x] T004: Implement strict frontend `.svg`/`.svgz`/XML-SVG blocking + toast feedback across `app/web/media_optimizer.js`, `app/src/platform/mod.rs`, `app/src/components/rich.rs`, `app/src/components/composer.rs`, `app/src/components/profile_modal.rs`, `app/src/components/settings_view.rs`, and `app/src/pages/chat.rs`.
- [x] T005: Upgrade `GL_FIDELITY_CAS_FS` to 3-scale edge-aware ultra-sharp anti-ringing deconvolution + dark-ink tightener in `app/src/storage/realesrgan_gpu.rs` and bump `PIPELINE_VERSION` to `v8` in `app/web/ai_restorer.js` & `app/src/storage/wasm_bridge.rs`.
- [x] T006: Run Rust unit tests, rebuild backend & restart live daemon on port 8085, build release WASM (`./app/build-web.sh release`), run `spec115` + full E2E regressions (`spec114`, `spec113`, `spec112`, `spec101`, `verify_clean_ai_ui`), and visually inspect screenshots.
- [x] T007: Update `tasks.md` and commit/push to `origin main` per `AGENTS.md`.
