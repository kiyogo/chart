# Plan — Spec 163

## 1. Stronger AI Sharpening (`realesrgan_gpu.rs`, `ai_restorer.js`)
- Upgrade `GL_FINALIZE_FS` with dual-scale edge-aware luminance sharpening (1px Fine CAS + 2px Medium Micro-Clarity with controlled soft-halo envelope `[mnL - 0.06*edgeRange, mxL + 0.06*edgeRange]`) and boost `uColor1.z` strengths (`NomosUniSpanX2: 1.12`, `SPAN 4x: 1.22`, `AnimeVideoV3: 1.32`, `General: 1.18`).
- Upgrade `GL_FSR1_RCAS_FS` and `ai_restorer.js` Worker/CPU CAS with 2px micro-clarity + stronger default sharpness (`1.25` normal / `0.75` text/UI) and bump `PIPELINE_VERSION` to `'hybrid-span-gfpgan-v29-crisp'`.

## 2. Zero-Jitter Image Send & Silent Local-File -> Cache Handoff (`messages.rs`, `rich.rs`, `chat.rs`, `adaptive_surface.js`, `core_bridge.js`, `media_optimizer.js`, `media_worker.js`, `platform/mod.rs`)
- Stabilize message DOM key in `messages.rs`: `stable_key = msg.id.strip_prefix("local-").unwrap_or(&msg.id)` so `local-{uuid}` -> `{uuid}` NEVER unmounts/remounts `<RichImage>`.
- Make `PendingSenderMediaList` pixel-identical in geometry to normal `MessageRow` with ZERO spinner/status pill during normal upload (only show error UI on failure).
- Always pass `preserve_original = true` for fresh image uploads (`rich.rs`, `media_optimizer.js`, `media_worker.js`) so fresh uploads go directly to server uncompressed.
- Add `registerExistingBlobUrl` in `core_bridge.js` & `platform/mod.rs`; reuse the sender's exact already-decoded `preview_url` in both `blobPool` and `senderOriginalSession` BEFORE removing `PendingSenderMedia` and inserting the message in the same tick.
- In `persist_uploaded_media_bg`: save original bytes to persistent local cache (`put_local_object` + `sender_orig:{downloadUrl}`) and silently swap the cached URL into session caches after `predecodeImageUrl` without re-rendering or scrolling.
- Stop `adaptive_surface.js` `smooth` scroll fighting `ResizeObserver` instant pinning; snap cleanly without jitter and skip redundant scroll calls on in-place `local-*` ack updates in `chat.rs`.

## 3. Receiver Persistent Cache + 15-Day Cold Flag + Auto-AI Restore (`router.rs`, `rpc_bin.rs`, `core_bridge.js`, `recovery.rs`, `rich.rs`)
- Emit `X-ChatW-Cold-Compressed: 1` from `serve_media_reference` when `o.cold_compressed_at IS NOT NULL`, and tag RPC `content_type` with `;cold15d=1` in `rpc_bin.rs`.
- In `RichImage` (`rich.rs`) & `core_bridge.js`: check persistent local app cache (`OPFS`/`IDB`/`native CAS`) FIRST; on cache miss, fetch from server, persist downloaded bytes into local app cache (respecting device quota), and if `coldCompressed` (15-day cold-compressed) or compressed AVIF on cache miss / new device -> automatically trigger strong AI enhancement + cache the restored image.

## 4. Cross-Platform Adaptive Storage Quota (`native.rs`, `storage_engine.js`)
- Implement real `GetDiskFreeSpaceExW` on Windows (`#[cfg(windows)]`) in `app/src/storage/native.rs` alongside Unix `statvfs` (Linux/macOS/Android/iOS), plus `source_path` original-file fallback when a sender's cached blob was evicted.
- Fix `computeAdaptiveLimitBytes()` in `app/web/storage_engine.js` so Auto (`0 MB`) dynamically scales to `navigator.storage.estimate()` free quota (never `MAX_SAFE_INTEGER`), with `QuotaExceededError` auto-eviction retry.
