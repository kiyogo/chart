# Tasks: Spec 154

- [x] Fix anime misclassification over `skin_tone_ratio` in `crates/chatw-media-core/src/ai/realesrgan.rs`
- [x] Reduce `AnimeVideoV3` CAS sharpen strength in `app/src/storage/realesrgan_gpu.rs`
- [x] Disable Post-AI RL halos & redundant hardware Stage 1 flash in `app/web/ai_restorer.js`
- [x] Add synchronous `register_sender_original_sync` across `media_optimizer.js`, `media_opt.rs`, and `chat.rs`
- [x] Gate `is_live_incoming` on `!msg.is_self` in `app/src/components/messages.rs`
- [x] Fix `PendingSenderMediaList` spacer, `decoding: "sync"`, and 2-rAF synchronized overlay reveal in `app/src/components/rich.rs`
- [x] Run `cargo test -p chatw-media-core`, rebuild web/binary, verify in real Chrome WebGL2 + screenshots, commit & push
