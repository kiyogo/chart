# Spec 078 — Tasks

- [x] **T1**: Add `ZoomInIcon`, `ZoomOutIcon`, and `RotateCwIcon` to `app/src/components/icons.rs`.
- [x] **T2**: Add `LightboxMedia`, `GLOBAL_LIGHTBOX`, and `GlobalImageLightbox` component (pixel-centered stage, mouse-wheel zoom, `+`/`−` buttons, live `%` reset pill, double-click 1×/2× toggle, drag-to-pan, 90° rotate, keyboard shortcuts, sleek topbar + floating bottom dock) in `app/src/components/rich.rs`; wire `RichImage` click to open `GLOBAL_LIGHTBOX`.
- [x] **T3**: Mount `crate::components::rich::GlobalImageLightbox {}` at the root of `ChatPage` in `app/src/pages/chat.rs`.
- [x] **T4**: Style the new `.lightbox`, `.lightbox-stage`, `.lightbox-topbar`, `.lightbox-meta`, `.lightbox-actions`, `.lightbox-dock`, `.lightbox-zoom-readout`, and `.lightbox-img` in `app/assets/main.css` (flat minimalist cinema dark frosted-glass, zero thick borders, pure viewport centering).
- [x] **T5**: Finalize Spec 077 `app/web/boot.js` (`networkidle`-safe pre-login `CWRT1` fetch, `SharedWorker` stealth fetch for authenticated cold boots, and 0-network IndexedDB warm boot).
- [x] **T6**: Add `"Zoom in"`, `"Reset zoom"`, `"Rotate image"` to all 197 world language packs (`scripts/i18n_add_spec078_keys.py`, 100% parity at 2,930 exact keys) and bump `i18n_runtime.js` cache to `:v4`.
- [x] **T7**: Build release bundle (`./app/build-web.sh release`), run Rust unit tests (32/32 pass) + Spec 077 E2E (9/9) + Spec 078 E2E (8/8: `0.0 px` centering error, zoom/wheel/dblclick/pan/rotate/reset/Escape, screenshots inspected) + regression suites (`spec072` 5/5, `spec073` 7/7, `spec076` 10/10).
- [x] **T8**: Mark Spec 077 & Spec 078 tasks `[x]` and commit + push to `origin main` (`https://github.com/kiyogo/chart`).
