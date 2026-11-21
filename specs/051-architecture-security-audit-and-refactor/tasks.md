# Spec 051 — Tasks Checklist

- [x] **T001**: Perform full repository audit & produce Baseline Report (Section 87)
- [x] **T002 (P0.1)**: Isolate plugin background Worker execution inside opaque-origin sandboxed frame (`app/web/plugin_runtime.js` & `web/backend/src/main.rs`)
- [x] **T003 (P0.2)**: Enforce per-user permission ownership (`principal_id`/`user_id`), conversation participant check for E2EE grants, and strict scope isolation (`web/backend/src/plugins/installations.rs` & `app/web/plugin_runtime.js`)
- [x] **T004 (P0.3)**: Enforce server-side E2EE taint tracking and outbound egress blocking without trusting client `e2ee_tainted` flag alone (`web/backend/src/plugins/installations.rs`)
- [x] **T005 (P0.4)**: Encrypt E2EE private keys and session tokens at rest with automatic non-destructive migration from legacy storage (`app/src/crypto/e2ee.rs`, `app/src/platform/web.rs`, `app/src/platform/native.rs`)
- [x] **T006 (P1.1)**: Fix `.github/workflows/ci.yml` with PostgreSQL service, migrations, `DATABASE_URL`, and integration test execution
- [x] **T007 (P1.2)**: Implement cryptographic plugin package signature & SHA-256 manifest verification (`web/backend/src/plugins/schema.rs`, `marketplace.rs`, migration `0031`)
- [x] **T008 (P1.3)**: Replace unbounded `resp.bytes().await` with bounded streaming reader in plugin proxy and enforce redirect-disabled SSRF protection in cloud providers (`web/backend/src/plugins/installations.rs`, `web/backend/src/cloud/providers.rs`)
- [x] **T009 (P1.4)**: Replace `starts_with("http://localhost")` CORS check with strict URL host parsing, tighten sandbox CSP, and add TTL + capacity limits to OAuth tickets (`web/backend/src/main.rs`, `web/backend/src/auth/mod.rs`)
- [x] **T010 (P1.5)**: Add boot timeout, queue bounds, and deterministic teardown to `app/web/plugin_runtime.js`
- [x] **T011 (Phase D/E)**: Modularize oversized backend & frontend files into clean domain submodules and optimize large message list rendering
- [x] **T012 (Phase G/H)**: Run 20-case security regression suite, unit/integration tests, Playwright E2E tests, update `README.md` + `docs/architecture/` + `sdk/README.md`, build Web & Desktop releases, and push to `origin/main`
