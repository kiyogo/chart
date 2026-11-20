# Feature Specification: Modern Flat Minimalist Poll Card UI (026)

## Problem Statement
In `app/src/components/rich.rs` and `app/assets/main.css`, poll cards (`.rich-card.poll-card`) currently look cluttered and broken:
1. Inside `.poll-option.chosen`, `CheckIcon` renders on a separate vertical line above the option text because `.poll-label` lacks flex alignment while `svg` defaults to `display: block`.
2. `.poll-option.chosen` uses a thick `2px` inset border (`box-shadow: inset 0 0 0 2px var(--accent)`) which violates the Flat Minimalist design standard.
3. The outer card uses a cramped `15rem` min-width, screaming uppercase header `"BÌNH CHỌN"`, stark white inner rows against a heavy blue tint, and raw `0` / `1` counts without visual percentage clarity or radio/check indicator.

## Acceptance Criteria
1. `.poll-card` has balanced width (`clamp(17.5rem, 22rem, 100%)`), clean flat surface (`var(--bg-surface-hover)` with subtle `color-mix(in srgb, var(--accent) 7%, var(--bg-surface-hover))` on sent messages), sentence-case `"Bình chọn"` header with `BarChartIcon`, and crisp question typography.
2. Each `.poll-option` uses single-row horizontal flex alignment (`align-items: center`) with:
   - A `18px × 18px` circular radio/check badge (`.poll-check`) on the left (`1.5px` subtle ring when unselected; filled `var(--accent)` circle with centered white `CheckIcon` when selected).
   - `.poll-label` strictly inline on the same row (`display: flex; align-items: center; gap: 6px;`), never wrapping checkmarks onto a separate line.
   - Smooth `.poll-bar` progress background (`color-mix(in srgb, var(--accent) 11%, transparent)`, slightly richer `18%` for the selected choice) with zero thick `2px` borders.
   - `.poll-count` showing `{pct}%` cleanly alongside `{count}` (when votes > 0) or muted `{count}`, right-aligned with `tabular-nums`.
3. Verified visually on desktop & mobile via Playwright screenshot and cargo test suite.
