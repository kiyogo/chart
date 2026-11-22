# Plan 063: File Changes & Verification

## Files to Modify
1. **`app/assets/main.css`**:
   - Replace solid `background: var(--accent)` + `color: var(--on-accent)` on active selection pills (`.settings-nav-item.active`, `.segmented button.active`, `.lang-tile.active`, `.filter-chip.active`, `.section-tab.active`, `.media-tab.active`, `.picker-tab.active`, `.rail-item.active:not(.has-logo)`, `.chat-item.selected:not(.has-nameplate)`, `.channel-row.selected`, `.cw-select-option.selected`, `.icon-btn.active`) with low-opacity translucent `var(--accent-soft)` (`14%` light / `22%` dark; hover `var(--accent-soft-strong)` `20%` light / `28%` dark) and surface-matched readable ink (`var(--text-primary)` / `#0F172A` light, `#F8FAFC` dark).
   - Update `html[data-rail-scrim="true"] .app-rail .rail-item.active:not(.has-logo)` to use low-opacity `var(--accent-soft)` instead of solid `var(--accent)`.
   - Soften `.segmented` track, `.btn-secondary` / `.btn-ghost` / `.btn-avatar-upload`, `.theme-card.active` / `.wallpaper-tile.active`, and translucent inputs to lighter, cleaner translucent fills while preserving crisp contrast and `0.00px` size stability.
   - Allow `[data-local-tone]` text/icon polarity to adapt transparent `.settings-nav-item.active`, `.rail-item.active`, and `.chat-item.selected` over split dark/light wallpapers (still excluding filled `.btn-secondary`, `.btn-ghost`, `.segmented`).
2. **`app/web/adaptive_surface.js`**:
   - Include `.settings-nav-item`, `.rail-item:not(.has-logo)`, `.chat-item:not(.has-nameplate)` (both idle and active since active pills are low-opacity translucent!) in `LOCAL_ROW_SELECTOR` so text stays crisp over both dark and bright halves of split wallpapers, while keeping `.btn-secondary`, `.btn-ghost`, and `.segmented button` excluded and keeping `data-local-scrim` deleted.
3. **`app/src/platform/mod.rs`**:
   - Ensure `--accent-soft` / `--accent-soft-strong` stay strictly low-opacity (`0.14–0.20` light, `0.22–0.30` dark) across all wallpaper transparency levels, and refine `--adaptive-btn-*` & translucent `--bg-input` to soft clean translucent levels (`~0.48–0.56`).
4. **Update E2E Tests**:
   - Update `app/e2e/spec062-i18n-all-33-languages-and-unified-ui.mjs` and `app/e2e/spec052-visual-and-contrast-audit.mjs` to assert **low-opacity translucent accent pills** (`0.11 <= alpha <= 0.32`, NOT solid `alpha === 1.0`) + exact accent RGB hue matching (Blue & Violet) + crisp ink contrast + zero patchy scrims.

## Verification
- Rebuild release bundle via `./app/build-web.sh release`.
- Run `spec062`, `spec052`, `spec053`, `spec059`, `spec061`, `modal-overlay-check` and visually inspect freshly rendered screenshots (opaque Light, English Appearance, 100% transparent busy wallpaper Blue & Violet, Messages view) via `read_file`.
- Commit & push to `origin main`.
