# Spec 082 — Tasks

- [x] **T1**: Craft the original ChatW Bubble-Spirit Mascot over the 6-stripe LGBTQ+ Pride Rainbow SVG in `app/public/self-chat-avatar.svg` and visually inspect rendered previews at 220px, 48px, and 38px.
- [x] **T2**: Update `SelfChatPrideAvatar` in `app/src/components/icons.rs` with unique atomic `clipPath` and `linearGradient`/`radialGradient` IDs so sidebar & header instances never collide.
- [x] **T3**: Update `app/e2e/spec081-self-chat-pride-svg-avatar.mjs` to verify the original ChatW curved mascot (`path` + `ellipse`/`circle` + 6 Pride `rect`s + unique IDs), rebuild release WASM (`./app/build-web.sh release`), run Rust tests + Playwright E2E, and visually inspect live screenshots.
- [x] **T4**: Mark tasks `[x]` and commit + push to `origin main` (`https://github.com/kiyogo/chart`).
