# Tasks: Spec 052 — Unified Spacing, Adaptive Contrast, Official SVGs & Chat Header Polish

- [x] **T001**: Fetch official SVG vector paths for all Cloud providers, OSes, Browsers, Auth badges, and all 33 Language flags and update `app/src/components/brand_logos.rs`.
- [x] **T002**: Update `day_label` in `app/src/components/messages.rs` (current year -> `"20 tháng 11"`, other year -> `"22 tháng 11 năm 2025"`) and fix `.date-separator` overlap in `app/assets/main.css`.
- [x] **T003**: Move Self-Chat device pills inline to the right of the `"Chính mình"` header title without `"Thiết bị:"` in `app/src/pages/chat.rs`, float `.chat-header` over `.chat-messages`, and add top unblurred surface gradient fade in `app/assets/main.css`.
- [x] **T004**: Move `"Mặc định"` buttons to the left of `<input type="color">`, make `<input type="color">` 100% full-bleed with zero inner border, and frame all `.btn-ghost` buttons across the web app.
- [x] **T005**: Remove cloud guide banners and explanatory `.settings-sublabel` / `.settings-row-desc` across `backup_cloud_ui.rs`, `storage_ui.rs`, `settings_more.rs`, and `main.css`, and shorten all labels.
- [x] **T006**: Upgrade `CONTRAST_ENGINE_JS` in `app/src/platform/mod.rs` and adaptive halo rules in `app/assets/main.css`, and synchronize `.app-rail` opacity/blur with `.settings-nav` and `.settings-body`.
- [x] **T007**: Apply unified `8px` left/right/top insets across all panels, enlarge `.settings-group-title` to match `"Tin nhắn"`, add crisp `1px` hover/active borders on `.chat-item.has-nameplate`, enlarge `.chat-item` avatar, and restore `.meet-column` bottom gap & rounded corners.
- [x] **T008**: Build Web release bundle, verify all requirements with an automated Playwright E2E test suite (`app/e2e/spec052-visual-and-contrast-audit.mjs`), build Desktop release, and `git push origin main`.
