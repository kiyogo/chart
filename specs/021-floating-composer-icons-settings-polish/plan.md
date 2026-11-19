# Technical Plan - Floating Composer, Icons & Settings Polish

## 1. Floating Composer (`app/src/components/composer.rs` & `app/assets/main.css`)
- Remove `.composer-hint` markup from `Composer` (`composer.rs` lines 671-677).
- Style `.composer` in `main.css`:
  - `position: absolute; left: 0; right: 0; bottom: 0; z-index: 12; background: transparent !important; pointer-events: none;`
  - `padding: var(--space-1) clamp(var(--space-3), 2.5vw, var(--space-8)) max(var(--space-3), env(safe-area-inset-bottom));`
- Set `.composer-inner { pointer-events: auto; }` and give `.composer-box` clean frosted backdrop (`background: color-mix(in srgb, var(--bg-input) 92%, var(--bg-primary)); backdrop-filter: blur(16px); box-shadow: var(--shadow-md);`).
- Add bottom scroll clearance to `.chat-messages` and `.vico-chat-scroll`: `padding-bottom: calc(var(--space-4) + 4.75rem);`.
- Position `.typing-indicator` cleanly floating above composer (`position: absolute; left: 0; right: 0; bottom: 4.5rem; pointer-events: none; z-index: 11;`) and hide when empty (`:empty { display: none; }`).

## 2. Invisible Resizer Hover (`app/assets/main.css`)
- Set `.panel-resizer::after` and `.panel-resizer:hover::after` / `.panel-resizer.is-dragging::after` to `background: transparent; box-shadow: none;` (only cursor `col-resize` appears on hover/drag).
- Remove `text-transform: uppercase;` from `.discord-role-header` and `.header-search-group`.

## 3. Redesigned "Mới" Icon (`app/src/components/icons.rs`)
- Update `FeedThreadsIcon` to render a crisp circular duotone Threads swirl without the heavy rectangular border.

## 4. Clean Sentence-Case Names Without `-` or `&`
- Backend (`web/backend/src/enterprise/mod.rs` & `migrations/0025_clean_enterprise_names.sql`):
  - Update default seeded categories (`Không gian doanh nghiệp`, `Phòng họp trực tuyến`), channels (`Thông báo nội bộ`, `Ban điều hành`, `Nhân sự hành chính`, `Kỹ thuật sản phẩm`, `Kinh doanh vận hành`, `Tự động hóa`, `Phòng họp điều hành`, `Phòng họp nhanh`), departments, and starter projects to remove `-` and `&` and use sentence case.
  - Add migration `0025_clean_enterprise_names.sql` so existing seeded database records update immediately.
- Frontend (`app/src/components/navigation/sidebar.rs`, `app/src/components/server_members_panel.rs`, `app/src/pages/chat.rs`, `app/src/services/enterprise.rs`):
  - Add `clean_display_name(raw: &str) -> String` helper converting `-` to spaces and applying sentence case (only first letter uppercase, preserving `AI`). Use in category titles, channel rows, and channel chat header.
  - Update `build_member_groups` in `server_members_panel.rs`: `"Chủ sở hữu {}"`, `"{} {}"`, `"Thành viên {}"`, `"Ngoại tuyến {}"` (no `—` dash).

## 5. Community Settings Stage & Flat Enterprise Nav (`app/src/components/server_settings.rs`, `app/src/components/enterprise/settings.rs`, `app/src/pages/chat.rs`)
- In `chat.rs`:
  - Keep `.nav-sidebar` and `.panel-resizer[data-resizer="sidebar"]` visible when `server_settings_open()` is active (only `is_settings_open()` hides `.nav-sidebar`).
  - Pass active highlight / toggle behavior to `on_open_server_settings`: clicking Gear `⚙` toggles `server_settings_open`.
  - Render `ServerSettingsView` inside `.main-stage` replacing only the chat/members area.
- In `server_settings.rs` & `enterprise/settings.rs`:
  - Remove `<header class="chat-header">` (`[Logo] Cài đặt cộng đồng · ...`) from `ServerSettingsView`. Render `.only-mobile` back link when viewport `< 768px` and `"Quay lại"` button at bottom of `.settings-nav`.
  - Integrate Enterprise setting sections directly into `ServerSettingsView`'s left `.settings-nav` under group `"Doanh nghiệp"` when `is_ent && can_ent_settings`:
    `Thiết lập`, `Nhân sự`, `Công việc`, `Chấm công`, `Phê duyệt`, `Bảo mật`, `Kiểm toán`
  - Refactor `EnterpriseSettingsPanel` to accept `active_section: &'static str` (and optional standalone toolbar mode fallback) so when rendered inside `ServerSettingsView`, there is NO nested horizontal pill toolbar! Remove `&`, `-`, and multi-word Title Case across all labels.
  - Adjust `.server-settings-stage .settings-nav` default width cleanly so sidebar + settings nav + settings body sit harmoniously at standard desktop sizes (`1280px+`).
