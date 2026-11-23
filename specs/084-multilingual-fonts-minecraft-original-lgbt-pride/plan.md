# Spec 084 — Technical Plan

## Architecture & Files
1. **Font Asset Pipeline (`app/public/fonts/` + `scripts/build_chatw_lgbt_font.py` + `scripts/fetch_open_fonts.py`)**:
   - Self-host compact `.woff2` files in `app/public/fonts/` (served under `/fonts/*.woff2`, automatically copied into `app/dist/fonts/` by `app/build-web.sh` and offline-cached by `app/web/sw.js`).
   - `scripts/fetch_open_fonts.py`: Downloads SIL OFL fonts (`Monocraft v4.2.1` Regular+Bold converted to compact WOFF2 + Fontsource WOFF2 subsets for `Plus Jakarta Sans`, `Nunito`, `Quicksand`, `Be Vietnam Pro`, `Bricolage Grotesque`, `Lora`, `Playfair Display`, `Roboto Slab`, `JetBrains Mono`, `VT323`, `Chakra Petch`, `Patrick Hand`, `Dancing Script`, `Lexend`, plus multi-script rounded Pride bases `Baloo 2`, `Baloo Bhaijaan 2`, `Fredoka`, `Mali`).
   - `scripts/build_chatw_lgbt_font.py`: Original `fontTools` vector font compiler creating `chatw-lgbt-latin-{500,700}.woff2`, `chatw-lgbt-cyrillic-{500,700}.woff2`, `chatw-lgbt-arabic-500.woff2`, `chatw-lgbt-devanagari-500.woff2`, `chatw-lgbt-thai-500.woff2`, `chatw-lgbt-hebrew-500.woff2` with bespoke sculpted Bézier heart-tittles (`♥`) on `i`, `j`, Vietnamese `í ì ỉ ĩ ị`, Cyrillic `і`/`ї`/`ј`, `!`, `?`, buoyant Pride geometry, and generous vertical metrics preventing clipped diacritics.
2. **CSS Typography & 7-Color LGBT Chromatic Engine (`app/assets/fonts.css` or `app/assets/main.css`)**:
   - Add `@font-face` rules with `font-display: swap` and exact `unicode-range` blocks in `app/assets/main.css`.
   - Define `html[data-font="..."]` custom `--font-family` stacks with 197-language Unicode fallbacks for all 17 fonts.
   - Define `html[data-font="lgbt"]` 7-color Pride gradient system:
     - High-contrast Light & Dark 7-color rainbow custom properties (`--lgbt-grad`).
     - Smooth `@keyframes cwLgbtPrismFlow` when `html[data-reduce-motion="false"]` (or unset).
     - Strict static freeze (`animation: none !important`) when `html[data-reduce-motion="true"]`.
     - Contrast-safe Pride pills on `.message-bubble.own` & `.btn-primary` (white text over flowing 7-color gradient), crisp `<input>`/`<textarea>` caret & selection, and live preview styling on `.cw-select-option[data-font-preview="..."]`.
3. **Rust Preferences & Settings UI**:
   - `app/src/prefs.rs`: Allow all 17 font IDs in `Prefs::sanitize` + add unit test verifying persistence & sanitization of `"lgbt"`, `"minecraft"`, and all 17 fonts.
   - `app/src/components/settings_more.rs`: Update `SelectMenu` to accept optional `font_picker: bool` (sets `data-font-picker="true"`), populate all 17 font options, and render a live language-aware **Font preview** card right beneath the selector.
4. **197-Language i18n & Live Picker Sync**:
   - `scripts/i18n_add_spec084_keys.py`: Append all new font names & preview strings to `app/web/locales/_keys.json` and translate across all 197 language packs (curated Vietnamese, preserving `"LGBT"` and `"Minecraft"`).
   - `app/web/i18n_runtime.js`: Bump IndexedDB static cache tag `v4` → `v5`.
   - `app/web/sw.js`: Bump `STATIC_CACHE_NAME` `chatw-static-v5` → `chatw-static-v6`.
   - `app/web/custom_pickers.js`: Live-translate dropdown labels via `window.__chatwI18n.translate` and tag font options with `data-font-preview="{opt.value}"`.
