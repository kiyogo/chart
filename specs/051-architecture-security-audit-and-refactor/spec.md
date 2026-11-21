# Spec 051 — Full Architecture Audit, P0/P1 Security Hardening, CI/CD, Modular Refactoring & Testing

## 1. Problem Statement & Audit Baseline (Commit `19cfa28`)

The ChatW codebase (`https://github.com/kiyogo/chart`) consists of:
- `app/`: Multi-platform Rust + Dioxus 0.7 client (WASM Web + Desktop WebKitGTK/WebView2 + custom JS engines in `app/web/`).
- `web/backend/`: Rust Axum 0.8 + Tokio + SQLx 0.9 + PostgreSQL backend (`chatw`).
- `crates/kiyo-kdata/`: Shared encrypted `.kdata` backup archive crate (`Argon2id` + `XChaCha20-Poly1305` + `HMAC-SHA256`).
- `sdk/`: Official TypeScript (`sdk/ts`), Rust WebAssembly (`sdk/rust`), and CLI (`sdk/cli`) Plugin SDK + sample plugins.

Direct code inspection at commit `19cfa28` confirms the following P0 and P1 issues:

### P0 Issues
1. **P0.1 — Plugin Worker Same-Origin Execution (`app/web/plugin_runtime.js`)**:
   - While plugin UI HTML mounts inside `<iframe sandbox="allow-scripts">` (`/plugin-sandbox-frame`), background plugin bundles (`bundle_code` / `wasm_base64`) are launched directly in the host window via `new Blob([src])` + `new Worker(URL.createObjectURL(blob))`.
   - A Blob Worker created from the host document inherits the host origin (`http://127.0.0.1:8085`), allowing raw `fetch('/api/...')` inside the worker to send same-origin session cookies (`credentials: 'include'`) and bypass the Host Permission Broker.
2. **P0.2 — Multi-User Plugin Grant & Resource Scope Isolation (`web/backend/src/plugins/installations.rs` & `app/web/plugin_runtime.js`)**:
   - In `fetch_installation_full`, all non-`device_only` rows in `plugin_grants` for a server/group installation are returned to every member (`viewer_uid`), and `hasPermission()` in `plugin_runtime.js` does not bind user-personal permissions (`profile.read`, `contacts.read`, `contacts.write`, `e2ee.*`) to `g.user_id === currentAccountId`.
   - Furthermore, `GrantSpec` and `plugin_grants` must enforce that personal permissions (`profile.read`, `contacts.read`, `contacts.write`, `e2ee.*`) are strictly evaluated per-granting-user (`principal_id` / `user_id = viewer_uid`), and conversation-scoped E2EE grants require verifying that the granting user is actually a participant of that conversation.
3. **P0.3 — E2EE Taint & Outbound Egress Server-Side Enforcement (`web/backend/src/plugins/installations.rs`)**:
   - `plugin_network_fetch` (`POST /api/plugins/installations/{id}/net/fetch`) checks `if req.e2ee_tainted` from the client JSON body, trusting the client flag alone instead of also checking server-side state (whether this installation+user has any active `e2ee.*` grant or recorded `e2ee_read` audit in `plugin_audit_logs`, or if `e2ee.process_local` is active).
   - Unverified publishers must never be granted E2EE permissions, and revoking an E2EE grant or disabling a plugin must immediately block both audit/read and network egress.
4. **P0.4 — E2EE Private Key & Session Token At-Rest Protection with Non-Destructive Migration (`app/src/crypto/e2ee.rs`, `app/src/platform/web.rs`, `app/src/platform/native.rs`)**:
   - On Web, `chatw_e2ee_keys_<uid>` stores raw base64url P-256 private scalars in `localStorage`.
   - On Native Desktop, `client_storage.json` stores `chatw_e2ee_keys_<uid>` and `chatw_session_token` in plaintext JSON on disk.
   - Keys and session tokens must be encrypted at rest (`AES-256-GCM` with a device-bound wrapping key stored in IndexedDB/OS vault file with `0600` permissions) while automatically and non-destructively migrating legacy plaintext entries in-place so existing users never lose access to old E2EE messages.

### P1 Issues
1. **P1.1 — GitHub Actions Backend CI Failure (`.github/workflows/ci.yml`)**:
   - `test-backend` runs `cargo test -p chatw --lib` without a PostgreSQL service container, migrations, or `DATABASE_URL`. Because `web/backend` uses `sqlx::query!` macros and `.env` is gitignored, `cargo test` fails at compile time in GitHub Actions.
2. **P1.2 — Cryptographic Plugin Package Signature Verification (`web/backend/src/plugins/marketplace.rs`, `schema.rs`)**:
   - `plugin_versions.signature` is stored as an optional string without Ed25519 cryptographic signature verification against the publisher's public key and canonical manifest + SHA-256 package digest.
3. **P1.3 — Unbounded Response Buffering in Plugin Network Proxy & Cloud SSRF Hardening (`web/backend/src/plugins/installations.rs`, `security.rs`, `web/backend/src/cloud/providers.rs`)**:
   - `plugin_network_fetch` calls `resp.bytes().await` before checking `bytes.len() > 256 * 1024`, allowing a malicious upstream server to stream gigabytes into RAM before the check runs.
   - Cloud `WebDavProvider` uses the shared `http` client without redirect disabled, allowing a remote WebDAV endpoint to `302` redirect to an internal/metadata IP (`169.254.169.254` / `127.0.0.1`).
4. **P1.4 — Permissive CORS Prefix Match, CSP & Unbounded OAuth Tickets (`web/backend/src/main.rs`, `web/backend/src/auth/mod.rs`)**:
   - `main.rs` checks `s.starts_with("http://localhost") || s.starts_with("http://127.0.0.1")`, which matches attacker origins like `http://localhost.attacker.evil`!
   - `oauth_tickets` in `AppState` is `HashMap<Uuid, Option<String>>` with no TTL expiration, no max capacity limit, and no periodic cleanup.
5. **P1.5 — Plugin Runtime Queue & Lifecycle Limits (`app/web/plugin_runtime.js`)**:
   - Worker boot (`starting`) lacks a boot timeout kill-switch, and pending RPC/event queues need strict capacity bounds and full teardown on disable/uninstall/revoke.

### Modular Architecture, Performance & Documentation
- Decompose oversized backend and frontend modules (`web/backend/src/plugins/installations.rs`, `web/backend/src/enterprise/mod.rs`, `web/backend/src/storage/router.rs`, `app/src/pages/chat.rs`, `app/src/components/plugins_view.rs`, `app/src/components/plugin_settings.rs`, `app/src/components/backup_cloud_ui.rs`, `app/assets/main.css`) into clean domain submodules without breaking routes, imports, or visual styling.
- Add windowed rendering / O(1) lookup optimization for large conversation message lists (10,000+ messages) without changing visual layout.
- Update `README.md`, `docs/architecture/`, and `sdk/README.md` to accurately reflect the unified architecture.

## 2. Acceptance Criteria
1. All P0 and P1 vulnerabilities are fixed with automated unit, integration, and 20-case security regression tests.
2. `.github/workflows/ci.yml` provisions PostgreSQL, runs all migrations, and passes `cargo test --workspace` + integration tests.
3. Oversized files are cleanly modularized into domain sub-files with zero broken features or visual regressions.
4. All existing Playwright E2E suites (`spec048`, `spec049`, `spec050`, `plugin-platform-e2e.mjs`) pass 100%.
