use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode, header},
    response::Response,
    routing::any,
};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::Write,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::net::TcpListener;
use tower::ServiceExt;
use uuid::Uuid;

use chatw::{
    attachments, audit,
    auth::{self, AppState, google::GoogleJwksCache, session::create_session},
    backup, cloud,
    config::Config,
    crypto, db, devices_sync, friends, groups, messages, privacy, servers, storage, users,
    websocket::{self, WsHub},
};

fn test_config(tag: &str) -> Config {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://chatw:chatw_dev_password@%2Ftmp%2Fkiyo-pg-run/chatw".into()
    });
    Config {
        database_url,
        db_max_connections: 5,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        public_origin: "http://localhost:8085".into(),
        google_client_id: "test-client-id".into(),
        google_client_secret: "test-client-secret".into(),
        google_redirect_url: "http://localhost:8085/api/auth/google/callback".into(),
        session_secret: b"test-session-secret-at-least-32-bytes-long!".to_vec(),
        session_ttl: Duration::from_secs(3600),
        cookie_secure: false,
        static_dir: None,
        uploads_dir: std::env::temp_dir().join(format!("chatw_spec027_{tag}_{}", Uuid::new_v4().simple())),
        max_ws_connections: 100,
        trust_proxy: false,
        dev_login_enabled: false,
        google_configured: false,
    }
}

async fn build_app(pool: sqlx::PgPool, config: Config) -> (Router, Arc<storage::StorageRegistry>) {
    let storage_reg = Arc::new(storage::StorageRegistry::from_uploads_dir(&config.uploads_dir));
    let state = AppState {
        config: Arc::new(config),
        db: pool,
        http_client: reqwest::Client::new(),
        jwks_cache: Arc::new(tokio::sync::RwLock::new(GoogleJwksCache::new())),
        ws_hub: Arc::new(WsHub::new(100)),
        oauth_tickets: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        storage: storage_reg.clone(),
        sfu: Arc::new(chatw::calls::sfu::SfuHandle::new()),
    };
    let router = Router::new()
        .nest("/api/auth", auth::routes())
        .nest("/api/users", users::routes())
        .nest("/api/keys", crypto::keys::routes().merge(devices_sync::key_backup_routes()))
        .nest("/api/devices", devices_sync::device_routes())
        .nest("/api/sync", devices_sync::sync_routes())
        .nest("/api/recovery", devices_sync::recovery_routes())
        .nest("/api/storage", storage::router::routes())
        .nest("/api/backup", backup::routes())
        .nest("/api/cloud", cloud::router::routes())
        .nest("/api/friends", friends::routes())
        .nest("/api/groups", groups::routes())
        .nest("/api/messages", messages::routes())
        .nest("/api/attachments", attachments::routes())
        .nest("/api/audit", audit::routes())
        .nest("/api/servers", servers::server_routes())
        .nest("/api/channels", servers::channel_routes())
        .nest("/api/privacy", privacy::routes())
        .merge(websocket::routes())
        .with_state(state);
    (router, storage_reg)
}

async fn setup_db() -> sqlx::PgPool {
    let cfg = test_config("mig");
    let pool = db::connect(&cfg.database_url, 5).await.expect("connect pg");
    db::run_migrations(&pool).await.expect("migrations");
    pool
}

async fn create_test_user(pool: &sqlx::PgPool, config: &Config, name: &str) -> (Uuid, String, String) {
    let uid = Uuid::new_v4();
    let suffix = &Uuid::new_v4().simple().to_string()[..6];
    let uname = format!("{name}_{suffix}");
    let norm = uname.to_lowercase();
    let sub = format!("sub_{uid}");
    sqlx::query("INSERT INTO users (id, google_sub, username, username_norm) VALUES ($1, $2, $3, $4)")
        .bind(uid)
        .bind(&sub)
        .bind(&uname)
        .bind(&norm)
        .execute(pool)
        .await
        .expect("insert user");
    let raw_cookie = create_session(pool, config, uid, None).await.expect("session");
    let cookie = raw_cookie.split(';').next().unwrap().to_string();
    (uid, uname, cookie)
}

async fn make_friends(pool: &sqlx::PgPool, u1: Uuid, u2: Uuid) {
    let (a, b) = if u1 < u2 { (u1, u2) } else { (u2, u1) };
    sqlx::query("INSERT INTO friendships (user_a, user_b) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(a)
        .bind(b)
        .execute(pool)
        .await
        .expect("make friends");
}

async fn json_req(
    app: &Router,
    method: &str,
    path: &str,
    cookie: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let req_body = match body {
        Some(v) => Body::from(v.to_string()),
        None => Body::empty(),
    };
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(req_body)
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

async fn raw_req(
    app: &Router,
    method: &str,
    path: &str,
    cookie: &str,
    extra_headers: &[(&str, &str)],
    body: Vec<u8>,
) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie);
    for (k, v) in extra_headers {
        builder = builder.header(*k, *v);
    }
    let req = builder.body(Body::from(body)).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    (status, bytes)
}

