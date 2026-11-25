# Implementation Plan: Spec 151

1. `crates/chatw-media-core/src/avif.rs`: Parameterize `candidate_configs` by `single_thread`, use faster `ravif` speeds (`8`/`9` instead of `5`/`6`) on single-threaded encoders, and lower single-thread RD verify threshold (`400_000` pixels).
2. `app/src/storage/wasm_bridge.rs`: Route `"fast"` / `"balanced"` cleanly to `AvifCompressionMode::FastAdaptive`.
3. `app/web/media_optimizer.js`: Cache `nativeCanvasAvifSupport` once probed, yield a paint frame before WASM encoding, and select `'fast'` for images > `400_000` pixels.
4. `app/src/components/rich.rs`: Switch `PendingSenderMediaList` to `.message-row.sent`, wrap with `.message-content-wrapper`, set `z-index: 2` on badge, and render inline spinner while pending.
5. `app/src/pages/chat.rs`: Extract `persist_uploaded_media_bg` so DM/group/channel send dispatches immediately after upload; emit explicit toast on missing `user_keys` or `active_friend` in `send_dm_text`.
