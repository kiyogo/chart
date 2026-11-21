# Implementation Tasks — Spec 046: Settings Bento 2-Column Layout, Custom Theme Pickers, Chat Scroll/Jitter Fix & UI Polish

- [x] **T001: Self-Chat Menu & Server Header Cleanup (Items 1 & 6)**
  - Remove permanent `.self-chat-create` banner from `app/src/components/navigation/conversations.rs`.
  - Upgrade Messages header `+` button in `app/src/components/navigation/sidebar.rs` into a clean custom dropdown menu (`.cw-dropdown-menu`) with "Tạo nhóm mới" and (when `!self_chat`) "Tạo lại 'Chính mình'".
  - Hide `.sidebar-top-header` ("Cộng đồng" + `+`) in `app/src/components/navigation/sidebar.rs` when `matches!(active_section, NavSection::Server(_))`.

- [x] **T002: Eliminate Chat/Right-Panel Jitter & Implement Latest-First Scroll-Up Infinite Pagination (Items 2 & 3)**
  - Fix `DmDetailsPanel` (`app/src/components/details_panel.rs`) so `get_user_profile` only fetches when `user_id` actually changes, never clearing `profile` on same-user re-renders.
  - Fix `MessageList` (`app/src/components/messages.rs`) so `plugin_actions` never loops/re-renders on message changes, and add scroll-near-top lazy pagination props (`has_more`, `loading_more`, `on_load_more`).
  - Switch `.chat-messages` in `app/assets/main.css` from `flex-direction: column-reverse` to rock-solid `flex-direction: column` with `scrollbar-gutter: stable` and update `platform::scroll_chat_to_bottom` to pin to `el.scrollHeight` via `requestAnimationFrame`.
  - Add `before_seq: Option<i64>` to `api::channels_get_messages` (`app/src/services/api.rs`).
  - Fix `app/src/pages/chat.rs`: guard `select_friend`/`select_group`/`select_channel` against wiping messages when clicking the active chat; handle `user_keys()` boot race cleanly so messages NEVER fail to load; sort channel & group messages ascending by `seq`; implement scroll-up older-page loading (`before_seq`, page size 30) with exact scroll-position preservation for DMs, Groups, and Channels.

- [x] **T003: Custom Theme-Synced Pickers (`app/web/custom_pickers.js`) for Select, Color, Date & Time (Item 5)**
  - Create `app/web/custom_pickers.js` intercepting native `<select>`, `<input type="color">`, `<input type="date">`, and `<input type="time">` across the entire app and rendering theme-synchronized ChatW popovers (dropdown menu, HSV/swatch color picker, Vietnamese calendar, hour/minute time picker).
  - Register `custom_pickers.js` in `app/web/index.html`, `app/build-web.sh`, and `app/src/main.rs` (both Web & Desktop!).

- [x] **T004: Rail Active Icon No-Nested-Box, Flatten Nested Boxes, Crisp 1px Input Borders & Equal 1/3 Bento 2-Column Settings CSS (Items 4, 7 & 8)**
  - Remove inner box on `.app-rail .rail-item.active` when `html[data-rail-scrim="true"]` (Item 4).
  - Flatten nested white/grey boxes inside big panels (`.plugin-dev-publish-box`, `.plugin-audit-row`, `.settings-list`, `.settings-row`) so content flows straight down without box-in-box clutter (Item 7).
  - Lighten `--bg-input` (`#F8F9FB` light / `#1B202A` dark) and apply crisp `1px solid var(--input-border)` always-visible hairline border on form inputs/selects/textareas, shifting cleanly to `1px solid var(--accent)` on `:focus` with zero thick ring (Item 7).
  - Set `.settings-body { padding: var(--space-4); }` (exact 1/3 of old `3rem`, 100% equal top = left = right), `.settings-content { width: 100%; max-width: none; }`, and `.settings-cols` 2-column Bento CSS Grid (Item 8).

- [x] **T005: Balanced 2-Column Bento Settings Layout & Zero-Jargon Simplification Across ALL Tabs (Items 7 & 8)**
  - Convert `settings_more.rs`, `settings_view.rs`, `settings_extra.rs`, `storage_ui.rs`, `backup_cloud_ui.rs` into clean, balanced Left & Right `.settings-cols` Bento layouts.
  - Eliminate remaining jargon & `(...)` parentheses in `storage_ui.rs`, `backup_cloud_ui.rs`, `settings_view.rs`, and `plugin_hub.rs` (Webhook form).

- [x] **T006: Comprehensive Playwright E2E Verification (`app/e2e/spec046-comprehensive-ux.mjs`), Visual Inspection, Release Builds & Auto-Push**
  - Verify all 8 user requirements in live Chromium against the real backend and visually inspect screenshots with `read_file`.
  - Run full regression suite (`spec045`, `spec044`, `spec043`, `spec042`, `spec041`, `spec040`, `spec046`).
  - Build WebAssembly release + Linux desktop release + `.deb` package, commit, and `git push origin main` per AGENTS.md Rule #1.
