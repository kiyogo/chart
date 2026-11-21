# Feature Specification: Headerless Ultra-Minimal Settings & Zero Inner White Boxes App-Wide (Spec 047)

**Feature Branch**: `main`
**Created**: 2026-11-21
**Status**: Active

## Problem Statement (From User Request + 4 Live Screenshots)
The user inspected the live Settings UI over a translucent anime wallpaper and reported two major visual friction points:
1. **Redundant Titles Everywhere in Settings**:
   - The large page title at the top of the Settings body (`.settings-page-title`, e.g. `"Trò chuyện"`, `"Quyền riêng tư"`, `"Bảo mật"`) repeats the already-highlighted tab in the left navigation.
   - Left navigation category headers (`.settings-nav-title`, e.g. `"Tài khoản"` directly above `"👤 Tài khoản"`, `"Ứng dụng"`, `"Sáng tạo"`, `"Khác"`) create duplicate noise.
   - Section subheadings inside Settings (`.settings-group-title`, e.g. `"Khi trò chuyện"`, `"Ai có thể liên hệ với bạn"`, `"Tài khoản"`, `"Xác minh hai bước"`, `"Nhật ký bảo mật"`, etc.) clutter the screen even though the rows themselves are already self-explanatory.
2. **Nested White Boxes ("Khung trắng") Still Rendering Across the App**:
   - Whenever the user enables panel transparency or wallpaper (`data-panel-translucent="true"`, `data-panel-glass="true"`, `data-panel-scrim="true"`), Spec 045 rules were still painting opaque/semi-opaque white rounded card boxes around every `.settings-row`, `.settings-list`, `.settings-nav-item`, `.chat-item`, `.chat-header-identity`, `.chat-header-actions`, `.meet-card`, `.feed-card`, `.mod-card`, etc.
   - In addition, static rows like `.account-row`, `.security-event`, `.sticker-pack-card`, `.segmented button.active`, `.partner-info-card`, `.event-card`, `.forum-row`, `.forum-post`, `.backup-codes`, etc. still painted nested white/grey boxes inside already-framed outer panels across the entire application.

## Requirements & Acceptance Criteria
1. **Zero Redundant Titles in Settings**:
   - Hide `.settings-page-title`, `.settings-nav-title`, and `.settings-group-title` completely across all Settings tabs (and Server Settings nav/body).
   - Ensure no control loses its meaning once section titles are removed:
     - In `accounts.rs`, show `"Xác minh hai bước"` inline as the `.settings-label` on the 2FA status row (with `"Chưa bật"` / `"Đang bật"` badge/sublabel).
     - In `settings_more.rs`, give `"Dán bảng tính"`, `"Hình nền trò chuyện"`, and `"Hình nền toàn ứng dụng"` clean inline `.settings-label` labels directly on their rows.
2. **Zero Inner White Boxes App-Wide (Both Opaque & Translucent/Wallpaper/Glass Modes)**:
   - Strip ALL inner card/box backgrounds (`background: transparent`) from nested items inside main panels across the entire application — including `.settings-row`, `.settings-list`, `.settings-profile-card`, `.settings-preview-chat:not([class*="wp-"])`, `.settings-nav-item` (idle), `.account-row`, `.security-event`, `.backup-codes`, `.sticker-pack-card`, `.chat-item` (idle), `.chat-header-identity`, `.chat-header-actions`, `.partner-info-card`, `.meet-card`, `.feed-card`, `.feed-compose-bar`, `.mod-card`, `.result-card`, `.product-card`, `.bot-card`, `.plugin-card`, `.event-card`, `.forum-row`, `.forum-post`, `.welcome-channel`, `.welcome-rules`, `.inv-card`, `.theme-card`, `.wallpaper-tile`.
   - Replace the stark white box inside `.segmented button.active` with a clean flat accent tint (`var(--accent-soft)` + `var(--accent-text)`).
   - Preserve high text readability over busy/translucent wallpapers via Spec 045's automatic light/dark surface tone flipping + subtle text halo (zero white box clutter!).
   - Keep interactive message bubbles, form inputs (crisp 1px border), buttons, switches, and floating popovers/modals cleanly styled.
