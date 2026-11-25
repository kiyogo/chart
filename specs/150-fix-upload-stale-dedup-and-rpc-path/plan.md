# Implementation Plan: Spec 150 — Upload Recovery, RPC Path Fix & Diagnostics

## Modified Files
1. `app/web/media_optimizer.js`
   - Add `removeLocalDedupEntry(refId)` and expose `window.__chatwDedupLocal.removeMedia(refId)` and `.clear()`.
   - Always stash `{ file, width, height, preserveOriginal, allowSenderOriginal: !needsSenderAi }` in `previewUrlToFile` for both image and non-image `localHit` results; guard `commitSenderOriginal` on `entry.allowSenderOriginal !== false`.
   - Add `skipLocalDedup = false` parameter to `processSingleFile` and expose `window.__chatwMediaOpt.reencodeFromPreviewUrl(previewUrl)`.
2. `app/src/storage/media_opt.rs`
   - Add `pub async fn reencode_from_preview_url(preview_url: &str) -> Result<OptimizedMediaPayload, String>`.
   - Add `pub fn evict_local_dedup_entry(ref_id: &str)`.
3. `app/src/services/storage_sync.rs`
   - On `copied_from_ref_id` claim miss (either `claimed != true` or `Err`), call `evict_local_dedup_entry(&src_ref_id)` and, if `payload.bytes.is_empty()` and `instant_preview_url` is present, call `reencode_from_preview_url(&preview_url)` and proceed with normal upload.
   - Emit `console::warn_1` with diagnostic details on claim miss fallback or remaining empty payload errors.
4. `app/src/pages/chat.rs`
   - Update `upload_error_message` to handle `"empty_payload_after_claim_miss"`, `"forbidden_rpc_path"`, `"storage_quota_exceeded"`, `"svg_not_allowed"`, `"network_error"`, `"worker_call_timeout"`, `"rpc_timeout"`, `"ws_handshake_timeout"`, `"ws_offline"`, and format unexpected non-empty codes via `"Unable to send ({code})"`.
   - Log exact upload failure reason to `web_sys::console::error_1` in upload failure handlers.
5. `web/backend/src/websocket/rpc_bin.rs`
   - Update `is_safe_internal_path` length cap (`8192`) and restrict `..` / `\` checks to the pathname segment (before `?`), keeping `\0` rejected everywhere, with unit tests covering query strings containing `..` and > 2KB metadata.
   - Log `tracing::warn!` for `status >= 400` on `/api/attachments`, `/api/storage/`, and `/api/dm/` alongside `/api/messages`.
6. `app/web/core_bridge.js` & `app/web/core_worker.js`
   - Use `45000ms` timeout for upload RPCs (`/api/attachments/` or `/api/storage/`) to avoid premature timeouts on larger encrypted payloads.
