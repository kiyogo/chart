# Tasks: Spec 031 — Feed Composer Bar & Vertical Story Cards

- [x] T001: Update `app/src/components/feed_view.rs` with compact top composer bar (`.feed-compose-bar`), vertical Story Cards carousel (`.feed-story-carousel`), and modals (Create Story, Share Thought, View Story, View Friend Note) wired to real `/api/feed` and profile APIs.
- [x] T002: Add pure-CSS styles in `app/assets/main.css` matching Image 2 (compact pill composer, tall rounded story cards, `"Tạo tin"` blue `+` card, avatar ring + online dot, dark bottom scrim, floating `>` / `<` carousel arrows, dark-theme compatibility).
- [x] T003: Update `app/e2e/spec030-ui-check.mjs` to assert the new FeedView composer bar, vertical story cards, `"Tạo tin"` modal trigger, and carousel layout.
- [x] T004: Build via `./app/build-web.sh`, run Playwright E2E suites (`spec030-ui-check.mjs`, `borderless-shadowless-check.mjs`, `scrollbar-switch-check.mjs`), visually inspect screenshots.
- [x] T005: Commit and push to `origin main`.
