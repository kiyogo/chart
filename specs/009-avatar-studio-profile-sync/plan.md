# Plan 009

## Database (migration 0015, additive only; rollback = drop the new columns / tables)
- users: avatar_motion JSONB, status_expires_at TIMESTAMPTZ, pronouns, location, website,
  occupation (TEXT with length checks), interests TEXT[], detail_visibility JSONB.
- user_settings(user_id PK, data JSONB ≤ 32 KB, version BIGINT, updated_at).
- contact_notes(owner_id, target_id, nickname ≤ 40, note ≤ 500, PK(owner, target), owner ≠ target).

## API (new module `profile_ext`)
- GET  /api/profile-ext/{user_id}: motion + details filtered by visibility (friends check, blocks hide all) + caller's nickname/note.
- PUT  /api/profile-ext/me: motion (validated struct), details, visibility, status expiry.
- GET  /api/settings-sync, PUT /api/settings-sync {data, base_version} → 409 + current on conflict; WS PrefsUpdated to own devices.
- GET  /api/contacts/notes, PUT /api/contacts/{id}/note.
- Background task (60 s): clear expired statuses and broadcast UserPresenceUpdate.
- Friends list carries avatar_motion + avatar_decoration. Banner upload sniffs type; GIF/WebP kept.

## Client
- `components/avatar_motion.rs`: effect catalogue (English names), `AvatarMotion` struct, CSS vars, editor.
- DecoratedAvatar renders motion (classes + CSS variables, trigger classes, click toggle).
- profile modal: "Avatar Studio" editor, details fields with visibility, status expiry.
- `services/profile_ext.rs`, `services/settings_sync.rs` (debounced push, pull on WS / start).
- Sticker picker Favorites + Recent tabs; sticker message menu; GIF favorites.
- Message action "Lưu vào Chính mình"; settings search box; Demo Gallery settings page; Help Center search.
