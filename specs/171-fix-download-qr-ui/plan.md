# Plan 171: Fix Download QR Code UI

1. Update `render_qr_svg` in `app/src/components/download_view.rs` to use `.quiet_zone(true)`.
2. Update `.rail-menu.download-rail-menu`, `.download-popover-qr`, and `.download-qr-box` in `app/assets/main.css` to provide a `#FFFFFF` surface with padding and viewport-safe `max-height`.
3. Rebuild `app/dist` release bundle, deploy to `chat.kiyo.vn`, verify in headless Chrome, and push to GitHub.
