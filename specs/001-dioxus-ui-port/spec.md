# Feature Specification: Port Web Chat UI to Dioxus 0.7 (Rust)

## Goal
Port the entire SolidJS web chat frontend (`web/frontend`) into a cross-platform Rust Dioxus 0.7 client (`app/`), achieving pixel-perfect visual fidelity and 100% feature parity while preserving the existing backend (`web/backend`) and existing web frontend untouched.

## Scope & Source of Truth
- **UI Source of Truth**: `web/frontend/src/App.tsx`, `Login.tsx`, `SetupUsername.tsx`, `Chat.tsx`, `Avatar.tsx`, `EmojiPicker.tsx`, `Icons.tsx`, `styles/main.css`, `index.html`, `public/*`.
- **Protocol Source of Truth**: `web/frontend/src/api/client.ts`, `websocket/ws.ts`, `crypto/e2ee.ts`, `utils/image.ts`, and `web/backend/src/*`.

## Acceptance Criteria
1. **Screens**: Loading, Login (Google OAuth + Dev Login), SetupUsername, Chat (Chats tab, Friends tab with live search & pending requests, Servers tab with server create/join, text channels, voice channels & voice bar, Settings view with AVIF avatar upload & E2EE key fingerprint, DM Chat view with column-reverse messages, reply quotes, typing indicator, EmojiPicker popup, Partner Info sidebar).
2. **Components**: `Avatar` (exact hash color palette & fallback), `EmojiPicker` (exact 24 emojis), all 11 SVG `Icons`.
3. **Real Backend Integration**: Auth session cookies, E2EE key generation/registration/verification/encryption/decryption (pure Rust P-256 + HKDF-SHA-256 + AES-256-GCM compatible with Web Crypto envelopes), WebSocket real-time events with exponential backoff reconnect, AVIF avatar upload, typing indicators, friend search/add/accept, servers/channels/voice.
4. **Visual Parity**: Automated screenshot comparison (Playwright + Python PIL pixel diff & SSIM) across desktop (`1280x800`), tablet (`900x800`), and mobile (`375x667`) viewports for Login, Chat empty state, Active DM chat, Emoji picker, Friends tab, Servers tab, Settings view, and Partner Info sidebar.
5. **Multi-platform Readiness**: `web` (`wasm32-unknown-unknown`), `desktop` (`dioxus/desktop` for Linux/Windows/macOS), `mobile` (`dioxus/mobile` for Android/iOS) cleanly separated in `app/src/platform/`.

