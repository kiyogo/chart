# Spec 089 — Tasks

- [x] **T1**: Implement `SelfDetailsPanel`, `SelfMediaThumb`, `extract_message_links`, vault categorization helpers, and Rust unit tests in `app/src/components/details_panel.rs`.
- [x] **T2**: Wire the `PanelRightIcon` toggle button, clickable `.self-header-identity`, and `SelfDetailsPanel` right drawer (with `.panel-resizer`) for `self_mode` in `app/src/pages/chat.rs`.
- [x] **T3**: Add flat minimalist Light/Dark CSS for `SelfDetailsPanel` in `app/assets/main.css` and extend `ChatW LGBT` text rules in `app/assets/fonts.css`.
- [x] **T4**: Rebuild release WASM (`./app/build-web.sh release`), run `cargo test -p chatw-app --lib`, run new Playwright E2E `app/e2e/spec089-self-chat-right-sidebar.mjs` + existing E2E regression tests, and visually inspect live screenshots via `read_file`.
- [x] **T5**: Mark all tasks `[x]` and commit + push to `origin main` (`https://github.com/kiyogo/chart`).
