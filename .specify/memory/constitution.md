# chatw Constitution

## Core Principles

1. **Real Security First**: End-to-End Encryption (ECDH P-256, ECDSA P-256, HKDF-SHA-256, AES-256-GCM) performed exclusively on the client; backend acts strictly as a blind relay. Never log session tokens or private keys.
2. **Backend Preservation**: Existing Axum 0.8 + SQLx + PostgreSQL backend endpoints, wire formats, and WebSocket protocol remain unchanged unless strictly required.
3. **100% Pixel-Perfect UI Port**: Porting the web UI to Dioxus 0.7 (Rust) must preserve exact DOM structure, CSS tokens, typography, spacing, colors, SVG icons, responsive breakpoints, and interactive behaviors of `web/frontend`. No redesign, no mock data, no placeholder buttons.
4. **Cross-Platform Rust Architecture**: Single Dioxus codebase in `app/` targeting Web (`wasm32`), Desktop (Windows, Linux, macOS via `dioxus/desktop`), and Mobile (Android, iOS via `dioxus/mobile`), with clean platform abstraction for HTTP, WebSocket, storage, and media.
5. **Verification Over Claims**: Every target build, functional flow, and visual screenshot comparison (pixel diff + SSIM + bounding boxes) must be executed against the real backend and reported honestly.

