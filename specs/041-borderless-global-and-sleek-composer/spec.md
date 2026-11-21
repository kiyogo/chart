# Feature Specification — Spec 041: Global Borderless Cleanliness & Sleek Slim Chat Composer Overhaul

## Problem Statement
1. **Unwanted Heavy Focus Borders Across the App**: `--focus-ring: 0 0 0 3px color-mix(...)` and `outline: 2px solid var(--accent)` applied a thick colored border ring around buttons, inputs, links, and containers when focused.
2. **Prominent Blue Frame on Chat Composer**: When clicking or typing into the chat input, `.composer-box:focus-within` triggered `--focus-ring`, creating a thick 3px blue outline ring (as shown in user upload `media_1795256575468_64fe0077.png`).
3. **Clunky / Thick Composer Structure**: The composer used `border-radius: var(--radius-xl)` (slightly rounded box) with `2.5rem` square-ish button geometry, making the floating chat input bar feel bulky, boxy, and thick ("dày") rather than an ultra-sleek, aerodynamic capsule pill.
4. **User Direct Directive**: "ở toàn bộ web xóa tất cả khung viền hiện tại đi ở khung nhập chat thì xóa luôn khung viền và thay những phần còn lại bằng khung chat đẹp hơn nữa đi sát nét nữa ,không được dầy nữa".

## Acceptance Criteria
1. **Global Borderless UI**:
   - Set `--focus-ring: none;` in `app/assets/theme.css`.
   - Remove `outline: 2px solid var(--accent); outline-offset: -2px;` on `:focus-visible` elements in `app/assets/main.css`.
   - Ensure inputs, textareas, buttons, and switches have zero focus border strokes globally (`box-shadow: none !important; outline: none !important;`).
2. **Zero Border & Zero Ring on Chat Composer**:
   - `.composer-box`, `.composer-box:focus-within`, `.composer-box:focus`, and `.composer-input` have strictly `border: none !important; outline: none !important; box-shadow: none !important;` (no thick blue ring, no stroke).
3. **Sleek, Crisp, Slim Chat Composer Redesign**:
   - **Full Capsule Pill**: `border-radius: 9999px;` on `.composer-box`, `.composer-box::before`, `.composer-box .icon-btn`, and `.send-btn`.
   - **Slim, Refined Proportions**: Total outer height is slim (~38px - 40px), input min-height `2.125rem` (34px), inner padding `3px 4px 3px 6px`, input text padding `0.375rem 0.625rem; line-height: 1.35;`.
   - **Crisp High-Definition Surface**: Semi-translucent luminous white glass (`rgba(255, 255, 255, 0.94)` in light mode, `rgba(30, 41, 59, 0.92)` in dark mode) with `backdrop-filter: blur(20px)` and soft subtle micro-elevation (`0 1px 4px rgba(15, 23, 42, 0.08), 0 2px 12px rgba(15, 23, 42, 0.04)`), turning pure white on focus without any colored border ring.
   - **Circular Action Controls**: 34px circular icon buttons (`+`, emoji, GIF, sticker) with smooth hover backgrounds, and circular 34px send button.
4. **Verification**:
   - Playwright E2E suite `app/e2e/spec041-borderless-sleek-composer.mjs` verifying zero focus rings on composer and inputs, compact slim height (< 44px), capsule pill radius (9999px), and zero page errors.
   - Pass all existing regression suites (`spec040`, `spec038`, `spec030`, `borderless-shadowless-check`).
   - Rebuild Web WASM release bundle and Linux desktop release binary + `.deb`.
   - Git commit and auto-push to `origin/main` per AGENTS.md Rule #1.
