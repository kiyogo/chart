# Spec 063: Low-Opacity Translucent Active Controls & Soft Frosted Surfaces

## Problem Statement

After Spec 062 unified active states onto solid 100% `var(--accent)` fills, the user shared a close-up screenshot of the active `Giao diện` navigation item showing solid opaque `#2E7CF6` blue and requested:

> "giao diện phải hiện không được đậm full thứ nó phải hiện dưới % độ trong suốt thấp thôi chứ"

Active pills, tabs, rail icons, segmented selections, language tiles, and selected list rows must NOT render as heavy 100% solid opaque color blocks — they must use a **low-opacity translucent tint** (`14%–24%`) of the user's chosen `--accent` color with crisp, high-contrast ink matched to the underlying surface tone.

## Goals & Acceptance Criteria

1. **Low-Opacity Translucent Active Pills (`12%–26%` alpha, NEVER 100% solid)**:
   - `.settings-nav-item.active`, `.segmented button.active`, `.lang-tile.active`, `.filter-chip.active`, `.section-tab.active`, `.media-tab.active`, `.picker-tab.active`, `.rail-item.active:not(.has-logo)`, `.chat-item.selected:not(.has-nameplate)`, `.channel-row.selected`, `.cw-select-option.selected`, `.icon-btn.active`:
     - Light opaque / light translucent: `14%–20%` alpha of `var(--accent)` (`0.12 <= alpha <= 0.24`), crisp dark ink (`#0F172A` / `var(--text-primary)`).
     - Dark opaque / dark translucent: `22%–28%` alpha of `var(--accent)` (`0.18 <= alpha <= 0.32`), crisp light ink (`#F8FAFC`).
     - Per-position `data-local-tone` seamlessly adapts ink on transparent active rows (`.settings-nav-item.active`, `.rail-item.active`, `.chat-item.selected`) over split light/dark wallpapers while STILL keeping zero patchy `data-local-scrim` boxes.
2. **Soft, Airy Translucent Control Tracks, Secondary Buttons & Inputs**:
   - `.segmented` tracks, `.btn-secondary` / `.btn-ghost` / `.btn-avatar-upload`, and inputs (`input`, `textarea`, `select`, `.search-field`) use light, subtle low-opacity surfaces in opaque mode (`4%–7%`) and soft breathable translucent surfaces (`42%–58%`) over transparent wallpapers while keeping text-vs-surface contrast strictly legible (`>= 4.5:1`).
   - `.theme-card.active` and `.wallpaper-tile.active` use soft low-opacity accent tint (`12%–22%`) with a delicate `1.5px` translucent accent ring.
3. **Preserve 100% of Spec 060, 061 & 062 Guarantees**:
   - All 33 languages & RTL continue working cleanly.
   - Zero `data-local-scrim="true"` patchy white boxes on nav buttons.
   - Zero white-text-on-light-button bugs.
   - Changing `Màu nhấn` (preset or custom color) immediately re-tints all low-opacity active pills to match the chosen accent, and wallpapers NEVER override `--accent`.
