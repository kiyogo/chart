# Implementation Plan: Spec 152

1. `app/assets/main.css`: Replace `background: var(--bg-hover);` in `.note-plus-badge:hover` with `background: color-mix(in srgb, var(--bg-elevated) 90%, var(--text-primary));`.
2. Rebuild bundle via `./app/build-web.sh`.
3. Verify in headless Chrome via hover + computed style + zoomed screenshot.
