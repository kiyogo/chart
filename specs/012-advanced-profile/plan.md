# Plan 012
- Migration 0019: profile_layouts, profile_layout_presets, profile_showcase, profile_guestbook, user_achievements.
- Backend `profile_plus`: GET /api/profile-plus/{id} (layout, filtered sections, showcase, achievements, guestbook,
  mutual friends, shared communities), layout / presets / showcase / guestbook endpoints with validation.
- Client: `services/profile_plus.rs`, `components/profile_plus_ui.rs` (ProfileSections, LayoutEditor with a
  native drag-and-drop bridge), ProfileCard templates, profile editor "Bố cục" tab + preview as, `components/inventory.rs`.
