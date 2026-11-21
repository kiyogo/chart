# Feature Specification: ChatW Full Plugin Platform, Sandbox Runtime, SDK, Event Hooks, Discord Webhooks, Multi-Device Sync & Marketplace

**Feature Branch**: `037-plugin-platform-architecture`
**Created**: 2026-11-21
**Status**: In Progress

## Overview
Rebuild ChatW's plugin system from a closed JSON customization tool into a real, extensible, sandboxed, permission-governed Plugin & Mod Platform spanning Backend (Rust/Axum), Database (PostgreSQL/SQLx), Frontend (Dioxus), Multi-Platform Sandbox Runtime (Web + Desktop), Official SDKs (TS/JS + Rust WASM + CLI), Event Bus, UI Extension Points, Discord-style Incoming/Outgoing Webhooks, Multi-Device Sync, Privacy Dashboard, and Marketplace/Developer Portal.

## Core Requirements & Acceptance Criteria
1. **Security & Bug Fixes**:
   - Eliminate auto-install/auto-enable on import; enforce manifest validation, SHA-256 verification, and explicit user consent modal.
   - Replace timestamp IDs with immutable namespaced `plugin_key` (`publisher.slug`), `publisher_id`, `installation_id` (UUID), and `version`.
   - Scope plugin CSS inside sandboxed surfaces; require `theme.apply` permission for app-wide themes with preview/restore/conflict handling.
   - Fix SSRF/CORS/secret leaks on webhooks; add SSRF DNS/IP blocking, HTTPS enforcement, HMAC-SHA256, rate/size/timeout limits, retry & audit logs.
   - Partition legacy local plugin data per account; replace hardcoded Cyber E2EE check with real crypto identity verification.
2. **True Sandboxed Plugin Runtime (JS/TS + WebAssembly + Rust WASM)**:
   - Execute JS/TS and compiled Rust `.wasm` plugins inside opaque-origin `sandbox="allow-scripts"` iframes and isolated Web Workers with execution timeout kill-switches and memory bounds.
   - Zero access to host DOM, cookies, `localStorage`, session tokens, or E2EE private keys.
3. **Official SDKs & CLI (`sdk/ts`, `sdk/rust`, `sdk/cli`)**:
   - Complete typed `chatw.*` API namespaces, Rust WASM ABI crate, and CLI packaging/hashing tool.
4. **Two-Tier Verification, Permission System & E2EE Data Governance**:
   - Separate Publisher Verification (blue checkmark) from Sensitive Permission Approval and User Consent Grants.
   - Distinct E2EE permissions (`e2ee.read_decrypted`, `e2ee.process_local`, `e2ee.egress_external`, `e2ee.store_external`) with conversation scoping, audit logging, egress taint blocking, and instant revocation via Privacy Dashboard.
5. **Comprehensive Event Hooks & UI Extension Points**:
   - Event Dispatcher with deduplication, filtering, rate limiting, and error isolation across message, group, server, channel, call, account, and plugin lifecycle events.
   - Dynamic UI contributions: custom pages/sidebars/dialogs, message action buttons, composer buttons/commands, server tabs, group tabs, and Discord-style Embed message rendering.
6. **Server, Group, Channel & Bot Integration**:
   - Install/manage/scope plugins per `user`, `device`, `server`, `group`, `channel`, or `conversation` with channel allowlist/blocklist and server/group RBAC.
   - Plugin-registered Bots with distinct bot identity and token management.
7. **Discord-Style Incoming & Outgoing Webhooks**:
   - Incoming `POST /api/webhooks/{id}/{token}` posting real messages + validated Discord-style Embeds into channels/groups with WS broadcast and idempotency.
   - Outgoing HMAC-SHA256 signed event webhooks with SSRF protection, exponential backoff retry, dead-letter tracking, and delivery logs.
8. **PostgreSQL Persistence, Multi-Device Realtime Sync & Marketplace**:
   - Migration `0030_plugin_platform.sql`, incremental/snapshot sync API, `WsEvent::PluginSync` realtime sync across devices, encrypted secret storage at rest, and Marketplace + Developer Portal.
9. **End-to-End Verification**:
   - Build and execute real third-party JS/TS and Rust-WASM plugins using ONLY the official SDK (zero hardcoding in ChatW core).
   - Run Rust backend integration tests (multi-user Alice/Bob/Charlie RBAC, webhooks, SSRF, HMAC, sync, E2EE permissions) and Playwright E2E multi-client tests.
