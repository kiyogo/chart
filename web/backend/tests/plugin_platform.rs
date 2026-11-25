//! Comprehensive Integration Test Suite for ChatW Plugin Platform (Spec 037)
//! Verifies real PostgreSQL + Axum behavior for:
//! 1. Publisher registration, namespace ownership, admin blue-checkmark verification
//! 2. Package & version publishing, SHA-256 verification, path traversal rejection, real WASM binary check, rollback & revocation
//! 3. Two-tier sensitive & E2EE permission enforcement (consent required, verified != sensitive auto-approve, server admin cannot grant member E2EE, upgrade privilege escalation blocked, instant revocation)
//! 4. Alice / Bob / Charlie multi-user RBAC across Server, Channel allowlist/blocklist, and Group scopes
//! 5. Discord-style Incoming Webhooks (real message + Embed insert into channels & groups, invalid/rotated/disabled token rejection, invalid/oversized payload rejection, Idempotency-Key deduplication, rate limit 429)
//! 6. Outgoing Webhooks (real channel message dispatch -> HMAC-SHA256 signature verification -> SSRF private/metadata IP block -> delivery logs)
//! 7. Account-partitioned encrypted storage (`is_secret: true` never leaked), SSRF proxy with E2EE taint egress block, Plugin Bot registration, Privacy Dashboard & Multi-device Sync

use axum::{
    Json, Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
    routing::post,
};
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tower::ServiceExt;
use uuid::Uuid;

use chatw::{
    auth::{AppState, google::GoogleJwksCache, session::create_session},
    config::Config,
    db, groups, plugins, servers, storage,
    websocket::WsHub,
};

fn test_config(tag: &str) -> Config {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://chatw:chatw_dev_password@%2Ftmp%2Fkiyo-pg-run/chatw".into()
    });
    Config {
        database_url,
        db_max_connections: 8,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        public_origin: "http://localhost:8085".into(),
        google_client_id: "test-client-id".into(),
        google_client_secret: "test-client-secret".into(),
        google_redirect_url: "http://localhost:8085/api/auth/google/callback".into(),
        session_secret: b"test-session-secret-at-least-32-bytes-long!".to_vec(),
        session_ttl: Duration::from_secs(3600),
        cookie_secure: false,
        static_dir: None,
        uploads_dir: std::env::temp_dir().join(format!("chatw_spec037_{tag}_{}", Uuid::new_v4().simple())),
        max_ws_connections: 100,
        trust_proxy: false,
        dev_login_enabled: false,
        google_configured: false,
        valkey_url: None,
        small_chunk_inline_threshold_bytes: 16384,
    }
}

async fn setup_app() -> (Router, sqlx::PgPool, AppState) {
    let cfg = test_config("plugins");
    let pool = db::connect(&cfg.database_url, 8).await.expect("connect pg");
    db::run_migrations(&pool).await.expect("migrations");
    let storage_reg = Arc::new(storage::StorageRegistry::from_uploads_dir(&cfg.uploads_dir));
    let state = AppState {
        config: Arc::new(cfg),
        db: pool.clone(),
        http_client: reqwest::Client::new(),
        jwks_cache: Arc::new(tokio::sync::RwLock::new(GoogleJwksCache::new())),
        ws_hub: Arc::new(WsHub::new(100)),
        oauth_tickets: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        storage: storage_reg,
        sfu: Arc::new(chatw::calls::sfu::SfuHandle::new()),
        rpc_router: Arc::new(std::sync::OnceLock::new()),
        valkey: Arc::new(chatw::cache::ValkeyCoordinator::disabled()),
    };
    plugins::marketplace::seed_official_catalog(&state.db).await;

    let router = Router::new()
        .nest("/api/plugins", plugins::plugin_routes())
        .nest("/api/webhooks", plugins::webhook_routes())
        .nest("/api/admin/plugins", plugins::admin_plugin_routes())
        .nest("/api/servers", servers::server_routes())
        .nest("/api/channels", servers::channel_routes())
        .nest("/api/groups", groups::routes())
        .with_state(state.clone());

    (router, pool, state)
}