#[tokio::test]
async fn test_reauth_security_and_rate_limit() {
    let pool = setup_db().await;
    let cfg = test_config("reauth");
    let (app, _) = build_app(pool.clone(), cfg.clone()).await;
    let (uid, _, cookie) = create_test_user(&pool, &cfg, "alice_reauth").await;

    // 1. Direct export without reauth token is blocked even with a valid session cookie
    let (st, v) = json_req(
        &app,
        "POST",
        "/api/backup/export/kdata",
        &cookie,
        Some(json!({ "encryption_password": "ArchivePassword#1" })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert_eq!(v["error"], "export_reauth_required");

    // 2. Set export password
    let (st, _) = json_req(
        &app,
        "PUT",
        "/api/backup/reauth/password",
        &cookie,
        Some(json!({ "new_password": "ExportSecret#2026" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // 3. 5 wrong attempts return 401 and trigger 15m lockout for subsequent attempts
    for _ in 0..5 {
        let (st, _) = json_req(
            &app,
            "POST",
            "/api/backup/reauth",
            &cookie,
            Some(json!({ "scope": "kdata", "method": "password", "secret": "wrong" })),
        )
        .await;
        assert_eq!(st, StatusCode::UNAUTHORIZED);
    }

    // Even valid password must be rejected with 429 while locked
    let (st, _) = json_req(
        &app,
        "POST",
        "/api/backup/reauth",
        &cookie,
        Some(json!({ "scope": "kdata", "method": "password", "secret": "ExportSecret#2026" })),
    )
    .await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS);

    // Clear lock and verify valid reauth succeeds & audit event recorded
    sqlx::query("DELETE FROM reauth_attempts WHERE user_id = $1")
        .bind(uid)
        .execute(&pool)
        .await
        .unwrap();

    let (st, ok_body) = json_req(
        &app,
        "POST",
        "/api/backup/reauth",
        &cookie,
        Some(json!({ "scope": "kdata", "method": "password", "secret": "ExportSecret#2026" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(!ok_body["export_token"].as_str().unwrap_or("").is_empty());

    let ev_cnt: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM security_events WHERE user_id = $1")
        .bind(uid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(ev_cnt >= 2);
}

#[tokio::test]
async fn test_kdata_roundtrip_tamper_dedup_selective_and_cross_account() {
    let pool = setup_db().await;
    let cfg = test_config("kdata");
    let (app, _storage) = build_app(pool.clone(), cfg.clone()).await;

    let (alice, _, alice_cookie) = create_test_user(&pool, &cfg, "alice_kd").await;
    let (bob, _, bob_cookie) = create_test_user(&pool, &cfg, "bob_kd").await;
    make_friends(&pool, alice, bob).await;

    // Upload a real PNG image attachment for Alice inside a group so it sits in durable CAS storage
    let (gst, gval) = json_req(&app, "POST", "/api/groups", &alice_cookie, Some(json!({ "name": "KdataGroup" }))).await;
    assert_eq!(gst, StatusCode::OK);
    let gid = gval["id"].as_str().unwrap().to_string();

    let mut png_bytes = Vec::new();
    let img = image::RgbaImage::from_pixel(16, 16, image::Rgba([20, 140, 240, 255]));
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .unwrap();
    let (st, up_raw) = raw_req(
        &app,
        "POST",
        &format!("/api/attachments?scope_type=group&scope_id={gid}&is_e2ee=false&file_name=pic.png"),
        &alice_cookie,
        &[("content-type", "image/png")],
        png_bytes,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let up_json: Value = serde_json::from_slice(&up_raw).unwrap();
    let att_url = up_json["download_url"].as_str().unwrap().to_string();

    // Insert 2 live DMs + 1 deleted DM + bio
    let dm1_id = Uuid::new_v4();
    let dm2_id = Uuid::new_v4();
    let dm_del_id = Uuid::new_v4();
    let env_a = vec![0xAAu8; 300];
    let env_b = vec![0xBBu8; 300];
    sqlx::query("INSERT INTO messages (id, sender_id, receiver_id, recipient_envelope, sender_envelope) VALUES ($1, $2, $3, $4, $5)")
        .bind(dm1_id)
        .bind(alice)
        .bind(bob)
        .bind(&env_a)
        .bind(&env_b)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO messages (id, sender_id, receiver_id, recipient_envelope, sender_envelope) VALUES ($1, $2, $3, $4, $5)")
        .bind(dm2_id)
        .bind(bob)
        .bind(alice)
        .bind(&env_a)
        .bind(&env_b)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO messages (id, sender_id, receiver_id, recipient_envelope, sender_envelope, deleted_for_all) VALUES ($1, $2, $3, $4, $5, TRUE)")
        .bind(dm_del_id)
        .bind(alice)
        .bind(bob)
        .bind(&env_a)
        .bind(&env_b)
        .execute(&pool)
        .await
        .unwrap();
    let _ = att_url;
    sqlx::query("INSERT INTO user_settings (user_id, data) VALUES ($1, $2)")
        .bind(alice)
        .bind(json!({ "theme": "dark" }))
        .execute(&pool)
        .await
        .unwrap();

    // Reauth & export .kdata
    let _ = json_req(
        &app,
        "PUT",
        "/api/backup/reauth/password",
        &alice_cookie,
        Some(json!({ "new_password": "ExportSecret#2026" })),
    )
    .await;
    let (_, reauth) = json_req(
        &app,
        "POST",
        "/api/backup/reauth",
        &alice_cookie,
        Some(json!({ "scope": "kdata", "method": "password", "secret": "ExportSecret#2026" })),
    )
    .await;
    let export_tok = reauth["export_token"].as_str().unwrap().to_string();

    let (st, kdata_bytes) = raw_req(
        &app,
        "POST",
        "/api/backup/export/kdata",
        &alice_cookie,
        &[("content-type", "application/json")],
        serde_json::to_vec(&json!({
            "export_token": export_tok,
            "encryption_password": "ContainerPass#999",
            "include_account": true,
            "include_messages": true,
            "include_media": true,
            "include_settings": true,
        }))
        .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(kdata_bytes.starts_with(b"KIYODATA"));
    assert!(kdata_bytes.windows(8).any(|w| w == b"KIYOEND1"));

    // Wrong password rejected
    let (st, _) = raw_req(
        &app,
        "POST",
        "/api/backup/inspect",
        &alice_cookie,
        &[("x-backup-password", "WrongPass123")],
        kdata_bytes.clone(),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // 1-byte tamper in ciphertext rejected
    let mut tampered = kdata_bytes.clone();
    let mid = tampered.len() / 2;
    tampered[mid] ^= 0x55;
    let (st, _) = raw_req(
        &app,
        "POST",
        "/api/backup/inspect",
        &alice_cookie,
        &[("x-backup-password", "ContainerPass#999")],
        tampered,
    )
    .await;
    assert!(st.is_client_error());

    // Truncated stream rejected
    let truncated = kdata_bytes[..kdata_bytes.len() - 40].to_vec();
    let (st, _) = raw_req(
        &app,
        "POST",
        "/api/backup/inspect",
        &alice_cookie,
        &[("x-backup-password", "ContainerPass#999")],
        truncated,
    )
    .await;
    assert!(st.is_client_error());

    // Valid inspect
    let (st, insp_raw) = raw_req(
        &app,
        "POST",
        "/api/backup/inspect",
        &alice_cookie,
        &[("x-backup-password", "ContainerPass#999")],
        kdata_bytes.clone(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let insp: Value = serde_json::from_slice(&insp_raw).unwrap();
    assert_eq!(insp["integrity_verified"], true);
    assert_eq!(insp["total_messages"], 2); // Deleted DM excluded!
    assert_eq!(insp["duplicate_messages"], 2);
    assert_eq!(insp["new_messages"], 0);
    assert!(insp["total_media"].as_u64().unwrap() >= 1);

    // Cross-account restore blocked
    let (st, cross_raw) = raw_req(
        &app,
        "POST",
        "/api/backup/restore?mode=full",
        &bob_cookie,
        &[("x-backup-password", "ContainerPass#999")],
        kdata_bytes.clone(),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let cross_v: Value = serde_json::from_slice(&cross_raw).unwrap();
    assert_eq!(cross_v["error"], "backup_belongs_to_another_account");

    // Repeated full restore inserts zero duplicates
    let (st, rr1_raw) = raw_req(
        &app,
        "POST",
        "/api/backup/restore?mode=full",
        &alice_cookie,
        &[("x-backup-password", "ContainerPass#999")],
        kdata_bytes.clone(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let rr1: Value = serde_json::from_slice(&rr1_raw).unwrap();
    assert_eq!(rr1["restored_messages"], 0);
    assert_eq!(rr1["skipped_duplicate_messages"], 2);

    // Delete 1 DM message & delete user_settings, then test selective modes (`messages` vs `settings`)
    sqlx::query("DELETE FROM messages WHERE id = $1")
        .bind(dm1_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM user_settings WHERE user_id = $1")
        .bind(alice)
        .execute(&pool)
        .await
        .unwrap();

    let (st, rr_msg_raw) = raw_req(
        &app,
        "POST",
        "/api/backup/restore?mode=messages",
        &alice_cookie,
        &[("x-backup-password", "ContainerPass#999")],
        kdata_bytes.clone(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let rr_msg: Value = serde_json::from_slice(&rr_msg_raw).unwrap();
    assert_eq!(rr_msg["restored_messages"], 1);
    assert_eq!(rr_msg["restored_settings"], false);

    let settings_after_msg: Option<Value> = sqlx::query_scalar("SELECT data FROM user_settings WHERE user_id = $1")
        .bind(alice)
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert!(settings_after_msg.is_none());

    let (st, rr_set_raw) = raw_req(
        &app,
        "POST",
        "/api/backup/restore?mode=settings",
        &alice_cookie,
        &[("x-backup-password", "ContainerPass#999")],
        kdata_bytes,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let rr_set: Value = serde_json::from_slice(&rr_set_raw).unwrap();
    assert_eq!(rr_set["restored_settings"], true);
    let settings_restored: Option<Value> = sqlx::query_scalar("SELECT data FROM user_settings WHERE user_id = $1")
        .bind(alice)
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert_eq!(settings_restored.unwrap()["theme"], "dark");
}

#[tokio::test]
async fn test_unencrypted_zip_locked_multi_step_and_zip_security() {
    let pool = setup_db().await;
    let cfg = test_config("zipsec");
    let (app, _) = build_app(pool.clone(), cfg.clone()).await;
    let (_, _, cookie) = create_test_user(&pool, &cfg, "alice_zip").await;

    let _ = json_req(
        &app,
        "PUT",
        "/api/backup/reauth/password",
        &cookie,
        Some(json!({ "new_password": "ExportSecret#2026" })),
    )
    .await;

    let (_, r1) = json_req(
        &app,
        "POST",
        "/api/backup/reauth",
        &cookie,
        Some(json!({ "scope": "zip_unencrypted", "method": "password", "secret": "ExportSecret#2026" })),
    )
    .await;
    assert_eq!(r1["requires_multi_step"], true);
    let tok1 = r1["export_token"].as_str().unwrap().to_string();

    // Skipping steps must fail 403
    let (st, err_raw) = raw_req(
        &app,
        "POST",
        "/api/backup/export/zip-unencrypted",
        &cookie,
        &[("content-type", "application/json")],
        serde_json::to_vec(&json!({ "export_token": tok1 })).unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let err_v: Value = serde_json::from_slice(&err_raw).unwrap();
    assert_eq!(err_v["error"], "export_reauth_required");

    // Step 1: ack_risk -> returns preview
    let (st, ack_v) = json_req(
        &app,
        "POST",
        "/api/backup/unencrypted-zip/unlock-step",
        &cookie,
        Some(json!({ "export_token": tok1, "step": "ack_risk" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(ack_v["ok"], true);
    assert!(ack_v["preview"].is_object());

    // Step 2: confirm_final
    let (st, _) = json_req(
        &app,
        "POST",
        "/api/backup/unencrypted-zip/unlock-step",
        &cookie,
        Some(json!({ "export_token": tok1, "step": "confirm_final" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Single-use export succeeds
    let (st, zip_bytes) = raw_req(
        &app,
        "POST",
        "/api/backup/export/zip-unencrypted",
        &cookie,
        &[("content-type", "application/json")],
        serde_json::to_vec(&json!({ "export_token": tok1 })).unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(zip_bytes.starts_with(b"PK\x03\x04"));

    // Second reuse must be rejected
    let (st, _) = raw_req(
        &app,
        "POST",
        "/api/backup/export/zip-unencrypted",
        &cookie,
        &[("content-type", "application/json")],
        serde_json::to_vec(&json!({ "export_token": tok1 })).unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Path traversal ZIP rejected
    let mut bad_path_zip = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut bad_path_zip));
        let opts = zip::write::SimpleFileOptions::default();
        zw.start_file("../evil.txt", opts).unwrap();
        zw.write_all(b"evil").unwrap();
        zw.finish().unwrap();
    }
    let (st, bp_err) = raw_req(
        &app,
        "POST",
        "/api/backup/inspect",
        &cookie,
        &[],
        bad_path_zip,
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let bp_v: Value = serde_json::from_slice(&bp_err).unwrap();
    assert_eq!(bp_v["error"], "unsafe_path_rejected");

    // ZIP bomb ratio (>100x) rejected
    let mut bomb_zip = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut bomb_zip));
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zw.start_file("bomb.bin", opts).unwrap();
        zw.write_all(&vec![0u8; 4 * 1024 * 1024]).unwrap();
        zw.finish().unwrap();
    }
    let (st, bomb_err) = raw_req(
        &app,
        "POST",
        "/api/backup/inspect",
        &cookie,
        &[],
        bomb_zip,
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let bomb_v: Value = serde_json::from_slice(&bomb_err).unwrap();
    assert_eq!(bomb_v["error"], "zip_bomb_rejected");
}

#[derive(Clone)]
struct MockDavState {
    files: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    dirs: Arc<RwLock<std::collections::HashSet<String>>>,
}

async fn mock_dav_handler(
    State(st): State<MockDavState>,
    req: Request<Body>,
) -> Response {
    let auth = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let expected = format!("Basic {}", B64.encode("davuser:davpass"));
    if auth != expected {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .unwrap();
    }

    let method = req.method().clone();
    let path = req.uri().path().trim_matches('/').to_string();
    let body = axum::body::to_bytes(req.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap_or_default()
        .to_vec();

    if method.as_str() == "PROPFIND" {
        if path.is_empty() || st.dirs.read().unwrap().contains(&path) {
            let xml = format!(
                r#"<?xml version="1.0" encoding="utf-8"?>
                <d:multistatus xmlns:d="DAV:">
                  <d:response>
                    <d:href>/{path}/</d:href>
                    <d:propstat><d:prop>
                      <d:quota-available-bytes>1073741824</d:quota-available-bytes>
                      <d:quota-used-bytes>1024</d:quota-used-bytes>
                    </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>
                  </d:response>
                </d:multistatus>"#
            );
            return Response::builder()
                .status(StatusCode::MULTI_STATUS)
                .body(Body::from(xml))
                .unwrap();
        }
        return Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap();
    }

    if method.as_str() == "MKCOL" {
        st.dirs.write().unwrap().insert(path);
        return Response::builder().status(StatusCode::CREATED).body(Body::empty()).unwrap();
    }

    match method {
        Method::PUT => {
            st.files.write().unwrap().insert(path, body);
            Response::builder().status(StatusCode::CREATED).body(Body::empty()).unwrap()
        }
        Method::GET => {
            if let Some(data) = st.files.read().unwrap().get(&path).cloned() {
                Response::builder().status(StatusCode::OK).body(Body::from(data)).unwrap()
            } else {
                Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()
            }
        }
        Method::HEAD => {
            if let Some(data) = st.files.read().unwrap().get(&path) {
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_LENGTH, data.len().to_string())
                    .body(Body::empty())
                    .unwrap()
            } else {
                Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()
            }
        }
        Method::DELETE => {
            st.files.write().unwrap().remove(&path);
            Response::builder().status(StatusCode::NO_CONTENT).body(Body::empty()).unwrap()
        }
        _ => Response::builder().status(StatusCode::METHOD_NOT_ALLOWED).body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn test_cloud_webdav_incremental_verify_tamper_restore_and_safe_gc() {
    unsafe {
        std::env::set_var("ALLOW_LOOPBACK_CLOUD_TEST", "1");
    }

    let dav_state = MockDavState {
        files: Arc::new(RwLock::new(HashMap::new())),
        dirs: Arc::new(RwLock::new(std::collections::HashSet::from(["dav".to_string()]))),
    };
    let dav_router = Router::new()
        .fallback(any(mock_dav_handler))
        .with_state(dav_state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dav_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, dav_router).await;
    });

    let pool = setup_db().await;
    let cfg = test_config("cloud");
    let (app, _) = build_app(pool.clone(), cfg.clone()).await;
    let (alice, _, alice_cookie) = create_test_user(&pool, &cfg, "alice_cloud").await;
    let (bob, _, _) = create_test_user(&pool, &cfg, "bob_cloud").await;
    make_friends(&pool, alice, bob).await;

    // Upload durable media in a group + insert DM
    let (gst, gval) = json_req(&app, "POST", "/api/groups", &alice_cookie, Some(json!({ "name": "CloudGroup" }))).await;
    assert_eq!(gst, StatusCode::OK);
    let gid = gval["id"].as_str().unwrap().to_string();

    let mut png_bytes = Vec::new();
    let img = image::RgbaImage::from_pixel(24, 24, image::Rgba([220, 40, 80, 255]));
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .unwrap();
    let (st, up_raw) = raw_req(
        &app,
        "POST",
        &format!("/api/attachments?scope_type=group&scope_id={gid}&is_e2ee=false&file_name=cloud_pic.png"),
        &alice_cookie,
        &[("content-type", "image/png")],
        png_bytes,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let up_v: Value = serde_json::from_slice(&up_raw).unwrap();
    let att_url = up_v["download_url"].as_str().unwrap().to_string();

    let dm1 = Uuid::new_v4();
    let env_a = vec![0xCCu8; 300];
    let env_b = vec![0xDDu8; 300];
    sqlx::query("INSERT INTO messages (id, sender_id, receiver_id, recipient_envelope, sender_envelope) VALUES ($1, $2, $3, $4, $5)")
        .bind(dm1)
        .bind(alice)
        .bind(bob)
        .bind(&env_a)
        .bind(&env_b)
        .execute(&pool)
        .await
        .unwrap();
    let _ = att_url;

    // 1. Connect WebDAV
    let (st, conn_v) = json_req(
        &app,
        "POST",
        "/api/cloud/connections",
        &alice_cookie,
        Some(json!({
            "provider": "webdav",
            "account_label": "Local Test DAV",
            "config": {
                "kind": "webdav",
                "endpoint": format!("http://127.0.0.1:{}/dav", dav_addr.port()),
                "username": "davuser",
                "password": "davpass",
                "folder": "kiyo_backups"
            }
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let conn_id = conn_v["id"].as_str().unwrap().to_string();

    // 2. Snapshot #1 (full upload of initial chunks)
    let (st, run1) = json_req(
        &app,
        "POST",
        &format!("/api/cloud/connections/{conn_id}/backup"),
        &alice_cookie,
        Some(json!({ "encryption_password": "CloudSecret#2026" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(run1["snapshot_seq"], 1);
    assert_eq!(run1["verified"], true);
    let snap1_id = run1["snapshot_id"].as_str().unwrap().to_string();
    let up1 = run1["uploaded_delta_bytes"].as_u64().unwrap();
    assert!(up1 > 0);
    assert!(run1["new_chunks"].as_u64().unwrap() >= 2);

    // Add 1 small DM without changing media
    let dm2 = Uuid::new_v4();
    sqlx::query("INSERT INTO messages (id, sender_id, receiver_id, recipient_envelope, sender_envelope) VALUES ($1, $2, $3, $4, $5)")
        .bind(dm2)
        .bind(alice)
        .bind(bob)
        .bind(&env_a)
        .bind(&env_b)
        .execute(&pool)
        .await
        .unwrap();

    // 3. Snapshot #2 (incremental delta must reuse unchanged media chunks!)
    let (st, run2) = json_req(
        &app,
        "POST",
        &format!("/api/cloud/connections/{conn_id}/backup"),
        &alice_cookie,
        Some(json!({ "encryption_password": "CloudSecret#2026" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(run2["snapshot_seq"], 2);
    assert_eq!(run2["verified"], true);
    let snap2_id = run2["snapshot_id"].as_str().unwrap().to_string();
    assert!(run2["reused_chunks"].as_u64().unwrap() >= 1);

    // 4. Corrupt one chunk on remote WebDAV -> restore must detect & reject!
    let chunk_key = {
        let map = dav_state.files.read().unwrap();
        map.keys()
            .find(|k| k.contains("/chunks/"))
            .unwrap()
            .clone()
    };
    let orig_chunk = dav_state.files.read().unwrap().get(&chunk_key).unwrap().clone();
    {
        let mut map = dav_state.files.write().unwrap();
        let mut corrupted = orig_chunk.clone();
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0xFF;
        map.insert(chunk_key.clone(), corrupted);
    }
    let (st, _) = json_req(
        &app,
        "POST",
        &format!("/api/cloud/connections/{conn_id}/restore"),
        &alice_cookie,
        Some(json!({
            "snapshot_id": snap2_id,
            "encryption_password": "CloudSecret#2026",
            "mode": "full"
        })),
    )
    .await;
    assert!(st.is_client_error() || st.is_server_error());

    // Restore valid chunk back
    dav_state.files.write().unwrap().insert(chunk_key, orig_chunk);

    // 5. Delete DM2 from DB, restore snapshot #2 -> DM2 comes back cleanly
    sqlx::query("DELETE FROM messages WHERE id = $1")
        .bind(dm2)
        .execute(&pool)
        .await
        .unwrap();
    let (st, rr_cloud) = json_req(
        &app,
        "POST",
        &format!("/api/cloud/connections/{conn_id}/restore"),
        &alice_cookie,
        Some(json!({
            "snapshot_id": snap2_id,
            "encryption_password": "CloudSecret#2026",
            "mode": "full"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(rr_cloud["restored_messages"], 1);
    assert_eq!(rr_cloud["integrity_verified"], true);

    // 6. Safe GC: delete Snapshot #1 -> shared chunks for Snapshot #2 must remain intact & verifiable!
    let (st, _) = json_req(
        &app,
        "DELETE",
        &format!("/api/cloud/connections/{conn_id}/backups/{snap1_id}"),
        &alice_cookie,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    let (st, ver_v) = json_req(
        &app,
        "POST",
        &format!("/api/cloud/connections/{conn_id}/verify"),
        &alice_cookie,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(ver_v["latest_snapshot_verified"], true);

    // Restore snapshot #2 again after GC of snapshot #1 succeeds cleanly
    let (st, rr_after_gc) = json_req(
        &app,
        "POST",
        &format!("/api/cloud/connections/{conn_id}/restore"),
        &alice_cookie,
        Some(json!({
            "snapshot_id": snap2_id,
            "encryption_password": "CloudSecret#2026",
            "mode": "full"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(rr_after_gc["integrity_verified"], true);
}

#[derive(Clone)]
struct MockS3State {
    objects: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

async fn mock_s3_handler(
    State(st): State<MockS3State>,
    req: Request<Body>,
) -> Response {
    let auth = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !auth.starts_with("AWS4-HMAC-SHA256") {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::empty())
            .unwrap();
    }

    let method = req.method().clone();
    let path = req.uri().path().trim_matches('/').to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let body = axum::body::to_bytes(req.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap_or_default()
        .to_vec();

    // Expect path style `{bucket}/...`
    let mut split = path.splitn(2, '/');
    let _bucket = split.next().unwrap_or("");
    let key = split.next().unwrap_or("").to_string();

    if method == Method::GET && query.contains("list-type=2") {
        let map = st.objects.read().unwrap();
        let mut contents_xml = String::new();
        for (k, v) in map.iter() {
            contents_xml.push_str(&format!(
                "<Contents><Key>{k}</Key><Size>{}</Size></Contents>",
                v.len()
            ));
        }
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><ListBucketResult><IsTruncated>false</IsTruncated>{contents_xml}</ListBucketResult>"#
        );
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/xml")
            .body(Body::from(xml))
            .unwrap();
    }

    match method {
        Method::HEAD => {
            if key.is_empty() {
                return Response::builder().status(StatusCode::OK).body(Body::empty()).unwrap();
            }
            if let Some(d) = st.objects.read().unwrap().get(&key) {
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_LENGTH, d.len().to_string())
                    .body(Body::empty())
                    .unwrap()
            } else {
                Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()
            }
        }
        Method::PUT => {
            st.objects.write().unwrap().insert(key, body);
            Response::builder().status(StatusCode::OK).body(Body::empty()).unwrap()
        }
        Method::GET => {
            if let Some(d) = st.objects.read().unwrap().get(&key).cloned() {
                Response::builder().status(StatusCode::OK).body(Body::from(d)).unwrap()
            } else {
                Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()
            }
        }
        Method::DELETE => {
            st.objects.write().unwrap().remove(&key);
            Response::builder().status(StatusCode::NO_CONTENT).body(Body::empty()).unwrap()
        }
        _ => Response::builder().status(StatusCode::METHOD_NOT_ALLOWED).body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn test_s3_minio_auto_scheduler_retention_and_passkey_reauth() {
    unsafe {
        std::env::set_var("ALLOW_LOOPBACK_CLOUD_TEST", "1");
    }
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use p256::ecdsa::{Signature, SigningKey, signature::Signer};
    use p256::elliptic_curve::rand_core::OsRng;

    let s3_state = MockS3State {
        objects: Arc::new(RwLock::new(HashMap::new())),
    };
    let s3_router = Router::new()
        .fallback(any(mock_s3_handler))
        .with_state(s3_state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let s3_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, s3_router).await;
    });

    let pool = setup_db().await;
    let cfg = test_config("s3auto");
    let (app, storage_reg) = build_app(pool.clone(), cfg.clone()).await;
    let (alice, _, alice_cookie) = create_test_user(&pool, &cfg, "alice_s3").await;
    let (bob, _, _) = create_test_user(&pool, &cfg, "bob_s3").await;
    make_friends(&pool, alice, bob).await;

    let dm1 = Uuid::new_v4();
    let env_a = vec![0xEEu8; 300];
    let env_b = vec![0xFFu8; 300];
    sqlx::query("INSERT INTO messages (id, sender_id, receiver_id, recipient_envelope, sender_envelope) VALUES ($1, $2, $3, $4, $5)")
        .bind(dm1)
        .bind(alice)
        .bind(bob)
        .bind(&env_a)
        .bind(&env_b)
        .execute(&pool)
        .await
        .unwrap();

    // 1. Register P-256 Passkey -> Challenge -> Sign -> Reauth -> Export .kdata
    let sk = SigningKey::random(&mut OsRng);
    let pk_sec1 = sk.verifying_key().to_encoded_point(false).as_bytes().to_vec();
    let cred_id = format!("cred-test-p256-{}", Uuid::new_v4().simple());
    let (st, reg_v) = json_req(
        &app,
        "POST",
        "/api/backup/reauth/passkeys/register",
        &alice_cookie,
        Some(json!({
            "credential_id": cred_id,
            "public_key_spki_b64": URL_SAFE_NO_PAD.encode(&pk_sec1),
            "label": "Test P256 Key"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(reg_v["id"].as_str().is_some());

    let (st, ch_v) = json_req(
        &app,
        "POST",
        "/api/backup/reauth/challenge",
        &alice_cookie,
        Some(json!({ "purpose": "reauth" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let ch_id = ch_v["challenge_id"].as_str().unwrap();
    let ch_b64 = ch_v["challenge_b64"].as_str().unwrap();
    let sig: Signature = sk.sign(ch_b64.as_bytes());
    let (st, reauth_v) = json_req(
        &app,
        "POST",
        "/api/backup/reauth",
        &alice_cookie,
        Some(json!({
            "scope": "kdata",
            "method": "passkey",
            "challenge_id": ch_id,
            "credential_id": cred_id,
            "signature_b64": URL_SAFE_NO_PAD.encode(sig.to_bytes()),
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let passkey_tok = reauth_v["export_token"].as_str().unwrap();

    let (st, kdata_pk) = raw_req(
        &app,
        "POST",
        "/api/backup/export/kdata",
        &alice_cookie,
        &[("content-type", "application/json")],
        serde_json::to_vec(&json!({
            "export_token": passkey_tok,
            "encryption_password": "PasskeyContainer#2026"
        }))
        .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(kdata_pk.starts_with(b"KIYODATA"));

    // 2. Connect MinIO/S3 (path-style SigV4)
    let (st, s3_conn) = json_req(
        &app,
        "POST",
        "/api/cloud/connections",
        &alice_cookie,
        Some(json!({
            "provider": "minio",
            "account_label": "Mock MinIO S3",
            "config": {
                "kind": "minio",
                "endpoint": format!("http://127.0.0.1:{}", s3_addr.port()),
                "region": "us-east-1",
                "bucket": "kiyo-test",
                "access_key": "minioadmin",
                "secret_key": "minioadminsecret",
                "prefix": "kiyo_backups",
                "force_path_style": true
            }
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let s3_conn_id = Uuid::parse_str(s3_conn["id"].as_str().unwrap()).unwrap();

    // Manual snapshots #1 and #2
    for seq in 1..=2 {
        let (st, r) = json_req(
            &app,
            "POST",
            &format!("/api/cloud/connections/{s3_conn_id}/backup"),
            &alice_cookie,
            Some(json!({ "encryption_password": "S3CloudSecret#2026" })),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(r["snapshot_seq"], seq);
        assert_eq!(r["verified"], true);
    }

    // 3. Enable scheduled auto-backup with retain_snapshots = 2 and execute scheduler
    let (st, pol_v) = json_req(
        &app,
        "PUT",
        "/api/cloud/policy",
        &alice_cookie,
        Some(json!({
            "enabled": true,
            "connection_id": s3_conn_id,
            "schedule_mode": "daily",
            "wifi_only": false,
            "charging_only": false,
            "bandwidth_limit_kbps": 0,
            "storage_limit_mb": 0,
            "retain_snapshots": 2,
            "paused": false,
            "auto_encryption_password": "S3CloudSecret#2026"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(pol_v["has_auto_key"], true);

    sqlx::query("UPDATE cloud_backup_policies SET next_run_at = now() - interval '5 minutes' WHERE user_id = $1")
        .bind(alice)
        .execute(&pool)
        .await
        .unwrap();

    let ran = cloud::incremental::run_due_cloud_backup_policies(
        &pool,
        &storage_reg,
        &reqwest::Client::new(),
        &cfg.session_secret,
    )
    .await
    .unwrap();
    assert!(ran >= 1);

    // Retention = 2 -> snapshot #1 pruned safely, #2 and #3 remain verified!
    let live_snaps: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cloud_snapshots WHERE user_id = $1 AND connection_id = $2 AND status = 'verified'",
    )
    .bind(alice)
    .bind(s3_conn_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(live_snaps, 2);

    let (st, ver_v) = json_req(
        &app,
        "POST",
        &format!("/api/cloud/connections/{s3_conn_id}/verify"),
        &alice_cookie,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(ver_v["latest_snapshot_verified"], true);
}
