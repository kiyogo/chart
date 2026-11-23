# Tasks: Spec 075 — Full-Viewport Image Lightbox Fix

- [x] T001 Guard `.message-row` (`content-visibility: auto`) and `.message-stream` (`contain: layout style`) with `:not(:has(.lightbox, .modal-overlay))`, disable containment/backdrop-filter on `.chat-column:has(.lightbox, .modal-overlay)`, and polish `.lightbox` + `.lightbox-actions` + `.lightbox-close` in `app/assets/main.css`
- [x] T002 Add `Escape` key handler & clean image click stop-propagation in `RichImage` lightbox (`app/src/components/rich.rs`)
- [x] T003 Rebuild release WASM (`./app/build-web.sh release`), extend `app/e2e/spec073-zero-double-load-blob-only-no-data-uri.mjs` to click the image and assert full-viewport `.lightbox` geometry + non-overlapping controls + Escape/close + visual screenshot inspection, run `spec073` (7/7) & `spec069` (18/18), commit & push to `origin main`
