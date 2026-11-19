# Technical Plan - Full-Page Community Settings & Resizer Spacing Fix

## 1. Fix Resizer Flex Gap (`app/assets/main.css` & `app/web/panel_resizer.js` & `app/web/boot.js`)
- Update `.panel-resizer` CSS:
  ```css
  .panel-resizer {
    position: relative;
    z-index: 15;
    width: 10px;
    margin: 0 calc(-0.5 * var(--shell-gap) - 5px);
    cursor: col-resize;
    flex-shrink: 0;
    user-select: none;
    touch-action: none;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .panel-resizer::after {
    content: "";
    position: absolute;
    top: 10px;
    bottom: 10px;
    width: 2px;
    background: transparent;
    border-radius: 999px;
    transition: background-color 0.15s ease, box-shadow 0.15s ease;
  }
  ```
- Support `.settings-nav` width customization via `--settings-nav-width` (`width: var(--settings-nav-width, 15rem);` in `main.css`) and `data-resizer="settings-nav"` in `panel_resizer.js` + `boot.js`.

## 2. Convert `ServerSettingsDialog` -> `ServerSettingsView` (`app/src/components/server_settings.rs`)
- Replace `.modal-overlay` dialog markup with full-page Settings structure matching `SettingsView`:
  - Outer `<div class="main-stage" tabindex="-1" onkeydown: Esc -> on_close>`
  - `<header class="chat-header">` with `BackIcon` button, community `Avatar` (28px), `"Cài đặt cộng đồng · {server.name}"`, spacer, `"Tải lại"` (`RefreshIcon`) button, and `"Xong"` primary button.
  - `<div class="settings-page">` (plus mobile `.has-detail` responsiveness):
    - Left `nav.settings-nav`:
      - Search input (`Tìm cài đặt`) matching `SettingsView` (`search-field`) to quickly filter tabs.
      - Grouped tabs (`NAV_TABS`) with clear vertical `.settings-nav-item` buttons & icons:
        - `"Cộng đồng"`:
          - `Tổng quan` (`GlobeIcon`)
          - `Chào mừng và nội quy` (`BookIcon`)
          - `Emoji và Sticker` (`ImageIcon`)
        - `"Quản lý"`:
          - `Vai trò và quyền` (`ShieldIcon`)
          - `Thành viên` (`FriendsIcon`)
          - `Nhật ký kiểm duyệt` (`FileTextIcon`)
        - `"Doanh nghiệp"` (if `is_ent && can_ent_settings`):
          - `Doanh nghiệp` (`BuildingIcon`)
      - Bottom footer action button in nav (`margin-top: auto`): `"Quay lại cộng đồng"` (`BackIcon`).
    - `.panel-resizer` (`data-resizer="settings-nav"`) between `.settings-nav` and `.settings-body`.
    - Right `div.settings-body > div.settings-content`:
      - `<h1 class="settings-page-title">{tab_title}</h1>`
      - Optional feedback banner (`status-badge`).
      - Tab content styled consistently with `SettingsView`:
        - `Overview`: `.settings-profile-card` (community banner + avatar + name + status badge + banner upload button) + `.settings-group` / `.settings-list` containing toggles for public listing, search indexing, and enterprise mode.
        - `Roles`: role layout (`role-list` + `RoleEditor`) utilizing `.settings-group` & `.settings-list` containers for permissions.
        - `Members`: wrapped inside `.settings-group` / `.settings-list` with clean member rows.
        - `Welcome`, `Expressions`, `Audit`, `Enterprise`: seamlessly embedded inside `.settings-group`.

## 3. Integrate `ServerSettingsView` in `app/src/pages/chat.rs` & `app/src/components/settings_view.rs`
- In `chat.rs`:
  - Update `is_chat_selected` to include `|| server_settings_open()`.
  - Hide `.nav-sidebar` and `.panel-resizer[data-resizer="sidebar"]` whenever `is_settings_open() || (server_settings_open() && selected_server().is_some())`.
  - Render `ServerSettingsView` inside `<main class="main-stage">` when `server_settings_open()` is active (removing the old modal popup block at lines 3615-3668).
  - Ensure switching rail sections, selecting DM/group/server/channel, or opening App Settings automatically resets `server_settings_open.set(false)`.
- In `settings_view.rs`:
  - Add `.panel-resizer[data-resizer="settings-nav"]` between `nav.settings-nav` and `div.settings-body`.

## 4. Build, Test, Browser Verify, Push
- `cargo test -p chatw-app --lib && cargo test -p chatw --lib`
- `bash app/build-web.sh release`
- Restart `./target/debug/chatw` and run browser verification for:
  1. Panel gap equals crisp normal spacing (12px) even with resizers present.
  2. Opening "Cài đặt cộng đồng" displays full-page left-nav + right-content UI matching App Settings.
  3. Switching tabs (`Tổng quan`, `Vai trò và quyền`, `Thành viên`, `Doanh nghiệp`, etc.), dragging `.settings-nav` resizer, and exiting back to chat work properly.
- Commit and push to `origin main`.
