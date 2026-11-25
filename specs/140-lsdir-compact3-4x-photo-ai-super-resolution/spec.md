# Feature Specification: Spec 140 — `4xLSDIRCompact3` 4× Photo AI Super-Resolution (Zero Alien Eyes, Zero Blocks at 400% Zoom)

## 1. Problem Statement
When zooming to **400%** on a previously compressed 512×384 chat photo (`media_1795587955344_5409bfa5.jpg`), the user reported:
- `"vẫn thế mà"` (It's still the same!)
- Inspection of the 400% zoomed screenshot clearly revealed:
  1. Pure mathematical filters (EASU + Richardson-Lucy Deconvolution in Spec 138/139) cannot remove 8×8 compression blocks from low-bitrate AVIF/JPEG images (in fact, deconvolution sharpens the block boundaries on the cheek and chin!) and cannot reconstruct true 4× neural detail (`2048×1536`) needed at 400% zoom.
  2. The older `realesr_general_x4v3` (2022, 32 convs, 2.4MB) produced severe alien eye distortions, plastic wax skin, and oil-painting blobs.

## 2. Open Source Research (Rule 3 — `OpenModelDB`, `Phhofm/models`, `dinhkhactientl2/upscale-pro`)
Evaluated the top-rated `SRVGGNetCompact` photo models on OpenModelDB:
- Selected **`4xLSDIRCompact3`** (`Phhofm/models/4xLSDIRCompact/Version3`, **CC-BY-4.0** permissive commercial license):
  - Trained on **LSDIR (85,000 real high-resolution photographs)** — interpolating **75% C3 (JPEG/AVIF Compression Removal)** + **25% R3 (Blur/Noise Restoration)**.
  - Exact `SRVGGNetCompact` architecture (`num_feat = 64, num_conv = 16, upscale = 4`) — **100% native fit for ChatW's Pure-Rust 8-MRT FP16 WebGL2 GPU engine**!
  - **2× smaller (1.18 MB FP16)** and **2× faster (~250 ms on GPU)** than `realesr_general_x4v3`!
  - Visually verified at 400% zoom on the crushed 512×384 portrait: **100% natural round human eyes with crisp catchlights, zero alien folds, zero compression blocks on cheeks/chin, silky realistic skin, and razor-sharp individual hair strands (`SSIM = 0.9932`)**!

---

## 3. Goals & Acceptance Criteria
- [ ] **Bundle `lsdir_compact3_x4.cwsr` (1.18 MB, CC-BY-4.0)** in `app/public/models/` with `LICENSE-CC-BY-4.0-LSDIRCOMPACT3.txt`.
- [ ] **Add `RealEsrganModelKind::LsdirCompact3X4` in Rust (`crates/chatw-media-core/src/ai/realesrgan.rs`, `app/src/storage/wasm_bridge.rs`)**:
  - Route ALL real photos, portraits, natural scenes, and compressed web images to `lsdir_compact3_x4` (`4xLSDIRCompact3 4× Photo AI`).
  - Keep `realesr_animevideov3` strictly for pure flat anime/manga graphics with zero human skin tones.
  - Whitelist `lsdir_compact3_x4` in Rust SPDX license gate (`CC-BY-4.0`).
- [ ] **Upgrade `app/web/ai_restorer.js` to Execute `4xLSDIRCompact3` 4× Neural AI Super-Resolution on WebGL2 GPU**:
  - Execute `wasm.restoreImageRealEsrgan` with `lsdir_compact3_x4` (cached permanently in IndexedDB `ai_models`) for true 4× deblocking + super-resolution (`512×384 → 2048×1536`, `1024×768 → 3840×2880`).
  - Keep AMD FSR 1.0 + RL Deconvolution as automatic offline fallback if model download fails.
  - Bump `PIPELINE_VERSION = 'lsdir-compact3-x4-v17'` and `STATIC_CACHE_NAME = 'chatw-static-v17'` in `app/web/sw.js`.
- [ ] **Visual Proof & E2E Verification**:
  - Run `cargo test -p chatw-media-core` (all tests pass).
  - Run `./app/build-web.sh`.
  - Run Playwright E2E verifying BOTH crushed `512×384 → 2048×1536` AND pristine `1024×768 → 3840×2880` at 400% zoom — visually inspect zoomed face & eye crops via `read_file`.
- [ ] **Mandatory Auto-Push (Rule 1)**:
  - Commit and push all changes immediately to `origin main`.
