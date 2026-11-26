# Implementation Tasks: Spec 169 — Security Audit Remediation

- [x] T001 Inspect all 5 findings (F-01 through F-05), runtime Postgres/Redis state, and write `specs/169-security-audit-remediation/{spec.md,plan.md,tasks.md}`
- [x] T002 Configure authenticated local Redis/Valkey, harden PostgreSQL (`0700` socket dir, `0700` socket permissions, `peer` + `scram-sha-256`, rotated passwords, non-superuser `chatw`, least-privilege `chatw_app`), and secure `.env` & `web/backend/.env` (`chmod 600`, rotated `SESSION_SECRET`, scrubbed `GOOGLE_CLIENT_SECRET`, `DEV_LOGIN=false`)
- [x] T003 Add migration `0044_security_hardening_roles_and_backups.sql` with `SECURITY DEFINER` `user_key_backups` accessors and explicit `REVOKE ALL` on `sessions` and `user_key_backups` for `chatw_app`
- [x] T004 Delete `handle_dev_login` / `/api/auth/dev-login` and `dev_login_enabled` across `web/backend` and `app`, updating tests and `.env.example`
- [x] T005 Implement Redis-backed session store in `web/backend/src/cache/valkey.rs` & `web/backend/src/auth/session.rs` with per-request `device_id` & user suspension re-checks, and update `auth/mod.rs`, `security/mod.rs`, `devices_sync/mod.rs`, `admin/mod.rs`
- [x] T006 Fix F-05 items: scope-check `handle_protected_banner_file`, tighten CORS to exact `public_origin`, harden `/plugin-sandbox-frame` CSP & `postMessage` origin validation, and enforce peer-IP HTTP-layer rate limiting on `/api/auth/login`, `/api/auth/register`, and `/api/auth/2fa/verify`
- [x] T007 Audit entire codebase (`clippy` + `Codebase Auditor`), fix cross-module logic/authz/storage/enterprise/plugin bugs, rebuild backend + Dioxus web bundle (`./app/build-web.sh release`), restart `chatw` as `chatw_app`, and pass all tests & live probes
- [x] T008 Disable `/api/auth/register`, implement Kiyo OIDC PKCE (`web/backend/src/auth/kiyo.rs`, `/api/auth/kiyo*`), remove login username/password/signup form, place "Đăng nhập bằng Kiyo" above Google button, re-verify backend/frontend builds, tests, and browser UI
- [x] T009 Verify `.env` is ignored (`git status`), commit all tracked remediation changes, and push to `origin main` (`https://github.com/kiyogo/chart`)
