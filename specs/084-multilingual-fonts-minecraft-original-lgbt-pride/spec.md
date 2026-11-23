# Spec 084 — Rich 17-Font Multilingual Library, Minecraft Font & Original Animated 7-Color LGBT Pride Font (197 Languages)

## Problem Statement
1. Settings → Appearance → Font currently offers only 5 generic font entries (`System`, `Rounded`, `Serif`, `Monospace`, `Readable`) relying on unbundled OS fonts that often fall back identically.
2. Users want many more expressive, genuinely self-hosted fonts — specifically including a **Minecraft** pixel typeface — with live localization so switching to any of the **197 supported languages** displays every font label and live preview in that exact language.
3. Users want a **brand-new original font designed from scratch named `LGBT`**:
   - Sculpted from scratch in an unmistakable celebratory LGBT Pride style (original Bézier heart-shaped tittles on `i`, `j`, Vietnamese `í ì ỉ ĩ ị`, Cyrillic `і`/`ї`/`ј`, `!`, `?`; buoyant friendly Pride geometry).
   - **7-color rainbow Pride typography** with a smooth, flowing animated color effect whenever **Reduce motion** (`Giảm chuyển động`) is OFF, automatically freezing to a gorgeous static 7-color Pride gradient whenever **Reduce motion** is ON.
   - Full seamless coverage across **all 197 languages** (Latin, Vietnamese, Cyrillic, Greek, Arabic/Persian/Urdu, Devanagari/Hindi, Thai, Hebrew, CJK, and all remaining Unicode scripts) with zero missing glyphs (`□□□`) and zero diacritic clipping.

## Acceptance Criteria
1. **17 distinct self-hosted fonts** selectable in Settings → Appearance → Font (`system`, `lgbt`, `minecraft`, `modern`, `rounded`, `geometric`, `humanist`, `expressive`, `serif`, `classic`, `slab`, `mono`, `pixel`, `futuristic`, `handwriting`, `calligraphy`, `dyslexic`), all persisted via `Prefs::font_family` and sanitized in `app/src/prefs.rs`.
2. **Original `ChatW LGBT` WOFF2 font** built cleanly via reproducible `scripts/build_chatw_lgbt_font.py` (with sculpted heart-tittles ♥ and multi-script `unicode-range` companion WOFF2 files in `app/public/fonts/`), rendering animated 7-color rainbow typography when `data-reduce-motion="false"` and static 7-color Pride typography (`animation-name: none` / frozen) when `data-reduce-motion="true"`.
3. **Authentic `Minecraft` font** (`Monocraft` SIL OFL WOFF2 Regular + Bold in `app/public/fonts/`) supporting Vietnamese, Latin Extended, Cyrillic, Greek, and 197-language fallback cleanly.
4. **Full 197-language localization**: All new font names + live font preview sentence added to `app/web/locales/_keys.json` and translated across all 197 packs in `app/web/locales/*.json`; `custom_pickers.js` translates options live and previews each font right inside the dropdown; live Font Preview card under the selector displays sample text in the active language.
5. Verified via Rust unit tests (`cargo test -p chatw-app --lib`), release WASM build, live Playwright E2E across multiple languages (`vi`, `en`, `ja`, `ar`, `ru`, `hi`, `th`) + `Reduce motion` OFF vs ON, and visual screenshot inspection before commit & push to `origin main`.
