# Spec 171: Fix Download QR Code Theme Contrast & Sharp Corners

## Problem
The download QR code used hardcoded `#0F172A` on `#FFFFFF` and rounded corners (`border-radius`), clashing with Dark Mode and violating the sharp-corner QR requirement.

## Requirements
- Keep QR containers strictly sharp-cornered (`border-radius: 0`) on both `.download-popover-qr` and `.download-qr-box` (no rounded clipping).
- Use `currentColor` foreground and `transparent` background in `render_qr_svg` (`quiet_zone(false)`) so the QR automatically adapts to Light Mode (`#0F172A` on `#FFFFFF`), Dark Mode (`#F4F6FA` on `#1C2230`), and surface-tone overrides.
- Keep `max-height: calc(100dvh - 16px); overflow-y: auto;` on `.rail-menu.download-rail-menu`.
