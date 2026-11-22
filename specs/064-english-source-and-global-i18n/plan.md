# Implementation Plan: Spec 064 — Canonical English Source & 197-Language Global i18n

## Architecture Overview
1. **Single Source of Truth = Canonical English in Code**:
   - All Rust UI code (`app/src/**/*.rs`), HTML (`app/web/index.html`), JS helpers (`app/web/custom_pickers.js`, `app/web/plugin_runtime.js`), and user-facing backend strings (`web/backend/src/**/*.rs`) are written exclusively in canonical English (`en`).
2. **Compact Indexed Locale Packs (`app/web/locales/`)**:
   - `_keys.json`: `{ "v": 2, "exact": [...], "templates": [...] }` — canonical English source strings & parameterized patterns.
   - `{code}.json` (197 files): `{ "v": 2, "lang": "<code>", "exact": [...], "templates": [...] }` — positional 1:1 arrays aligned with `_keys.json`, cutting total repo & network payload by ~60%.
   - `en` requires zero fetch at runtime (identity); switching to any of the 196 other languages fetches `_keys.json` (once, cached) + `/locales/{code}.json` (~55 KB) and applies instantaneously.
3. **Browser Auto-Translate Lockout**:
   - `<html lang="en" translate="no" class="notranslate">` + `<meta name="google" content="notranslate" />` + `translate="no"` on `#main` and `.lang-tile` stops Chrome/Edge Google Translate from corrupting UI translations.
4. **197 World Languages + Official Vector SVG Flags**:
   - Download official MIT-licensed 4×3 SVGs from `lipis/flag-icons` into `app/public/flags/{cc}.svg` (+ `globe.svg`).
   - Update `LanguageFlagLogo` in `app/src/components/brand_logos.rs` to render crisp 4×3 vector SVG flags for all 197 languages without bloating WASM.
   - Update `LanguageSettings` in `app/src/components/settings_more.rs` with a live search bar filtering native name, English name, and code.

## Execution Phases
- **Phase 1**: Extract 100% of UI strings (plain + interpolated) from `app/src/`, `app/web/`, and user-facing `web/backend/src/`; build verified `en <-> vi` master glossary preserving hand-curated core UI terms and original authentic Vietnamese.
- **Phase 2**: Rewrite source files (`app/src/**/*.rs`, `app/web/index.html`, `app/web/custom_pickers.js`, `app/web/plugin_runtime.js`, user-facing `web/backend/src/**/*.rs`) to 100% canonical English; refactor `app/src/i18n.rs`, `app/src/prefs.rs`, `app/src/components/brand_logos.rs`, and `app/src/components/settings_more.rs` for 197 world languages + SVG flags + search filter; verify zero remaining Vietnamese UI strings and green `cargo check` + `cargo test`.
- **Phase 3**: Download official 4×3 SVG flags into `app/public/flags/` and update `app/build-web.sh` (already copies `app/public/` and `app/web/locales/`).
- **Phase 4**: Generate & validate all 197 indexed `/locales/{code}.json` packs + `_keys.json` from canonical English with strict placeholder protection, brand/technical glossary preservation, authentic `vi.json`, and hand-curated core overrides for major languages.
- **Phase 5**: Rewrite `app/web/i18n_runtime.js` as a clean English-canonical runtime supporting indexed `_keys.json` + `{code}.json`, full template matching, `notranslate` enforcement, `document.title` localization, and instant lossless switching across all 197 languages.
- **Phase 6**: Rebuild backend (`cargo build`) + release WASM (`./app/build-web.sh release`), update & run all E2E test suites, visually verify multi-language screenshots (EN, VI, ZH-CN, HE RTL, AR RTL, JA, HI/SW), and commit + push to `origin main`.
