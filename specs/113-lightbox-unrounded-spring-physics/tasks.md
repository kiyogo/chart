# Spec 113 Tasks

- [x] T001: Implement Pure-Rust `LightboxSpringEngine` (velocity tracker, zoomed inertia glide + Hooke's law damped harmonic oscillator, mid-flight grab, transform formatter) and unit tests in `app/src/components/rich.rs`.
- [x] T002: Wire `GlobalImageLightbox` in `app/src/components/rich.rs` to drive `.lightbox-img` and `.lightbox-ai-gen` directly at native 60/120/240Hz via `requestAnimationFrame` with zero VDOM re-renders during drag/spring motion.
- [x] T003: Enforce `border-radius: 0 !important;` on all Lightbox photo/overlay layers and remove conflicting CSS `transition: transform` in `app/assets/main.css`.
- [x] T004: Run `cargo test`, build release WASM bundle (`./app/build-web.sh release`), verify with new `app/e2e/spec113-lightbox-unrounded-spring-physics.mjs` + `spec101` + `spec112` + `verify_clean_ai_ui_and_flows`.
- [x] T005: Update `tasks.md` checkboxes and commit/push to `origin main` per `AGENTS.md`.
