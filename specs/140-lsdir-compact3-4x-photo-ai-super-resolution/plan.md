# Technical Plan: Spec 140 — `4xLSDIRCompact3` 4× Photo AI Super-Resolution

## 1. Model Asset (`app/public/models/`)
- Copy verified FP16 `lsdir_compact3_x4.cwsr` (1,242,872 bytes, `num_feat=64, num_conv=16, upscale=4`) into `app/public/models/lsdir_compact3_x4.cwsr`.
- Add `app/public/models/LICENSE-CC-BY-4.0-LSDIRCOMPACT3.txt` attributing Philip Hofmann (`Phhofm/models`, LSDIRCompact Version 3, CC-BY-4.0).

## 2. Core Rust Model Routing (`crates/chatw-media-core/src/ai/realesrgan.rs`)
- Add `RealEsrganModelKind::LsdirCompact3X4`:
  - `model_id()` -> `"lsdir_compact3_x4"`
  - `model_label()` -> `"LSDIR Photo Compact v3 (4×)"`
  - `cwsr_url()` -> `"/models/lsdir_compact3_x4.cwsr"`
  - `license()` -> `"CC-BY-4.0"`
  - `from_str_name()` -> matches `"lsdir_compact3_x4"` / `"lsdir"` / `"compact3"`.
- Update `select_realesrgan_model`:
  - Route ALL portraits, photos, low-light, HDR, compressed images, and general scenes to `RealEsrganModelKind::LsdirCompact3X4` (replacing `GeneralX4V3` as the primary photo model!).
  - Route pure flat anime/manga (`ImageClass::AnimeManga`, sparse palette flat graphics with zero human skin) to `RealEsrganModelKind::AnimeVideoV3`.
- Update existing unit tests in `realesrgan.rs` to assert portraits/photos route to `LsdirCompact3X4` and NEVER to `AnimeVideoV3`.

## 3. WASM Bridge (`app/src/storage/wasm_bridge.rs`)
- Whitelist `"lsdir_compact3_x4"` in `isModelLicenseAllowed`.

## 4. Browser AI Restorer (`app/web/ai_restorer.js`)
- Bump `PIPELINE_VERSION = 'lsdir-compact3-x4-v17'`.
- Add `lsdir_compact3_x4` to `APPROVED_MODELS` (`cwsrUrl: '/models/lsdir_compact3_x4.cwsr'`, `license: 'CC-BY-4.0'`, `scale: 4`).
- In `restoreImageFromUrl`:
  1. Keep `maxInEdge = hw.webgl2Available && !hw.webgl2Software ? 1080 : 960` so 4× output fits safely inside 4K (`<= 4096px`) and runs in ~250–650 ms on GPU!
  2. Call `wasm.selectRealEsrganModel(srcU8, procW, procH, origProfileJson, meta.modelId || '')` -> selects `lsdir_compact3_x4` for all photos/portraits/scenes.
  3. Load `lsdir_compact3_x4.cwsr` (1.18 MB, cached permanently in IndexedDB `ai_models`, pre-warmed on idle!).
  4. Execute `wasm.restoreImageRealEsrgan(srcU8, procW, procH, sel.modelId, modelBytes, origProfileJson, reportProgress)` on WebGL2 8-MRT FP16 GPU!
  5. If model download or GPU execution fails, automatically fall back to our Zero-Artifact WebGL2 AMD FSR 1.0 EASU + RL Deconvolution (`wasm.upscaleFsr1`)!
- In `app/web/sw.js`: bump `STATIC_CACHE_NAME = 'chatw-static-v17'`.

## 5. Verification (`app/e2e/spec140-lsdir-compact3-4x-photo.mjs`)
- Verify `512×384` crushed portrait -> `lsdir_compact3_x4` -> `2048×1536`, `SSIM >= 0.96`, zero blocks, natural eyes at 400% zoom!
- Verify `1024×768` pristine portrait -> `lsdir_compact3_x4` -> `3840×2880` (4K!), `SSIM >= 0.98`, crisp eyelashes & natural skin!
- Visually inspect zoomed face crops via `read_file`.
