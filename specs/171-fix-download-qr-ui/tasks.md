# Tasks 171: Fix Download QR Code UI

- [x] T001 Switch `render_qr_svg` callers to `"currentColor"` / `"transparent"` with `quiet_zone(false)`
- [x] T002 Remove `border-radius` (`border-radius: 0`) and forced `#FFFFFF` background on `.download-popover-qr` and `.download-qr-box` in `app/assets/main.css`
- [x] T003 Rebuild `app/dist` release bundle and deploy to `chat.kiyo.vn`
- [x] T004 Verify QR UI in Light & Dark themes and push to GitHub
