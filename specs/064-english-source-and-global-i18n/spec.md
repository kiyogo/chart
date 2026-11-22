# Feature Specification: Canonical English Source Code & Complete 197-Language Global i18n (Spec 064)

**Feature Branch**: `main`
**Created**: 2026-11-22
**Status**: Active
**Input**: User reported browser auto-translate corrupting non-Vietnamese languages into garbled Vietnamese (e.g. "Thiên đường" for both Cloud & Chat, "Shabbat" for Language, "Ngoại cảnh" for Appearance), untranslated Vietnamese strings remaining across Account/Settings/Chat/etc., and requested:
1. All UI strings inside source code MUST ALWAYS be written in canonical English (`en`), and ALL other languages MUST be translated cleanly from canonical English with 100% coverage.
2. Support ALL languages in the world (comprehensive 197 living world languages covering every country, continent, and script, including all RTL languages).
3. Guarantee accurate, complete, uncorrupted translations with zero browser auto-translate interference.

## Root Causes Identified
1. **Browser Auto-Translate Corruption**: `app/web/index.html` lacked `<meta name="google" content="notranslate">` and `translate="no" class="notranslate"`. When users switched to `zh-CN`, `he`, `fa`, or `ar`, Chrome's built-in Google Translate auto-translated the page back into mangled Vietnamese (`Thiên đường`, `Ngoại cảnh`, `Shabbat`, `Thay đổi mặt`, `Cùng bước`) and even mangled native language names in `.lang-tile`.
2. **Vietnamese Hardcoded in Source Code**: 74 Rust files under `app/src/` (3,738 occurrences / ~2,900 unique strings), `app/web/index.html`, `app/web/custom_pickers.js`, `app/web/plugin_runtime.js`, and user-facing messages in `web/backend/src/` were hardcoded in Vietnamese instead of canonical English.
3. **Incomplete Coverage**: Previous locale packs only translated 305 strings and 16 templates, leaving ~2,600+ UI strings across Account, Privacy, Security, Devices, Storage, Backup, Cloud, Enterprise, Feed, Meet, Communities, Plugins, Profile, Calls, Draw Studio, Inventory, Guide, and Login untranslated.
4. **Limited Language Catalog**: Only 33 languages were supported instead of a comprehensive global catalog covering all living world languages, and `.lang-grid` lacked a search filter for navigating a global list.

## Requirements & Acceptance Criteria

### FR-1: Browser Auto-Translate Protection (`notranslate`)
- `app/web/index.html` sets `<html lang="en" translate="no" class="notranslate">` and `<meta name="google" content="notranslate" />`.
- `#main` and every `.lang-tile` carry `translate="no"` and `notranslate`.
- `i18n_runtime.js` and `app/src/i18n.rs` continuously enforce `translate="no"` and `notranslate` on `<html>` on every language change.

### FR-2: 100% Canonical English Source Code
- Every user-facing UI string in `app/src/**/*.rs`, `app/web/index.html`, `app/web/custom_pickers.js`, `app/web/plugin_runtime.js`, and user-facing strings in `web/backend/src/**/*.rs` is rewritten to clean, natural, idiomatic English (zero Vietnamese diacritics left in UI string literals; zero `...` ellipsis placeholders per `AGENTS.md` Rule #6).
- `app/web/index.html` title, meta description, OpenGraph, Twitter Card, and JSON-LD are written in canonical English.
- `app/src/i18n.rs` uses English (`en`) as index `0` canonical base language.

### FR-3: 197 World Languages Catalog & Official 4×3 SVG Flags
- `LANGUAGES` in `app/src/i18n.rs` defines all 197 verified world languages with `(code, native_name, english_name, flag_cc, rtl)`.
- Covers every UN nation's official language plus major regional, script, and indigenous living languages across all continents, including all 12 RTL languages (`ar`, `ar-EG`, `fa`, `he`, `ur`, `ps`, `ckb`, `sd`, `ug`, `yi`, `dv`, `bal`).
- Every language tile renders an official crisp 4×3 vector SVG flag (`app/public/flags/{cc}.svg` from MIT-licensed `lipis/flag-icons`, plus crisp globe SVG for international auxiliary languages like Esperanto) with zero grey text-code fallbacks and zero native-name truncation.
- `LanguageSettings` provides a live search box (`placeholder: "Search languages"`) filtering all 197 languages instantaneously by native name, English name, or BCP-47 code.

### FR-4: Indexed Canonical-English Locale Architecture (Zero Bloat, 100% Coverage)
- `app/web/locales/_keys.json` stores the single canonical English index (`exact` array of all UI strings + `templates` array of all parameterized patterns).
- Every language (`{code}.json`, 197 packs total) stores matching index-aligned arrays (`exact` + `templates`) with:
  - **100% parity**: `pack.exact.length === keys.exact.length` and `pack.templates.length === keys.templates.length`.
  - **100% authentic Vietnamese (`vi.json`)**: mapped directly from original authentic Vietnamese source strings + hand-curated core glossary (zero round-trip degradation).
  - **Hand-curated core overrides preserved** for major languages (`zh-CN`, `zh-TW`, `ja`, `ko`, `ar`, `fa`, `he`, `ur`, `fr`, `de`, `es`, `pt-BR`, `ru`, `uk`, `tr`, `pl`, `cs`, `it`, `nl`, `sv`, `ro`, `hu`, `el`, `th`, `id`, `ms`, `fil`, `km`, `lo`, `hi`, `bn`) merged with full English-to-target translations of all remaining strings.
  - **Strict validation**: zero empty values, zero `"..."`, 100% intact `{0}`, `{1}`, `{2}` placeholders, valid UTF-8 JSON.
- `app/web/i18n_runtime.js` operates purely from canonical English (`en`), lazy-loads `_keys.json` + `/locales/{code}.json`, handles instant lossless switching across all 197 languages (including RTL/LTR flipping), protects user-generated content (chat messages, usernames, fingerprints, code blocks, `.lang-tile`), and translates text nodes, dynamic templates, `document.title`, and `placeholder`/`aria-label`/`data-tip`/`title` attributes.

### FR-5: Verification & Zero Regressions
- Backend (`cargo build`) and Release WASM frontend (`./app/build-web.sh release`) build cleanly with zero errors.
- Automated source audit confirms ZERO Vietnamese UI string literals remain in `app/src/**/*.rs`.
- Automated locale validator confirms all 197 `/locales/{code}.json` packs pass 100% parity, placeholder integrity, non-empty, and zero-`...` checks.
- All live Playwright E2E suites pass 100%, including multi-script live switching across English, Vietnamese, Chinese, Japanese, Korean, Arabic RTL, Hebrew RTL, Persian RTL, Hindi, Russian, French, German, Spanish, Swahili, and Amharic, plus visual screenshot verification via `read_file`.
