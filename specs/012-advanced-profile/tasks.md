# Tasks 012
- [x] T1 Migration 0019 and backend `profile_plus` with validation, visibility filtering and rate limits
- [x] T2 Achievements computed from real tables, unlock history, hidden achievement
- [x] T3 ProfileCard renders sections in layout order with 21 templates
- [x] T4 Layout editor: drag and drop, ↑↓, show / hide, size, guestbook mode, presets, export / import, reset
- [x] T5 Showcase manager (links, images, favourite stickers, pin, order); guestbook sign / delete
- [x] T6 Preview as owner / friend / stranger, desktop / mobile
- [x] T7 Inventory page with equipped items, libraries, drawings, presets, achievements, search
- [x] T8 Tests: backend section / ref validation, client layout import / export; `app/e2e/profile-check.mjs` 23/23; UI audit

## Found and fixed while testing
- Drag and drop did nothing through the framework's event layer; replaced with a native HTML5 bridge.
- Text was unreadable on dark templates (light achievement tiles); dark templates now keep inner surfaces dark.
