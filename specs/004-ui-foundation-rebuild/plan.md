# Plan 004

## CSS architecture
- `assets/theme.css`: tokens (màu semantic light/dark, accent, radius, spacing 4→48, typography scale theo rem,
  density, motion, z-index). `[data-theme]`, `[data-accent]`, `[data-density]` trên `<html>`; cỡ chữ gốc qua
  `--font-size-root` (auto: 15px → 16px ≥1680px → 17px ≥2400px).
- `assets/main.css`: viết lại theo thứ tự reset → base → controls → shell → rail → sidebar → chat → messages → composer →
  pickers → details → settings → modals → auth → calls → responsive. Không `!important` (trừ reduced-motion).

## Component changes
- `pages/chat.rs`: render `NavigationRail` + `NavigationSidebar` + `main-stage`; thêm state `nav_section`,
  `server_categories`, `voice_participants`, modal cộng đồng/danh mục/kênh/mời; `SettingsView` thay cho settings nội tuyến;
  sửa trùng tin nhắn; thay toàn bộ render tin nhắn bằng `MessageList`.
- Mới `components/messages.rs`: `MessageList` (gom nhóm, phân cách ngày, sticker, bubble), `Composer` (textarea).
- Mới `components/details_panel.rs`: hồ sơ (banner, avatar, tên, bio, trạng thái), hành động (nhắn, gọi, video, xem hồ
  sơ), tìm trong cuộc trò chuyện (lọc tin đã giải mã cục bộ), mục "Bảo mật và mã hóa" thu gọn.
- `components/emoji_picker.rs`: dữ liệu emoji theo danh mục + từ khóa, tìm kiếm, gần đây (localStorage), điều hướng phím.
- `pages/login.rs`: bố cục 2 cột (giới thiệu + thẻ đăng nhập), responsive.
- `components/settings_view.rs`: thêm mục Giao diện (theme/system/accent/cỡ chữ/mật độ), mobile list→detail.
- `platform/*`: `apply_appearance()` đặt data-* lên `<html>`, theo dõi `prefers-color-scheme` cho chế độ system.

## Test
- `cargo build` (wasm + native check), `cargo test --workspace`.
- `node app/e2e/ui-audit.mjs artifacts/screenshots/after` — 14 viewport × 2 theme + login + settings + community.
