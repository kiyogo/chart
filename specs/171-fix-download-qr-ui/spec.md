# Spec 171: Fix Download QR Code Contrast, Quiet Zone & Popover Framing

## Problem
In dark mode, the left-rail download QR popover (`.download-popover-qr`) and download page QR box (`.download-qr-box`) rendered with `quiet_zone(false)`, `padding: 0`, and `background: transparent`. This caused the `#0F172A` outer finder squares to merge into the `#1C2230` dark popover background and get clipped by `border-radius: var(--radius-md); overflow: hidden;`.

## Requirements
- Enable `quiet_zone(true)` in `render_qr_svg` (`app/src/components/download_view.rs`) so the SVG includes the standard quiet zone around all 4 edges.
- Give `.download-popover-qr` and `.download-qr-box` a crisp `#FFFFFF` background with inner padding (`6px` / `8px`) while keeping `border: none` and `svg` width `>= 155px`.
- Guard `.rail-menu.download-rail-menu` with `max-height: calc(100dvh - 16px); overflow-y: auto;` so it never overflows shorter viewports.
