# Spec 107 Tasks: Group Avatar, Cover Photo Banner & Rich Group Management Features

- [ ] Task 1: Update backend models (`web/backend/src/models.rs`) and routes (`web/backend/src/groups/mod.rs`) to include `banner_url` and implement `POST /api/groups/:group_id/avatar` and `POST /api/groups/:group_id/banner`.
- [ ] Task 2: Update frontend models (`app/src/models.rs`) and services (`app/src/services/groups.rs`) with group avatar/banner uploads and settings APIs.
- [ ] Task 3: Redesign `app/src/components/groups/group_info.rs` with cover photo banner, avatar upload overlay, name/desc edit, member search, and group permissions/settings.
- [ ] Task 4: Add CSS styles in `app/assets/main.css` for group cover banner, avatar overlay badge, edit controls, and settings toggles.
- [ ] Task 5: Build backend (`cargo check -p server` / `cargo build -p server`) and frontend WebAssembly bundle (`./app/build-web.sh release`).
- [ ] Task 6: Create and run automated Playwright E2E test `app/e2e/spec107-group-avatar-cover-and-settings.mjs` verifying all features.
- [ ] Task 7: Commit and push all changes to GitHub `origin main` following the mandatory auto-push rule.
