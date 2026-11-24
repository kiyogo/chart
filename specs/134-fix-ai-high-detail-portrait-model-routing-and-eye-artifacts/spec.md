# Feature Specification: Spec 134 — Fix AI High-Detail Portrait Model Routing and Eye Artifacts

## 1. Problem Statement
Users reported critical image quality corruption when running AI enhancement on high-detail images and portraits:
- `"làm nét bằng AI nó vẫn lỗi quá nhiều ,... và lỗi ở những ảnh có chi tiết cao á"` (Sharpening with AI still has too many bugs,... and bugs in images that have high detail).
- Real human portraits, cosplay photos, and high-detail images exhibit horrifying, wrinkled, charred black trenches around the eyes, double eyelids, and tear troughs.
- The root cause is a classification inversion in `crates/chatw-media-core/src/ai/realesrgan.rs`:
  1. In `extract_original_quality_profile`, a broad wildcard guard `_ if analysis.flat_pixel_ratio > 0.34 || (analysis.flat_pixel_ratio > 0.20 && outline_r >= 0.0025) || outline_r >= 0.0045` was placed **before** `ImageClass::Portrait` and `ImageClass::Photo`. Any portrait with smooth skin / bokeh background (`flat_pixel_ratio > 0.20`) and dark hair/collar/eyelashes (`outline_r >= 0.0025`), or any photo with rich detail (`outline_r >= 0.0045`), was tagged as `"anime"`.
  2. In `select_realesrgan_model`, when `orig_profile` was present, the condition `let is_anime = matches!(qp.cls.as_str(), "anime" | "art" | "meme" | "text_ui") || qp.outline_r >= 0.005;` unconditionally selected `AnimeVideoV3` if `outline_r >= 0.005`, even for real portraits and photos.
  3. `AnimeVideoV3` is trained exclusively on 2D cartoon cel animations. It hallucinates thick, jagged black pencil/ink lines onto eyelid creases and skin gradients, causing grotesque "zombie eye" artifacts.
  4. Previously enhanced photos remain stuck in IndexedDB under pipeline cache version `v9`.

## 2. Goals & Acceptance Criteria
- [x] **Strict Semantic Model Routing**:
  - `ImageClass::Portrait` or `skin_tone_ratio > 0.04` MUST ALWAYS map to `cls = "portrait"` and select `RealEsrganModelKind::GeneralX4V3`.
  - `ImageClass::Photo`, `LowLightNoisy`, `HdrWideGamut`, `AlreadyHeavilyCompressed` MUST map to `cls = "photo"` and select `RealEsrganModelKind::GeneralX4V3`.
  - Only genuine illustrations, manga, memes, and flat 2D graphics with high palette sparsity (`palette_sparsity > 0.55`) may select `RealEsrganModelKind::AnimeVideoV3`.
  - Remove `|| qp.outline_r >= 0.005` from forcing `AnimeVideoV3` on high-detail photos.
- [x] **Cache Invalidation**:
  - Bump `PIPELINE_VERSION` in `app/web/ai_restorer.js` from `v9` to `v10` so all distorted cartoon outputs for real portraits are discarded and re-upscaled with `GeneralX4V3`.
- [x] **Pristine Eye & Detail Quality**:
  - High-detail human portraits and cosplay images produce natural, soft skin gradients and crisp iris/lash details with ZERO black eye craters, ZERO zombie wrinkles, and ZERO cel-shading hallucinations.
- [x] **Auto-Push (Rule 1)**:
  - Commit and push all changes immediately upon turn completion.
