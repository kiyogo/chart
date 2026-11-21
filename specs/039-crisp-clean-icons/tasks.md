# Tasks — Spec 039: Crisp Non-Muddy Icon Styling

- [x] T001 Add pure-outline inactive / luminous-active `.cw-icon [fill-opacity]` rules and richer non-gray icon ink contrast in `app/assets/main.css`.
- [x] T002 Clean up `FeedThreadsIcon` (remove background circle blob) and `CompassIcon` (remove muddy solid gray needle half) in `app/src/components/icons.rs`.
- [x] T003 Update `app/e2e/spec038-chat-bottom-icons.mjs` to assert zero muddy gray fill on inactive icons, rebuild Web WASM release, run tests, and visually inspect Light & Dark screenshots.
- [x] T004 Run full Playwright regression suites, rebuild & install Linux Desktop release + `.deb`, mark tasks complete, commit, and push to `origin/main`.
