# Implementation Plan — Spec 036

1. **`app/assets/main.css`**:
   - Align `.sidebar-search` L/R padding to `0.45rem` (`padding: 0 0.45rem 2px;`).
   - Align `.add-friend-container` to `padding: 0 0.45rem 0.3rem;`.
   - Unify `.friends-scroll` to `padding: 0 0.45rem var(--space-2); gap: 0.3rem;` and `.friends-scroll .chat-item.friend-row, .friends-scroll .request-card` to `padding: 0.45rem; gap: 0.625rem; min-height: 0; margin: 0;` with `var(--avatar-list)` avatars inside `.request-card`.
   - Align `.server-banner` (`margin: 0 0.45rem 0.3rem`) and `.voice-status-panel` (`margin: 0.45rem`).
2. **`app/e2e/spec030-ui-check.mjs`**:
   - Add assertion checking Friends tab (`.friends-scroll` & search `.request-card` / `.friend-row`) matches Messages & Communities 100% (`6.75px` inset, `6.75px` padding, `4.5px` gap, `64px` card height).
3. **Build, Verify, Reinstall Desktop & Auto-Push**:
   - `./app/build-web.sh`
   - Run `spec030-ui-check.mjs` + `scrollbar-switch-check.mjs` and inspect Friends screenshot.
   - Rebuild & reinstall Linux Desktop app and push to `origin/main`.
