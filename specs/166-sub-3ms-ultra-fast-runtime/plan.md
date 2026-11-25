# Technical Implementation Plan: Spec 166

## 1. Keep-Alive Layered Mounting (`app/src/pages/chat.rs`)
- Keep `NavigationSidebar` + `ConversationList` + main chat stage mounted in DOM when `is_settings_open()` flips to `true`, toggling visibility via CSS `.cw-layer-hidden { display: none !important; }` instead of destroying & remounting 1,500+ DOM nodes and re-firing `feed::list_feed` on return!
- Pre-warm a Keep-Alive `SettingsView` container (`settings_mounted` flipped true after 120ms idle post-boot OR immediately on Settings icon hover/`onpointerdown`) and toggle it via `.cw-layer-hidden` -> Settings open/close creates **0 DOM nodes** and destroys **0 DOM nodes** (`< 1 ms` Rust/WASM, `< 3 ms` browser paint!).

## 2. Zero-Latency Conversation Switching (`app/src/pages/chat.rs`, `conversations.rs`, `platform/mod.rs`)
- **Idle L0 RAM Pre-Warm**: Right after initial `friends` & `groups` populate, silently hydrate the top 12 DMs and top 6 Groups from IndexedDB into `dm_cache` / `group_cache` + synchronous `draft_cache: HashMap<String, String>`.
- **Speculative Hover Prefetch (`on_prefetch` prop on `ConversationList`)**: `onmouseenter` on any uncached DM/Group row immediately hydrates it from IndexedDB into L0 RAM before click!
- **Instant `onpointerdown` Dispatch**: Primary mouse button press triggers `on_open` immediately (suppressing duplicate `onclick` while preserving full keyboard `Enter`/`Space` accessibility).
- **Per-Message E2EE Decrypted Plaintext RAM Cache**: `DM_DECRYPT_CACHE` (`HashMap<(String, String), String>`) skips redundant P-256 ECDH + ECDSA verify + AES-256-GCM decrypt on background refresh for already-decrypted messages (wiped cleanly on logout/lock/key-import!).
- **Direct `web_sys` Instant Scroll**: Replace `dioxus::document::eval(...)` string compilation in `scroll_chat_to_bottom(true)` on `wasm32` with direct `web_sys` DOM property assignment (`el.set_scroll_top(el.scroll_height())`).

## 3. Component & CSS Engine Micro-Optimizations
- `app/src/components/navigation/rail.rs`: Gate `render_qr_svg` behind `if download_hover_open()` (zero QR computation on normal clicks!), fire Settings/nav clicks on `onpointerdown`, and expose `on_warm_settings` on `onmouseenter`.
- `app/src/components/settings_view.rs`: Fire category & sub-tab switches on `onpointerdown`, keep tab switching synchronous with zero blocking async calls.
- `app/assets/main.css`: Add `.cw-layer-hidden`, `contain: layout style paint` on `.nav-sidebar`, `.settings-page`, `.settings-body`, `.settings-nav`, `.message-row`, `.conv-item`, and eliminate expensive `:has(.lightbox, .modal-overlay)` selector sweeps on every `.message-row` when no lightbox is open.

## 4. Boot, Compiler Profile & Backend Transport
- `app/web/index.html`: Add `defer` to non-critical enhancement scripts (`face_restorer.js`, `ai_restorer.js`, `custom_pickers.js`, `living_motion.js`, `canvas_scrollbars.js`, `panel_resizer.js`, `adaptive_surface.js`, `media_optimizer.js`, `plugin_runtime.js`) and `<link rel="modulepreload" href="/pkg/chatw_app.js">`.
- `app/web/boot.js`: 32-bit `Uint32Array` XOR in `unpackCwrt1` + cache unpacked WASM bytes in IndexedDB (`runtime:raw:<ver>`) for instant warm reload (skipping 24MB gzip decompression on every F5!).
- `Cargo.toml`: Add `[profile.release]` (`lto = "thin"`, `codegen-units = 1`) for tighter, faster WASM & native codegen.
- `web/backend/src/main.rs`: Set `TCP_NODELAY` on accepted sockets (via `TcpListener` accept config / hyper) and set `Cache-Control: public, max-age=31536000, immutable` on `?v=` versioned static assets.

## 5. Real Browser Verification
- Instrument `window.__chatwPerf` with `performance.now()` click-to-DOM & click-to-rAF-paint timers.
- Measure live in Chromium via `browser` subagent (dev-login -> click Settings -> switch tabs -> close Settings -> switch DM conversations -> verify < 3ms JS/DOM commit and real frame paint metrics!).
