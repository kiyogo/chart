# Implementation Tasks: Spec 166

- [x] T001 Audit hot paths and create `specs/166-sub-3ms-ultra-fast-runtime/{spec.md,plan.md,tasks.md}`
- [x] T002 Implement Keep-Alive SettingsView layer & Keep-Alive NavigationSidebar/ChatStage/ActiveDmStage in `app/src/pages/chat.rs` (zero DOM teardown on Settings open/close and isolated DM switches)
- [x] T003 Implement L0 RAM boot pre-warming, hover speculative prefetch, synchronous `draft_cache`, `DM_DECRYPT_CACHE` + `PROFILE_RAM_CACHE` (zero redundant E2EE re-decrypt, wiped on logout/lock), and `onmousedown` primary switching across `chat.rs` & `conversations.rs`
- [x] T004 Optimize `AppRail` (gate `render_qr_svg`, isolated `is_settings_open`, guarded plugin overlay clears), `SettingsView` (Keep-Alive `SettingsKeepAlivePane` with `Memo<bool>` + staggered tab pre-warm + hover pre-mount), `MessageList`/`DmDetailsPanel` (zero render-time signal writes + direct `web_sys` scroll), and CSS containment in `app/assets/main.css`
- [x] T005 Accelerate `app/web/index.html` (`defer` + `modulepreload`), `app/web/boot.js` (32-bit XOR + unpacked WASM warm cache), `app/web/core_bridge.js` & `i18n_runtime.js` (RAF-batched observers), `Cargo.toml` release profile, and `web/backend/src/main.rs` (immutable `?v=` caching)
- [x] T006 Build Web release bundle (`./app/build-web.sh release`) + backend, restart live daemon, run tests, and measure real click-to-DOM & click-to-paint latencies in live Chromium
- [x] T007 Mark all tasks `[x]`, `git add .`, commit, and `git push origin main` to `https://github.com/kiyogo/chart`
