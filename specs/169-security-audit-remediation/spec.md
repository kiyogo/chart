# Feature Specification: Spec 169 — Full Security Audit Remediation (F-01 through F-05)

## 1. Context & Findings
A local security assessment against `chatw` (`127.0.0.1:8085`) identified a full-compromise chain from operational & authorization gaps:
1. **F-01 (`DEV_LOGIN` passwordless auth bypass — Critical)**: `POST /api/auth/dev-login` (`handle_dev_login`) mints valid session cookies for arbitrary usernames with no credentials whenever `DEV_LOGIN=true` on loopback origins, and `.env` ships `DEV_LOGIN=true`.
2. **F-02 (World-readable `.env` with live secrets — Critical)**: `.env` and `web/backend/.env` have mode `0755`, exposing `DATABASE_URL`, `SESSION_SECRET`, and `GOOGLE_CLIENT_SECRET` to any local process.
3. **F-03 (PostgreSQL reachable with `.env` superuser credentials & open socket — Critical)**: `/tmp/kiyo-pg-run` has mode `0775` and `.s.PGSQL.5432` has `0777`; `pg_hba.conf` uses `trust` auth; `chatw` DB role is a PostgreSQL `Superuser` with hardcoded `chatw_dev_password` in `.env`.
4. **F-04 (Writable `sessions` table + readable `user_key_backups` + missing per-request device validation — Critical)**: Because the app connects with full write access to `sessions` (and `SESSION_SECRET` is in `.env`), any DB write allows forging valid session cookies for any account including admins. Sessions should move to authenticated Redis/Valkey, the app DB role must be least-privilege (`chatw_app`, no `sessions` write, no direct `user_key_backups` table read), and bound `device_id` must be re-verified on every request.
5. **F-05 (Second-order gaps — Lower Severity)**:
   - `handle_protected_banner_file` (`GET /uploads/banners/{file_name}`) checks only session validity with no scope/ownership authorization and no `enforce_anti_direct_media_access`.
   - `CORS` (`is_allowed_cors_origin_for_config`) accepts any `localhost` / `127.0.0.1` port whenever `PUBLIC_URL` is loopback or `dev_login_enabled` is true, enabling cross-port credential leaks with `allow_credentials(true)`.
   - `/plugin-sandbox-frame` CSP sets `frame-ancestors 'self' http://127.0.0.1:* http://localhost:* https://*` and posts to `window.parent` with `'*'` using a `location.hash` nonce.
   - `/api/auth/login` and `/api/auth/register` (and `/api/auth/2fa/verify`) lacked true peer-IP HTTP-layer rate limiting when `TRUST_PROXY=false` because `ip_masked_with_trust(..., false)` collapsed all clients into `"Direct"`.
6. **Auth Surface Simplification & Kiyo OIDC PKCE**:
   - Disable public self-registration (`POST /api/auth/register` -> `403 registration_disabled`) and remove username/password/create-account inputs and "or" divider from `app/src/pages/login.rs`.
   - Add Kiyo OIDC Authorization Code + PKCE (`S256`) login endpoints (`GET /api/auth/kiyo`, `GET /api/auth/kiyo/callback`) and place **"Đăng nhập bằng Kiyo"** above **"Continue with Google"** on the Login page.

## 2. Acceptance Criteria
1. **F-01**: `/api/auth/dev-login` route and `handle_dev_login` are completely deleted from the backend router, `Config` no longer enables dev-login, `/api/auth/providers` always returns `dev: false`, and the frontend removes the dev-login form. `POST /api/auth/dev-login` returns `404` / `405`.
2. **F-02**: `.env` and `web/backend/.env` have mode `0600`; `SESSION_SECRET` is regenerated with 256-bit cryptographically random entropy; the leaked `GOOGLE_CLIENT_SECRET` is scrubbed from disk (replaced with a rotation placeholder until updated from Google Cloud Console); `.env` remains git-ignored and untracked.
3. **F-03**: `/tmp/kiyo-pg-run` has mode `0700`, `unix_socket_permissions = 0700` in `postgresql.conf`, `pg_hba.conf` replaces `trust` with `peer` (mapped via `pg_ident.conf` for `chatw_app` only) and `scram-sha-256`, `chatw_dev_password` is rotated for `chatw` and `postgres`, `chatw` has `NOSUPERUSER`, and `.env` uses passwordless peer auth as least-privilege `chatw_app`.
4. **F-04**:
   - Local `redis-server` runs bound to `127.0.0.1` with strong `requirepass` configured via `VALKEY_URL` in `0600` `.env`.
   - Runtime sessions are stored in Redis/Valkey with bounded TTL (`session_ttl`) and per-user/per-device indexes, with no writes to `public.sessions` when Valkey is active.
   - `chatw_app` has `REVOKE ALL ON public.sessions` and `REVOKE ALL ON public.user_key_backups` (with `SECURITY DEFINER` accessor functions owned by `chatw` for authenticated per-user key backup operations, preventing bulk table reads/dumps).
   - `authenticate_session` re-checks on every request that the user is not suspended and that any bound `device_id` belongs to the user and is not `revoked`, and validates `X-ChatW-Device-Id` when provided.
5. **F-05**:
   - `handle_protected_banner_file` enforces `enforce_anti_direct_media_access` and verifies scope/ownership against `groups`, `servers`, and `users` (`is_blocked_either` + friendship/membership/discoverability), returning `403` for unauthorized or unreferenced files.
   - CORS allows only the exact configured `public_origin` (plus explicit `CORS_ALLOWED_ORIGINS` if set), rejecting arbitrary cross-port `localhost` / `127.0.0.1` origins.
   - `/plugin-sandbox-frame` CSP sets `frame-ancestors 'self'` (plus `http://ipc.localhost` only if needed), validates `ev.source === window.parent` and `ev.origin === PARENT_ORIGIN`, uses explicit `PARENT_ORIGIN` instead of `'*'` for `window.parent.postMessage`, and clears `location.hash` after bootstrap.
   - `axum::serve` uses `into_make_service_with_connect_info::<SocketAddr>()` and auth endpoints (`/api/auth/login`, `/api/auth/register`, `/api/auth/2fa/verify`) enforce per-peer-IP + per-account HTTP rate limits.
6. **Kiyo OIDC & Signup Removal**:
   - `POST /api/auth/register` returns `403` (`registration_disabled`).
   - `/api/auth/providers` includes `"kiyo": <bool>` reflecting whether `KIYO_ISSUER` and `KIYO_CLIENT_ID` are configured.
   - `GET /api/auth/kiyo` returns `400` (`kiyo_not_configured`) when unconfigured, or redirects to `<KIYO_ISSUER>/oauth2/authorize` with PKCE `S256` challenge, HMAC state cookie, nonce, and `ticket` support when configured; `GET /api/auth/kiyo/callback` exchanges code + PKCE verifier at `/oauth2/token`, verifies RS256 ID token against `/.well-known/jwks.json`, and signs the user in.
   - Login UI renders `"Đăng nhập bằng Kiyo"` above `"Continue with Google"` without username/password/signup inputs.
