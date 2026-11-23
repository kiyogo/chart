# Spec 107: Group Avatar, Cover Photo Banner & Rich Group Management Features

## 1. Problem Statement & User Request
User feedback and attached screenshot `media_1795451357269_5231e65e.jpg`:
> "thêm nhiều nhiều tính năng cho nhóm nữa đi chứ nhóm mà không đổi được ảnh bìa + avatar à"

Currently, the group information panel (`GroupInfoPanel` in `app/src/components/groups/group_info.rs`) is bare:
- No cover photo banner at the top of the group card.
- No ability to upload, change, or remove the group's cover photo ("ảnh bìa").
- No ability to upload, change, or remove the group's avatar ("ảnh đại diện").
- No group name or description editing for admins/owners.
- Rich group permissions and governance controls (approval for join requests, link join toggle, admin message broadcast only, pin permissions, poll permissions) that exist in the Axum backend (`manage.rs`) are not surfaced in the frontend UI.

## 2. Goals & Acceptance Criteria
1. **Group Avatar Upload & Management**:
   - Admins/owners can click on the group avatar to upload a custom group avatar.
   - Dedicated backend endpoint `POST /api/groups/:group_id/avatar` processes and stores the avatar honestly as `.avif` with thumbnail.
   - UI updates immediately and displays the new avatar across all members.
2. **Group Cover Photo Banner (Ảnh bìa nhóm)**:
   - Group info header features an elegant cover photo banner (with aesthetic gradient backdrop fallback when no custom banner is set).
   - Admins/owners can click the cover edit button to upload a custom cover image or remove the cover image.
   - Dedicated backend endpoint `POST /api/groups/:group_id/banner` validates and stores the banner in `uploads/banners/`.
3. **Group Name & Description Editing**:
   - Admins/owners can edit the group name and bio/description directly from the group panel.
4. **Rich Group Settings & Permissions (Cài đặt & Quyền hạn nhóm)**:
   - Provide an intuitive settings toggle section for admins/owners:
     - Require approval for new members (`require_approval`).
     - Enable/disable invite link joining (`allow_link_join`).
     - Announcement mode (only admins can send messages - `perm_send`).
     - Member permissions for polls (`perm_poll`), pins (`perm_pin`), renaming (`perm_rename`), and avatar changes (`perm_avatar`).
5. **Member Search & Member Count**:
   - Search filter for members list in groups with quick member count.
6. **Automated Verification & Mandatory Push**:
   - Automated Playwright E2E test verifying group avatar upload, cover photo banner, name/desc edit, and permissions update.
   - Automatic `git commit` and `git push origin main`.
