# Spec 149: Fix Toast/Sync/Modal UI Defects & Upload Stale-Session Error Handling

## 1. Problem Statement
Users reported `"lỗi giao diện + lỗi tải"` alongside screenshot evidence:
1. **UI Bug 1 — `.app-toast` Close Button Unreadable**: Global `.icon-btn:not(.danger):not(.btn-danger):not(.profile-close)` rules in `app/assets/main.css` (`9447`, `9522`) forced `color: var(--text-secondary) !important` (`rgb(91, 100, 117)`) and `background: var(--bg-hover) !important` onto `.app-toast .icon-btn`, producing a low-contrast grey box with a dark `×` on dark `#171c26` toast pills.
2. **UI Bug 2 — Full-Height Black `"Device sync"` Column**: `storage_ui.rs:1195` rendered `class: "toast"` directly inside `.app-layout` (`display: flex`), stretching `"Device sync"` into a full-height vertical black bar on the right edge of the viewport.
3. **UI Bug 3 — Overlapping Double Modals**: `WelcomeTour` (`chat.rs:5832`) and `E2eeUnlockRecoveryModal` (`chat.rs:5853`) rendered simultaneously at `--z-modal`, overlapping text on new-device logins.
4. **Upload Error Swallowing (`"Không tải tệp lên được"`)**: `send_dm_media`, `send_group_media`, and `send_channel_media` discarded `ApiError` / error codes (`unauthorized`, `not_friends`, `blocked`, `not_a_group_member`, `payload_too_large`), stranding stale/post-DB-reset browser tabs in offline-hydrated UI with generic `"Unable to upload file"` instead of clearing the expired session or reporting the exact reason.

## 2. Acceptance Criteria
1. `.app-toast .icon-btn` and its SVG icon match `var(--bg-elevated)` (`#fff` on dark pill) with subtle translucent pill hover and zero grey-box contrast loss.
2. `e2ee-auto-sync-pill` floats cleanly at bottom-center (`class: "app-toast"`), never altering `.app-layout` flex columns.
3. `WelcomeTour` defers while `locked()` or `E2EE_RECOVERY_REQUIRED` is active — zero modal collision.
4. Upload failures (`unauthorized`, `not_friends`, `blocked`, `not_a_group_member`, `payload_too_large`) surface localized i18n messages matching `en.json`/`vi.json`; `"unauthorized"` wipes stale client session and returns cleanly to Login.
5. Zero compiler warnings in `chatw-app` (`realesrgan_gpu.rs` unused imports removed).
