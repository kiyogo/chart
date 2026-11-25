# Spec 154: Fix AI Anime Artifacts & Post-Send / Post-Enhance Reveal Jump

## Problem Statement
1. **Degraded Anime Enhancement**: Cel-shaded anime illustrations with skin-toned characters (`skin_tone_ratio > 0.04`) are misclassified as `"portrait"` (`ImageClass::Portrait` path) and routed to `nomosuni_span_x4` + Post-AI Richardson-Lucy deconvolution (`postRlStrength = 0.55`). This creates visible wavy ripples, smeared flat areas, and ringing halos worse than the original.
2. **UI Jump/Jerk After Send & After AI Enhancement (`giật giật 1 cái`)**:
   - `messages.rs` sets `is_live_incoming: true` even when `msg.is_self == true`, while `persist_uploaded_media_bg` runs asynchronously in a detached task, causing the sender's newly uploaded image to miss `senderOriginalSession`, flash a `3% AI` blur overlay, and re-swap.
   - `PendingSenderMediaList` omits the `.message-time-standalone` vertical spacer, causing an ~18px row height jump on pending-to-sent swap.
   - `RichImage` and `Lightbox` mutate `src` on `"stage1"` and toggle `ai_revealing` / `lb_revealing` on the same frame as `src` replacement with `decoding="async"`, causing a 1-frame decode/paint flicker when AI finishes.

## Acceptance Criteria
1. Cel-shaded anime/manga art routes deterministically to `realesr_animevideov3` even when peach skin tones exceed `skin_tone_ratio > 0.04`, with gentle `0.10` CAS sharpening and zero Post-AI RL deconvolution halos.
2. Sender's newly sent images render immediately from synchronous `senderOriginalSession` with `is_sender_original = true`, matching pending height (`0px` vertical jump), and never trigger live-incoming AI blur overlay.
3. AI completion in both `RichImage` and `GlobalImageLightbox` uses `decoding="sync"` and waits 2 animation frames after `src` swap before fading `.ai-gen-overlay`, eliminating single-frame decode flicker.
