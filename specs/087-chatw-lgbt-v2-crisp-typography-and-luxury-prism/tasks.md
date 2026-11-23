# Spec 087 — Tasks

- [x] **T1**: Upgrade `scripts/fetch_open_fonts.py` (fetch upright `Nunito` 600/800 + rounded multi-script 600/700 bases) and overhaul `scripts/build_chatw_lgbt_font.py` to compile `ChatW LGBT v2.0` (100% upright crisp stems, zero `sin()` wobble, balanced symmetric ♥ heart-tittles across all scripts). Visually verify the v2.0 specimen PNG.
- [x] **T2**: Upgrade `#cw-lgbt-icon-grad` in `app/src/lib.rs` and overhaul `app/assets/fonts.css` with OKLCH-balanced vibrant 7-color Light/Dark spectrums, pixel-locked `280px` unified liquid-prism flow, crisp `.rail-brand` Pride logo tile, `.range-value` + `.settings-sublabel` coverage, clean `<select>` styling, and velvet diagonal Pride pills.
- [x] **T3**: Rebuild release WASM (`./app/build-web.sh release`), update & run Rust + Playwright E2E (`spec084`, `spec085`, `spec086`), and visually inspect Light Mode + Dark Mode + Settings + Chat live screenshots via `read_file`.
- [x] **T4**: Mark tasks `[x]` and commit + push to `origin main` (`https://github.com/kiyogo/chart`).
