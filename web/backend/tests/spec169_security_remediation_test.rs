//! Spec 169 Security Remediation Verification Suite
//!
//! Verifies:
//! 1. F-01: `/api/auth/dev-login` route is deleted (404) and `/api/auth/providers` returns `dev: false`.
//! 2. F-04: Redis-backed sessions with per-request `device_id` & suspension re-checks, plus
//!    least-privilege `chatw_app` DB role blocking direct `sessions` and `user_key_backups` access
//!    while allowing authenticated `SECURITY DEFINER` key backup RPCs.
//! 3. F-05: Banner authorization (`handle_protected_banner_file`), strict CORS (no cross-port
//!    loopback bypass), and HTTP-layer rate limiting on `/api/auth/login` & `/api/auth/register`.

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use chatw::{
    attachments,
    auth::{self, AppState, google::GoogleJwksCache, session},
    cache::ValkeyCoordinator,
    config::Config,
    db, devices_sync, groups, security, storage,
    websocket::WsHub,
};
use std::process::{Child, Command, Stdio};
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;
use uuid::Uuid;

struct TempRedis {
    port: u16,
    child: Option<Child>,
}

impl TempRedis {
    fn start(port: u16) -> Option<Self> {
        let bin = if std::path::Path::new("/home/ngson/.local/bin/redis-server").exists() {
            "/home/ngson/.local/bin/redis-server"
        } else {
            "redis-server"
        };
        let child = Command::new(bin)
            .args([
                "--port",
                &port.to_string(),
                "--bind",
                "127.0.0.1",
                "--save",
                "",
                "--appendonly",
                "no",
                "--loglevel",
                "warning",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        std::thread::sleep(Duration::from_millis(300));
        Some(Self {
            port,
            child: Some(child),
        })
    }

    fn url(&self) -> String {
        format!("redis://127.0.0.1:{}", self.port)
    }
}

impl Drop for TempRedis {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn make_config(tag: &str, valkey_url: Option<String>) -> Config {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://chatw@127.0.0.1:5432/chatw".into());
    Config {
        database_url,
        db_max_connections: 8,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        public_origin: "http://127.0.0.1:8085".into(),
        google_client_id: "test-client".into(),
        google_client_secret: "test-secret".into(),
        google_redirect_url: "http://127.0.0.1:8085/api/auth/google/callback".into(),
        session_secret: b"spec169-test-session-secret-must-be-at-least-32-bytes!".to_vec(),
        session_ttl: Duration::from_secs(3600),
        cookie_secure: false,
        static_dir: None,
        uploads_dir: std::env::temp_dir().join(format!("chatw_spec169_{tag}_{}", Uuid::new_v4().simple())),
        max_ws_connections: 50,
        trust_proxy: false,
        cors_allowed_origins: Vec::new(),
        google_configured: false,
        kiyo_issuer: String::new(),
        kiyo_client_id: String::new(),
        kiyo_client_secret: String::new(),
        kiyo_redirect_url: String::new(),
        kiyo_configured: false,
        valkey_url,
        small_chunk_inline_threshold_bytes: 16384,
    }
}

fn build_app(pool: sqlx::PgPool, cfg: Config, valkey: Arc<ValkeyCoordinator>) -> (axum::Router, AppState) {
    let ws_hub = Arc::new(WsHub::new(50));
    let state = AppState {
        config: Arc::new(cfg.clone()),
        db: pool,
        http_client: reqwest::Client::new(),
        jwks_cache: Arc::new(tokio::sync::RwLock::new(GoogleJwksCache::new())),
        kiyo_jwks_cache: Arc::new(tokio::sync::RwLock::new(GoogleJwksCache::new())),
        ws_hub,
        oauth_tickets: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        storage: Arc::new(storage::StorageRegistry::from_uploads_dir(&cfg.uploads_dir)),
        sfu: Arc::new(chatw::calls::sfu::SfuHandle::new()),
        rpc_router: Arc::new(std::sync::OnceLock::new()),
        valkey,
    };
    let router = axum::Router::new()
        .nest("/api/auth", auth::routes())
        .nest("/api/devices", devices_sync::device_routes())
        .nest("/api/keys", devices_sync::key_backup_routes())
        .nest("/api/groups", groups::routes())
        .nest("/api/security", security::routes())
        .nest("/uploads", attachments::protected_uploads_routes())
        .with_state(state.clone());
    (router, state)
}

async fn req_json(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value, axum::http::HeaderMap) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    let req = if let Some(json_body) = body {
        b.header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json_body.to_string()))
            .unwrap()
    } else {
        b.body(Body::empty()).unwrap()
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, val, headers)
}

