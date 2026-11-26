# Plan 171: Fix Download QR Code UI

1. Set `render_qr_svg` in `app/src/components/download_view.rs` (`quiet_zone(false)`) and call sites in `rail.rs` & `download_view.rs` to use `"currentColor"` and `"transparent"`.
2. Update `.download-popover-qr` and `.download-qr-box` in `app/assets/main.css` to `border-radius: 0; background: transparent;` inheriting theme `color`.
3. Rebuild `app/dist` release bundle, deploy to `chat.kiyo.vn`, verify visually, and push to GitHub.
