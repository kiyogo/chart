# Spec 107 Plan: Group Avatar, Cover Photo Banner & Rich Group Management Features

## 1. Backend Architecture (`web/backend/`)
1. **Model Updates (`web/backend/src/models.rs`)**:
   - Add `pub banner_url: Option<String>` to `GroupItem`.
   - Add `pub banner_url: Option<String>` to `UpdateGroupRequest`.
2. **Endpoint Implementations (`web/backend/src/groups/mod.rs`)**:
   - `POST /api/groups/:group_id/avatar`:
     - Checks user is member with admin/owner role (or `perm_avatar` enabled).
     - Validates payload size (<= 5MB).
     - Calls `crate::media_opt::process_avatar_honest(&body_vec)`.
     - Writes to `uploads/avatars/group_{group_id}.{ext}`.
     - Updates `groups.avatar_url`.
     - Returns `{"avatar_url": avatar_url}`.
   - `POST /api/groups/:group_id/banner`:
     - Checks user is member with admin/owner role.
     - Validates payload size (<= 8MB).
     - Sniffs image format (avif/webp/gif/png/jpg).
     - Writes to `uploads/banners/group_{group_id}.{ext}`.
     - Updates `groups.banner_url`.
     - Returns `{"banner_url": banner_url}`.
   - Update `list_my_groups`, `get_group_details`, and `update_group` SQL queries to include `g.banner_url`.

## 2. Frontend Models & Services (`app/src/`)
1. **Model Updates (`app/src/models.rs`)**:
   - Add `#[serde(default)] pub banner_url: Option<String>` to `GroupItem`.
2. **Services (`app/src/services/groups.rs`)**:
   - `groups_upload_avatar(group_id: &str, file_bytes: Vec<u8>) -> Result<String, ApiError>`
   - `groups_upload_banner(group_id: &str, file_bytes: Vec<u8>) -> Result<String, ApiError>`
   - `groups_get_settings(group_id: &str) -> Result<serde_json::Value, ApiError>`
   - `groups_update_settings(group_id: &str, payload: serde_json::Value) -> Result<serde_json::Value, ApiError>`

## 3. UI Component Architecture (`app/src/components/groups/group_info.rs`)
1. **Cover Photo Banner**:
   - Render top banner `<div class="group-cover-banner">` with `background-image: url(...)` or sleek gradient fallback.
   - If admin: render camera/edit icon button `<button class="group-cover-edit-btn">` triggering file input.
2. **Avatar Overlay Edit**:
   - Render camera icon overlay on `<div class="group-avatar-wrapper">` with hidden `<input type="file" accept="image/*">`.
3. **Group Name & Description Editor**:
   - If admin: show edit pencil icon next to name and description with inline editing or modal form.
4. **Group Settings & Permissions Section**:
   - Collapsible / dedicated section with toggle switches for `require_approval`, `allow_link_join`, `perm_send` (announcements only), `perm_poll`, etc.
5. **Member Search Bar**:
   - Quick search input filtering `members()` signal in real time.

## 4. CSS Styling (`app/assets/main.css`)
- `.group-cover-banner`: height 120px, border-radius 12px 12px 0 0 (or flush header), relative, background-size cover, background-position center.
- `.group-cover-edit-btn`: frosted glass pill at top-right of cover.
- `.group-avatar-wrapper`: positioned overlapping bottom half of cover, relative, circle avatar with 3px solid border.
- `.group-avatar-edit-badge`: camera icon button at bottom-right of avatar.
- `.group-settings-group`: clean rows with label, description, and modern switch toggle.

## 5. Verification & Testing
- Automated Playwright E2E test `app/e2e/spec107-group-avatar-cover-and-settings.mjs`.
- Automatic git commit and push to `origin main`.
