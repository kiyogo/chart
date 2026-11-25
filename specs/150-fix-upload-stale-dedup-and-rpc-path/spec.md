# Feature Specification: Stale Local-Dedup Recovery, RPC Path Validation, and Upload Error Diagnostics (Spec 150)

**Feature Branch**: `main`  
**Created**: 2026-11-25  
**Status**: In Progress

## Problem Statement

After Spec 149 fixed toast contrast and permission error mapping, authenticated friends (`123` <-> `12`) still hit `"Unable to upload file"` (`"Không tải tệp lên được"`) with 0 server messages/attachments written and 0 server warning logs. Investigation identified two compounding failure causes and an observability gap:

1. **Poisoned Cross-Session / Post-Reset Local Dedup Cache (`empty_payload_after_claim_miss`)**:
   - `app/web/media_optimizer.js` persists up to 256 authorized media fingerprints (`localDedupEntries`) in browser `localStorage` (`chatw_dedup_local_v1`).
   - Whenever a user selects or pastes media matching a cached entry (including across account switches in the same browser, after DB resets, or after server retention cleanup), `processSingleFile` returns `bytesB64: ""` with `copiedFromRefId: localHit.refId`.
   - `storage_sync::upload_or_claim_media_for_scope` attempts `POST /api/storage/dedup/claim`. When the server rejects or returns `claimed: false`, `payload.bytes` is still empty and upload aborts permanently with `Err("empty_payload_after_claim_miss")`, never evicting the stale cache entry or falling back to uploading the actual bytes.
2. **CWBF2 RPC Internal Path Length & Query Punctuation Rejections (`forbidden_rpc_path`)**:
   - `web/backend/src/websocket/rpc_bin.rs::is_safe_internal_path` enforces `path.len() <= 2048` and rejects `..` anywhere in the raw URL including query parameters (blocking filenames like `a..b.png` or uploads with `video_fingerprint_json` / long `orig_profile_json` query strings).
   - `rpc_bin.rs` only logged HTTP `>= 400` warnings for `/api/messages`, hiding `/api/attachments` and `/api/storage/` rejections from server logs.
3. **Opaque Fallback Error Message**:
   - `upload_error_message()` mapped every unrecognized error code to generic `"Unable to upload file"` without logging to console or surfacing the error code.

## Acceptance Criteria

1. **Automatic Stale Dedup Eviction & Transparent Re-Encode Fallback**:
   - `window.__chatwDedupLocal.removeMedia(refId)` removes a stale entry from both memory and `localStorage`.
   - `window.__chatwMediaOpt.reencodeFromPreviewUrl(previewUrl)` re-runs `processSingleFile` with `skipLocalDedup = true` for both file-picker uploads and clipboard-pasted images.
   - When `claim_dedup` on `copied_from_ref_id` fails or returns `claimed != true`, `storage_sync::upload_or_claim_media_for_scope` evicts `src_ref_id` and transparently re-encodes and uploads the media instead of failing with `empty_payload_after_claim_miss`.
2. **Safe & Generous CWBF2 RPC Path Validation + Logging**:
   - `is_safe_internal_path` checks path traversal (`..`, `\`) strictly against the route pathname (before `?`), keeps null-byte blocking everywhere, and allows query-bearing internal paths up to 8192 bytes.
   - `rpc_bin.rs` logs HTTP `>= 400` responses on `/api/attachments` and `/api/storage/` (in addition to `/api/messages`) at `WARN`.
   - `core_bridge.js` & `core_worker.js` allow sufficient timeout (`45000ms`) for large `/api/attachments` / `/api/storage/` RPCs.
3. **Diagnostic Upload Error Reporting**:
   - `upload_error_message()` logs raw error codes to `console.error`, maps specific client/server failure codes cleanly, and preserves localization on unknown non-empty error codes via `"Unable to send ({code})"`.
4. **End-to-End Verification**:
   - Chrome CDP verifies stale `localDedupEntries` upload recovery (pre-seeded bogus `refId` auto-evicts and uploads cleanly), normal image upload, clipboard paste upload, non-image file upload, and database persistence in Postgres with zero error toasts.
