# Feature Specification: Remove Faint Dark Borders & Blur Shadows App-Wide

**Feature Branch**: `main`  
**Created**: 2026-11-20  
**Status**: Approved  

## User Request
"ở toan bộ web xóa tất cả khung có viền mờ đen ,... đi nhé" (with screenshot showing `.feed-card` panels in `MeetView` rendered with faint dark outline/shadow on white background).

## Functional & Visual Requirements
- **FR-001**: Remove all faint dark/gray box-shadows (`box-shadow` with dark rgba or `--shadow-sm`/`--shadow-md`/`--shadow-lg`) from cards, containers, floating composers, segmented/tab buttons, active rail tiles, note bubbles, badges, studio panels/swatches, modals, lock screens, toasts, and popovers across the entire web UI.
- **FR-002**: Remove remaining hairline dark/subtle frame borders (`border: 1px solid ...`, `border-top: 1px solid ...`, `box-shadow: 0 0 0 1px var(--border-strong)`) from `.settings-select`, `.details-recovery-box`, `.header-server-search:focus-within`, and `.studio-tools`.
- **FR-003**: Preserve clear visual separation between cards/popovers/controls and their parent canvas strictly through Flat Minimalist tonal surface tokens (`--bg-secondary`, `--bg-surface`, `--bg-surface-hover`, `--bg-input`, `--bg-hover`) in both Light and Dark themes (e.g., `.meet-column` uses `var(--bg-secondary)` matching `.feed-column`, `.note-bubble` / floating popovers use `var(--bg-surface-hover)` / `var(--bg-secondary)` where sitting over `var(--bg-primary)` / `var(--bg-chat)` / `var(--bg-sidebar)`).
- **FR-004**: Preserve intentional non-dark UI indicators: modal dimming overlays (`--bg-overlay`), accent focus rings (`--focus-ring`), active accent borders/glows, avatar circular cutout rings (`var(--bg-primary)` / `var(--bg-sidebar)`), and existing `42x24` switch dimensions + canvas overlay scrollbar behavior.

## Success Criteria
- Zero `.feed-card`, `.meet-card`, `.composer-box`, `.auth-box`, `.note-bubble`, `.section-tab.active`, `.segmented button.active`, `.rail-item.active`, `.settings-select`, `.modal-dialog`, `.modal-content`, or `.picker-popover` displays any blurry dark border or dark drop-shadow (`computedStyle.boxShadow === "none"`, no dark hairline `border`).
- MeetView cards, FeedView cards, Chat composer, Notes strip, Settings & Storage selects, and floating menus remain cleanly readable and visually distinct in Light and Dark themes.
- `./app/build-web.sh` builds cleanly and live headless browser checks + screenshot inspection confirm zero dark outlines across screens.
