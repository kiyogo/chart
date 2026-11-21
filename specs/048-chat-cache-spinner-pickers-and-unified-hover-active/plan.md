# Technical Implementation Plan — Spec 048

## Modules & Files
1. `app/assets/theme.css` + `app/src/platform/mod.rs` + `app/assets/main.css`:
   - Convert `--bg-hover`, `--bg-active`, `--accent-soft`, `--accent-soft-strong`, and `--bg-selected` into translucent alpha tokens across Light, Dark, all 5 preset accents, custom accents, and `data-surface-tone` overrides.
   - Add `.app-rail { padding: var(--space-2) 0; }` (Item 1).
   - Align `.sidebar-server-header` height (`3.125rem`) and first `.category-block` top margin (`0`) (Item 5).
   - Unify symmetric left/right padding (`--sidebar-pad: 0.5rem`) across all sidebars (`nav-sidebar`, `settings-nav`, `server-settings-stage .settings-nav`, `details-panel`) with sleek overlay-width scrollbars so left & right insets are 100% equal (Item 8).
   - Increase `.settings-cols` column-gap (`clamp(2.25rem, 3.5vw, 3.5rem)`) (Item 8).
   - Enforce unified hover (translucent grey) & active (translucent theme accent) across `.chat-item`, `.channel-row`, `.settings-nav-item`, `.rail-item`, `.segmented button`, `.section-tab`, `.filter-chip`, `.theme-card`, `.wallpaper-tile`, `.sticker-cell`, `.conv-more`, `.menu-item`, `.cw-dropdown-item`, `.cw-select-option` with **100% unchanged text color, unchanged icon color/fill, and unchanged font-weight (no bolding on click/active)** (Item 6).
   - Remove blanket `.settings-group-title { display: none !important; }` and style `.settings-group-title` subtly (`font-size: var(--text-sm); color: var(--text-secondary); font-weight: var(--weight-medium);`) (Item 7).
2. `app/web/custom_pickers.js`:
   - Pre-position popovers off-paint at anchor coordinates with `visibility: hidden; opacity: 0;` BEFORE `document.body.appendChild(pop)`, never set `top: 0px; left: 0px;`, compute clamped coordinates synchronously, then reveal (`visibility: visible; opacity: 1`) (Item 4).
3. `app/src/components/messages.rs` + `app/src/pages/chat.rs`:
   - Add `loading_initial: bool` prop to `MessageList`; render centered `.chat-loading-initial` spinner while `loading_initial && messages.is_empty()`, hiding `.conversation-intro` & `.e2ee-notice` until `!loading_initial && !has_more` (Item 2).
   - Add per-conversation in-memory caches (`dm_cache`, `group_cache`, `channel_cache`) in `chat.rs` for instant zero-reload switching back to already-opened chats + silent background refresh + sync on send/receive/edit/delete/paginate (Item 3).
4. `app/src/components/settings_more.rs`, `app/src/components/settings_view.rs`, `app/src/components/settings_extra.rs`, `app/src/components/accounts.rs`:
   - Remove redundant `.settings-group-title` elements where rows are self-explanatory, keeping ONLY essential titles (`Xem trước`, `Chủ đề`, `Màu nhấn`, `Sao lưu chìa khóa bảo mật`, `Nhật ký bảo mật`, `Danh sách chặn`, `Đặt mật khẩu khóa`, `Nội dung tin nhắn tự động`, etc.) (Item 7).
5. `app/e2e/spec048-ux-refinements.mjs`:
   - Verify ALL 8 items live in Chromium + inspect visual screenshots via `read_file`.
