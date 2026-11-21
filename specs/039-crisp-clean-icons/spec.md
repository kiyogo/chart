# Feature Specification — Spec 039: Crisp Non-Muddy Icon Styling

## Problem Statement
After Spec 038 enlarged icons and increased internal `fill-opacity` to `0.22–0.34`, inactive navigation and toolbar icons rendered over `--bg-app` (`#E9EDF3`) with `--text-secondary` (`#5B6475`) looked muddy and grayish ("icon nó cứ bị xám xám á xấu quá"), especially inside hollow icons (`SearchIcon`, `VideoIcon`, `MessageIcon`, `CompassIcon`, `FeedThreadsIcon`, `BellIcon`, `DownloadIcon`, `SettingsIcon`).

## Acceptance Criteria
1. **Zero muddy gray interior fill on inactive/neutral icons**: All `.cw-icon` interior duotone fills default to `fill-opacity: 0` (100% transparent, crystal-clear crisp outline strokes), eliminating the cloudy gray blob look completely.
2. **Luminous accent duotone ONLY on active/brand surfaces**: Active navigation items (`.rail-item.active`, `.settings-nav-item.active`, `.icon-btn.active`), `.rail-brand`, `.send-btn:not(:disabled)`, `.btn-primary`, and `.self-chat-avatar` illuminate `.cw-icon [fill-opacity]` at a clean `0.16` accent tint.
3. **Cleaner icon geometry**:
   - Remove the background circle blob behind `FeedThreadsIcon` (`@`) so it renders as a pure crisp `@` glyph.
   - Refine `CompassIcon` so the needle has crisp hollow geometry when inactive (no solid gray half-triangle) with a tiny centered pivot dot and clean accent tint when active.
4. **Crisp high-contrast icon ink**: Upgrade `.rail-item` and `.icon-btn` from dull gray (`--text-secondary`) to rich crisp ink (`color-mix(in srgb, var(--text-primary) 82%, var(--text-secondary))` on rail and `75%` on toolbar buttons), with vivid `--accent` on light active rail and `--accent-text` on dark active rail.
5. **Visual & Regression Verification**: Verify Light & Dark screenshots in Chromium via Playwright, confirm zero regressions on `spec038`, `spec030`, `borderless-shadowless`, `scrollbar-switch`, `plugin-system`, rebuild Web & Linux Desktop + `.deb`, commit, and push to `origin/main`.
