# Implementation Tasks: Spec 168 — Fix Wallpaper, Banner & Background Image Uploads

- [x] T001 Diagnose all wallpaper/banner/background upload failure root causes and write `specs/168-fix-wallpaper-banner-media-uploads/{spec.md,plan.md,tasks.md}`
- [x] T002 Add migration `0043_library_media_registry.sql`, update `web/backend/src/library/mod.rs` (10 MB limit + `library_media` registration), and update `web/backend/src/attachments/mod.rs` (`handle_protected_media_file` serves registered `library_media` with auth & anti-direct-navigation checks)
- [x] T003 Upgrade client image processors & upload transport in `app/src/platform/mod.rs`, `app/src/platform/web.rs`, `app/web/core_bridge.js`, and `app/src/services/api.rs`
- [x] T004 Fix error handling & custom wallpaper preview in `app/src/components/settings_more.rs`, `app/src/components/chat_extras.rs`, `app/src/components/settings_view.rs`, and `app/src/components/group_manage.rs`
- [x] T005 Build backend + Web release bundle (`./app/build-web.sh release`), restart daemon, run backend tests, and run real Chrome E2E verification uploading App wallpaper, Chat wallpaper, and Profile banner
- [x] T006 Mark all tasks `[x]`, `git add .`, commit, and `git push origin main` to `https://github.com/kiyogo/chart`
