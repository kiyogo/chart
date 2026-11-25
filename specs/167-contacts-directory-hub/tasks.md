# Implementation Tasks: Spec 167 — Contacts Directory Hub ("Danh bạ")

- [x] T001 Audit backend friends/invites routes, migrations, i18n packs, and CSS theme tokens; write `specs/167-contacts-directory-hub/{spec.md,plan.md,tasks.md}`
- [x] T002 Add migration `0042_direct_group_server_invites.sql` and backend endpoints in `web/backend/src/friends/mod.rs` (sent requests, withdraw/decline, unfriend, direct group/community invites)
- [x] T003 Add frontend models (`app/src/models.rs`), API wrappers (`app/src/services/api.rs`), and direct invite persistence in `app/src/components/invite.rs`
- [x] T004 Implement `app/src/components/contacts_view.rs` (`ContactsTab`, `ContactsSidebarMenu`, `ContactsMainPanel`) and wire into `sidebar.rs`, `rail.rs`, `conversations.rs`, `chat.rs`
- [x] T005 Add 100% theme-synced `.contacts-*` CSS in `app/assets/main.css` and update `app/src/i18n.rs`, `app/web/locales/_keys.json`, and `app/web/locales/vi.json` (`Bạn bè` -> `Danh bạ` + all Contacts UI strings)
- [x] T006 Build backend + Web release bundle (`./app/build-web.sh release`), restart live daemon, test all 4 Contacts tabs & real withdraw/accept/invite flows in live Chrome (Vietnamese + Light/Dark themes), inspect screenshots
- [x] T007 Mark all tasks `[x]`, `git add .`, commit, and `git push origin main` to `https://github.com/kiyogo/chart`
