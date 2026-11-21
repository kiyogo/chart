# Tasks — Spec 041: Global Borderless Cleanliness & Sleek Slim Chat Composer Overhaul

- [x] T001 Update `app/assets/theme.css` (`--focus-ring: none;`) and `app/assets/main.css` (remove global outline rings on `:focus-visible` and `box-shadow` rings on `input:focus`).
- [x] T002 Redesign `.composer-box`, `.composer-box::before`, `.composer-box:focus-within`, `.composer-box .icon-btn`, `.composer-input`, and `.send-btn` in `app/assets/main.css` for slim capsule pill geometry (`border-radius: 9999px;`), crisp luminous glass surface, and zero focus ring/border.
- [x] T003 Remove residual divider borders on `.plugin-task-footer` and `.feed-compose-expanded` in `app/assets/main.css`.
- [x] T004 Create Playwright test `app/e2e/spec041-borderless-sleek-composer.mjs` verifying borderless focus state, slim composer height, capsule pill radius, and run all regression suites.
- [x] T005 Rebuild Web WASM release bundle (`./app/build-web.sh release`) and verify live browser rendering with visual screenshot.
- [x] T006 Rebuild Linux Desktop release binary & Debian package (`./app/package-deb.sh`), mark tasks complete, commit, and auto-push to `origin/main`.
