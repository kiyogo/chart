# Spec 112 Tasks

- [x] T001: Add deterministic pure-Rust sizing helpers (`compute_lightbox_target_dimensions`, `compute_lightbox_fit_box`, `compute_chat_image_box`, `read_viewport_size`) and unit tests in `app/src/components/rich.rs`.
- [x] T002: Extend `LightboxMedia` with `width`, `height`, `orig_width`, `orig_height` and populate them in `RichImage` (`app/src/components/rich.rs`) and `SelfMediaTile` (`app/src/components/details_panel.rs`).
- [x] T003: Refactor `GlobalImageLightbox` in `app/src/components/rich.rs` for Frame-0 deterministic sizing, Frame-0 enhancing overlay (no flash), single rAF-debounced resize listener, GPU opacity crossfade reveal (`is-revealing`), and direct `.lightbox-img` transform invariant (Spec 078 / Spec 101).
- [x] T004: Stabilize `RichImage` chat bubble dimensions and smooth `is-revealing` crossfade in `app/src/components/rich.rs`.
- [x] T005: Add off-DOM `predecodeImageUrl` in `app/web/ai_restorer.js` and smooth 4-layer GPU progress flushes in `app/src/storage/realesrgan_gpu.rs`.
- [x] T006: Redesign `.ai-gen-overlay`, `.lightbox-img-frame`, `.lightbox-img`, and `.rich-image` CSS in `app/assets/main.css` using static pre-blurred GPU preview `.ai-gen-blur-img` + pure GPU-composited (`transform`/`opacity`) aurora/micro-shimmer layers and zero-radius Lightbox frame.
- [x] T007: Run `cargo test`, build release WASM bundle (`./app/build-web.sh release`), verify 0.0px bounding-box stability and inspect real screenshots via Playwright, and pass all E2E regression suites (`spec112`, `spec101`, `verify_clean_ai_ui`, `spec111`, `spec110`).
- [x] T008: Update `tasks.md` checkboxes and commit/push to `origin main` per `AGENTS.md`.
