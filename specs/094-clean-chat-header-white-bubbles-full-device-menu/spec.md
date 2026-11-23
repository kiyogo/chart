# Spec 094: Pure White Bubbles, Neutral Timestamps, Clean Header & Unified Right Details Scroll

## 1. Context & User Request
- **User Request**:
  > "không tin nhắn hiện 100% là màu trắng và không được hiện màu đỏ ở giờ ,... nữa
  > và cái thiết bị của bạn xóa cho tôi ở trên đi nhé + ở menu ở bên phải hiện full không làm thanh kéo ở thiết của em và nếu kéo là nó sẽ kéo cả luôn"
- **User Attachments**:
  - `media_1795438410926_8ccff1b7.jpg` (Image 1): Message bubbles ("Không đọc được trên máy này") showing reddish/danger colored timestamps (`17:51`, `22:00`, `22:24`) and off-white/tinted bubble surfaces.
  - `media_1795438467015_eafdeebf.jpg` (Image 2): Right-side details panel ("Thiết bị của bạn") trapped in a nested scroll box with its own vertical scrollbar.
  - `media_1795438484934_9df4c7de.jpg` (Image 3): Top chat header showing a horizontal row of device chips (`header-inline-devices`: `[Linux 3 thiết bị này] [Thiết bị 2] ...`) cluttering the header bar.

## 2. Requirements & Acceptance Criteria
1. **100% Solid White Message Bubbles & Zero Red in Timestamps**:
   - [x] Message bubbles (`.message-row.received .message-bubble`, `.message-bubble.error-bubble`) must render in 100% solid white (`#FFFFFF !important;`) on light & wallpaper themes.
   - [x] Timestamps (`.bubble-meta`, `.message-time-standalone`) must NEVER be colored red or danger tint. They must use neutral muted color (`var(--msg-recv-meta)` / `#64748B`).
   - [x] Remove red text color from `.message-row.sent .message-bubble.error-bubble` so message text uses clean primary color.
2. **Remove Devices Strip from Top Chat Header**:
   - [x] Remove `.header-inline-devices` (`device-strip`) from the "Chính mình" top chat header in `app/src/pages/chat.rs`.
   - [x] Header displays only the clean identity (Pride avatar, "Chính mình", "Chỉ mình bạn · X thiết bị") and the top-right action buttons.
3. **Full-Height Synced Devices in Right Sidebar & Unified Single Scroll**:
   - [x] Remove `max-height: 15.5rem; overflow-y: auto;` from `.self-device-list` and `.self-vault-list` in `app/assets/main.css`.
   - [x] Device items render at full natural height with no inner scrollbar.
   - [x] When the content overflows, scrolling scrolls the entire right sidebar (`.details-scroll`) as one unified container.
