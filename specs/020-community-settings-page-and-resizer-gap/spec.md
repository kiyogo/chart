# Spec 020: Full-Page Community Settings View & Resizer Spacing Fix

## 1. Context & User Problem
1. **Modal vs Full Page Community Settings**:
   Currently, Community Settings (`Cài đặt cộng đồng`) opens inside a centered modal overlay dialog (`ServerSettingsDialog` with `.modal-overlay > .modal-content.wide.manage-dialog`) with top wrapped tab pills (`filter-chip`). The user requested converting Community Settings into a full-page workspace view replacing the chat view, with navigation buttons arranged vertically on the left side, matching the visual layout of App Settings (`SettingsView`):
   > "tất cả Cài đặt cộng đồng đổi hiện là 1 trang mới thay thay chat á đổi chat thành setting ,... và mấy cái nút hiện bên trái ,... giao diện nó sẽ giống kiểu như ở setting á"
2. **Excessive Gap Around Panel Resizers**:
   After inserting `.panel-resizer` elements (`width: 6px; margin: 0 -3px;`) as flex children inside `.workspace-shell` and `.chat-main-container` (which both apply `gap: var(--shell-gap)`), Flexbox added `var(--shell-gap)` on BOTH sides of every resizer, doubling the gap between columns (~24px blue band instead of 12px):
   > "và lỗi nó bị cách xa nhau quá rồi á"

## 2. Acceptance Criteria
- [ ] `.panel-resizer` uses exact negative horizontal margins (`width: 10px; margin: 0 calc(-0.5 * var(--shell-gap) - 5px);`) so total visual spacing between adjacent cards remains exactly `1 * var(--shell-gap)` (`0px` when `--shell-gap` is `0px`).
- [ ] Hover/drag indicator line on `.panel-resizer::after` is cleanly inset vertically (`top: 10px; bottom: 10px`) with rounded caps and zero resting visual clutter.
- [ ] Clicking "Cài đặt cộng đồng" switches out of chat into a full-page `ServerSettingsView` using `.main-stage > .settings-page` (`nav.settings-nav` on the left + `div.settings-body` on the right), completely eliminating the modal overlay.
- [ ] All community settings tabs (`Tổng quan`, `Vai trò và quyền`, `Thành viên`, `Chào mừng và nội quy`, `Emoji và Sticker`, `Nhật ký kiểm duyệt`, and `Doanh nghiệp` when enabled) appear vertically in the left `.settings-nav` column with appropriate SVG icons, search filter input, and grouped headers matching `SettingsView`.
- [ ] Content inside `.settings-body > .settings-content` matches `SettingsView` styling (`.settings-page-title`, `.settings-profile-card` with community banner/avatar in Overview, `.settings-group` & `.settings-list` containers).
- [ ] Exiting Community Settings via Back button, "Xong" button, `Escape` key, or switching navigation restores the server/chat view cleanly and refreshes updated roles/members.
- [ ] Left `.settings-nav` panel in both `SettingsView` and `ServerSettingsView` is also resizable via `data-resizer="settings-nav"`.
- [ ] Unit tests pass, release web bundle builds cleanly, live browser verification passes, and changes are committed & pushed to `origin main`.
