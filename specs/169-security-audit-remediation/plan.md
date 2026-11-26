# Technical Plan: Spec 169 — Security Audit Remediation

## 1. Operational Hardening (F-02, F-03, F-04 Infra)
- Create `0600` Redis config (`/home/ngson/.local/share/chatw/redis.conf`) binding `127.0.0.1:6379` with generated random `requirepass`, and launch `redis-server`.
- Update `/home/ngson/.local/share/chatw/pgdata/postgresql.conf` (`unix_socket_permissions = 0700`, `password_encryption = scram-sha-256`).
- Update `/home/ngson/.local/share/chatw/pgdata/pg_ident.conf` (`chatw_map ngson chatw_app`) and `pg_hba.conf` (`local chatw chatw_app peer map=chatw_map`, all other local/host `scram-sha-256`).
- Rotate `postgres` and `chatw` passwords in PostgreSQL (`ALTER ROLE chatw NOSUPERUSER NOCREATEDB NOCREATEROLE`), `chmod 700 /tmp/kiyo-pg-run`, reload/restart Postgres cleanly.
- Create least-privilege role `chatw_app` (`NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS`) with `SELECT, INSERT, UPDATE, DELETE` on app tables/sequences EXCEPT `sessions` and `user_key_backups` (fully revoked).
- Truncate legacy `sessions` rows (invalidated by `SESSION_SECRET` rotation).
- Update `.env` and `web/backend/.env` (`chmod 600`):
  - `DATABASE_URL=postgresql://chatw_app@%2Ftmp%2Fkiyo-pg-run/chatw`
  - `VALKEY_URL=redis://:<redis_pass>@127.0.0.1:6379`
  - Fresh 64-hex-char `SESSION_SECRET`
  - Scrub `GOOGLE_CLIENT_SECRET` with `REPLACE_WITH_ROTATED_GOOGLE_CLIENT_SECRET`
  - Remove `DEV_LOGIN=true` (set `DEV_LOGIN=false`)

## 2. Database Migration (`web/backend/migrations/0044_security_hardening_roles_and_backups.sql`)
- Create `SECURITY DEFINER` helper functions owned by table owner (`fetch_user_key_backup(UUID)`, `upsert_user_key_backup(...)`) so the app never needs direct `SELECT/INSERT/UPDATE` on `user_key_backups`.
- If `chatw_app` role exists, enforce `REVOKE ALL ON TABLE public.sessions FROM chatw_app` and `REVOKE ALL ON TABLE public.user_key_backups FROM chatw_app`, granting `EXECUTE` on the helper functions to `chatw_app`.

## 3. Backend Changes (`web/backend/`)
- **`src/config.rs`**: Remove `dev_login_enabled` and `DEV_LOGIN` parsing; add `cors_allowed_origins: Vec<String>` and optional Kiyo OIDC settings (`kiyo_issuer`, `kiyo_client_id`, `kiyo_client_secret`, `kiyo_redirect_url`, `kiyo_configured`).
- **`src/auth/kiyo.rs`**: Implement OIDC PKCE (`S256`) authorize URL builder, `/oauth2/token` code+verifier exchange, `/.well-known/jwks.json` cache, and RS256 ID-token signature/claims validation.
- **`src/auth/mod.rs`**:
  - Delete `DevLoginRequest`, `handle_dev_login`, and `/dev-login` route; return `dev: false` and `kiyo` status in `handle_providers`.
  - Disable `POST /api/auth/register` (`403 registration_disabled`) while preserving `/login` & Settings `/password`.
  - Add `/kiyo` & `/kiyo/callback` routes backed by `kiyo_jwks_cache` on `AppState` and shared `upsert_oidc_user` / `complete_oidc_callback` helpers.
  - Tighten `is_allowed_cors_origin_for_config` / `is_allowed_cors_origin` to exact `public_origin` (and explicit `cors_allowed_origins`), rejecting cross-port loopback origins.
  - Wire `ConnectInfo<SocketAddr>` into `/register`, `/login`, and `/2fa/verify` for true per-IP + per-user HTTP-layer rate limiting via `ValkeyCoordinator`.
  - Route session listing, renaming, single/other revocation, and password-change session revocation through `auth::session` helpers so Redis-backed sessions work identically to DB sessions.
