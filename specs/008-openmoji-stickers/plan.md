# Plan 008

## Data
- `tools/import_openmoji.py` reads the zip in memory (no extraction), verifies SHA-256,
  keeps standard emoji without skin tones plus single code point extras with a Unicode name.
- Names: CLDR short names from `/usr/share/ibus/dicts/emoji-en.dict`; keywords: English +
  Vietnamese CLDR annotations (`emoji-vi.dict`), folded like `composer::fold`.
- Category and order follow the client's own emoji picker (`emoji_data.rs`).
- SVG sanitizer: allow-listed SVG elements/attributes, `url(#id)` only, no text, no DOCTYPE;
  unreferenced ids, empty groups and attribute-less groups removed; output re-checked.
- Output: `app/public/stickers/openmoji/<HEX>.svg`, `catalog.json` (+ `.gz`), `LICENSE.md`.

## Server
- No schema change. Static `ServeDir` gets `precompressed_gzip()` (catalog 227 KB → 67 KB).
- `web/backend/build.rs` re-embeds migrations when they change (fixes a stale-build
  `VersionMissing` startup error found while testing).

## Client
- `services/openmoji.rs`: catalogue types, validated parse, cached load, search, suggest.
- `sticker_picker.rs`: OpenMoji tab, category chips, merged search (limit 120), credit line.
- `composer.rs`: OpenMoji suggestions after the pack-based ones.
- `guide.rs`: FAQ entry with credit. CSS for chips and credit.
- Sticker messages reuse `/stickers/...svg` bodies, already accepted by `sticker_url_of`.

## Tests
- Unit: catalogue parse drops bad hexcodes; folded search; exact-keyword suggestions.
- UI audit: tab, categories, credit, image load, Vietnamese search, send + receive, suggestion.
