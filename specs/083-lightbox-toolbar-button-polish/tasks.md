# Spec 083 — Tasks

- [x] **T1**: Remove `.btn-secondary` from the lightbox Download anchor in `app/src/components/rich.rs` and render harmonized, uniform-weight (`1.85px` stroke, balanced optical core) toolbar icons inside `.lightbox-actions`.
- [x] **T2**: Refine `.lightbox-topbar`, `.lightbox-actions`, `.lightbox-tool-btn`, and `.lightbox-zoom-readout` in `app/assets/main.css` (stronger borderless top scrim, move `drop-shadow` strictly to `svg`, eliminate white wallpaper `.btn-secondary` leaks, clean hover/active feel & visual rhythm).
- [x] **T3**: Update `app/e2e/spec073-zero-double-load-blob-only-no-data-uri.mjs` & `app/e2e/spec078-global-lightbox-centered-zoom-pan.mjs` to test Download hover + rest styles under active custom translucent wallpaper over a bright 200%+ zoomed photo, build release WASM, run Rust + Playwright suites, and visually inspect close-up screenshots.
- [x] **T4**: Mark tasks `[x]` and commit + push to `origin main` (`https://github.com/kiyogo/chart`).
