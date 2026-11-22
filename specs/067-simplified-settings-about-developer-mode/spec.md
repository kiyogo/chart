# Spec 067 — Simplified Settings Redesign, iOS/Android-Style About Screen & Conditional Developer Mode

## 1. Problem Statement
Currently, the Settings left sidebar displays **22 vertical buttons** (`Account`, `Privacy`, `Security`, `App lock`, `Devices`, `Storage`, `Sync`, `Backup`, `Cloud`, `Chat`, `Notifications`, `Auto reply`, `Appearance`, `Customize`, `Language`, `Library`, `Draw`, `Inventory`, `Showcase`, `Business`, `Extensions`, `Guide`). This creates heavy cognitive load for everyday users and mixes simple daily settings with complex technical controls (e.g., `Advanced data export`, raw E2EE JSON private key import/export, WebRTC multi-device recovery, plugin host extensions).

The user requested:
1. Redesign the entire Settings UI to be as easy to use, scannable, and self-explanatory as possible.
2. Consolidate the left sidebar into drastically fewer buttons while preserving **100% of every existing feature** (zero features removed).
3. Add an **About** (`Giới thiệu`) tab modeled cleanly after iOS / Android About screens (app identity, version, build number, platform, encryption details, diagnostics, guide) containing an explicit **`Developer mode`** (`Chế độ nhà phát triển`) toggle switch (plus Android-style 7-tap build number unlock).
4. Turning ON `Developer mode` immediately reveals a dedicated **`Developer`** (`Nhà phát triển`) tab in the left sidebar that consolidates **all advanced, technical, power-user, and complex features** (including `Advanced export` / `Xuất dữ liệu nâng cao`, raw `Encryption keys`, `Sync & recovery`, `Extensions`, and `Diagnostics`). Turning OFF `Developer mode` hides `Developer` cleanly.

## 2. Information Architecture (Zero Feature Loss)

### Left Sidebar — Only 8 Everyday Items (down from 22 = 64% reduction) + 1 Conditional `Developer` Item
1. **`Account`** (`Tài khoản`) — Sub-tabs: `Profile` (avatar, banner, edit profile, username/password) | `Devices` (trusted devices & sign out) | `Business` (public storefront & quick replies)
2. **`Privacy & security`** (`Quyền riêng tư & bảo mật`) — Sub-tabs: `Privacy` (friend requests, calls, group invites, discoverability, receipts, typing, blocked list) | `App lock` (device PIN lock) | `Security` (2FA, security log, live E2EE protection status badge)
3. **`Chat & notifications`** (`Trò chuyện & thông báo`) — Sub-tabs: `Chat` (chat behavior & paste dropdown) | `Notifications` (alerts & sound tone dropdown) | `Auto reply` (busy auto-reply schedule & message)
4. **`Appearance`** (`Giao diện`) — Sub-tabs: `Theme & look` (5 theme cards, accent swatches, font size, display density, bubble/avatar/width/font dropdowns, preview, wallpapers, transparency & glass blur) | `Personalize` (pin communities/groups, profile effects, avatar list animations, settings sync)
5. **`Language`** (`Ngôn ngữ`) — Direct 1-click access to all 197 world languages with official SVG flags & live search
6. **`Data & backup`** (`Dữ liệu & sao lưu`) — Sub-tabs: `Storage` (cache usage, limits, auto-download, original photos) | `Backup` (encrypted backup & restore) | `Cloud` (10 cloud providers with SVG logos)
7. **`Creative`** (`Sáng tạo`) — Sub-tabs: `Library` (stickers, emoji, GIFs) | `Draw` (animation & drawing studio) | `Inventory` (badges & items) | `Showcase` (live effects gallery)
8. **`About`** (`Giới thiệu`) — Sub-tabs: `Overview` (iOS/Android-style app card, version, interactive build number, platform, encryption spec, languages, license, copy diagnostics, replay tour, and prominent **`Developer mode`** toggle switch) | `Guide` (user guide, FAQ & shortcuts)
9. **`Developer`** (`Nhà phát triển`) — **Conditional (ONLY visible when `Developer mode` is ON)** — Sub-tabs:
   - `Advanced export` (`AdvancedSecurityExportSection` — export password, AES-256 ZIP, unencrypted ZIP gate)
   - `Encryption keys` (E2EE fingerprint, raw JSON key backup/export, manual key import)
   - `Sync & recovery` (`SyncRecoverySettingsPanel` — incremental delta sync, WebRTC peer recovery, Argon2id key vault)
   - `Extensions` (`BotsSettings` — sandboxed plugin platform & legacy plugin archive)
   - `Diagnostics` (live runtime inspector, copy diagnostic report, reset appearance, quick disable developer mode)

## 3. UI & Localization Requirements
- **Modern Flat Underline Sub-Tabs (`.settings-subtabs`)**: Sleek borderless underline sub-navigation bar (inspired by Linear / GitHub / Apple Web — NEVER clunky pill-track segmented boxes), compact 38px height, accent 2px bottom indicator, horizontal scroll on narrow mobile screens, full light/dark/translucent wallpaper & RTL parity.
- **Smart Settings Search**: Searching any keyword (English or Vietnamese, e.g., `"cloud"`, `"âm báo"`, `"xuất dữ liệu"`, `"khóa"`, `"vẽ"`, `"nhà phát triển"`) matches the exact parent tab AND automatically activates the target sub-tab on click/Enter.
- **Canonical English Source Code + 197-Language Parity**: All Rust strings stay in canonical English (`en`), and every newly introduced English UI string is added to `app/web/locales/_keys.json` and translated across all 197 language packs (`app/web/locales/{code}.json`), with authentic native Vietnamese (`vi`) translations.

## 4. Acceptance Criteria
- [ ] Default Settings sidebar displays **8** everyday items (down from 22), with **0** `Developer` tab when `developer_mode` is OFF.
- [ ] Every single feature from the original 22 tabs remains 100% accessible (0 features deleted).
- [ ] `About` displays an iOS/Android-style App Info & System Details card + `Developer mode` switch + Build number 7-tap unlock + Copy diagnostics + Guide.
- [ ] Toggling `Developer mode` ON persists to `Prefs` (`DEVICE_ONLY`) and immediately adds `Developer` as the 9th sidebar item containing `Advanced export`, `Encryption keys`, `Sync & recovery`, `Extensions`, and `Diagnostics`. Toggling OFF hides `Developer` and redirects cleanly back to `About`.
- [ ] Zero legacy `.segmented` / `.ent-segmented` elements; all new strings translate cleanly in Vietnamese (`vi`), Arabic (`ar` RTL), Chinese (`zh-CN`), and all 197 language packs (0 empty strings, 0 `...`).
- [ ] Rust unit tests, release WASM build, all Playwright E2E suites, and visual screenshot inspections pass 100%.
