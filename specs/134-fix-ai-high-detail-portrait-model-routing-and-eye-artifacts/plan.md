# Technical Plan: Spec 134 — Fix AI High-Detail Portrait Model Routing and Eye Artifacts

## Architecture & Design Decisions

### 1. Classification & Quality Profile Extraction (`crates/chatw-media-core/src/ai/realesrgan.rs`)
- In `extract_original_quality_profile`:
  - Priority 1: Check `is_human_or_photo`:
    ```rust
    let is_human_or_photo = matches!(
        analysis.class,
        ImageClass::Portrait
            | ImageClass::Photo
            | ImageClass::LowLightNoisy
            | ImageClass::HdrWideGamut
            | ImageClass::AlreadyHeavilyCompressed
    ) || analysis.skin_tone_ratio > 0.04;
    ```
    If `is_human_or_photo` is true:
    - If `analysis.class == ImageClass::Portrait || analysis.skin_tone_ratio > 0.04` -> `"portrait"`
    - Else -> `"photo"`
  - Priority 2: Genuine anime/illustrations:
    - `ImageClass::AnimeManga => "anime"`
    - `ImageClass::Meme | ImageClass::FlatGraphic | ImageClass::AlphaGraphic => "art"`
    - `ImageClass::CrispTextUi => if sat_m > 0.14 { "anime" } else { "text_ui" }`
    - Fallback heuristic ONLY for non-photos with sparse palette:
      `_ if analysis.palette_sparsity > 0.55 && (analysis.flat_pixel_ratio > 0.35 || outline_r >= 0.006) => "anime"`
    - Default fallback -> `"photo"`.

### 2. Model Selection (`select_realesrgan_model`)
- When `orig_profile` is provided:
  ```rust
  if matches!(qp.cls.as_str(), "portrait" | "photo") {
      return Some(RealEsrganModelKind::GeneralX4V3);
  }
  let is_anime = matches!(qp.cls.as_str(), "anime" | "art" | "meme" | "text_ui");
  if is_anime {
      return Some(RealEsrganModelKind::AnimeVideoV3);
  }
  return Some(RealEsrganModelKind::GeneralX4V3);
  ```
  Strictly remove `|| qp.outline_r >= 0.005` from the anime check so sharp edge details in photos never trigger cartoon anime models.
- When `orig_profile` is None:
  Keep strict photo/portrait check routing to `GeneralX4V3`.

### 3. Pipeline Version Bump (`app/web/ai_restorer.js`)
- Bump `PIPELINE_VERSION = 'rust-realesrgan-upscayl-v10'`.
- This ensures all clients automatically clear stale `AnimeVideoV3` renders of photos and regenerate them with `GeneralX4V3`.

### 4. Build & Verification
- Clean up test scripts in `crates/chatw-media-core/examples/`.
- Add unit tests in `chatw-media-core` verifying model routing for portrait, photo, and anime test inputs.
- Re-run `./app/build-web.sh`.
- Run Playwright E2E verification test.
