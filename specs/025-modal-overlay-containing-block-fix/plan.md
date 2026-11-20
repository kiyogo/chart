# Technical Plan (025)

1. **`app/assets/main.css`**:
   - Refactor `.composer-context` and `.composer-box` to `position: relative; z-index: 0; background: transparent;` with translucent background + `backdrop-filter: blur(...)` moved onto `::before` (`position: absolute; inset: 0; border-radius: inherit; z-index: -1; pointer-events: none;`).
   - Update `.composer-box:focus-within::before` for focus background and `.chat-column.wp-image .chat-header::before` for wallpaper header blur.
   - Add `:has(...)` z-index elevation rules for popovers (`--z-popover`) and modals/lightboxes (`--z-modal`) across `.composer`, `.chat-header`, `.chat-column`, `.nav-sidebar`, `.details-panel`, `.partner-info-sidebar`, `.server-members-panel`, `.settings-nav`, and `.settings-body`.
2. **`app/src/components/plugins_view.rs`**:
   - Guard glass theme `backdrop-filter` selector with `:not(:has(.modal-overlay, .settings-modal-backdrop, .lightbox))` so modals inside panels remain full-viewport even under custom glass themes.
3. **Verification**:
   - Rebuild Web release bundle (`./app/build-web.sh release`).
   - Run automated Playwright verification (`app/e2e/modal-overlay-check.mjs`) asserting `1440×900` `.modal-overlay` / `.menu-dismiss` bounds, unclipped centered dialogs, and proper mobile `.picker-popover` placement.
   - Run full workspace cargo test suite.
