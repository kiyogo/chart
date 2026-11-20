# Feature Specification: Scrollbar Refinement, Left Rail Notifications & Download App, Settings Header Cleanup, and Messenger Thought Bubble Polish

**Feature Branch**: `main`  
**Created**: 2026-11-20  
**Status**: Approved  

## Functional Requirements
- **FR-001 (Refined Canvas Overlay Scrollbar)**:
  - Keep idle/proximity scrollbar slender (`3px`).
  - Reduce hover expansion from `9px` to a subtle `5px` (`THUMB_EXPANDED_WIDTH = 5`) with softer contrast so hovering does not visually overwhelm the sidebar or settings panels.
  - Inset top/bottom vertical track bounds (`TRACK_INSET_Y = 12`) and right edge margin (`THUMB_EDGE_MARGIN = 4`) so the scrollbar thumb always stays strictly inside rounded container borders (`border-radius: 18px`).
- **FR-002 (Left Rail Notifications & Download App + White Menus + No Avatar Ring)**:
  - Add `"Thông báo"` (Bell icon) and `"Tải ứng dụng"` (Download icon) items in `.rail-bottom`.
  - Style all rail popovers (`.account-menu`, notification popover, download QR popover) with pure `#FFFFFF` surface and subtle faint dark hairline border (`rgba(15, 23, 42, 0.08)`).
  - Remove the white ring (`background` / `padding` highlight) around `.rail-account` avatar when clicked (`aria-expanded="true"`).
  - Hovering `"Tải ứng dụng"` displays a pure white QR Download menu containing a scannable SVG QR code, `"Tải cho máy tính"` button, and single-logo icon buttons for CH Play and App Store.
  - Clicking `"Tải ứng dụng"` opens a dedicated `DownloadView` page with hero title, intro text, top/bottom platform action buttons, desktop OS selector (Windows, macOS, Linux `.deb`/`.AppImage`), and mobile selector (Android `.apk`, iOS).
- **FR-003 (Settings Header Removal)**:
  - Remove the top `<- Cài đặt` header bar from desktop `SettingsView` so Settings opens directly with `.settings-nav` and `.settings-body`.
- **FR-004 ("Chia sẻ suy nghĩ" Thought Cloud & "Mới" Feed Strip)**:
  - Position the `"Chia sẻ suy nghĩ"` thought cloud at the top-left above the user's avatar (`#FFFFFF`, subtle faint dark border, authentic two-dot thought tail, clean separation above avatar) and move the avatar strip closer beneath `"Tìm cuộc trò chuyện"`.
  - Ensure clicking `+` always opens Story creation (`"story"`) and clicking `"Chia sẻ suy nghĩ"` always opens Thought note creation (`"thought"`).
  - Align the `"Mới"` (`FeedView`) top Stories & Thoughts strip (`#feed-notes-strip`) with the exact same visual layout and full top-clearance for thought clouds.
