# Feature Specification: Fix Voice Status Bar Icon Button Sizing & Crushed SVG

**Feature Branch**: `035-voice-status-icon-btn-fix`
**Created**: 2026-11-21
**Status**: In Progress

## Problem Statement
In the sidebar active voice channel bar (`.voice-status-panel`), the mute button (when muted) and disconnect button (`Ngắt kết nối thoại`) used `class="icon-btn btn-danger"`. Because `.btn-danger` sets `padding: 0 var(--space-4)` (`30px` total horizontal padding) and `min-height: var(--control-height)` (`38px`) while `.icon-btn` fixes `width: var(--icon-btn)` (`32px`), the inner content width collapsed to `~2px`, crushing the `16×16` `PhoneOffIcon` and `MicOffIcon` SVGs into a microscopic red dot and stretching the button taller than the adjacent mic button.

## Acceptance Criteria
- **AC1**: `.icon-btn` always enforces `padding: 0; min-height: 0;` and `.icon-btn.danger` / `.icon-btn.btn-danger` explicitly preserves exact `var(--icon-btn) × var(--icon-btn)` geometry with transparent resting background, `var(--danger)` icon color, and `var(--bg-error)` hover background.
- **AC2**: Both buttons inside `.voice-status-panel` (`Chuyển trạng thái micro` in unmuted & muted states, and `Ngắt kết nối thoại`) render at identical width & height with full crisp `16×16` centered SVGs (`svgRect.width === 16`, `svgRect.height === 16`).
- **AC3**: Verified via Playwright E2E joining a live voice channel, inspecting computed dimensions and screenshot, and rebuilt for both Web and Linux Desktop.
