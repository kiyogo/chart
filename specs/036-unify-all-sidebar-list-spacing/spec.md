# Feature Specification: Unify All Sidebar List Spacing & Geometry (Messages, Friends, Communities)

**Feature Branch**: `036-unify-all-sidebar-list-spacing`
**Created**: 2026-11-21
**Status**: In Progress

## Problem Statement
While Messages (`.chat-item`) and Communities (`.community-item`, `.create-community-btn`) share unified `0.45rem` (`6.75px`) inset/padding and `0.3rem` (`4.5px`) vertical row gaps, the Friends tab (`.friends-scroll`, `.friend-row`, `.request-card`, `.add-friend-container`) still overrides these values with wider `var(--space-3)` (`11.25px`) insets, `var(--space-2)` (`7.5px`) gaps, and `var(--space-2) var(--space-3)` padding, making cards across sidebar tabs visibly inconsistent.

## Acceptance Criteria
- **AC1**: `.friends-scroll` uses the exact same `padding: 0 0.45rem var(--space-2)` (`6.75px` L/R) and `gap: 0.3rem` (`4.5px`) as `.sidebar-scrollable`.
- **AC2**: `.friends-scroll .chat-item.friend-row` and `.friends-scroll .request-card` use identical `padding: 0.45rem` (`6.75px`), `gap: 0.625rem`, `margin: 0`, and `var(--avatar-list)` avatars (`64px` card height), matching `.chat-item` and `.community-item` 100%.
- **AC3**: `.sidebar-search`, `.add-friend-container`, `.server-banner`, and `.voice-status-panel` all align flush at `0.45rem` (`6.75px`) left/right inset.
- **AC4**: Playwright E2E verifies 3-way sub-pixel parity across Messages, Friends, and Communities tabs.
