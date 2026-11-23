# Plan 094: Pure White Bubbles, Neutral Timestamps, Clean Header & Unified Right Details Scroll

## Architecture & Implementation Steps

### 1. Style Message Bubbles & Remove Red Timestamps
- In `app/assets/main.css`:
  - Update `.message-row.received .message-bubble`:
    - Set `background: #FFFFFF !important;` in light and wallpaper themes.
  - Update `.message-bubble.error-bubble`:
    - Set `background: #FFFFFF !important; color: var(--text-primary) !important;`.
    - Set `.message-row.sent .message-bubble.error-bubble`: `background: #FFFFFF !important; color: var(--text-primary) !important;`.
    - Update `.bubble-meta` inside `.message-bubble.error-bubble`: change red `color-mix(in srgb, var(--danger) 76%, var(--text-muted))` to `color: var(--msg-recv-meta, #64748B) !important;`.
  - Update CSS variable `--msg-recv: #FFFFFF;` in light theme rules.

### 2. Remove Device Strip from Chat Header
- In `app/src/pages/chat.rs`:
  - Delete `div { class: "device-strip header-inline-devices", ... }` (lines 4329-4346) from `self_mode` header rendering.
  - Header remains clean with `chat-header-identity self-header-identity` and right-side toggle buttons.

### 3. Expand Right Sidebar Device List to Full Height
- In `app/assets/main.css`:
  - Update `.self-device-list` and `.self-vault-list`:
    - Remove `max-height: 15.5rem; overflow-y: auto;`.
    - Set `max-height: none !important; overflow-y: visible !important;`.
  - The outer `.details-scroll` container seamlessly handles all vertical scrolling without any nested scrollbar traps.

### 4. Build, Test & Auto-Push
- Rebuild WASM release bundle: `./app/build-web.sh release`.
- Run Playwright automated E2E tests:
  - Verify message bubble background is `#ffffff` / `rgb(255, 255, 255)`.
  - Verify timestamp is neutral (`rgb(100, 116, 139)` / `#64748b`), NOT red (`rgb(239, 68, 68)` / `rgb(220, 38, 38)`).
  - Verify top chat header has no `.header-inline-devices`.
  - Verify right sidebar `.self-device-list` has no inner scrollbar (`overflow-y: visible`, `max-height: none`).
  - Capture and inspect screenshots.
- Git commit & push to `origin main`.
