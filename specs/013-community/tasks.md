# Tasks 013
- [x] T1 Migration 0020 and backend `community` module with permission checks
- [x] T2 Channel messages: community nickname / avatar, forum replies hidden, rules gate; audit hooks
- [x] T3 Welcome screen + rules (member) and editor (settings)
- [x] T4 Community profile dialog
- [x] T5 Community emoji / stickers (settings + pickers), Manage Expressions permission
- [x] T6 Forum view (topics, replies, pin, lock) and events (panel, event channels, RSVP)
- [x] T7 Audit log view
- [x] T8 Tests: backend unit, `app/e2e/community-check.mjs` 23/23, UI audit

## Found and fixed while testing
- Event channels could not be created: an old database CHECK only allowed text / voice / announcement / forum (migration 0021).
- Announcement channels never loaded their messages (only "text" channels did); both now load.
