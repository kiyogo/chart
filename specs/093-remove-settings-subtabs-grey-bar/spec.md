# Spec 093: Remove Horizontal Grey Border Line Under Settings Subtabs

## 1. Context & User Request
- **User Request**: "xóa cho tôi cái thanh xám đi nhé" ("Remove the grey bar/line for me").
- **Attachment**: `media_1795438166917_904fb5ce.png` showing Settings → Privacy & Security sub-tabs: `"Quyền riêng tư"`, `"Khóa ứng dụng"`, `"Bảo mật"`.
- **Problem**: In `.settings-subtabs`, a full-width bottom border (`border-bottom: 1px solid rgba(15, 23, 42, 0.09)`) runs across the entire settings stage underneath the sub-tabs. On translucent/wallpaper backgrounds, this appears as an intrusive, harsh grey divider cutting across the background image and separating the sub-tabs from the settings panel content ("Danh sách chặn", etc.).
- **Goal**:
  1. Eliminate the full-width horizontal grey border under `.settings-subtabs` across all themes and translucent wallpaper modes (`border-bottom: none !important`).
  2. Retain the active sub-tab underline indicator (`.settings-subtab.active::after`, 2px accent underline) cleanly under the selected sub-tab item.
  3. Ensure no visual or functional regressions in Settings sub-tab navigation or layout.

## 2. Acceptance Criteria
- [x] **Border Removal**: `.settings-subtabs` has `border-bottom: none !important;` in light, dark, and translucent wallpaper (`[data-panel-translucent="true"][data-app-wp]`) modes.
- [x] **Active Indicator Intact**: The active tab underline (`.settings-subtab.active::after`) remains visible and correctly positioned at the bottom of the active sub-tab.
- [x] **Clean Wallpaper Appearance**: When an anime or custom wallpaper is set with panel translucency, no horizontal grey line cuts across the settings stage.
- [x] **Build & Verification**: Web WASM compiles cleanly via `./app/build-web.sh release`. Playwright visual verification confirms the grey line is gone in Settings → Privacy & Security.
- [x] **Auto-Push**: Changes are committed and pushed to `origin main` with the commit hash reported.
