# Implementation Tasks — Spec 049

- [x] **T001: Pure-SVG Brand Logo Library (`app/src/components/brand_logos.rs` + `mod.rs` + `icons.rs`)**
  - Create multicolor pure-SVG brand logos for 10 Cloud providers, 6 OSes (Linux Tux, Windows, macOS, Android, iOS, Web), 7 Browsers/Clients (Chrome, Firefox, Safari, Edge, Brave, Opera, ChatW Desktop), Google Auth, E2EE Shield, 2FA, Passkey, Encrypted ZIP, Security Events, and Language Flags.
  - Upgrade `LinuxLogoIcon` in `icons.rs` to render multicolor Linux Tux.

- [x] **T002: Integrate Brand Logos & Plain-Vietnamese Cloud Copy**
  - `backup_cloud_ui.rs`: Add `CloudProviderLogo` to all providers, connected accounts, and setup form; add 3-step visual Cloud beginner guide; replace raw OAuth env-var jargon with clear friendly descriptions.
  - `accounts.rs`: Add `DeviceBrandPair` (Browser + OS logos) and `SecurityEventIcon` to every Security Log entry + `TwoFactorLogo` on 2FA row.
  - `storage_ui.rs` & `settings_extra.rs`: Add `DeviceBrandPair` to `DeviceTrustPanel` & `DevicesDialog` with live current-device pulse; add `GoogleAuthLogo` to `AccountSecurity`.
  - `settings_more.rs`: Add `LanguageFlagLogo` to `LanguageSettings`.
  - `download_view.rs`: Add auto-OS detection highlight pill + animated QR scan line.

- [x] **T003: Living Empty States, Animated Illustrations & Interactive Beginner Guide**
  - `guide.rs`: Animate `Illustration` SVG layers; upgrade `WelcomeTour` with spring progress bar + completion spark celebration; add interactive 5-step persisted Beginner Checklist to `GuideSettings`.
  - `chat.rs`: Upgrade Messages/Communities `.empty-state` with D3Motion-inspired Living Luminous Orb (`cw-living-orb`) + 4 interactive beginner quick-start cards linking directly to Add Friend, Create Group, Appearance, and Guide.

- [x] **T004: ChatW Living Motion System (`app/web/living_motion.js`, `app/assets/theme.css`, `app/assets/main.css`, `app/web/index.html`, `app/build-web.sh`, `app/src/platform/mod.rs`)**
  - Implement lightweight pointer spotlight (`--mx`, `--my`) and celebration spark burst (`window.cwSparkBurst`) in `living_motion.js` (loaded cleanly on both Web & Desktop).
  - Add spring physics tokens, elastic switch transitions, living brand badge micro-motion, sonar pulse online dots, staggered panel entrances, message arrival spring, and living orb animations in CSS — preserving 100% of Spec 047/048 zero-white-box, zero-color-shift, and zero-bolding rules, plus full `reduce-motion` support.

- [x] **T005: Build Web, Run Spec 049 & Spec 048 E2E Tests, Visual Audit, Build Desktop + `.deb`, Commit & Push**
  - Compile & build Web release, run `app/e2e/spec049-brand-logos-living-motion.mjs` and `spec048`, inspect every screenshot with `read_file`, fix any visual polish items, build Desktop release + `.deb`, commit, and push to `origin/main`.
