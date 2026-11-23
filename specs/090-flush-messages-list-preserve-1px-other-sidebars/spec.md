# Spec 090 — Flush 0px Gap Strictly for Messages Conversation List While Keeping Exact 1px Gap in Settings, Communities, Channels & Friends

## Problem Statement
Spec 088 set `gap: 1px` across all left sidebar lists, including the **Messages ("Tin nhắn") conversation list** (`.conv-row`). As shown in `media_1795434177863_5c0787bb.png`, the user wants conversation cards in **Messages ("khung chat")** to touch flush (`0px` gap, NO `1px` separation), while keeping **Settings, Server Settings, Communities, Channels, and Friends** at exact `1px` gap, and preserving the restored `3px` centered canvas scrollbar inside the `5px` equal left/right inset (`--panel-inset: 5px`).

## Acceptance Criteria
1. **Messages Conversation List (`Tin nhắn`)**: `.nav-sidebar .sidebar-scrollable.conv-list-scroll` (and `:has(> .conv-row)`) has **`gap: 0 !important`** so adjacent conversation rows (`.conv-row`) touch flush with **`0px`** vertical separation (zero `1px` white line between adjacent conversation pills).
2. **All Other Left Sidebars Unchanged**: Settings (`.settings-nav`), Server Settings (`.server-settings-stage .settings-nav`, `.settings-nav-footer`), Communities (`.nav-sidebar .sidebar-scrollable` without `.conv-row`), Channels (`.nav-sidebar .category-block`), and Friends (`.nav-sidebar .friends-scroll`) retain exact **`gap: 1px !important`**.
3. **5px Equal Inset & Original 3px Centered Scrollbar Preserved**: `--panel-inset: 5px` and the constant `3px` centered canvas scrollbar (`1px | 3px | 1px`) remain 100% intact everywhere.
4. **Build, E2E Verification & Auto-Push**: Update `app/e2e/spec086-vibrant-lgbt-icons-and-tight-sidebars.mjs` to assert `0px` gap in Messages AND `1px` gap in Settings/Friends/Communities, rebuild release bundle, inspect live screenshot, and commit + push to `origin main`.
