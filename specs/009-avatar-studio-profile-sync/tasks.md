# Tasks 009 (phase 1 of the master prompt)
- [x] T1 Audit existing features (see spec.md "Audit")
- [x] T2 Migration 0015 (avatar_motion, details + visibility, status expiry, user_settings, contact_notes) and 0016 (name effects)
- [x] T3 Backend `profile_ext`: motion validation (40 effects, bounded numbers, deny unknown fields), details with server-side visibility, status expiry task, versioned settings sync (409 on stale writes, PIN fields stripped), contact notes; friends list carries motion
- [x] T4 Uploads: avatar must really be an image (magic bytes); GIF / WebP covers kept animated, real extension stored
- [x] T5 Client Avatar Motion Studio (40 effects, speed, intensity, colour, direction, easing, trigger, repeat) shown on profile card, details panel, conversation and friend lists; list budget (always / hover / off); reduced motion
- [x] T6 Profile editor: Avatar Studio tab, pronouns / occupation / location / website / interests with visibility, status expiry, animated cover
- [x] T7 16 name effects (was 6)
- [x] T8 Settings sync service (debounced, conflict retry, WS refresh) + favourites / recents (stickers, GIFs, emoji)
- [x] T9 Sticker message menu (favourite, send again, find similar); save any message to "Chính mình" (E2EE)
- [x] T10 Contact nickname + private note (details panel, used in lists)
- [x] T11 Settings search; Demo Gallery (desktop / mobile / compact / reduced-motion previews, real "Áp dụng"); Help Center search + 6 tutorials
- [x] T12 Tests: backend unit, client unit, API smoke (23 checks), UI audit 313/313; reports; push

## Found and fixed while testing
- Name effects beyond the original six failed with a 500: a DB CHECK constraint (migration 0016).
- Sticker menu was clipped by the message media box.
- Settings pushes were cancelled when the sticker picker closed (now `spawn_forever`).
- Backend build broke when SQLx guessed LEFT JOIN nullability differently (`"group_name?"`).
