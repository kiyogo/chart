# Implementation Plan — Spec 039

1. **CSS Icon Clarity & Ink Contrast (`app/assets/main.css`)**:
   - Set `.cw-icon [fill-opacity] { fill-opacity: 0; transition: fill-opacity var(--dur-hover) var(--ease); }` so all inactive/neutral icons are 100% crystal-clear transparent inside.
   - Restore clean `fill-opacity: 0.16` ONLY on active/accented contexts (`.rail-brand`, `.rail-item.active`, `.settings-nav-item.active`, `.icon-btn.active`, `.send-btn:not(:disabled)`, `.btn-primary`, `.empty-state-icon`, `.self-chat-avatar`).
   - Deepen `.rail-item` ink color to `color-mix(in srgb, var(--text-primary) 82%, var(--text-secondary))` and `.icon-btn` to `color-mix(in srgb, var(--text-primary) 75%, var(--text-secondary))`.
   - Use vivid `var(--accent)` on `.rail-item.active` in light theme and `var(--accent-text)` in dark theme.
2. **Icon Geometry Cleanup (`app/src/components/icons.rs`)**:
   - Remove the opaque/tinted circle behind `FeedThreadsIcon` (`@`) so it is a pure crisp `@` symbol.
   - Refine `CompassIcon` needle so it uses `fill_opacity: "0.24"` (cleanly hidden when inactive, tinted when active) plus crisp stroke and `r="1.1"` center dot — zero muddy gray fill triangle.
   - Refine `SearchIcon` lens glint (`stroke_opacity: "0.45"` -> subtle crisp accent) and clean `stroke_width: "1.95"` balance on `Duo` for razor-sharp visual clarity at `26–28px`.
3. **Verification & Delivery**:
   - Extend `app/e2e/spec038-chat-bottom-icons.mjs` to assert inactive rail icons have computed `fillOpacity === '0'` and crisp high-contrast ink (no muddy gray), while active rail icons show luminous accent tint.
   - Rebuild Web WASM release, inspect Light & Dark screenshots visually, run all Playwright regression suites, rebuild & install Linux Desktop release + `.deb`, commit, and push to `origin/main`.
