# Spec 089 — Technical Plan

## Files to Modify / Add
1. `app/src/components/details_panel.rs`
   - Add pure helper functions + unit tests:
     - `extract_message_links(text: &str) -> Vec<String>` (extracts clean `http://` / `https://` URLs while stripping trailing punctuation).
     - `SelfVaultTab` enum (`Photo`, `Documents`, `Link`, `Pinned`) and categorization logic over `&[ViewMessage]` using `crate::components::rich::{parse_rich, rich_preview, Rich}`.
   - Add lightweight `SelfMediaThumb` component resolving E2EE/cached image thumbnails via `platform::get_cached_blob_url_sync` + `platform::resolve_e2ee_blob_media_url` with instant blurhash fallback and Lightbox click-to-open (`GLOBAL_LIGHTBOX`).
   - Add `#[component] pub fn SelfDetailsPanel(...)` reusing `.details-panel` + `.details-scroll` structure with Hero, 4 Quick Actions, 4-tab `Storage` vault, `Search in chat`, `Your device` list, and collapsible `Security and encryption`.
2. `app/src/pages/chat.rs`
   - Import `SelfDetailsPanel`.
   - Convert `.self-header-identity` into a clickable button toggling `is_partner_info_open`.
   - Remove `if !self_mode` guard on `PanelRightIcon` toggle button (`data-testid="toggle-details"`).
   - Render `SelfDetailsPanel` when `is_partner_info_open() && self_mode` alongside the existing `.panel-resizer.resizer-right`.
3. `app/assets/main.css`
   - Add clean flat minimalist CSS for `.self-details-banner`, `.self-details-avatar`, `.self-vault-tabs`, `.self-vault-tab`, `.self-media-grid`, `.self-media-tile`, `.self-vault-list`, `.self-vault-row`, `.self-device-list`, `.self-device-row` (supporting Light, Dark, Translucent, and Glass surface modes).
4. `app/assets/fonts.css`
   - Include `.details-panel` & `.self-vault-*` text classes in the `html[data-font="lgbt"]` liquid-prism typography rules so LGBT font renders harmoniously inside the right panel too.
5. `app/e2e/spec089-self-chat-right-sidebar.mjs`
   - Live Playwright test verifying:
     - Top-right `PanelRightIcon` toggle button (`data-testid="toggle-details"`) is visible next to `MoreIcon` inside `"Chính mình"`, toggles right panel open/closed cleanly, and has `.active` blue pill state when open.
     - Right panel displays `SelfChatPrideAvatar`, `"Chính mình"`, 4 Quick Actions, 4 Shared Storage tabs (`Ảnh`, `Tài liệu`, `Liên kết`, `Đã ghim`), `Tìm trong cuộc trò chuyện`, `Thiết bị của bạn`, and collapsible `Bảo mật và mã hóa`.
     - Sending a message with a link + pinning a message updates live counts & tabs (`Liên kết`, `Đã ghim`), search hits appear, clicking jumps to message, and clicking Wallpaper action opens the wallpaper picker.
     - Capture live screenshots in Light Mode (standard & LGBT font) and Dark Mode.
