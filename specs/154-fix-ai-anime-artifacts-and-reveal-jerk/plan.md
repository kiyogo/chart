# Technical Plan: Spec 154

1. **`crates/chatw-media-core/src/ai/realesrgan.rs`**:
   - Evaluate flat/cel-shaded anime art BEFORE `skin_tone_ratio > 0.04` in both `extract_original_quality_profile` and `select_realesrgan_model_adaptive`.
2. **`app/src/storage/realesrgan_gpu.rs`**:
   - Lower `AnimeVideoV3` CAS `sharpenStrength` (`color1[2]`) from `1.0` to `0.10` so cel outlines remain clean without ringing halos.
3. **`app/web/ai_restorer.js`**:
   - Skip redundant Stage 1 intermediate `onStage1` URL swap when hardware WebGL2 neural SR is available (keep Stage 1 exclusively as fast fallback when WebGL2 neural SR is unavailable/fails).
   - Disable Post-AI Richardson-Lucy deconvolution (`upscaleFsr1`) for all anime models (`sel.isAnime`) and all neural SPAN/Anime models (`nomosuni_span_x2`, `nomosuni_span_x4`, `clearreality_span_x4`, `realesr_animevideov3`) and inputs `>= 360px`.
4. **`app/web/media_optimizer.js`, `app/src/storage/media_opt.rs`, `app/src/pages/chat.rs`**:
   - Add synchronous `registerSenderOriginalSync(previewUrl, downloadUrl)` called immediately before sending DM/Group/Channel media messages so `RichImage` hits `senderOriginalSession` on Frame 0.
5. **`app/src/components/messages.rs` & `app/src/components/rich.rs`**:
   - Gate `is_live_incoming` with `!msg.is_self`.
   - Add hidden `.message-time-standalone` height spacer to `PendingSenderMediaList`.
   - Use `decoding: "sync"` on `.rich-image-full` and `.lightbox-img`, remove Lightbox `"stage1"` intermediate rewrite, and await 2 animation frames after `"done"` `src` update before fading `.ai-gen-overlay`.
