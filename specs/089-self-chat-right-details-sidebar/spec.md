# Spec 089 — Right-Side Info & Personal Vault Menu for "Chính mình" ("Note to Self")

## Problem Statement
In regular direct messages, ChatW provides the top-right `PanelRightIcon` toggle button (next to `MoreIcon ...`) and a resizable right-side `DmDetailsPanel`. However, in **"Chính mình" ("Note to self")**, lines 4287 and 4508 of `app/src/pages/chat.rs` explicitly hid both the `PanelRightIcon` toggle button (`if !self_mode`) and the right sidebar (`if is_partner_info_open() && !self_mode`). Users expect "Chính mình" to offer the exact same right-side menu button and a dedicated Saved Messages / Cloud Vault right sidebar modeled after best-in-class open-source & web messengers (Telegram Web K "Saved Messages", Signal Desktop "Note to Self", Zalo "Cloud của tôi").

## Open-Source Research Summary
- **Telegram Web K (`TelegramOrg/Telegram-web-k`) "Saved Messages"**: Clicking the top-right toggle or header identity opens `column-right` with segmented Shared Storage tabs (**Photos/Videos**, **Files/Documents**, **Links**, **Pinned**), instant search, and jump-to-message navigation.
- **Signal Desktop (`signalapp/Signal-Desktop`) "Note to Self"**: Provides a dedicated right details pane with categorized shared media/files/links, synced devices awareness, and zero peer-identity TOFU noise.

## Acceptance Criteria
1. **Identical Top-Right Toggle Button**: In `"Chính mình"` (`self_mode == true`), render the exact `PanelRightIcon { size: 22 }` button (`data-testid="toggle-details"`, active state styling when open) right beside `MoreIcon (...)` in `.chat-header-actions`, and make the `"Chính mình"` header identity clickable to toggle the right panel.
2. **Resizable `SelfDetailsPanel` Right Sidebar**:
   - Mount `SelfDetailsPanel` alongside `<div class="panel-resizer resizer-right" data-resizer="details">` when `is_partner_info_open()` is true in `self_mode`.
   - **Hero Header**: Soft Pride-accented banner + bright 80px `SelfChatPrideAvatar`, title (`t("chat.self")` = "Chính mình"), subtitle (`Only you · N Devices`).
   - **4 Quick Actions**: `Notes` (focus composer), `Search` (focus panel search input), `Theme` (open chat wallpaper picker), `Delete all` (open clear-self confirmation modal).
   - **Shared Storage Vault (`Storage`) with 4 Live Tabs** parsed from real decrypted `ViewMessage` payloads (`rich::parse_rich` + URL extraction + `is_pinned`, zero mock data):
     - **`Photo`**: 3-column media grid for `Rich::Image` (real E2EE/cached blob thumbnails + blurhash fallback, opens `GLOBAL_LIGHTBOX` on click) and `Rich::Video` (duration + jump to message).
     - **`Documents`**: Compact list of `Rich::File`, `Rich::Audio`, `Rich::Note`, `Rich::Reminder` with filename/title, size/duration, timestamp, and jump-to-message on click.
     - **`Link`**: Extracted `http://` & `https://` URLs with host, path, copy-link action, and jump-to-message on click.
     - **`Pinned`**: All `msg.is_pinned` messages with clean readable preview and jump-to-message on click.
   - **In-Chat Instant Search (`Search in chat`)**: Live local search across plain text and rich message titles/filenames with click-to-jump.
   - **Synced Devices (`Your device`)**: Lists active `SessionItem` devices with `"this device"` badge and click-to-rename modal support.
   - **Collapsible E2EE Security Disclosure (`Security and encryption`)**: Displays personal E2EE fingerprint, copy button, and user code without any peer TOFU warning.
3. **197-Language Localization & Theme Harmony**: Reuse exact 197-language keys from `app/web/locales/en.json`, style cleanly in flat minimalist CSS supporting Light/Dark/Translucent/Glass panels and `ChatW LGBT v2.0` 7-color typography, with zero `...` ellipsis placeholders.
4. **Verification & Auto-Push**: Add Rust unit tests for link extraction & vault categorization + Playwright live E2E (`app/e2e/spec089-self-chat-right-sidebar.mjs`) + visual screenshots, rebuild release WASM, and commit + push to `origin main`.
