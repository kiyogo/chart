# Tasks: Remove Faint Dark Borders & Blur Shadows App-Wide

- [x] T001 Audit all `box-shadow` and `border` rules across `app/assets/theme.css`, `app/assets/main.css`, `web/admin/admin.css`, and `app/src/`
- [x] T002 Neutralize `--shadow-sm`, `--shadow-md`, `--shadow-lg` in `app/assets/theme.css` (`:root` and `[data-theme="dark"]`)
- [x] T003 Remove dark shadows and dark hairline borders from `app/assets/main.css` (`.feed-card`, `.meet-column`, `.composer-box`, `.note-bubble`, `.note-plus-badge`, `.studio-tools`, `.studio-swatch`, `.settings-select`, `.details-recovery-box`, `.header-server-search`, range slider thumbs, switch knob, `.voxel-cell.filled`, `.voice-controls-dock`, popovers & modals)
- [x] T004 Ensure tonal surface separation (`--bg-secondary` / `--bg-surface-hover`) where cards or popovers previously relied on shadows over white backgrounds
- [x] T005 Rebuild bundle (`./app/build-web.sh`), run `node app/e2e/scrollbar-switch-check.mjs`, verify MeetView / FeedView / Chat / Settings screenshots visually, and push to `origin main`
