# Tasks: Floating Composer, Icons & Settings Polish

- [x] Task 1: Remove `.composer-hint` in `app/src/components/composer.rs` and float `.composer` transparently in `app/assets/main.css` with bottom message clearance
- [x] Task 2: Remove blue vertical bar on `.panel-resizer:hover::after` / `.is-dragging::after` and remove `text-transform: uppercase` on `.discord-role-header` & `.header-search-group` in `app/assets/main.css`
- [x] Task 3: Redesign `FeedThreadsIcon` in `app/src/components/icons.rs` into a clean duotone Threads mark
- [x] Task 4: Remove `-` / `&` and enforce sentence case in `web/backend/src/enterprise/mod.rs` + add migration `web/backend/migrations/0025_clean_enterprise_names.sql` + normalize display in `sidebar.rs`, `chat.rs`, and `server_members_panel.rs`
- [x] Task 5: Refactor `app/src/components/enterprise/settings.rs` and `app/src/components/server_settings.rs` to flatten Enterprise settings directly into left `.settings-nav`, remove top header logo bar, remove `&`/`-`, and shorten all labels cleanly
- [x] Task 6: Update `app/src/pages/chat.rs` & `app/src/components/navigation/sidebar.rs` so clicking Community Settings keeps `.nav-sidebar` visible on the left and swaps only the chat frame into Settings
- [x] Task 7: Run unit tests (`cargo test -p chatw-app --lib`, `cargo test -p chatw --lib`, `cargo test -p chatw --test integration_test`)
- [x] Task 8: Build release web bundle (`bash app/build-web.sh release`) and restart backend daemon
- [x] Task 9: Verify all 6 fixes visually and functionally in live browser
- [x] Task 10: Auto commit and push to GitHub `main` branch
