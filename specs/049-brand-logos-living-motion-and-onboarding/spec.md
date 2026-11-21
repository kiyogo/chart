# Feature Specification — Spec 049: Brand Logos Suite, Living Motion System & Interactive Beginner Guide

## Context & Problem
After achieving clean visual symmetry, flat minimalism, and translucent glass parity (Specs 046–048), several screens still feel static or overly plain:
1. **Missing Official-Style Brand Logos**:
   - Cloud backup providers (`Google Drive`, `Microsoft OneDrive`, `Dropbox`, `Nextcloud`, `WebDAV`, `Amazon S3`, `Cloudflare R2`, `MinIO`, `NAS cá nhân`, `Apple iCloud`) show bare text with zero brand logos, and unconfigured OAuth providers display raw technical env-var jargon (`GOOGLE_DRIVE_CLIENT_ID (...)`).
   - Security log entries (`Chrome trên Linux`) and Device lists (`Thiết bị`, `Thiết bị đã đăng nhập`) show plain text without OS (`Linux`, `Windows`, `macOS`, `Android`, `iOS`) or browser (`Chrome`, `Firefox`, `Safari`, `Edge`, `Brave`, `Opera`) brand logos.
   - `Tải ứng dụng` uses a monochrome terminal icon for Linux instead of the iconic Linux Tux logo and lacks automatic OS detection highlighting.
   - `Tài khoản` lacks the Google brand logo on the Google sign-in row; `Ngôn ngữ` lacks crisp vector language flags.
2. **Low Visual Vitality & Static Interactions**:
   - Views, switches, buttons, empty states, messages, presence indicators, and illustrations feel static rather than physical and responsive (unlike modern motion-craft standards such as D3 Motion resources and Linear/Emil Kowalski spring physics).
3. **Beginner Guidance**:
   - New users landing on empty chats, Cloud settings, or Guide have limited interactive onboarding cues and no progressive beginner checklist.

## Goals & Acceptance Criteria
1. **Crisp Pure-SVG Brand Logo Suite (`app/src/components/brand_logos.rs`)**:
   - Multicolor SVG logos for all 10 cloud providers, 6 OSes (Linux Tux, Windows, macOS, Android, iOS, Web), 6+ browsers (Chrome, Firefox, Safari, Edge, Brave, Opera, ChatW Desktop), Google Auth, E2EE Shield, 2FA, Passkey, Encrypted ZIP, and language flags.
   - Wired into Cloud Backup (`backup_cloud_ui.rs`), Security Log (`accounts.rs`), Device Trust (`storage_ui.rs`), Logged-in Devices (`settings_extra.rs`), Account Sign-in (`settings_extra.rs`), Download Page (`download_view.rs` + `icons.rs`), and Language Settings (`settings_more.rs`).
   - Replace raw technical OAuth env-var copy on Cloud Backup with friendly plain-Vietnamese descriptions (zero `(...)`, zero jargon).
2. **ChatW Living Motion System (`app/assets/main.css` + `app/web/living_motion.js`)**:
   - Physics-tuned spring curves (`--spring-snappy`, `--spring-smooth`, `--spring-bounce`), strictly GPU-composited (`transform`/`opacity`), zero layout reflows, 100% disabled under `data-reduce-motion="true"` / `prefers-reduced-motion: reduce`.
   - Elastic spring toggles, tactile button press feedback (zero text color shift, zero bolding), subtle living hover micro-tilt on brand badges, breathing sonar pulse on online dots & live status pills, smooth staggered tab/panel entrances, organic message & reaction springs, animated SVG illustrations, animated QR scan beam on Download page, cursor-following translucent spotlight shimmer on interactive showcase/quick-start tiles, and celebration spark burst on onboarding completion.
3. **Living Empty States & Interactive Beginner Onboarding (`chat.rs`, `guide.rs`, `backup_cloud_ui.rs`)**:
   - Upgraded animated living orb hero + 1-click beginner quick-start cards in empty Messages/Communities states.
   - Interactive 5-step Beginner Checklist with animated progress bar & persisted completion inside `Hướng dẫn` (`GuideSettings`), plus upgraded animated `WelcomeTour` with smooth spring progress bar and completion spark celebration.
   - Friendly 3-step visual Cloud guide banner in `Đám mây` and auto-detected OS badge ("Gợi ý cho máy của bạn") in `Tải ứng dụng`.
4. **Zero Regressions**:
   - Preserves 100% of Spec 041–048 invariants: zero nested solid white boxes, unified translucent hover/active with zero text/icon color change and zero bolding, symmetrical 7.5px sidebar insets, zero `...` or `(...)` jargon, and full pass across Spec 048 + Spec 049 E2E tests.
