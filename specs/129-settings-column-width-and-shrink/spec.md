# Spec 129: Settings Column Default Width & Minimum Width Synchronization with Messages Column

## 1. Problem Statement
The user reported:
> *"cột setting hiện thu nhỏ được bằng với tin nhắn đi chứ và nó đang bị lỗi hiện mặc định quá rộng rồi"*
(The settings column should be able to shrink down to the same width as the messages column, and currently it has a bug where it defaults to being way too wide!)

### Root Causes:
1. **Default Width Mismatch**:
   - The messages column (`.nav-sidebar`) defaults to `--sidebar-width: clamp(16rem, 20vw, 21rem)` (~256px at 1280px).
   - In contrast, the settings column (`.settings-nav`) had `--settings-nav-width: clamp(19rem, 23vw, 24rem)` in `theme.css` and fallback `20rem` (300px) in `main.css`, making it noticeably wider by default than the messages column.
2. **Artificial Minimum Width Blockers**:
   - `.settings-nav` and `.server-settings-stage .settings-nav` had a hardcoded `min-width: 19rem` (285px) in `main.css`.
   - `panel_resizer.js` had `minWidth: 240` and `fallbackWidth: 296` for `'settings-nav'`, whereas `sidebar` had `minWidth: 180` and `fallbackWidth: 272`.
   - As a result, when the user tried to drag the resizer to make the settings column compact like the messages column, it got stuck and could not shrink below 285px.

## 2. Requirements & Acceptance Criteria
1. **Default Width Synchronization**:
   - The settings column default width must match the messages column default width (`--settings-nav-width: var(--sidebar-width)`).
   - When switching between "Tin nhắn" and "Cài đặt", both columns align to the exact same visual width by default.
2. **Shrinkable to Same Minimum Width**:
   - Remove `min-width: 19rem` from `.settings-nav` and `.server-settings-stage .settings-nav` in `main.css` (set to `min-width: 0`).
   - Update `panel_resizer.js` for `'settings-nav'` to use `minWidth: 180` (identical to `sidebar`), `fallbackWidth: 272`, and `maxWidth: () => Math.min(500, Math.round(window.innerWidth * 0.45))`.
   - Users can freely resize the settings column down to 180px, exactly like the messages column.
3. **Stale Cache Cleanup in `boot.js`**:
   - In `boot.js`, ignore old stale hardcoded defaults (`294px` or `296px`) so the new synchronized default takes effect immediately.
4. **Verification**:
   - Playwright E2E test verifying:
     - Default width of `.settings-nav` equals `.nav-sidebar` width within 1px.
     - Resizing `.settings-nav` down to <= 200px (e.g. 185px) works smoothly without getting stuck.
     - Items within `.settings-nav` truncate cleanly with ellipsis without overflow or broken layout.
   - Auto-commit and git push to `origin main` (Rule 1).
