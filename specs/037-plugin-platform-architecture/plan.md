# Technical Architecture Plan — Spec 037

## 1. Database (`web/backend/migrations/0030_plugin_platform.sql`)
- Tables: `plugin_publishers`, `plugin_packages`, `plugin_versions`, `plugin_installations`, `plugin_grants`, `plugin_storage`, `plugin_webhooks`, `plugin_webhook_deliveries`, `plugin_reviews`, `plugin_reports`, `plugin_audit_logs`, `plugin_sync_cursors`, `plugin_bots`.
- All queries use runtime `sqlx::query` / `sqlx::query_as` for seamless migration execution.

## 2. Backend (`web/backend/src/plugins/`)
- `schema.rs`: Permission catalog, manifest schema & validator, Discord Embed JSON validator, package path-traversal & SHA-256 checks.
- `security.rs`: SSRF IP/DNS guard, AES-256-GCM secret encryption via HKDF(`session_secret`), constant-time HMAC-SHA256 signing/verification, per-key sliding-window rate limiter, scope RBAC helpers.
- `marketplace.rs`: Publisher registration/verification, admin approval endpoints, package & version publishing/rollback/revocation, Marketplace search/reviews/reports, built-in package seeder.
- `installations.rs`: Consent-first install preview & install, enable/disable/update/uninstall, scoped permission grants & immediate revocation, Privacy Dashboard API, account-partitioned KV + secret storage with quota, multi-device sync API + `WsEvent::PluginSync`, plugin bot registration, SSRF-protected network proxy.
- `webhooks.rs`: Incoming Webhook CRUD + `POST /api/webhooks/{id}/{token}` (inserting real channel/group messages with Discord embeds + WS broadcast + idempotency), Outgoing Webhook CRUD + async HMAC-signed event dispatcher + exponential backoff retry worker.
- `mod.rs`: Router wiring (`/api/plugins`, `/api/webhooks`, `/api/servers/{id}/plugins`, `/api/groups/{id}/plugins`, `/api/admin/plugins`), CSP update (`frame-src 'self' blob: data:`), and hooks in `servers/mod.rs` & `groups/mod.rs` to dispatch outgoing webhook events.

## 3. Client Sandbox Runtime (`app/web/plugin_runtime.js` + `app/src/services/plugin_api.rs`)
- Loaded in both Web (`app/web/index.html`, `app/build-web.sh`) and Desktop (`app/src/main.rs`).
- Isolated Worker (JS & Rust WASM via `WebAssembly.instantiate` + `chatw_host_call` ABI) + opaque-origin `sandbox="allow-scripts"` iframe for arbitrary custom plugin UI.
- Typed `postMessage` JSON-RPC bridge enforcing live permission grants, E2EE taint-tracked local vs external egress boundaries, rate limits, crash/hang watchdog (`terminate()`), Event Bus, and dynamic UI extension registry.

## 4. Frontend UI & Integration (`app/src/components/`)
- `rich.rs`: Discord-style `embed:{json}` message renderer (`title`, `description`, `url`, `color`, `author`, `thumbnail`, `image`, `footer`, `timestamp`, `fields`).
- `messages.rs` & `composer.rs`: Dynamic plugin message actions, composer quick actions, and dynamic `/command` dispatch.
- `plugin_hub.rs` & `plugin_settings.rs` & `plugins_view.rs`: Marketplace, Consent Permission Dialog, Installed Manager, Privacy Dashboard, Webhooks Manager, Developer Portal, Sandboxed Plugin Surface Host, account-partitioned Legacy Adapter, and real E2EE identity check.
- `server_settings.rs` & `group_manage.rs`: `Server Settings > Plugins & Webhooks` and `Group Manage > Plugins & Webhooks`.
- `pages/chat.rs`: `WsEvent::PluginSync` realtime multi-device sync & Event Bus hook emission.

## 5. Official SDKs, CLI & Verification Plugins (`sdk/`)
- `sdk/ts/`: TypeScript definitions & JS SDK runtime.
- `sdk/rust/`: `chatw-plugin-sdk` crate for `wasm32-unknown-unknown` Rust plugins.
- `sdk/cli/chatw-plugin.mjs`: Init/validate/pack/hash CLI.
- `sdk/examples/task-tracker-js/` & `sdk/examples/calc-wasm-rust/`: Real third-party JS and Rust-WASM plugins tested end-to-end.
