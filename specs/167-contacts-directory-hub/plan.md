# Technical Plan: Spec 167 — Contacts Directory Hub ("Danh bạ")

## 1. Database Migration (`web/backend/migrations/0042_direct_group_server_invites.sql`)
- Add `friend_requests_sender_pending_idx` index on `friend_requests (sender_id, created_at DESC) WHERE status = 'pending'`.
- Create `direct_target_invites` table storing direct group & community invitations (`id`, `kind` `'group'|'server'`, `target_id`, `code`, `inviter_id`, `invitee_id`, `status`, `created_at`, `UNIQUE(kind, target_id, invitee_id)`) with partial pending indexes on `invitee_id` and `inviter_id`.

## 2. Backend API Extensions (`web/backend/src/friends/mod.rs`)
- Enrich `PendingRequestItem` with `sender_display_name: Option<String>`.
- Add `GET /api/friends/requests/sent` → `Vec<SentRequestItem>` (`id`, `receiver_id`, `receiver_username`, `receiver_display_name`, `receiver_avatar_url`, `created_at`).
- Add `DELETE /api/friends/requests/{id}` → withdraw (by sender) or decline (by receiver) pending friend request.
- Add `DELETE /api/friends/{friend_id}` → unfriend cleanly (removes `friendships` row and clears accepted `friend_requests` between the pair).
- Add Direct Group/Community Invites endpoints:
  - `GET /api/friends/target-invites` → `DirectTargetInvitesResponse { received, sent }` joined live with `groups` / `servers` / `users` and filtering out already-joined targets.
  - `POST /api/friends/target-invites` → create/upsert direct pending invite (verifies inviter membership + block check).
  - `POST /api/friends/target-invites/{id}/accept` → joins invitee to group/server (enforcing group block & approval settings) and marks accepted.
  - `DELETE /api/friends/target-invites/{id}` → declines (by invitee) or withdraws (by inviter).

## 3. Frontend Models & API Client (`app/src/models.rs`, `app/src/services/api.rs`, `app/src/components/invite.rs`)
- Add `SentRequestItem`, `DirectTargetInviteItem`, `DirectTargetInvitesResponse`, and `sender_display_name` on `PendingRequestItem`.
- Add `friends_get_sent_requests`, `friends_delete_request`, `friends_remove`, `target_invites_list`, `target_invites_create`, `target_invites_accept`, `target_invites_delete`.
- Update `InviteFriendsDialog` (`app/src/components/invite.rs`) so sending a group/community invite to friends also persists a real `POST /api/friends/target-invites` entry so it immediately appears in both users' "Lời mời vào nhóm và cộng đồng" tabs.

## 4. Frontend UI & i18n
- Create `app/src/components/contacts_view.rs` containing:
  - `ContactsTab` enum (`FriendsList`, `GroupsCommunities`, `FriendRequests`, `TargetInvites`).
  - `ContactsSidebarMenu` (left sidebar matching Reference Image 1: search input + 4 icon category rows with counts & badges + instant `onmousedown` selection).
  - `ContactsMainPanel` (right main stage matching Reference Image 2: header + dedicated views for all 4 tabs, responsive 3-column card grid for received & sent invitations with `Thu hồi lời mời`, alphabetical friend directory, unified groups & communities directory, add-friend search, and invite-link join).
- Wire into `app/src/components/mod.rs`, `app/src/components/navigation/sidebar.rs`, `app/src/components/navigation/rail.rs`, `app/src/components/navigation/conversations.rs`, and `app/src/pages/chat.rs`.
- Add crisp theme-synced flat minimalist CSS rules (`.contacts-*`) in `app/assets/main.css` using exclusively ChatW CSS variables (compatible with Light, Ash, Dark, Obsidian, custom accents, and glass/scrim mode).
- Update `app/src/i18n.rs`, `app/web/locales/_keys.json`, and `app/web/locales/vi.json` so **Contacts** renders as **Danh bạ** and all 4 category labels, card buttons (`Thu hồi lời mời`, `Từ chối`, `Chấp nhận`), and empty states localize cleanly in Vietnamese & English.

## 5. Verification
- Backend unit/integration checks (`cargo test -p chatw --lib`) + frontend native & `wasm32` checks + release Web bundle build (`./app/build-web.sh release`).
- Live verification in real Google Chrome (CDP 1440×900, both Light & Dark themes, Vietnamese locale `vi`, real seeded friend requests sent & received + group/community invite) + screenshot visual audit + zero console errors.
- Mandatory `git add . && git commit && git push origin main` to `https://github.com/kiyogo/chart`.