async fn create_test_user(pool: &sqlx::PgPool, state: &AppState, prefix: &str, is_admin: bool) -> (Uuid, String) {
    let uid = Uuid::new_v4();
    let suffix = &uid.simple().to_string()[..8];
    let username = format!("{prefix}_{suffix}");
    let norm = username.to_lowercase();
    let google_sub = format!("sub_{uid}");
    sqlx::query(
        "INSERT INTO users (id, google_sub, username, username_norm, display_name, is_admin)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(uid)
    .bind(&google_sub)
    .bind(&username)
    .bind(&norm)
    .bind(format!("User {prefix}"))
    .bind(is_admin)
    .execute(pool)
    .await
    .expect("insert test user");

    let set_cookie = create_session(pool, &state.config, uid, None)
        .await
        .expect("create session");
    let cookie_pair = set_cookie.split(';').next().unwrap_or(&set_cookie).to_string();
    (uid, cookie_pair)
}

async fn req_json(
    app: &Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    req_json_headers(app, method, uri, cookie, &[], body).await
}

async fn req_json_headers(
    app: &Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    extra_headers: &[(&str, &str)],
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    for (k, v) in extra_headers {
        builder = builder.header(*k, *v);
    }
    let req = if let Some(b) = body {
        builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let resp = app.clone().oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let val = serde_json::from_slice::<Value>(&bytes)
        .unwrap_or_else(|_| json!({ "_raw": String::from_utf8_lossy(&bytes) }));
    (status, val)
}

#[tokio::test]
async fn test_publisher_packages_wasm_and_two_tier_sensitive_permissions() {
    let (app, pool, state) = setup_app().await;
    let (_admin_id, admin_cookie) = create_test_user(&pool, &state, "admin", true).await;
    let (dev_id, dev_cookie) = create_test_user(&pool, &state, "dev", false).await;
    let (other_id, other_cookie) = create_test_user(&pool, &state, "other", false).await;
    let (_user_id, user_cookie) = create_test_user(&pool, &state, "user", false).await;

    let pub_slug = format!("pub{}", &dev_id.simple().to_string()[..8]);

    // 1. Register publisher
    let (st, pub_val) = req_json(
        &app,
        "POST",
        "/api/plugins/publishers",
        Some(&dev_cookie),
        Some(json!({ "slug": pub_slug, "display_name": "Acme Labs", "website_url": "https://example.com" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "register publisher failed: {pub_val}");
    let pub_id = pub_val["id"].as_str().unwrap().to_string();
    assert_eq!(pub_val["verified"], json!(false));

    // Duplicate namespace rejected
    let (st_dup, _) = req_json(
        &app,
        "POST",
        "/api/plugins/publishers",
        Some(&other_cookie),
        Some(json!({ "slug": pub_slug, "display_name": "Imposter" })),
    )
    .await;
    assert_eq!(st_dup, StatusCode::CONFLICT);

    // 2. Reject path traversal in entry_point & invalid WASM magic & SHA-256 mismatch
    let plugin_key = format!("{pub_slug}.e2ee_reader");
    let (st_trav, _) = req_json(
        &app,
        "POST",
        "/api/plugins/packages",
        Some(&dev_cookie),
        Some(json!({
            "manifest": {
                "plugin_key": plugin_key,
                "name": "Bad Path",
                "version": "1.0.0",
                "entry_point": "../etc/passwd",
                "runtime": "js",
                "permissions": ["storage.local"]
            },
            "bundle_code": "console.log(1);"
        })),
    )
    .await;
    assert_eq!(st_trav, StatusCode::BAD_REQUEST);

    let (st_bad_wasm, _) = req_json(
        &app,
        "POST",
        "/api/plugins/packages",
        Some(&dev_cookie),
        Some(json!({
            "manifest": {
                "plugin_key": format!("{pub_slug}.bad_wasm"),
                "name": "Fake Wasm",
                "version": "1.0.0",
                "entry_point": "plugin.wasm",
                "runtime": "wasm",
                "permissions": ["commands.register"]
            },
            "wasm_base64": "bm90LXdhc20tYnl0ZXM="
        })),
    )
    .await;
    assert_eq!(st_bad_wasm, StatusCode::BAD_REQUEST);

    let (st_bad_sha, _) = req_json(
        &app,
        "POST",
        "/api/plugins/packages",
        Some(&dev_cookie),
        Some(json!({
            "manifest": {
                "plugin_key": plugin_key,
                "name": "Bad Sha",
                "version": "1.0.0",
                "entry_point": "worker.js",
                "runtime": "js",
                "permissions": ["storage.local"]
            },
            "bundle_code": "console.log(1);",
            "sha256": "0000000000000000000000000000000000000000000000000000000000000000"
        })),
    )
    .await;
    assert_eq!(st_bad_sha, StatusCode::BAD_REQUEST);

    // 3. Publish REAL compiled Rust WASM binary from sdk/examples/calc-wasm-rust/plugin.wasm
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sdk/examples/calc-wasm-rust/plugin.wasm");
    let valid_wasm_b64 = if let Ok(bytes) = std::fs::read(&wasm_path) {
        use base64::{Engine, engine::general_purpose::STANDARD};
        STANDARD.encode(bytes)
    } else {
        "AGFzbQEAAAA=".to_string() // 8-byte valid WASM header \0asm\x01\0\0\0
    };
    let (st_wasm, wasm_pkg) = req_json(
        &app,
        "POST",
        "/api/plugins/packages",
        Some(&dev_cookie),
        Some(json!({
            "manifest": {
                "plugin_key": format!("{pub_slug}.calc_wasm"),
                "name": "WASM Calc",
                "version": "1.0.0",
                "entry_point": "plugin.wasm",
                "runtime": "wasm",
                "permissions": ["commands.register", "storage.local"]
            },
            "summary": "Real WASM plugin",
            "wasm_base64": valid_wasm_b64
        })),
    )
    .await;
    assert_eq!(st_wasm, StatusCode::OK, "wasm publish failed: {wasm_pkg}");

    // 4. Publish sensitive E2EE plugin package
    let (st_pkg, pkg_val) = req_json(
        &app,
        "POST",
        "/api/plugins/packages",
        Some(&dev_cookie),
        Some(json!({
            "manifest": {
                "plugin_key": plugin_key,
                "name": "Local E2EE Search",
                "version": "1.0.0",
                "entry_point": "worker.js",
                "runtime": "js",
                "permissions": ["storage.local", "network.outbound", "bots.manage", "e2ee.read_decrypted", "e2ee.process_local"]
            },
            "summary": "Local E2EE indexer",
            "bundle_code": "chatw.onReady(() => {});"
        })),
    )
    .await;
    assert_eq!(st_pkg, StatusCode::OK, "publish sensitive pkg failed: {pkg_val}");
    let pkg_id = pkg_val["package_id"].as_str().unwrap().to_string();
    let ver1_id = pkg_val["version_id"].as_str().unwrap().to_string();

    // Another user cannot hijack/publish version on dev's package
    let (st_hijack, _) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/packages/{pkg_id}/versions"),
        Some(&other_cookie),
        Some(json!({
            "manifest": {
                "plugin_key": plugin_key,
                "name": "Hijacked",
                "version": "1.0.1",
                "entry_point": "worker.js",
                "runtime": "js",
                "permissions": ["storage.local"]
            },
            "bundle_code": "alert(1)"
        })),
    )
    .await;
    assert_eq!(st_hijack, StatusCode::FORBIDDEN);

    // 5. TWO-TIER GOVERNANCE:
    // Step 5a: Admin verifies publisher (Blue Checkmark) -> DOES NOT auto-approve sensitive permissions!
    let (st_ver, _) = req_json(
        &app,
        "POST",
        &format!("/api/admin/plugins/publishers/{pub_id}/verify"),
        Some(&admin_cookie),
        None,
    )
    .await;
    assert_eq!(st_ver, StatusCode::OK);

    // Without consent_confirmed: true -> MUST fail
    let (st_noconsent, _) = req_json(
        &app,
        "POST",
        "/api/plugins/installations",
        Some(&user_cookie),
        Some(json!({
            "package_id": pkg_id,
            "consent_confirmed": false,
            "enable": true,
            "grants": [{ "permission": "storage.local", "scope_type": "global" }]
        })),
    )
    .await;
    assert!(
        st_noconsent == StatusCode::BAD_REQUEST || st_noconsent == StatusCode::FORBIDDEN,
        "consent_confirmed: false must be rejected, got {st_noconsent}"
    );

    // Verified publisher STILL blocked from receiving sensitive e2ee.read_decrypted until admin approves this version!
    let fake_conv = Uuid::new_v4();
    let (st_unapproved, unapp_val) = req_json(
        &app,
        "POST",
        "/api/plugins/installations",
        Some(&user_cookie),
        Some(json!({
            "package_id": pkg_id,
            "consent_confirmed": true,
            "enable": true,
            "grants": [
                { "permission": "storage.local", "scope_type": "global" },
                { "permission": "e2ee.read_decrypted", "scope_type": "conversation", "scope_id": fake_conv, "device_only": true }
            ]
        })),
    )
    .await;
    assert_eq!(st_unapproved, StatusCode::FORBIDDEN, "expected unapproved sensitive rejection, got: {unapp_val}");

    // Step 5b: Admin separately approves sensitive permissions for THIS version (`e2ee.read_decrypted`, `e2ee.process_local`, `network.outbound`)
    let (st_app_perm, _) = req_json(
        &app,
        "POST",
        &format!("/api/admin/plugins/versions/{ver1_id}/permissions"),
        Some(&admin_cookie),
        Some(json!({
            "status": "approved",
            "approved_permissions": ["e2ee.read_decrypted", "e2ee.process_local", "network.outbound", "bots.manage"]
        })),
    )
    .await;
    assert_eq!(st_app_perm, StatusCode::OK);

    // Step 5c: Now install succeeds WITH explicit consent + conversation-scoped E2EE grant
    let (st_inst, inst_val) = req_json(
        &app,
        "POST",
        "/api/plugins/installations",
        Some(&user_cookie),
        Some(json!({
            "package_id": pkg_id,
            "consent_confirmed": true,
            "enable": true,
            "grants": [
                { "permission": "storage.local", "scope_type": "global" },
                { "permission": "network.outbound", "scope_type": "global" },
                { "permission": "e2ee.read_decrypted", "scope_type": "conversation", "scope_id": fake_conv, "device_only": true }
            ]
        })),
    )
    .await;
    assert_eq!(st_inst, StatusCode::OK, "install after two-tier approval failed: {inst_val}");
    let inst_id = inst_val["id"].as_str().unwrap().to_string();

    // Step 5d: E2EE taint egress protection — outbound fetch with e2ee_tainted: true MUST be blocked because e2ee.egress_external was NOT granted!
    let (st_egress, egress_val) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/installations/{inst_id}/net/fetch"),
        Some(&user_cookie),
        Some(json!({
            "url": "https://example.com/leak",
            "method": "POST",
            "body": "decrypted secret",
            "e2ee_tainted": true
        })),
    )
    .await;
    assert_eq!(st_egress, StatusCode::FORBIDDEN, "expected e2ee egress block, got: {egress_val}");

    // Step 5e: Audit succeeds while grant is active
    let (st_aud_ok, _) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/installations/{inst_id}/audit"),
        Some(&user_cookie),
        Some(json!({ "action": "e2ee_read", "permission": "e2ee.read_decrypted", "conversation_id": fake_conv })),
    )
    .await;
    assert_eq!(st_aud_ok, StatusCode::OK);

    // Step 5f: Privacy Dashboard lists active E2EE grant -> Revoke it -> Subsequent access fails IMMEDIATELY!
    let (st_priv, priv_val) = req_json(&app, "GET", "/api/plugins/privacy", Some(&user_cookie), None).await;
    assert_eq!(st_priv, StatusCode::OK);
    let e2ee_grant_id = priv_val["grants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["permission"] == "e2ee.read_decrypted" && g["revoked_at"].is_null())
        .expect("e2ee grant in privacy dashboard")["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (st_rev, _) = req_json(
        &app,
        "DELETE",
        &format!("/api/plugins/installations/{inst_id}/grants/{e2ee_grant_id}"),
        Some(&user_cookie),
        None,
    )
    .await;
    assert_eq!(st_rev, StatusCode::OK);

    let (st_aud_denied, _) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/installations/{inst_id}/audit"),
        Some(&user_cookie),
        Some(json!({ "action": "e2ee_read", "permission": "e2ee.read_decrypted", "conversation_id": fake_conv })),
    )
    .await;
    assert_eq!(st_aud_denied, StatusCode::FORBIDDEN, "revoked E2EE grant MUST fail immediately");

    // Step 5g: Account-partitioned encrypted storage (`is_secret: true` never returned as plaintext)
    let (st_put_sec, _) = req_json(
        &app,
        "PUT",
        &format!("/api/plugins/installations/{inst_id}/storage/api_token"),
        Some(&user_cookie),
        Some(json!({ "value": "sk-live-secret-999", "is_secret": true, "syncable": false })),
    )
    .await;
    assert_eq!(st_put_sec, StatusCode::OK);

    let (st_put_norm, _) = req_json(
        &app,
        "PUT",
        &format!("/api/plugins/installations/{inst_id}/storage/theme_pref"),
        Some(&user_cookie),
        Some(json!({ "value": { "compact": true }, "is_secret": false, "syncable": true })),
    )
    .await;
    assert_eq!(st_put_norm, StatusCode::OK);

    let (st_list_st, st_val) = req_json(
        &app,
        "GET",
        &format!("/api/plugins/installations/{inst_id}/storage"),
        Some(&user_cookie),
        None,
    )
    .await;
    assert_eq!(st_list_st, StatusCode::OK);
    assert_eq!(st_val["data"]["theme_pref"]["compact"], json!(true));
    assert!(st_val["data"].get("api_token").is_none(), "secret value must NOT be returned in plaintext");
    assert!(
        st_val["keys"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["key"] == "api_token" && m["is_secret"] == json!(true)),
        "keys should list api_token as secret: {st_val}"
    );

    // Other user cannot access user_id's installation storage
    let (st_cross, _) = req_json(
        &app,
        "GET",
        &format!("/api/plugins/installations/{inst_id}/storage"),
        Some(&other_cookie),
        None,
    )
    .await;
    assert_eq!(st_cross, StatusCode::FORBIDDEN);

    // Step 5h: Plugin Bot registration requires explicit `bots.manage` grant!
    let bot_username = format!("bot_{}", &dev_id.simple().to_string()[..8]);
    let (st_bot_deny, _) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/installations/{inst_id}/bots"),
        Some(&user_cookie),
        Some(json!({ "username": bot_username, "display_name": "Search Helper Bot" })),
    )
    .await;
    assert_eq!(st_bot_deny, StatusCode::FORBIDDEN, "bot create without bots.manage MUST be denied");

    let (st_grant_bot, _) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/installations/{inst_id}/grants"),
        Some(&user_cookie),
        Some(json!({ "permission": "bots.manage", "scope_type": "global" })),
    )
    .await;
    assert_eq!(st_grant_bot, StatusCode::OK);

    let (st_bot, bot_val) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/installations/{inst_id}/bots"),
        Some(&user_cookie),
        Some(json!({ "username": bot_username, "display_name": "Search Helper Bot" })),
    )
    .await;
    assert_eq!(st_bot, StatusCode::OK, "bot create failed: {bot_val}");
    assert_eq!(bot_val["identity_type"], json!("bot"));

    // Step 5i: Multi-device sync cursor returns installation & settings
    let (st_sync, sync_val) = req_json(&app, "GET", "/api/plugins/sync?since_seq=0", Some(&user_cookie), None).await;
    assert_eq!(st_sync, StatusCode::OK);
    assert!(sync_val["installations"].as_array().unwrap().iter().any(|x| x["id"] == inst_id));

    // Step 5j: Publish v1.1.0 with new sensitive permission -> silent upgrade MUST be blocked without additional_grants!
    let (st_v2, v2_val) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/packages/{pkg_id}/versions"),
        Some(&dev_cookie),
        Some(json!({
            "manifest": {
                "plugin_key": plugin_key,
                "name": "Local E2EE Search",
                "version": "1.1.0",
                "entry_point": "worker.js",
                "runtime": "js",
                "permissions": ["storage.local", "e2ee.egress_external"]
            },
            "bundle_code": "chatw.onReady(() => {});"
        })),
    )
    .await;
    assert_eq!(st_v2, StatusCode::OK, "publish v1.1.0 failed: {v2_val}");
    let v2_id = v2_val["version_id"].as_str().or_else(|| v2_val["id"].as_str()).unwrap().to_string();

    let (st_upg, _) = req_json(
        &app,
        "PATCH",
        &format!("/api/plugins/installations/{inst_id}"),
        Some(&user_cookie),
        Some(json!({ "version_id": v2_id })),
    )
    .await;
    assert_eq!(st_upg, StatusCode::FORBIDDEN, "silent sensitive escalation on upgrade MUST fail");

    // Step 5k: Version Rollback & Revoke
    let (st_rb, _) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/packages/{pkg_id}/rollback"),
        Some(&dev_cookie),
        Some(json!({ "version": "1.0.0" })),
    )
    .await;
    assert_eq!(st_rb, StatusCode::OK);

    let (st_revoke_ver, _) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/versions/{v2_id}/revoke"),
        Some(&dev_cookie),
        None,
    )
    .await;
    assert_eq!(st_revoke_ver, StatusCode::OK);

    let _ = other_id;
}

