# Spec 091 — Increase Non-Chat Sidebar Item Gap by +1px (`1px` → `2px`)

## Problem Statement
In Settings, Server Settings, Communities, Channels, and Friends sidebars (`media_1795435350448_cba6408d.png`), adjacent items currently use `gap: 1px`. The user requested increasing vertical spacing on those non-chat sidebar lists by `+1px` more (`2px`), while keeping the Messages conversation list (`khung chat`, `.conv-list-scroll`) flush at `0px` and keeping `--panel-inset: 5px` + the restored `3px` centered scrollbar intact.

## Acceptance Criteria
1. **Exact `2px` Gap on Non-Chat Sidebars**: `.settings-nav`, `.server-settings-stage .settings-nav`, `.settings-nav-footer`, `.nav-sidebar .sidebar-scrollable` (Communities/Channels), `.nav-sidebar .friends-scroll` (Friends), and `.nav-sidebar .category-block` use **`gap: 2px !important`** (including `.settings-nav-item.settings-nav-dev` = `Nhà phát triển` uniformly at `2px`).
2. **Messages Conversation List Stays `0px` Flush**: `.nav-sidebar .sidebar-scrollable.conv-list-scroll` (and `:has(> .conv-row)`) keeps `gap: 0 !important`.
3. **5px Inset & Original 3px Centered Scrollbar Unchanged**: `--panel-inset: 5px` and `canvas_scrollbars.js` remain untouched.
4. **Verify & Auto-Push**: Update E2E check in `app/e2e/spec086-vibrant-lgbt-icons-and-tight-sidebars.mjs`, rebuild release bundle, run E2E, inspect zoomed screenshot, and commit + push to `origin main`.
