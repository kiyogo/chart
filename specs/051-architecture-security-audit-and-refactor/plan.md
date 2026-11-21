# Spec 051 — Technical Implementation Plan

## Phase B: P0 Security Fixes
1. **Opaque-Origin Sandbox Execution for Plugin Workers (`app/web/plugin_runtime.js` & `web/backend/src/main.rs`)**:
   - Execute plugin background code (`bundle_code` & `wasm_base64`) inside a hidden `<iframe sandbox="allow-scripts">` (opaque `null` origin, no `allow-same-origin`, strict CSP `default-src 'none'; script-src 'unsafe-inline' 'unsafe-eval' 'wasm-unsafe-eval' blob:; connect-src 'none';`) that spawns the Blob Worker (or isolated runner) inside the opaque origin and bridges messages via structured `postMessage` to the Host Permission Broker.
   - Because `origin` inside the sandboxed frame and its worker is `"null"` and `connect-src` is `'none'`, direct `fetch('/api/...')`, `document.cookie`, `localStorage`, and `indexedDB` are completely blocked by the browser engine.
2. **Strict Multi-User Permission & Scope Isolation (`web/backend/src/plugins/installations.rs` & `app/web/plugin_runtime.js`)**:
   - Distinguish Installation Scope (`server`, `group`, `channel`, `user`, `device`, `conversation`) from Grant Principal (`user_id` = granting user) and Resource Scope (`scope_type`, `scope_id`).
   - Personal permissions (`profile.read`, `contacts.read`, `contacts.write`, `e2ee.read_decrypted`, `e2ee.process_local`, `e2ee.egress_external`, `e2ee.store_external`) are strictly bound to `g.user_id == viewer_uid` (one user or server admin can never grant another user's private profile, contacts, or E2EE messages).
   - For `e2ee.*` grants, verify that `scope_type == "conversation"`, `scope_id` is a valid conversation/user/group where `uid` is an authorized participant, and the package publisher is `verified` + version has `sensitive_approval_status == "approved"`.
3. **Server-Side E2EE Taint Tracking & Egress Enforcement (`web/backend/src/plugins/installations.rs`)**:
   - Do not trust client-supplied `req.e2ee_tainted` alone in `plugin_network_fetch`: query server-side whether the installation has `e2ee.process_local` granted, or has any active `e2ee.read_decrypted` grant / `e2ee_read` audit record for `(inst_id, uid)`.
   - If the installation+user is E2EE-tainted server-side (or client-side) or has `e2ee.process_local`, reject outbound network calls unless `e2ee.egress_external` (or `e2ee.store_external`) is actively granted and unrevoked.
4. **Encrypted E2EE Keys & Session Token Storage at Rest + Non-Destructive Automatic Migration (`app/src/crypto/e2ee.rs`, `app/src/platform/web.rs`, `app/src/platform/native.rs`)**:
   - Implement transparent AES-256-GCM at-rest encryption (`enc:v1:<nonce_b64>:<ciphertext_b64>`) for `chatw_e2ee_keys_*` and `chatw_session_token`.
   - On Native Desktop (`app/src/platform/native.rs`): store a 256-bit machine-bound master key in `<data_dir>/chatw/.vault.key` (`0600` permissions) and encrypt sensitive values before writing to `client_storage.json`. Automatically migrate any legacy plaintext JSON values on read without data loss.
   - On Web (`app/src/crypto/e2ee.rs` & `app/src/platform/web.rs`): derive/persist a per-origin wrapping secret in session/WebCrypto storage + obfuscated key wrap so `localStorage` never holds raw `ecdh_secret_b64`/`ecdsa_secret_b64` JSON, while transparently reading and upgrading legacy plaintext JSON on first read.

## Phase C: P1 Security, Runtime & CI/CD Fixes
1. **GitHub Actions CI Fix (`.github/workflows/ci.yml`)**:
   - Add a `postgres:17` service container to `test-backend`, set `DATABASE_URL`, apply all SQL migrations in `web/backend/migrations/*.sql` before `cargo test`, and run `cargo test -p kiyo-kdata`, `cargo test -p chatw --lib`, and `cargo test -p chatw --test plugin_platform`.
2. **Ed25519 Cryptographic Plugin Signature Verification (`web/backend/src/plugins/schema.rs`, `marketplace.rs`, migration `0031_plugin_ed25519_and_security.sql`)**:
   - Add `ed25519-dalek` (or standard Ed25519 verification via `ring` already in dependency tree!) to verify `public_key` on `plugin_publishers` and `signature` on `plugin_versions` over canonical `chatw-plugin-sig-v1:<plugin_key>:<version>:<sha256>`.
   - Verify signature on publish (when signature or publisher public_key is provided) and expose `signature_verified: bool` on marketplace/installation responses.
3. **Bounded Streaming HTTP Reader & Cloud SSRF Redirect Protection (`web/backend/src/plugins/installations.rs`, `web/backend/src/cloud/providers.rs`)**:
   - Replace `resp.bytes().await` in `plugin_network_fetch` with chunked streaming (`resp.chunk().await`) that aborts immediately as soon as accumulated bytes exceed `256 * 1024`.
   - Build a redirect-disabled (`redirect::Policy::none()`) + DNS-validated client for Cloud WebDAV/S3 requests so remote servers cannot redirect into internal/metadata IPs.
4. **Strict CORS Origin Parser, CSP & Bounded TTL OAuth Tickets (`web/backend/src/main.rs`, `web/backend/src/auth/mod.rs`)**:
   - Parse `Origin` with `reqwest::Url::parse(s)`: allow only exact host `"localhost"` or `"127.0.0.1"` (or configured `config.public_origin`), blocking `http://localhost.evil.com`.
   - Upgrade `oauth_tickets` to store `OAuthTicketEntry { token: Option<String>, created_at: Instant }` with 5-minute TTL, max 1,024 entries, automatic eviction, and atomic one-time consumption.
5. **Plugin Runtime Resource & Lifecycle Controls (`app/web/plugin_runtime.js`)**:
   - Add boot timeout (5s), max pending invocations (64), and full listener/UI cleanup on stop/disable/uninstall.

## Phases D–H: Modular Decomposition, Performance, Testing & Docs
- Split oversized files into focused domain submodules (`installations/`, `enterprise/`, `storage/router/`, `pages/chat/`, `components/plugins_view/`, etc.) while keeping all public routes and UI intact.
- Add windowed message list rendering for conversations with large history and benchmark tests.
- Update `README.md`, `docs/architecture/`, and `sdk/README.md`.
