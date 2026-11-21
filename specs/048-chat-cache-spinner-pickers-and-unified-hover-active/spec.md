# Feature Specification: Rail Avatar Padding, Instant Chat Cache + Loading Spinner, Zero-Flash Pickers, Unified Translucent Hover/Active System & Balanced Layout (Spec 048)

**Feature Branch**: `main`
**Created**: 2026-11-21
**Status**: Active

## User Requirements (Mapped 1-to-1 to 11 Live Screenshots)
1. **Left Rail Avatar Bottom Clearance (Screenshot 1)**:
   - `.app-rail` must have symmetric top and bottom padding so the bottom user avatar never touches or clips against the lower rounded edge of `.app-rail`.
2. **Initial Chat Loading Spinner Instead of Intro Flash (Screenshot 2)**:
   - When opening an uncached conversation (DM, Group, or Channel), show a clean centered loading spinner (`loading_initial = true`) — NEVER flash `"Đây là khởi đầu cuộc trò chuyện..."` / `"Tin nhắn được mã hóa đầu cuối..."` before messages finish loading.
3. **Instant Conversation Re-Entry via In-Memory Cache (Screenshot 2)**:
   - Once a conversation (DM, Group, or Channel) has been loaded in the current session, switching to another conversation and clicking back MUST display cached messages immediately with zero blanking, zero spinner, and zero forced reload — while silently syncing any newer messages in the background.
4. **Zero Top-Left `(0,0)` Flash on Custom Pickers**:
   - `<select>`, `<input type="color">`, `<input type="date">`, and `<input type="time">` custom popovers (`app/web/custom_pickers.js`) MUST be positioned at the clicked anchor's exact coordinates while hidden before being attached/revealed — NEVER flashing at `(0,0)` top-left first.
5. **Balanced Server Sidebar Header & Top Category Spacing (Screenshot 3)**:
   - `.sidebar-server-header` must align vertically with `.sidebar-top-header` (not pressed against the top edge) and the first category/channel section below it must sit at a tight, harmonious distance (no excessive empty vertical gap).
6. **Unified Translucent Hover & Accent Active System Across the Entire App (Screenshots 4, 5, 6, 7, 10, 11)**:
   - Hover on ALL interactive items (sidebar chat rows, channel rows, settings nav items, left rail icons, segmented buttons, stickers, menu items, buttons) MUST always render a uniform translucent soft grey (`rgba(15, 23, 42, 0.06)` light / `rgba(255, 255, 255, 0.08)` dark) that blends smoothly with white, dark, translucent, blurred glass, and wallpaper backgrounds underneath — zero opaque white/grey stickers, and zero solid white circle behind `.conv-more` (`...`).
   - Active / selected states MUST render the exact same shape tinted with the app's translucent theme Accent (`color-mix(in srgb, var(--accent) 16%, transparent)` light / `24%` dark) that blends smoothly with any background underneath.
   - **100% Zero Text/Icon Color Change**: Hover and Active states MUST NOT change text color or SVG icon color/fill (`fill-opacity: 0`, `color: var(--text-primary)` preserved).
   - **100% Zero Bolding on Click/Active**: Buttons, tabs, segmented options, nav items, and list rows MUST NEVER jump to bold `font-weight` when clicked or active.
7. **Selective Essential Titles in Settings (Screenshots 8 & 9)**:
   - Keep `.settings-page-title` and `.settings-nav-title` hidden, and delete redundant section titles where rows are already self-explanatory (`Khi trò chuyện`, `Trạng thái`, `Ai có thể liên hệ với bạn`, `Tài khoản`, `Thông tin tài khoản`, `Mật khẩu và bảo mật`, `Thông tin định danh`, `Tùy chọn thông báo`, `Bật trả lời tự động`, `Màu sắc nâng cao`, `Hình dạng và bố cục`, `Bảo vệ tin nhắn`, `Đăng xuất`, etc.).
   - Restore clean, subtle section titles ONLY on blocks that would otherwise be unclear without a label — such as `"Xem trước"` (Screenshot 9), `"Chủ đề"`, `"Màu nhấn"`, `"Sao lưu chìa khóa bảo mật"` (Screenshot 8), `"Nhật ký bảo mật"`, `"Mật khẩu xuất dữ liệu"`, `"Xuất dữ liệu nâng cao"`, `"Danh sách chặn"`, `"Đặt mật khẩu khóa"`, `"Nội dung tin nhắn tự động"`, Storage/Backup/Cloud sections, and Server Settings Plugin/Webhook sections.
8. **Synchronized Equal Left/Right Edge Spacing Across All Sidebars & Settings Columns App-Wide (Screenshots 4, 5, 7, 11)**:
   - Unify left & right horizontal padding (`--sidebar-pad: 0.5rem`) across `.sidebar-scrollable`, `.sidebar-search`, `.sidebar-top-header`, `.sidebar-server-header`, `.add-friend-container`, `.friends-scroll`, `.settings-nav`, `.server-settings-stage .settings-nav`, and `.details-panel` so every sidebar/panel has identical, symmetrical left & right insets matching the Messages list (no lopsided scrollbar asymmetry).
   - Increase `.settings-cols` horizontal column gap (`clamp(2.25rem, 3.5vw, 3.5rem)`) so Column 1's toggle switches never visually crowd Column 2's labels (Screenshot 11).
