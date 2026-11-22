# Plan 062: Architecture & File Changes

## 1. Open-Source Architecture Comparison (`AGENTS.md` Rule #3)
- Evaluated `dioxus-i18n` (Project Fluent `.ftl`), `rust-i18n`, and lazy-loaded JSON locale packs (Telegram Web / Element Web / Signal Desktop architecture).
- Embedding 33 full locale dictionaries in Rust WASM bloats the binary and compiles slowly. Combining:
  1. Expanded zero-latency reactive core keys in `app/src/i18n.rs` (for instant Rust-native reactivity + desktop support), and
  2. External lazy-loaded UTF-8 JSON packs (`app/web/locales/{code}.json` served from `app/dist/locales/` + cached in memory) driven by a Dioxus-safe `WeakMap`-backed runtime (`app/web/i18n_runtime.js`)
  achieves **100% coverage of all 33 languages with near-zero WASM size increase**, instant cache hits, clean RTL support, and zero VDOM/user-content corruption.

## 2. Files to Create / Modify

### A. 33-Language i18n
1. **Create `app/web/locales/{code}.json` (32 non-Vietnamese locale files)**:
   - Generated cleanly via reproducible Python source modules containing genuine, native-quality translations for every one of the 32 target languages (`en`, `zh-CN`, `zh-TW`, `ja`, `ko`, `th`, `id`, `ms`, `fil`, `km`, `lo`, `hi`, `bn`, `ar`, `fa`, `he`, `ur`, `tr`, `ru`, `uk`, `pl`, `cs`, `de`, `fr`, `es`, `pt-BR`, `it`, `nl`, `sv`, `ro`, `hu`, `el`).
   - Each JSON pack contains `exact` (~350+ UI labels/headings/buttons/placeholders/tooltips across all screens) and `templates` (dynamic patterns for transparency %, blur px, dim %, dates, poll counts, sync version, typing/DM/dice/step labels).
2. **Create `app/web/i18n_runtime.js`**:
   - Loads active locale pack immediately on boot (from `localStorage.chatw_lang` or saved `chatw_prefs_*`), caches packs in memory, observes `#main` via `MutationObserver` (`childList`, `characterData`, `attributes` for `placeholder`, `aria-label`, `data-tip`, `title`).
   - Maintains `WeakMap` of original Vietnamese strings & last-written translations so switching `vi -> langA -> langB -> vi` and reactive Dioxus updates work cleanly without drift.
   - Strictly skips user content containers (`.message-bubble` outside `.settings-preview-chat`, `.chat-item-name`, `.chat-item-preview`, `.settings-profile-name`, `.settings-profile-sub`, `.account-name`, `.member-name`, `.result-name`, `.channel-row-name`, `.sidebar-server-name`, `.lang-tile`, `.fingerprint`, `code`, `pre`, `[data-no-i18n]`, `.plugin-frame`).
   - Exposes `window.__chatwI18n` (`setLanguage`, `getLanguage`, `translateString`, `loadedLocales`).
3. **Update `app/src/i18n.rs`**:
   - Expand core Rust keys to include Rail items (`nav.meet`, `nav.feed`, `nav.notifications`, `nav.download`, `nav.plugins`, `nav.my_profile`, `nav.open_settings`), persist `localStorage.chatw_lang`, call `window.__chatwI18n.setLanguage(code)`, and provide `init_from_storage()` helper so pre-login screens respect saved language.
4. **Update `app/src/components/navigation/rail.rs`**:
   - Wire `t("nav.meet")`, `t("nav.feed")`, `t("nav.notifications")`, `t("nav.download")`, `t("nav.plugins")` on rail buttons.
5. **Update `app/src/components/settings_view.rs`**:
   - Ensure Settings search also matches against `window.__chatwI18n` / current tab label and English/Vietnamese keywords seamlessly.
6. **Update `app/web/index.html` & `app/build-web.sh`**:
   - Include `<script src="/i18n_runtime.js"></script>` (with cache-busting `?v=${TS}` in `build-web.sh`) and copy `app/web/locales/` -> `app/dist/locales/`.

### B. Unified Accent-Synchronized Controls, Inputs & Scrim Removal
1. **Update `app/web/adaptive_surface.js`**:
   - Delete `needsScrim`, `data-local-scrim`, `--local-scrim-bg` completely.
   - Remove `.settings-body .btn-secondary`, `.settings-body .btn-ghost`, `.settings-body .segmented button` from `LOCAL_ROW_SELECTOR`.
   - Never apply `data-local-tone` to `.active` / `.selected` filled elements so their `--on-accent` text/icon contrast stays locked.
