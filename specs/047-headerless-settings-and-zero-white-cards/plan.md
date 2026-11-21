# Technical Implementation Plan — Spec 047

## Target Files
1. `app/assets/main.css`:
   - Hide `.settings-page-title`, `.settings-nav-title`, and `.settings-group-title` inside `.settings-page` (`display: none !important;`).
   - Remove ALL white/scrim card box overrides under `data-panel-translucent`, `data-panel-glass`, and `data-panel-scrim` (lines 3726–3837), keeping only subtle outer panel translucency, active/hover highlights, and gentle adaptive text-shadow readability when `data-panel-scrim="true"`.
   - Flatten ALL static nested card boxes across the entire app (`.account-row`, `.security-event`, `.backup-codes`, `.partner-info-card`, `.settings-profile-card`, `.settings-preview-chat:not([class*="wp-"])`, `.theme-card`, `.wallpaper-tile`, `.sticker-pack-card`, `.meet-card`, `.feed-card`, `.feed-compose-bar`, `.mod-card`, `.result-card`, `.product-card`, `.bot-card`, `.plugin-card`, `.event-form`, `.event-card`, `.forum-new`, `.forum-row`, `.forum-post`, `.welcome-channel`, `.welcome-rules`, `.inv-card`, `.studio-project`, `.studio-target`, `.achievement`, `.template-tile`, `.section-row`, `.preset-row`, `.private-note`, `.segmented`).
2. `app/src/components/accounts.rs`:
   - Label the 2FA row inline (`"Xác minh hai bước"` + status `"Chưa bật"` / `"Đang bật"`) so it remains 100% self-explanatory without `.settings-group-title`.
3. `app/src/components/settings_more.rs`:
   - Wrap `Dán bảng tính` segmented control in `.settings-row.column-layout` with inline `.settings-label`.
   - Give both wallpaper grids (`Hình nền trò chuyện`, `Hình nền toàn ứng dụng`) subtle inline `.settings-label` rows so the two grids remain unmistakable without section titles.
4. `app/e2e/spec047-headerless-no-white-boxes.mjs`:
   - Live Chromium E2E test verifying:
     - Zero visible `.settings-page-title`, `.settings-nav-title`, `.settings-group-title` across all Settings tabs.
     - Zero white card boxes (`rgba(0, 0, 0, 0)`) on `.settings-row`, `.settings-list`, `.account-row`, `.security-event`, `.settings-nav-item` (idle), `.chat-item` (idle), `.chat-header-identity`, `.chat-header-actions` in BOTH default opaque mode AND translucent anime-like wallpaper mode (`panel_transparency = 25%` and `75%`).
     - High visual contrast & crisp readability verified via captured screenshots with `read_file`.
   - Update existing test assertions in `spec045-adaptive-contrast.mjs` and `storage-sync-check.mjs` if they explicitly asserted the old `.settings-page-title` or white `.settings-row` scrim boxes that the user just explicitly instructed us to remove.