- **`src/auth/session.rs` & `src/cache/valkey.rs`**:
  - Implement Redis-backed session CRUD (`store_session`, `get_session_by_hash`, `get_session_by_id`, `list_user_sessions`, `update_session_label`, `bind_session_device`, `delete_session_by_hash`, `delete_session_by_id`, `delete_other_user_sessions`, `delete_user_sessions`, `delete_device_sessions`, `count_user_sessions`, `count_total_active_sessions`) when Valkey is configured, keeping PG fallback only for unit/integration tests with `ValkeyCoordinator::disabled()`.
  - In `authenticate_session` (+ `authenticate_session_with_headers`), re-check `users.suspended_at IS NULL`, verify bound `device_id` exists in `devices` for `user_id` with `trust_state <> 'revoked'`, and verify `X-ChatW-Device-Id` (if sent) matches the bound `device_id` / `client_device_id`.
- **`src/devices_sync/mod.rs`, `src/security/mod.rs`, `src/admin/mod.rs`**:
  - Route session mutations/queries through `auth::session` helpers so Redis-backed sessions are properly bound to `device_id`, revoked on device revocation / suspension / logout, and counted in admin stats.
  - Route `user_key_backups` reads/writes through `fetch_user_key_backup` / `upsert_user_key_backup` RPCs.
- **`src/attachments/mod.rs`**:
  - Harden `handle_protected_banner_file` with `enforce_anti_direct_media_access` and full DB scope/ownership checks (`groups`, `servers`, `users` with block/privacy/membership checks).
- **`src/main.rs`**:
  - Handle `db::run_migrations` cleanly if connected as non-owner `chatw_app` (verify migrations are already applied instead of failing on `CREATE TABLE`).
  - Tighten `/plugin-sandbox-frame` CSP `frame-ancestors` to `'self'` (derived from `public_origin`) and require exact `PARENT_ORIGIN` in `postMessage` + `ev.source === window.parent` + clear `location.hash`.
  - Serve with `app.into_make_service_with_connect_info::<SocketAddr>()`.

## 4. Frontend Changes (`app/`)
- Remove `auth_dev_login`, `auth_register`, `auth_login`, `handle_dev_login`, and all username/password/create-account/dev-login UI from `app/src/pages/login.rs`, `app/src/services/api.rs`, `app/src/platform/web.rs`, and `app/assets/main.css`.
- Add `KiyoIcon` (`app/src/components/icons.rs`), `ProvidersResponse.kiyo` (`app/src/models.rs`), and `start_kiyo_oauth` (`app/src/platform/{web,native}.rs`), placing `"Đăng nhập bằng Kiyo"` above `"Continue with Google"` in `app/src/pages/login.rs`.
- Update `app/web/plugin_runtime.js`, `app/web/core_bridge.js`, and `app/src/platform/web.rs` for sandbox origin checks, `X-ChatW-Device-Id` propagation, and 2FA worker sync.
- Rebuild `app/dist/` via `./app/build-web.sh release`.

## 5. Verification
- Run `cargo test -p chatw` (with `chatw` migration role for tests + unit tests).
- Restart `chatw` backend running as `chatw_app` via peer auth + Redis sessions.
- Live probe verification against `http://127.0.0.1:8085`:
  - `POST /api/auth/dev-login` returns 404/405.
  - `GET /api/auth/providers` returns `"dev":false`.
  - `psql` as `chatw_app` fails on `SELECT * FROM sessions`, `INSERT INTO sessions`, and `SELECT * FROM user_key_backups`.
  - `/tmp/kiyo-pg-run` is `0700`, `.env` & `web/backend/.env` are `0600`.
  - Password register/login works via Redis sessions, cross-port CORS is rejected, unauthorized banner fetch returns `403`, `/plugin-sandbox-frame` CSP & headers verified, login rate limiting triggers `429`.
