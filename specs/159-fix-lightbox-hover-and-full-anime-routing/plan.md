# Technical Plan: Spec 159

1. **`app/assets/main.css`**:
   - Set `background: transparent !important; transform: none !important; transition: none;` across `.lightbox-tool-btn`, `.lightbox-enhance-toggle.is-active`, `.lightbox-download`, `.lightbox-close`, and `.lightbox-zoom-readout` on hover/active/focus-visible, keeping only clean icon tint (`#60a5fa` when active).
2. **`crates/chatw-media-core/src/ai/realesrgan.rs`**:
   - Extract `compute_dark_outline_ratio(rgba, width, height) -> f32` shared by `extract_original_quality_profile` and `select_realesrgan_model_adaptive`.
   - Classify illustrations with strong comic outlines + high color saturation (`outline_r >= 0.012 && sat >= 0.28`) as `"anime"` (`AnimeVideoV3`) before `skin_tone_ratio > 0.04` portrait matching.
3. **`app/web/ai_restorer.js`**:
   - Bump `PIPELINE_VERSION` from `'hybrid-span-gfpgan-v25'` to `'hybrid-span-gfpgan-v28'`.
4. Verify `cargo test -p chatw-media-core`, rebuild `./app/build-web.sh`, verify in Chrome via screenshot of hovered Lightbox button (confirm `backgroundColor === 'rgba(0, 0, 0, 0)'` & no white circle) + `luffy_full_banner.png` (`AnimeVideoV3` clarity), commit & push.
