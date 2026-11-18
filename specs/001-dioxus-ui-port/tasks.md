# Tasks: Port Web Chat UI to Dioxus 0.7 (Rust)

## Phase 1-2: Analysis & Setup
- [x] Inspect entire `web/frontend` and `web/backend` codebase, CSS, assets, APIs, WebSocket, E2EE protocol
- [x] Verify Rust toolchain, targets, backend tests, and Playwright environment
- [x] Configure workspace `Cargo.toml` and `app/Cargo.toml` + `app/Dioxus.toml` + static assets/CSS

## Phase 3-5: Core Services, Crypto, Platform & UI Components
- [x] Implement `app/src/models.rs` (API & WS models matching backend and original frontend)
- [x] Implement `app/src/crypto/e2ee.rs` (pure Rust P-256 ECDH/ECDSA + HKDF + AES-256-GCM + unit tests)
- [x] Implement `app/src/platform/` (`mod.rs`, `web.rs`, `native.rs`) for HTTP, WS, storage, media, DOM helpers
- [x] Implement `app/src/services/api.rs` and `app/src/services/ws.rs`
- [x] Port `app/src/components/icons.rs`, `avatar.rs`, `emoji_picker.rs`
- [x] Port `app/src/pages/login.rs`, `setup_username.rs`, `chat.rs`, and `app/src/lib.rs` + `app/src/main.rs`

## Phase 6-10: Build, Visual Comparison, Functional Testing & Delivery
- [x] Build Dioxus WASM bundle (`wasm32-unknown-unknown --features web` + `wasm-bindgen`) and run `cargo test` & `cargo clippy`
- [x] Verify desktop/mobile target compatibility and report exact status
- [x] Start real backend, SolidJS original, and Dioxus app; run automated functional + pixel-perfect visual comparison (SSIM + pixel diff + bounding boxes) across desktop/tablet/mobile viewports
- [x] Fix any visual or functional discrepancies and re-verify

## Phase 11: Desktop Layout Overhaul & Multi-Viewport Verification
- [x] Fix `.app-layout` flex column bug in `app/src/pages/chat.rs` by wrapping sidebar and chat view in `.app-content`
- [x] Fix CSS horizontal flex overflow with `min-width: 0` on `.chat-view` in `app/assets/main.css`
- [x] Implement comprehensive dark mode tokens and replace hardcoded colors in partner info cards
- [x] Validate across 10 viewports (375x667 to 2560x1440) and 11 UI states (zero overflow, composer anchored)

## Phase 12: Real Linux Desktop Binary & Packaging
- [x] Resolve native build dependencies (vendored OpenSSL, GTK3/WebKit2GTK stubs, libxdo stub)
- [x] Compile release native desktop binary `target/release/chatw-app` (18 MB)
- [x] Implement native session persistence (`~/.local/share/chatw/client_storage.json`)
- [x] Verify host runtime execution with WebKitGTK display
- [x] Package Debian release package `target/chatw_0.1.0_amd64.deb` (4.3 MB)

## Phase 13: CI/CD & Delivery
- [x] Add GitHub Actions multi-platform CI workflow (`.github/workflows/ci.yml`)
- [x] Commit and push all changes to `origin main`