2. **Update `app/src/platform/mod.rs`**:
   - Remove `needsFramedControls` / `data-framed-controls="true"` per-button white box clutter (keep attribute `"false"` so legacy consumers never trigger ugly individual white boxes).
   - Remove washed-out `makeCleanTint(..., 0.24, 0.76)` `--accent-soft` override for busy photos; ensure `--on-accent` is always computed dynamically (white on saturated/dark accents, `#0F172A` on bright yellow/lime custom accents) for BOTH preset accents and custom accents.
   - Clean up `--adaptive-btn-*` and `--bg-input` / `--input-border` tokens so translucent controls & inputs stay crisp and unified across light & dark surfaces.
3. **Update `app/assets/main.css`**:
   - Delete all `[data-local-scrim="true"]` rules and `[data-framed-controls="true"]` segmented box rules.
   - Guard `[data-local-tone="dark"]` / `[data-local-tone="light"]` so active/selected pills and filled buttons (`.btn-secondary`, `.btn-ghost`, `.btn-avatar-upload`, `.btn-small-secondary`, `.segmented button`, `.active`, `.selected`) are NEVER inverted.
   - Style active pills (`.settings-nav-item.active`, `.segmented button.active`, `.lang-tile.active`, `.filter-chip.active`, `.section-tab.active`, `.media-tab.active`, `.picker-tab.active`, `.rail-item.active:not(.has-logo)`, `.chat-item.selected:not(.has-nameplate)`, `.channel-row.selected`) with solid `var(--accent)` + `var(--on-accent)` text & SVG icons.
   - Style `.theme-card.active` and `.wallpaper-tile.active` with `2px` `var(--accent)` inset ring + clean matched surface + high-contrast caption.
   - Style `.segmented` as ONE cohesive rounded pill track with clean transparent idle buttons (`0.00px` size stability) and `var(--accent)` active button.
   - Upgrade `.btn-secondary`, `.btn-ghost`, `.btn-avatar-upload`, `.btn-small-secondary`, and all inputs (`input`, `textarea`, `select`, `.search-field`, `.sidebar-search`, `.composer-box`) with unified refined radius, crisp light/dark opaque & translucent surfaces, matched icon/placeholder ink, and `var(--accent)` focus rings.
   - Widen `.lang-grid` (`minmax(12.25rem, 1fr)`) and ensure `.lang-tile .lang-native` displays full native language names with zero `...` ellipsis.
   - Add RTL polish rules (`html[dir="rtl"] .search-field svg`, `.search-field input`, `.settings-nav-item .chevron`).

## 3. Verification Plan
- Build release WASM + assets via `./app/build-web.sh release`.
- Run new comprehensive Playwright E2E suite `app/e2e/spec062-i18n-all-33-languages-and-unified-ui.mjs`:
  - Verify all 32 `/locales/{code}.json` packs fetch 200 OK, parse cleanly, have high coverage, zero empty strings, and zero `...`.
  - Verify all 33 language tiles in UI have zero text truncation (`scrollWidth <= clientWidth` for `Bahasa Indonesia`, `Português Brasil`, etc.).
  - Test live switching across diverse scripts (`en`, `ja`, `ko`, `zh-CN`, `th`, `ru`, `de`, `fr`, `ar` with `dir="rtl"`, `hi`, and back to `vi`) across Settings -> Language, Settings -> Appearance, Messages Sidebar, and Chat view, plus reload persistence!
  - Test over a high-contrast colorful anime-style test image at `100%` transparency:
    - ZERO `[data-local-scrim="true"]` elements anywhere in DOM.
    - `"Tải ảnh của bạn"` (`.btn-secondary`), `"Khôi phục giao diện mặc định"`, and all segmented buttons have real WCAG contrast ratio `>= 4.5:1` (text vs button fill).
    - Active nav tab, active segmented buttons, active rail icon, active theme ring, sliders, and swatches are 100% synchronized with `--accent` (`#2E7CF6` blue -> switch to `#8B5CF6` violet -> verify instant sync across all active controls!).
    - Capture full screenshots (opaque Light, 100% transparent busy wallpaper, English UI, Japanese UI, Arabic RTL UI, Language Grid) and visually inspect EVERY screenshot via `read_file`!
- Run regression E2E suites (`spec052`, `spec053`, `spec059`, `spec061`, `modal-overlay-check`).
- Commit & push to `origin main`.
