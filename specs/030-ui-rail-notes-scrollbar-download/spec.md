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
- **FR-005 (Refined Popover Spacing, Borderless QR/Menu Internals, Notifications Full Page & DownloadView White Rounded Surface)**:
  - Rail popovers (`.account-menu.rail-menu`, `.download-rail-menu`) float cleanly spaced away from the panel border (`left: calc(100% + 12px)`), with pure `#FFFFFF` surface, NO gray inner boxes/frames (`border: none` around QR & store icons), equal 4-sided inner spacing, and a subtle soft dark outer blurred border (`box-shadow: 0 6px 24px rgba(15, 23, 42, 0.10), 0 1px 4px rgba(15, 23, 42, 0.05); border: none`).
  - Clicking `"Thông báo"` on the left rail navigates directly to a dedicated full page (`NavSection::Notifications`, `NotificationsView`) displaying real unread direct messages and incoming friend requests.
  - `DownloadView` preserves the signature white (`#FFFFFF`) rounded (`border-radius: var(--panel-radius)`) panel appearance of `.main-stage`, has zero gray box containers (`#F4F6F9`), zero frames around QR SVGs, and removes the top-left `<-` back arrow, logo, and `"chatw"` title bar text.
  - All menus, popovers, dialogs, and rail icons open/hover cleanly at exact `1:1` scale (zero `scale(...)` bounce/inflation in `pop-in`, `dialog-in`, rail icons, or controls), and the QR download popover renders a full-width crisp QR (`quiet_zone(false)`) with uniform `8px` (`--space-2`) edge margins matching all other rail menus.
