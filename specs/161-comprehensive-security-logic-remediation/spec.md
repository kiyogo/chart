# Spec 161: Comprehensive Security, Logic, Concurrency, Media & Compliance Remediation

## Problem Statement
Following a full-codebase audit (`full_project_audit.md`) across all 5 workspace crates, 37 PostgreSQL migrations, Dioxus frontend state machines, JS bridges/workers, and test suites at commit `48cfdc7`, multiple Critical/High security vulnerabilities, race conditions, failing integration tests, client state desyncs, native desktop regressions, media algorithm edge-case panics, missing DB indexes, and `AGENTS.md` compliance issues were identified.

## Goals & Scope
Remediate all verified P0 (Critical), P1 (High), and P2/P3 (Medium/Compliance) defects without breaking existing E2EE contracts or Spec 160 zero-friction password sync:
1. **P0 Security & Auth/ACL**:
   - Bind OAuth ticket exchange (`POST /api/auth/oauth-ticket` / `GET /api/auth/oauth-ticket/{id}`) with a cryptographic claim secret (`ticket_secret`) so attackers cannot hijack victim Google logins via Login CSRF; restrict `"local"` port fallback redirect.
   - Require ECDSA curve validation and cryptographic proof-of-possession against registered `device_keys` before promoting new/unverified devices (`POST /api/devices/register`, `POST /api/devices/recover-lost`, `POST /api/keys` `allow_rotation: true`) while preserving Spec 160 password-unwrapped recovery proofs.
   - Lock down Meet/SFU (`join_meet`, `accept_call`, `sfu_join`, `relay_signal`, `end_call`) and WebRTC voice channels (`handle_webrtc_signal`, `handle_join_voice_channel`, `handle_get_voice_participants`, `handle_update_voice_state`) to active authorized participants and respect `is_private` + `ADMINISTRATOR` checks.
   - Enforce ACLs and tenant scoping across `/api/sync/delta`, `enterprise/work.rs` (`source_message_id`, FKs), `/uploads/media/{file_name}` fallback, `restore_backup_archive` (require `manifest.json` + `owner_id`), `relay_ref_id` recovery, `/api/storage/{cleanup,migration/run,gc/incomplete,metrics}` (admin-only), `community` private forums, `groups` private channels, `messages` pinned DMs, `feed` reposts (`are_friends`/blocks/suspended), and `stickers::get_pack`.
   - Enforce strict SSRF validation across `vico`, `enterprise`, and `cloud/providers` (block loopback/RFC1918/link-local/CGNAT, normalize IPv6 brackets, disable untrusted HTTP redirects); enforce exact `scope_id` matching for `e2ee.egress_external`/`e2ee.store_external` in both backend and `plugin_runtime.js`; block inline `text/html` / active-content XSS in non-E2EE uploads.
   - Reject unauthenticated 0-chunk `CWCH2`/`CWCH3` ciphertexts, add context binding to `seal_authenticated_manifest`, eliminate plaintext/local-kwrap bypasses and enforce anti-replay on client DMs (`app/src/crypto/e2ee.rs`), and isolate per-account local caches cleanly in `storage_engine.js` / `media_optimizer.js`.
2. **P0 Failing Tests & Cold-Compression Dedup Fix**:
   - Update `compress_inactive_images_15d` so cold-compressed `media_objects` update/dissociate `cas_scope` and never cause newly uploaded active originals to deduplicate onto 15d cold-compressed AVIF rows.
   - Align `storage_sync_test.rs` (`perceptual_pdq_mih_match`, admin setup for Scenario 24–25) and add regression tests verifying security fixes.
3. **P1 Auth, Enterprise Concurrency, Client State, Native Desktop, WebGL2 & Media Core**:
   - Offload Argon2id to `tokio::task::spawn_blocking` + add login rate limiting and uniform timing; enforce `guard_attempts` and atomic single-use TOTP/backup codes; fix `accounts_add` limit & WebSocket termination on revocation/suspension.
   - Fix `enterprise` leave/approval/attendance races, cross-midnight shift check-out, `export_all_json` HR sensitivity, `enterprise_reports` date filters, dept cleanup, and realistic `ai_summarize` metrics; fix `servers` invite `max_uses` race, duplicate `@everyone` roles, `groups` avatar/banner 10 MiB body limit, `friends` re-request after unfriend, `stickers`/`business` validation, and `plugins/webhooks` UTF-8 safe truncation.
   - Fix `pages/chat.rs` per-channel IDB keys and async race guards, `sync_manager.rs` group/channel cursor handling, offline `CACHED_AUTH_USER_ID`, `settings_sync` 409 merge, WebRTC track cleanup & `call_id` matching, Native Desktop E2EE image decryption & `app_origin()` & `<a download>`/`img` bridge, WebGL2 `GL_ENGINE` async queue serialization, `chatw-media-core` `checked_mul`/bounds/odd-width AV1 & animated/FLAC memory bounds, JS/CSS injection sanitization, and client cold compression worker reuse + EXIF stripping.
4. **P2/P3 DB Indexes, Workers, UI & Compliance**:
   - Add migration `0038_security_indexes_and_constraints.sql` for missing FK/GC/query indexes and cascade consistency.
   - Fix `messages.rs` virtualized load-more, `sw.js` suffix ranges, `core_bridge.js` DOM-safe `blob:` eviction, listener leaks (`StoriesNotesBar`, `feed_view`, `idle_watch`), UI defects (`details_panel`, `incoming_call`), `group_info.rs` `"..."` strings, `STATIC_DIR` priority (`app/dist`), SEO/Cache-Control headers & `/llms.txt`.

## Acceptance Criteria
- `cargo test -p chatw-media-core -p kiyo-kdata -p chatw --lib --tests` passes **100%** (0 failed), including `storage_sync_test` and new security regression tests.
- `./app/build-web.sh` builds clean WASM + web bundle; `cargo check -p chatw-app` (desktop) compiles cleanly; `node --check` passes on all `app/web/*.js`.
- `cargo clippy --workspace --no-deps` builds cleanly with zero errors.
- Changes are committed cleanly and pushed to `https://github.com/kiyogo/chart` (`main`).
