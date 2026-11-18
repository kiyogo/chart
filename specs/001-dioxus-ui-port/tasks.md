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
- [x] Commit and push to `origin main`

