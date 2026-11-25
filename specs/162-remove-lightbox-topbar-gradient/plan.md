# Plan — Spec 162

- Update `.lightbox-topbar` in `app/assets/main.css` from `background: linear-gradient(...)` to `background: transparent !important; border: none !important; box-shadow: none !important;`.
- Keep lightweight shadow readability on `.lightbox-meta`, `.lightbox-zoom-readout`, and toolbar SVG icons.
- Rebuild web bundle via `./app/build-web.sh` (which embeds `MAIN_CSS` via `include_str!("../assets/main.css")`), verify with live browser/screenshot or CSS check, update `tasks.md`, commit and push to `origin main`.
