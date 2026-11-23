# Spec 090 — Technical Plan

1. `app/src/components/navigation/conversations.rs`:
   - Add class `conv-list-scroll` to the Messages conversation list container (`div { class: "sidebar-scrollable conv-list-scroll", ... }`).
2. `app/assets/main.css`:
   - Add `.nav-sidebar .sidebar-scrollable.conv-list-scroll, .nav-sidebar .sidebar-scrollable:has(> .conv-row) { gap: 0 !important; }` right below `.nav-sidebar .sidebar-scrollable, .nav-sidebar .friends-scroll { padding: 0 var(--panel-inset, 5px) 6px !important; gap: 1px !important; }`.
3. `app/e2e/spec086-vibrant-lgbt-icons-and-tight-sidebars.mjs`:
   - Assert Messages `.conv-list-scroll` has `gap === '0px'` (flush touching) while Settings, Friends & Communities keep `gap === '1px'`, and `5px` inset + `3px` centered scrollbar remain intact.