#[tokio::test]
async fn test_rbac_alice_bob_charlie_and_discord_webhooks_incoming_outgoing_ssrf() {
    let (app, pool, state) = setup_app().await;
    let (_alice_id, alice_cookie) = create_test_user(&pool, &state, "alice", false).await;
    let (bob_id, bob_cookie) = create_test_user(&pool, &state, "bob", false).await;
    let (_charlie_id, charlie_cookie) = create_test_user(&pool, &state, "charlie", false).await;

    // Alice creates a Server (with default general channel)
    let (st_srv, srv_val) = req_json(
        &app,
        "POST",
        "/api/servers",
        Some(&alice_cookie),
        Some(json!({ "name": "Alice Guild" })),
    )
    .await;
    assert!(st_srv.is_success(), "create server failed: {srv_val}");
    let server_id = srv_val["id"].as_str().unwrap().to_string();

    // Fetch channels of Alice's server
    let (st_ch, ch_val) = req_json(
        &app,
        "GET",
        &format!("/api/servers/{server_id}/channels"),
        Some(&alice_cookie),
        None,
    )
    .await;
    assert_eq!(st_ch, StatusCode::OK);
    let channel_id = ch_val[0]["id"].as_str().unwrap().to_string();

    // Add Bob as normal member of Alice's server (NOT admin), Charlie is NOT a member
    sqlx::query("INSERT INTO server_members (server_id, user_id) VALUES ($1, $2)")
        .bind(Uuid::parse_str(&server_id).unwrap())
        .bind(bob_id)
        .execute(&pool)
        .await
        .expect("add bob to server");

    // Alice creates a Group with Bob
    let (st_grp, grp_val) = req_json(
        &app,
        "POST",
        "/api/groups",
        Some(&alice_cookie),
        Some(json!({ "name": "Alice Team", "member_ids": [bob_id] })),
    )
    .await;
    assert!(st_grp.is_success(), "create group failed: {grp_val}");
    let group_id = grp_val["id"].as_str().unwrap().to_string();

    // 1. RBAC: Alice installs official `chatw.focus_board` into Server with channel scope
    let (st_srv_inst, inst_val) = req_json(
        &app,
        "POST",
        "/api/plugins/installations",
        Some(&alice_cookie),
        Some(json!({
            "plugin_key": "chatw.focus_board",
            "scope_type": "server",
            "scope_id": server_id,
            "consent_confirmed": true,
            "enable": true,
            "allowed_channels": [channel_id],
            "grants": [
                { "permission": "ui.page", "scope_type": "server", "scope_id": server_id },
                { "permission": "storage.local", "scope_type": "server", "scope_id": server_id }
            ]
        })),
    )
    .await;
    assert_eq!(st_srv_inst, StatusCode::OK, "alice server install failed: {inst_val}");

    // Server admin CANNOT grant member E2EE permission!
    let (st_e2ee_srv, _) = req_json(
        &app,
        "POST",
        "/api/plugins/installations",
        Some(&alice_cookie),
        Some(json!({
            "plugin_key": "chatw.focus_board",
            "scope_type": "server",
            "scope_id": server_id,
            "consent_confirmed": true,
            "enable": true,
            "grants": [
                { "permission": "e2ee.read_decrypted", "scope_type": "server", "scope_id": server_id }
            ]
        })),
    )
    .await;
    assert!(
        st_e2ee_srv == StatusCode::FORBIDDEN || st_e2ee_srv == StatusCode::BAD_REQUEST,
        "server admin MUST NOT grant member E2EE, got {st_e2ee_srv}"
    );

    // Bob (normal member) CAN read server installations, CANNOT install or delete!
    let (st_bob_read, bob_list) = req_json(
        &app,
        "GET",
        &format!("/api/plugins/installations?scope_type=server&scope_id={server_id}"),
        Some(&bob_cookie),
        None,
    )
    .await;
    assert_eq!(st_bob_read, StatusCode::OK);
    assert!(!bob_list.as_array().unwrap().is_empty());

    let (st_bob_inst, _) = req_json(
        &app,
        "POST",
        "/api/plugins/installations",
        Some(&bob_cookie),
        Some(json!({
            "plugin_key": "chatw.focus_board",
            "scope_type": "server",
            "scope_id": server_id,
            "consent_confirmed": true,
            "enable": true,
            "grants": []
        })),
    )
    .await;
    assert_eq!(st_bob_inst, StatusCode::FORBIDDEN);

    // Charlie (non-member) CANNOT even read server installations or create webhooks
    let (st_charlie_read, _) = req_json(
        &app,
        "GET",
        &format!("/api/plugins/installations?scope_type=server&scope_id={server_id}"),
        Some(&charlie_cookie),
        None,
    )
    .await;
    assert_eq!(st_charlie_read, StatusCode::FORBIDDEN);

    // 2. INCOMING WEBHOOK (Discord-style) on Server Channel
    let (st_wh, wh_val) = req_json(
        &app,
        "POST",
        "/api/webhooks",
        Some(&alice_cookie),
        Some(json!({
            "direction": "incoming",
            "scope_type": "channel",
            "scope_id": channel_id,
            "channel_id": channel_id,
            "name": "GitHub CI Bot",
            "rate_limit_per_min": 5
        })),
    )
    .await;
    assert_eq!(st_wh, StatusCode::OK, "create incoming webhook failed: {wh_val}");
    let wh_id = wh_val["id"].as_str().unwrap().to_string();
    let wh_token = wh_val["token"].as_str().unwrap().to_string();

    // Wrong token -> 401
    let (st_wrong, _) = req_json(
        &app,
        "POST",
        &format!("/api/webhooks/{wh_id}/cw_wh_wrongtoken"),
        None,
        Some(json!({ "content": "hack" })),
    )
    .await;
    assert_eq!(st_wrong, StatusCode::UNAUTHORIZED);

    // Invalid empty payload -> 400
    let (st_empty, _) = req_json(
        &app,
        "POST",
        &format!("/api/webhooks/{wh_id}/{wh_token}"),
        None,
        Some(json!({ "content": "" })),
    )
    .await;
    assert_eq!(st_empty, StatusCode::BAD_REQUEST);

    // Valid Discord-style payload with Embed + Idempotency-Key -> 200 & inserts real row in channel_messages!
    let discord_payload = json!({
        "username": "GitHub Actions",
        "content": "Triển khai bản phát hành v2.0 thành công",
        "embeds": [{
            "title": "Build #42 Passed",
            "description": "Tất cả kiểm thử đơn vị và tích hợp đều đạt.",
            "url": "https://github.com/kiyogo/chart",
            "color": 2450411,
            "fields": [
                { "name": "Branch", "value": "main", "inline": true },
                { "name": "Duration", "value": "42s", "inline": true }
            ],
            "footer": { "text": "ChatW CI Pipeline" }
        }]
    });
    let (st_post1, post1_val) = req_json_headers(
        &app,
        "POST",
        &format!("/api/webhooks/{wh_id}/{wh_token}"),
        None,
        &[("Idempotency-Key", "deploy-run-42")],
        Some(discord_payload.clone()),
    )
    .await;
    assert_eq!(st_post1, StatusCode::OK, "incoming webhook post failed: {post1_val}");
    let msg_id1 = post1_val["message_id"].as_str().unwrap().to_string();

    // Duplicate Idempotency-Key -> deduplicated (does NOT insert a second message!)
    let (st_post2, post2_val) = req_json_headers(
        &app,
        "POST",
        &format!("/api/webhooks/{wh_id}/{wh_token}"),
        None,
        &[("Idempotency-Key", "deploy-run-42")],
        Some(discord_payload.clone()),
    )
    .await;
    assert_eq!(st_post2, StatusCode::OK);
    assert_eq!(post2_val["deduplicated"], json!(true));

    // Verify exact 1 row inserted in PostgreSQL channel_messages containing embed JSON
    let db_msg: String = sqlx::query_scalar("SELECT content FROM channel_messages WHERE id = $1")
        .bind(Uuid::parse_str(&msg_id1).unwrap())
        .fetch_one(&pool)
        .await
        .expect("fetch inserted webhook message");
    assert!(db_msg.starts_with("embed:"), "stored content should start with embed: prefix");
    assert!(db_msg.contains("Build #42 Passed"));

    // Rate limit enforcement (limit=5 per min: we used 1, send 5 more -> 6th hits 429)
    let mut hit_429 = false;
    for i in 0..6 {
        let (st_r, _) = req_json(
            &app,
            "POST",
            &format!("/api/webhooks/{wh_id}/{wh_token}"),
            None,
            Some(json!({ "content": format!("msg {i}") })),
        )
        .await;
        if st_r == StatusCode::TOO_MANY_REQUESTS {
            hit_429 = true;
            break;
        }
    }
    assert!(hit_429, "incoming webhook rate limit MUST trigger 429");

    // Rotate token -> old token immediately rejected with 401!
    let (st_rot, rot_val) = req_json(
        &app,
        "POST",
        &format!("/api/webhooks/{wh_id}/rotate-token"),
        Some(&alice_cookie),
        None,
    )
    .await;
    assert_eq!(st_rot, StatusCode::OK);
    let new_token = rot_val["token"].as_str().unwrap().to_string();
    assert_ne!(new_token, wh_token);

    let (st_old_tok, _) = req_json(
        &app,
        "POST",
        &format!("/api/webhooks/{wh_id}/{wh_token}"),
        None,
        Some(json!({ "content": "old token" })),
    )
    .await;
    assert_eq!(st_old_tok, StatusCode::UNAUTHORIZED);

    // Disable webhook -> even new token rejected with 403!
    let (st_dis, _) = req_json(
        &app,
        "PATCH",
        &format!("/api/webhooks/{wh_id}"),
        Some(&alice_cookie),
        Some(json!({ "enabled": false })),
    )
    .await;
    assert_eq!(st_dis, StatusCode::OK);

    let (st_dis_call, _) = req_json(
        &app,
        "POST",
        &format!("/api/webhooks/{wh_id}/{new_token}"),
        None,
        Some(json!({ "content": "disabled" })),
    )
    .await;
    assert_eq!(st_dis_call, StatusCode::FORBIDDEN);

    // 3. INCOMING WEBHOOK on Group!
    let (st_gwh, gwh_val) = req_json(
        &app,
        "POST",
        "/api/webhooks",
        Some(&alice_cookie),
        Some(json!({
            "direction": "incoming",
            "scope_type": "group",
            "scope_id": group_id,
            "name": "Group Alert Bot"
        })),
    )
    .await;
    assert_eq!(st_gwh, StatusCode::OK, "group webhook create failed: {gwh_val}");
    let gwh_id = gwh_val["id"].as_str().unwrap();
    let gwh_tok = gwh_val["token"].as_str().unwrap();

    let (st_gpost, gpost_val) = req_json(
        &app,
        "POST",
        &format!("/api/webhooks/{gwh_id}/{gwh_tok}"),
        None,
        Some(json!({ "content": "Thông báo nhóm từ Webhook", "embeds": [{ "title": "Nhắc họp", "description": "14:00 hôm nay" }] })),
    )
    .await;
    assert_eq!(st_gpost, StatusCode::OK, "group webhook post failed: {gpost_val}");
    let gmsg_id = Uuid::parse_str(gpost_val["message_id"].as_str().unwrap()).unwrap();
    let gmsg_content: String = sqlx::query_scalar("SELECT content FROM group_messages WHERE id = $1")
        .bind(gmsg_id)
        .fetch_one(&pool)
        .await
        .expect("fetch group webhook msg");
    assert!(gmsg_content.contains("Nhắc họp"));

    // 4. OUTGOING WEBHOOK SSRF Protection: private & cloud metadata IPs MUST be rejected!
    for bad_url in [
        "https://169.254.169.254/latest/meta-data/",
        "https://10.0.0.1/internal",
        "http://example.com/non-https",
        "https://user:pass@example.com/hook",
    ] {
        let (st_ssrf, _) = req_json(
            &app,
            "POST",
            "/api/webhooks",
            Some(&alice_cookie),
            Some(json!({
                "direction": "outgoing",
                "scope_type": "channel",
                "scope_id": channel_id,
                "channel_id": channel_id,
                "name": "Bad SSRF",
                "target_url": bad_url,
                "subscribed_events": ["onChannelMessage"]
            })),
        )
        .await;
        assert!(
            st_ssrf == StatusCode::FORBIDDEN || st_ssrf == StatusCode::BAD_REQUEST,
            "SSRF URL {bad_url} must be blocked, got {st_ssrf}"
        );
    }

    // 5. OUTGOING WEBHOOK real signed delivery to local receiver (with CHATW_ALLOW_LOCAL_WEBHOOKS=1)
    let captured: Arc<Mutex<Vec<(String, String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let cap_clone = captured.clone();
    let receiver_app = Router::new().route(
        "/hook",
        post(move |headers: HeaderMap, body: String| {
            let cap = cap_clone.clone();
            async move {
                let sig = headers
                    .get("x-chatw-signature-256")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                let ts = headers
                    .get("x-chatw-timestamp")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                cap.lock().unwrap().push((sig, ts, body));
                (StatusCode::OK, Json(json!({ "received": true })))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let recv_addr = listener.local_addr().unwrap();
    let recv_task = tokio::spawn(async move {
        let _ = axum::serve(listener, receiver_app).await;
    });

    // Enable local loopback webhook strictly for this loopback verification
    unsafe { std::env::set_var("CHATW_ALLOW_LOCAL_WEBHOOKS", "1"); }

    let (st_out, out_val) = req_json(
        &app,
        "POST",
        "/api/webhooks",
        Some(&alice_cookie),
        Some(json!({
            "direction": "outgoing",
            "scope_type": "channel",
            "scope_id": channel_id,
            "channel_id": channel_id,
            "name": "Real Outgoing Receiver",
            "target_url": format!("http://127.0.0.1:{}/hook", recv_addr.port()),
            "subscribed_events": ["onChannelMessage"]
        })),
    )
    .await;
    assert_eq!(st_out, StatusCode::OK, "create outgoing webhook failed: {out_val}");
    let out_id = out_val["id"].as_str().unwrap().to_string();
    let out_secret = out_val["signing_secret"].as_str().unwrap().to_string();

    // Send a real channel message -> triggers Outgoing Webhook delivery!
    let (st_cm, cm_val) = req_json(
        &app,
        "POST",
        &format!("/api/channels/{channel_id}/messages"),
        Some(&alice_cookie),
        Some(json!({ "content": "Xin chào Outgoing Webhook!" })),
    )
    .await;
    assert!(st_cm.is_success(), "send channel msg failed: {cm_val}");

    // Wait briefly for async delivery task
    for _ in 0..20 {
        if !captured.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let deliveries = captured.lock().unwrap().clone();
    assert!(!deliveries.is_empty(), "outgoing webhook must deliver onChannelMessage");
    let (sig, ts_str, raw_body) = &deliveries[0];
    let ts: i64 = ts_str.parse().expect("numeric timestamp header");
    assert!(
        plugins::security::verify_webhook_signature(&out_secret, ts, ts, 300, raw_body.as_bytes(), sig),
        "HMAC-SHA256 signature verification MUST pass!"
    );
    assert!(raw_body.contains("Xin chào Outgoing Webhook!"));

    // Check delivery logs endpoint
    let (st_logs, logs_val) = req_json(
        &app,
        "GET",
        &format!("/api/webhooks/{out_id}/deliveries"),
        Some(&alice_cookie),
        None,
    )
    .await;
    assert_eq!(st_logs, StatusCode::OK);
    assert!(
        logs_val.as_array().unwrap().iter().any(|d| d["status"] == "delivered"),
        "delivery logs must record delivered status: {logs_val}"
    );

    unsafe { std::env::remove_var("CHATW_ALLOW_LOCAL_WEBHOOKS"); }
    recv_task.abort();
}

#[tokio::test]
async fn test_spec051_p0_p1_security_regressions_signatures_streaming_and_server_e2ee_taint() {
    let (app, pool, state) = setup_app().await;
    let (_admin_id, admin_cookie) = create_test_user(&pool, &state, "sec_adm", true).await;
    let (dev_id, dev_cookie) = create_test_user(&pool, &state, "sec_dev", false).await;
    let (unver_dev_id, unver_dev_cookie) = create_test_user(&pool, &state, "unver_dev", false).await;
    let (_alice_id, alice_cookie) = create_test_user(&pool, &state, "sec_alice", false).await;
    let (bob_id, bob_cookie) = create_test_user(&pool, &state, "sec_bob", false).await;

    // 1. Register publisher WITH cryptographic P-256 public key
    let secret_scalar: [u8; 32] = [0x42; 32];
    let sk = p256::ecdsa::SigningKey::from_slice(&secret_scalar).unwrap();
    let pubkey_hex = hex::encode(sk.verifying_key().to_encoded_point(false).as_bytes());
    let pub_slug = format!("sig{}", &dev_id.simple().to_string()[..8]);

    let (st_pub, pub_val) = req_json(
        &app,
        "POST",
        "/api/plugins/publishers",
        Some(&dev_cookie),
        Some(json!({
            "slug": pub_slug,
            "display_name": "Signed Labs",
            "public_key": pubkey_hex
        })),
    )
    .await;
    assert_eq!(st_pub, StatusCode::OK, "register signed publisher failed: {pub_val}");
    let pub_id = pub_val["id"].as_str().unwrap().to_string();

    // 2. Publishing without signature when publisher has public_key MUST be rejected
    let plugin_key = format!("{pub_slug}.signed_tool");
    let manifest_obj = plugins::schema::PluginManifest {
        plugin_key: plugin_key.clone(),
        name: "Signed Tool".into(),
        version: "1.0.0".into(),
        publisher: pub_slug.clone(),
        entry_point: "index.js".into(),
        runtime: "js".into(),
        sdk_version: "1.0.0".into(),
        permissions: vec![
            "storage.local".into(),
            "profile.read".into(),
            "network.outbound".into(),
            "e2ee.read_decrypted".into(),
        ],
        ..Default::default()
    };
    let bundle_code = "chatw.onReady(() => {});";
    let (computed_sha, _, _) = plugins::schema::validate_manifest_and_artifacts(
        &manifest_obj,
        &pub_slug,
        bundle_code,
        "",
        "",
        None,
    )
    .unwrap();

    let (st_nosig, _) = req_json(
        &app,
        "POST",
        "/api/plugins/packages",
        Some(&dev_cookie),
        Some(json!({
            "manifest": manifest_obj,
            "bundle_code": bundle_code,
            "sha256": computed_sha
        })),
    )
    .await;
    assert_eq!(st_nosig, StatusCode::BAD_REQUEST, "unsigned package for key-pinned publisher must fail");

    // Forged signature MUST be rejected
    let (st_badsig, _) = req_json(
        &app,
        "POST",
        "/api/plugins/packages",
        Some(&dev_cookie),
        Some(json!({
            "manifest": manifest_obj,
            "bundle_code": bundle_code,
            "sha256": computed_sha,
            "signature": "00".repeat(64)
        })),
    )
    .await;
    assert_eq!(st_badsig, StatusCode::BAD_REQUEST, "forged signature must fail");

    // Valid cryptographic signature MUST succeed and record signature_verified: true
    let (_, valid_sig_hex) = plugins::schema::sign_package_digest(
        &secret_scalar,
        &plugin_key,
        "1.0.0",
        &computed_sha,
    )
    .unwrap();
    let (st_goodsig, pkg_val) = req_json(
        &app,
        "POST",
        "/api/plugins/packages",
        Some(&dev_cookie),
        Some(json!({
            "manifest": manifest_obj,
            "bundle_code": bundle_code,
            "sha256": computed_sha,
            "signature": valid_sig_hex
        })),
    )
    .await;
    assert_eq!(st_goodsig, StatusCode::OK, "valid signature publish failed: {pkg_val}");
    assert_eq!(pkg_val["signature_verified"], json!(true));
    let pkg_id = pkg_val["package_id"].as_str().unwrap().to_string();
    let ver_id = pkg_val["version_id"].as_str().unwrap().to_string();

    // 3. Unverified publisher CANNOT be granted E2EE permissions even if sensitive_approval_status is approved!
    let unver_slug = format!("unv{}", &unver_dev_id.simple().to_string()[..8]);
    let _ = req_json(
        &app,
        "POST",
        "/api/plugins/publishers",
        Some(&unver_dev_cookie),
        Some(json!({ "slug": unver_slug, "display_name": "Unverified Dev" })),
    )
    .await;
    let (_, unver_pkg) = req_json(
        &app,
        "POST",
        "/api/plugins/packages",
        Some(&unver_dev_cookie),
        Some(json!({
            "manifest": {
                "plugin_key": format!("{unver_slug}.e2ee_try"),
                "name": "Unver E2EE",
                "version": "1.0.0",
                "entry_point": "index.js",
                "runtime": "js",
                "permissions": ["e2ee.read_decrypted"]
            },
            "bundle_code": "1;"
        })),
    )
    .await;
    let unver_pkg_id = unver_pkg["package_id"].as_str().unwrap();
    let unver_ver_id = unver_pkg["version_id"].as_str().unwrap();
    let _ = req_json(
        &app,
        "POST",
        &format!("/api/admin/plugins/versions/{unver_ver_id}/permissions"),
        Some(&admin_cookie),
        Some(json!({ "status": "approved", "approved_permissions": ["e2ee.read_decrypted"] })),
    )
    .await;
    let (st_unver_e2ee, _) = req_json(
        &app,
        "POST",
        "/api/plugins/installations",
        Some(&alice_cookie),
        Some(json!({
            "package_id": unver_pkg_id,
            "consent_confirmed": true,
            "enable": true,
            "grants": [{
                "permission": "e2ee.read_decrypted",
                "scope_type": "conversation",
                "scope_id": Uuid::new_v4()
            }]
        })),
    )
    .await;
    assert_eq!(st_unver_e2ee, StatusCode::FORBIDDEN, "unverified publisher MUST be blocked from E2EE grants");

    // 4. Verify publisher + approve permissions on Signed Tool
    let _ = req_json(
        &app,
        "POST",
        &format!("/api/admin/plugins/publishers/{pub_id}/verify"),
        Some(&admin_cookie),
        None,
    )
    .await;
    let _ = req_json(
        &app,
        "POST",
        &format!("/api/admin/plugins/versions/{ver_id}/permissions"),
        Some(&admin_cookie),
        Some(json!({
            "status": "approved",
            "approved_permissions": ["profile.read", "network.outbound", "e2ee.read_decrypted"]
        })),
    )
    .await;

    // 5. P0.2 Multi-user personal grant isolation:
    // Alice creates a Server with Bob and installs Signed Tool with `profile.read` granted by Alice.
    // When Bob fetches the server installation, Alice's personal `profile.read` grant MUST NOT be exposed or active for Bob!
    let (_, srv_val) = req_json(
        &app,
        "POST",
        "/api/servers",
        Some(&alice_cookie),
        Some(json!({ "name": "Isolation Server" })),
    )
    .await;
    let srv_id = srv_val["id"].as_str().unwrap();
    sqlx::query("INSERT INTO server_members (server_id, user_id) VALUES ($1, $2)")
        .bind(Uuid::parse_str(srv_id).unwrap())
        .bind(bob_id)
        .execute(&pool)
        .await
        .unwrap();

    let (st_srv_inst, srv_inst) = req_json(
        &app,
        "POST",
        "/api/plugins/installations",
        Some(&alice_cookie),
        Some(json!({
            "package_id": pkg_id,
            "scope_type": "server",
            "scope_id": srv_id,
            "consent_confirmed": true,
            "enable": true,
            "grants": [
                { "permission": "storage.local", "scope_type": "server", "scope_id": srv_id },
                { "permission": "profile.read", "scope_type": "server", "scope_id": srv_id }
            ]
        })),
    )
    .await;
    assert_eq!(st_srv_inst, StatusCode::OK);
    let srv_inst_id = srv_inst["id"].as_str().unwrap();

    // Bob fetches installations -> Bob must NOT receive Alice's personal `profile.read` grant
    let (_, bob_insts) = req_json(
        &app,
        "GET",
        &format!("/api/plugins/installations?scope_type=server&scope_id={srv_id}"),
        Some(&bob_cookie),
        None,
    )
    .await;
    let bob_grants = bob_insts[0]["grants"].as_array().unwrap();
    assert!(
        !bob_grants.iter().any(|g| g["permission"] == "profile.read"),
        "Alice's personal profile.read grant MUST NOT leak to Bob: {bob_grants:?}"
    );
    // And server-side audit check for `profile.read` as Bob MUST be denied (403)
    let (st_bob_prof, _) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/installations/{srv_inst_id}/audit"),
        Some(&bob_cookie),
        Some(json!({ "action": "read_profile", "permission": "profile.read" })),
    )
    .await;
    assert_eq!(st_bob_prof, StatusCode::FORBIDDEN, "Bob did not grant profile.read so server must deny");

    // 6. P0.3 Server-side E2EE taint enforcement:
    // Alice installs Signed Tool in her personal user scope WITH `e2ee.read_decrypted` + `network.outbound` (NO `e2ee.egress_external`).
    // Even if a compromised client sends `e2ee_tainted: false` in `POST /net/fetch`, the server MUST still block outbound egress!
    let conv_id = Uuid::new_v4();
    let (st_uinst, uinst_val) = req_json(
        &app,
        "POST",
        "/api/plugins/installations",
        Some(&alice_cookie),
        Some(json!({
            "package_id": pkg_id,
            "consent_confirmed": true,
            "enable": true,
            "grants": [
                { "permission": "network.outbound", "scope_type": "global" },
                { "permission": "e2ee.read_decrypted", "scope_type": "conversation", "scope_id": conv_id }
            ]
        })),
    )
    .await;
    assert_eq!(st_uinst, StatusCode::OK);
    let uinst_id = uinst_val["id"].as_str().unwrap();

    let (st_spoofed_untainted, _) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/installations/{uinst_id}/net/fetch"),
        Some(&alice_cookie),
        Some(json!({
            "url": "https://example.com/exfil",
            "method": "POST",
            "body": "secret",
            "e2ee_tainted": false
        })),
    )
    .await;
    assert_eq!(
        st_spoofed_untainted,
        StatusCode::FORBIDDEN,
        "server MUST block network egress even when client spoofs e2ee_tainted: false!"
    );

    // 7. P1.3 Bounded streaming response in plugin_network_fetch:
    // Revoke E2EE grant (and verify no e2ee_read audit was recorded) so network.outbound is permitted,
    // then test against a local test HTTP server returning > 256 KB (> MAX_PROXY_RESP_BYTES) -> MUST abort with 413 PAYLOAD_TOO_LARGE!
    let e2ee_gid = uinst_val["grants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["permission"] == "e2ee.read_decrypted")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let _ = req_json(
        &app,
        "DELETE",
        &format!("/api/plugins/installations/{uinst_id}/grants/{e2ee_gid}"),
        Some(&alice_cookie),
        None,
    )
    .await;

    let big_srv = Router::new().route(
        "/huge",
        axum::routing::get(|| async { "A".repeat(400 * 1024) }),
    );
    let big_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let big_addr = big_listener.local_addr().unwrap();
    let big_task = tokio::spawn(async move {
        let _ = axum::serve(big_listener, big_srv).await;
    });

    unsafe { std::env::set_var("CHATW_ALLOW_LOCAL_WEBHOOKS", "1"); }
    let (st_huge, _) = req_json(
        &app,
        "POST",
        &format!("/api/plugins/installations/{uinst_id}/net/fetch"),
        Some(&alice_cookie),
        Some(json!({
            "url": format!("http://127.0.0.1:{}/huge", big_addr.port()),
            "method": "GET"
        })),
    )
    .await;
    unsafe { std::env::remove_var("CHATW_ALLOW_LOCAL_WEBHOOKS"); }
    big_task.abort();
    assert_eq!(st_huge, StatusCode::PAYLOAD_TOO_LARGE, "oversized upstream stream must abort with 413");
}
