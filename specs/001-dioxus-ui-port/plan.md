# Technical Implementation Plan: Dioxus 0.7 Cross-Platform Chat Client

## Workspace Integration
- Add `"app"` to root `Cargo.toml` workspace members alongside `"web/backend"`.
- Keep `web/frontend` and `web/backend` untouched.

## Module Architecture (`app/`)
- `app/Cargo.toml`: `chatw-app` with `dioxus = "0.7.10"` and platform features `web`, `desktop`, `mobile`.
- `app/Dioxus.toml`: Dioxus CLI config (web/desktop/mobile metadata, assets).
- `app/assets/main.css`: Exact copy of `web/frontend/src/styles/main.css` (adding `#main` alongside `#root` for Dioxus mount container) + `logo.svg`, `og-image.svg`, `robots.txt`, `sitemap.xml`, `llms.txt`.
- `app/web/index.html`: Exact SEO/OpenGraph/Twitter/JSON-LD/favicon HTML shell loading the compiled Dioxus WASM bundle.
- `app/src/main.rs`: Platform-aware launch (`dioxus::launch` with desktop window config on native, web launch on wasm32).
- `app/src/lib.rs`: Module exports & `App` root component.
- `app/src/models.rs`: Serde models matching `web/backend` + `web/frontend/src/api/client.ts` & `ws.ts`.
- `app/src/crypto/e2ee.rs`: Pure-Rust E2EE protocol (ECDH P-256, ECDSA P-256, HKDF-SHA-256, AES-256-GCM, 81-byte AAD, base64url no-pad, TOFU peer verification) wire-compatible with `web/frontend/src/crypto/e2ee.ts` & `web/backend/src/crypto/mod.rs`.
- `app/src/platform/`:
  - `mod.rs`: Cross-platform traits/functions (storage, window width, document title, audio chime, avatar AVIF processing, URL resolution, OAuth navigation).
  - `web.rs`: `wasm32` implementation (`web-sys` fetch with `credentials: include`, `web-sys` WebSocket, `localStorage`, Canvas AVIF encoder, Web Audio API chime).
  - `native.rs`: Desktop/Mobile implementation (`reqwest` cookie jar, `tokio-tungstenite` WS with Cookie/Origin, local private file storage with 0600 permissions, backend OAuth ticket flow).
- `app/src/services/`:
  - `api.rs`: Typed async API client covering all backend endpoints used by `web/frontend`.
  - `ws.rs`: Real-time WebSocket manager with exponential backoff reconnect (`1s..10s`).
- `app/src/components/`:
  - `icons.rs`: All 11 exact inline SVG components.
  - `avatar.rs`: Exact JS-compatible color hash, size-based font scaling, image error fallback.
  - `emoji_picker.rs`: Exact 24-emoji popup dialog.
- `app/src/pages/`:
  - `login.rs`: Exact `Login` view.
  - `setup_username.rs`: Exact `SetupUsername` view.
  - `chat.rs`: Full `Chat` layout + Sidebar (Chats/Friends/Servers) + Main View (Empty/Settings/Channel/Voice/DM) + Partner Info Sidebar.

## Verification Strategy
1. Pure-Rust unit tests (`cargo test`): E2EE envelope encrypt/decrypt round-trip, signature tamper detection, avatar hash parity with JS, model deserialization.
2. Build verification: `cargo check`, `cargo clippy`, `cargo test`, `wasm32-unknown-unknown --features web` + `wasm-bindgen`, and check `desktop` target toolchain status.
3. Live functional & pixel-perfect visual comparison: Start real `web/backend` (8085), SolidJS `web/frontend` (5173), and Dioxus web bundle (5174 proxying `/api` & `/uploads` to 8085); run automated Playwright script capturing identical seeded states across desktop, tablet, and mobile viewports, computing pixel diff & SSIM via Python PIL, iterating CSS/RSX until differences are minimized.

