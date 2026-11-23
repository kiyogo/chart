# Spec 081 — Tasks

- [x] **T1**: Compute mathematically exact isometric voxel bunny + 6-stripe Pride rainbow SVG (`viewBox="0 0 128 128"`), save standalone `app/public/self-chat-avatar.svg`, and visually inspect a rendered preview at 220px, 48px, and 38px.
- [x] **T2**: Implement reusable `SelfChatPrideAvatar` inline SVG component with unique atomic `clipPath` IDs in `app/src/components/icons.rs`.
- [x] **T3**: Replace `DevicesIcon` inside the "Note to self" avatar in `app/src/components/navigation/conversations.rs` and `app/src/pages/chat.rs` with `SelfChatPrideAvatar`, and style `.self-chat-avatar.pride-voxel` cleanly in `app/assets/main.css` (full-bleed circle, `overflow: hidden`, zero border).
- [x] **T4**: Run Rust unit tests (36/36 pass), rebuild release bundle (`./app/build-web.sh release`), write & run `app/e2e/spec081-self-chat-pride-svg-avatar.mjs` (5/5 pass), and visually inspect live screenshots of both sidebar and chat header.
- [x] **T5**: Mark tasks `[x]` and commit + push to `origin main` (`https://github.com/kiyogo/chart`).