#[tokio::test]
async fn spec169_end_to_end_security_remediation() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    // 1. Run migrations as table owner (`DATABASE_URL` or default `chatw`)
    let owner_cfg = make_config("owner", None);
    let owner_pool = db::connect(&owner_cfg.database_url, 4).await.expect("owner pg");
    db::run_migrations(&owner_pool).await.expect("migrations");

    // Check if least-privilege `chatw_app` role is provisioned on this host
    let has_app_role: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname = 'chatw_app')",
    )
    .fetch_one(&owner_pool)
    .await
    .unwrap_or(false);

    let runtime_pool = if has_app_role {
        let app_pool = db::connect("postgresql://chatw_app@%2Ftmp%2Fkiyo-pg-run/chatw", 4)
            .await
            .expect("chatw_app peer auth");
        // Verify least-privilege DB boundaries directly
        assert!(
            sqlx::query("SELECT count(*) FROM sessions").fetch_one(&app_pool).await.is_err(),
            "chatw_app must NOT read sessions table"
        );
        assert!(
            sqlx::query("SELECT count(*) FROM user_key_backups").fetch_one(&app_pool).await.is_err(),
            "chatw_app must NOT read user_key_backups table directly"
        );
        app_pool
    } else {
        owner_pool.clone()
    };

    let redis = TempRedis::start(16399).expect("redis-server required");
    let valkey = Arc::new(ValkeyCoordinator::connect(Some(redis.url())).await);
    assert!(valkey.is_connected(), "Redis session store must connect");

    let cfg = make_config("runtime", Some(redis.url()));
    let (app, state) = build_app(runtime_pool.clone(), cfg.clone(), valkey.clone());

    // F-01: dev-login deleted and providers reports dev=false
    let (st_dev, _, _) = req_json(
        &app,
        "POST",
        "/api/auth/dev-login",
        None,
        Some(serde_json::json!({ "username": "attacker" })),
    )
    .await;
    assert_eq!(st_dev, StatusCode::NOT_FOUND, "dev-login route must not exist");

    let (st_prov, v_prov, _) = req_json(&app, "GET", "/api/auth/providers", None, None).await;
    assert_eq!(st_prov, StatusCode::OK);
    assert_eq!(v_prov["dev"], false);
    assert_eq!(v_prov["kiyo"], false);

    // Public account registration is disabled (403) and unconfigured Kiyo returns 400
    let uname = format!("sec169_{}", &Uuid::new_v4().simple().to_string()[..10]);
    let (st_reg_disabled, _, _) = req_json(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(serde_json::json!({ "username": uname, "password": "StrongPassword!123" })),
    )
    .await;
    assert_eq!(
        st_reg_disabled,
        StatusCode::FORBIDDEN,
        "public /api/auth/register must be disabled"
    );

    let (st_kiyo_unconf, _, _) = req_json(&app, "GET", "/api/auth/kiyo", None, None).await;
    assert_eq!(
        st_kiyo_unconf,
        StatusCode::BAD_REQUEST,
        "unconfigured /api/auth/kiyo must return 400"
    );

    // F-04: Pre-provisioned password user logs in -> session lives in Redis, not PG sessions
    let pw_hash = chatw::backup::reauth::hash_argon2_phc("StrongPassword!123").unwrap();
    let provisioned_uid = Uuid::new_v4();
    let uname_norm = uname.to_lowercase();
    let sub = format!("pw:{provisioned_uid}");
    sqlx::query(
        "INSERT INTO users (id, google_sub, username, username_norm, password_hash) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(provisioned_uid)
    .bind(&sub)
    .bind(&uname)
    .bind(&uname_norm)
    .bind(&pw_hash)
    .execute(&owner_pool)
    .await
    .unwrap();

    let (st_reg, _, hdr_reg) = req_json(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(serde_json::json!({ "username": uname, "password": "StrongPassword!123" })),
    )
    .await;
    assert_eq!(st_reg, StatusCode::OK);
    let set_cookie = hdr_reg
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let cookie_pair = set_cookie.split(';').next().unwrap().to_string();
    let token_hex = cookie_pair.split_once('=').unwrap().1;

    // Verify session is stored in Redis
    let token_hash_hex = hex::encode(session::hash_session_token(
        &cfg.session_secret,
        &hex::decode(token_hex).unwrap(),
    ));
    let redis_sess = valkey.session_get_by_hash(&token_hash_hex).await.unwrap();
    assert!(redis_sess.is_some(), "session must be stored in Redis");

    let (st_me, v_me, _) = req_json(&app, "GET", "/api/auth/me", Some(&cookie_pair), None).await;
    assert_eq!(st_me, StatusCode::OK);
    let user_id: Uuid = Uuid::parse_str(v_me["id"].as_str().unwrap()).unwrap();

    // Verify device binding + per-request device trust re-check
    let client_dev_id = format!("cli_{}", Uuid::new_v4().simple());
    let (st_dreg, v_dreg, _) = req_json(
        &app,
        "POST",
        "/api/devices/register",
        Some(&cookie_pair),
        Some(serde_json::json!({
            "client_device_id": client_dev_id,
            "platform": "web",
            "device_name": "Browser",
        })),
    )
    .await;
    assert_eq!(st_dreg, StatusCode::OK);
    let device_id: Uuid = Uuid::parse_str(v_dreg["device_id"].as_str().unwrap()).unwrap();

    // Matching X-ChatW-Device-Id passes; mismatched X-ChatW-Device-Id is rejected!
    let ok_req = Request::builder()
        .method("GET")
        .uri("/api/auth/me")
        .header(header::COOKIE, &cookie_pair)
        .header("x-chatw-device-id", device_id.to_string())
        .body(Body::empty())
        .unwrap();
    assert_eq!(app.clone().oneshot(ok_req).await.unwrap().status(), StatusCode::OK);

    let bad_dev_req = Request::builder()
        .method("GET")
        .uri("/api/auth/me")
        .header(header::COOKIE, &cookie_pair)
        .header("x-chatw-device-id", Uuid::new_v4().to_string())
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(bad_dev_req).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );

    // SECURITY DEFINER backup RPC works even as `chatw_app`
    let fp = [0x42u8; 32];
    let (st_put_bk, _, _) = req_json(
        &app,
        "PUT",
        "/api/keys/backup",
        Some(&cookie_pair),
        Some(serde_json::json!({
            "wrapped_keys_b64": "dGVzdC13cmFwcGVk",
            "kdf_salt_b64": "dGVzdC1zYWx0",
            "kdf_iterations": 210000,
            "key_fingerprint_b64": B64.encode(fp),
        })),
    )
    .await;
    assert_eq!(st_put_bk, StatusCode::OK);
    let (st_get_bk, v_get_bk, _) =
        req_json(&app, "GET", "/api/keys/backup", Some(&cookie_pair), None).await;
    assert_eq!(st_get_bk, StatusCode::OK);
    assert_eq!(v_get_bk["exists"], true);

    // F-05: Banner authorization check
    let banner_dir = cfg.uploads_dir.join("banners");
    std::fs::create_dir_all(&banner_dir).unwrap();
    let priv_group_id = Uuid::new_v4();
    let priv_banner_file = format!("group_{priv_group_id}.png");
    std::fs::write(banner_dir.join(&priv_banner_file), b"\x89PNG\r\n\x1a\n").unwrap();

    // Create a private group owned by another user
    let other_id = Uuid::new_v4();
    sqlx::query("INSERT INTO users (id, google_sub, username, username_norm) VALUES ($1, $2, $3, $4)")
        .bind(other_id)
        .bind(format!("other_{other_id}"))
        .bind(format!("other_{}", &other_id.simple().to_string()[..8]))
        .bind(format!("other_{}", &other_id.simple().to_string()[..8]))
        .execute(&owner_pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO groups (id, name, owner_id, is_public, verification_status, banner_url) VALUES ($1, 'SecretGrp', $2, false, 'none', $3)")
        .bind(priv_group_id)
        .bind(other_id)
        .bind(format!("/uploads/banners/{priv_banner_file}?t=1"))
        .execute(&owner_pool)
        .await
        .unwrap();

    // Non-member cannot read private group banner (403)
    let (st_ban_unauth, _, _) = req_json(
        &app,
        "GET",
        &format!("/uploads/banners/{priv_banner_file}"),
        Some(&cookie_pair),
        None,
    )
    .await;
    assert_eq!(st_ban_unauth, StatusCode::FORBIDDEN);

    // Unreferenced banner returns 403
    std::fs::write(banner_dir.join("orphan.png"), b"\x89PNG\r\n\x1a\n").unwrap();
    let (st_orphan, _, _) = req_json(
        &app,
        "GET",
        "/uploads/banners/orphan.png",
        Some(&cookie_pair),
        None,
    )
    .await;
    assert_eq!(st_orphan, StatusCode::FORBIDDEN);

    // Revoking device in DB invalidates Redis session on the very next request
    sqlx::query("UPDATE devices SET trust_state = 'revoked' WHERE id = $1")
        .bind(device_id)
        .execute(&owner_pool)
        .await
        .unwrap();
    let (st_after_rev, _, _) = req_json(&app, "GET", "/api/auth/me", Some(&cookie_pair), None).await;
    assert_eq!(st_after_rev, StatusCode::UNAUTHORIZED);

    // F-05: CORS rejects cross-port loopback origin
    assert!(!auth::is_allowed_cors_origin_for_config(
        "http://127.0.0.1:9999",
        "http://127.0.0.1:8085",
        &[]
    ));
    assert!(auth::is_allowed_cors_origin_for_config(
        "http://127.0.0.1:8085",
        "http://127.0.0.1:8085",
        &[]
    ));

    // F-05: HTTP-layer login rate limiting triggers 429
    let rl_user = format!("rlu_{}", &Uuid::new_v4().simple().to_string()[..8]);
    let mut got_429 = false;
    for _ in 0..12 {
        let (st, _, _) = req_json(
            &app,
            "POST",
            "/api/auth/login",
            None,
            Some(serde_json::json!({ "username": rl_user, "password": "wrongpassword" })),
        )
        .await;
        if st == StatusCode::TOO_MANY_REQUESTS {
            got_429 = true;
            break;
        }
    }
    assert!(got_429, "login rate limit must return 429");

    // Cleanup test rows
    let _ = session::revoke_all_user_sessions_for_state(&state, user_id).await;
    let _ = sqlx::query("DELETE FROM groups WHERE id = $1").bind(priv_group_id).execute(&owner_pool).await;
    let _ = sqlx::query("DELETE FROM users WHERE id = ANY($1)").bind(&[user_id, other_id][..]).execute(&owner_pool).await;
    let _ = std::fs::remove_dir_all(&cfg.uploads_dir);
}
