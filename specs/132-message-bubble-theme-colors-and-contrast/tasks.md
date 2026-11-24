# Tasks 132: Theme-Adaptive Message Bubble Colors & High-Contrast Borders

- [x] Task 1: Update base received bubble styles in `app/assets/main.css` (remove hardcoded `!important`, add tokenized background & crisp border).
- [x] Task 2: Refactor error bubble styles in `app/assets/main.css` (remove hardcoded `#FFFFFF !important` and `* { color: #111827 !important; }`, inherit theme colors properly).
- [x] Task 3: Add dark theme overrides for received bubbles and error bubbles in `app/assets/main.css` covering `html[data-theme="dark"]`, `html[data-surface-tone="dark"]`, `chat-column[data-chat-tone="dark"]`, and `.wp-night`.
- [x] Task 4: Run `./app/build-web.sh` to compile frontend assets.
- [x] Task 5: Implement automated Playwright E2E verification test `app/e2e/spec132-message-bubble-colors.mjs` and capture screenshots.
- [x] Task 6: Commit and push changes to GitHub `origin main` per Rule 1.
