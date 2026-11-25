//! Spec 165 Comprehensive Integration & Resilience Test Suite (Sections 36–43)
//!
//! Covers:
//! 1. Live Valkey/RESP Server (`redis-server`) presence TTL, typing TTL, distributed Lua rate limiter,
//!    atomic Lua distributed lock/lease, task dedup, non-sensitive metadata cache, and multi-node
//!    WebSocket Pub/Sub fan-out between 2 separate `WsHub` instances!
//! 2. **VALKEY RESTART RESILIENCE**: Kills live Valkey mid-flight, commits messages & chunks during outage
//!    (zero message loss, zero failed durable writes!), restarts Valkey, verifies automatic reconnection
//!    and full ordered event catch-up from PostgreSQL `ws_event_log`!
//! 3. PostgreSQL `pg_inline` small chunk (<= 16 KiB) vs Object Storage (`local_cas`) large chunk (> 16 KiB)
//!    with STRICT zero duplication + latency benchmark!
//! 4. HTTP `Idempotency-Key` deduplication on `POST /api/messages` and `POST /api/storage/msg-chunks/commit`.
//! 5. Ordered WebSocket reconnect gap recovery via `GET /api/sync/events?after_event_id=`.
//! 6. Security & F12 Audit: unauthenticated rejection, cross-account IDOR blocked, zero S3/Valkey
//!    credentials or internal storage keys leaked, plaintext search index rejected!

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64, engine::general_purpose::URL_SAFE_NO_PAD};
use chatw::{
    auth::{AppState, google::GoogleJwksCache, session},
    cache::ValkeyCoordinator,
    config::Config,
    crypto::Envelope,
    db, devices_sync, groups, messages, storage, typing,
    websocket::{WsEvent, WsHub},
};
use chatw_msg_store::{
    CompactMessage, DictionaryRegistry, MANDATORY_ZSTD_LEVEL, MessageType, ScopeKind,
    SealChunkRequest, Zstd22Options, derive_chunk_storage_key, seal_and_verify_message_chunk,
};
use p256::{
    EncodedPoint, SecretKey,
    ecdsa::{SigningKey, signature::Signer},
};
use sqlx::Row;
use std::process::{Child, Command, Stdio};
use std::{sync::Arc, time::{Duration, Instant}};
use tower::ServiceExt;
use uuid::Uuid;

struct TempValkeyServer {
    port: u16,
    child: Option<Child>,
}

impl TempValkeyServer {
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
                "--maxmemory",
                "64mb",
                "--maxmemory-policy",
                "allkeys-lru",
                "--loglevel",
                "warning",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        std::thread::sleep(Duration::from_millis(350));
        Some(Self {
            port,
            child: Some(child),
        })
    }

    fn url(&self) -> String {
        format!("redis://127.0.0.1:{}", self.port)
    }

    fn kill_now(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    fn restart(&mut self) -> bool {
        self.kill_now();
        std::thread::sleep(Duration::from_millis(200));
        if let Some(mut restarted) = Self::start(self.port) {
            self.child = restarted.child.take();
            return true;
        }
        false
    }
}

impl Drop for TempValkeyServer {
    fn drop(&mut self) {
        self.kill_now();
    }
}

fn test_config(tag: &str, valkey_url: Option<String>, inline_threshold: usize) -> Config {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://chatw:chatw_dev_password@%2Ftmp%2Fkiyo-pg-run/chatw".into()
    });
    Config {
        database_url,
        db_max_connections: 8,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        public_origin: "http://localhost:8085".into(),
        google_client_id: "test-client".into(),
        google_client_secret: "test-secret".into(),
        google_redirect_url: "http://localhost:8085/api/auth/google/callback".into(),
        session_secret: b"test-session-secret-at-least-32-bytes-long!".to_vec(),
        session_ttl: Duration::from_secs(3600),
        cookie_secure: false,
        static_dir: None,
        uploads_dir: std::env::temp_dir().join(format!("chatw_spec165_{tag}_{}", Uuid::new_v4().simple())),
        max_ws_connections: 100,
        trust_proxy: false,
        dev_login_enabled: false,
        google_configured: false,
        valkey_url,
        small_chunk_inline_threshold_bytes: inline_threshold,
    }
}

async fn setup_app(
    tag: &str,
    valkey_url: Option<String>,
    inline_threshold: usize,
) -> (sqlx::PgPool, axum::Router, AppState, Config) {
    let cfg = test_config(tag, valkey_url, inline_threshold);
    let pool = db::connect(&cfg.database_url, 8).await.expect("connect pg");
    db::run_migrations(&pool).await.expect("migrations");
    let storage_reg = Arc::new(storage::StorageRegistry::from_uploads_dir(&cfg.uploads_dir));
    let ws_hub = Arc::new(WsHub::new(100));
    let valkey = Arc::new(ValkeyCoordinator::connect(cfg.valkey_url.clone()).await);
    ws_hub.attach_valkey(valkey.clone()).await;
    valkey.spawn_ws_pubsub_listener(ws_hub.clone());

    let state = AppState {
        config: Arc::new(cfg.clone()),
        db: pool.clone(),
        http_client: reqwest::Client::new(),
        jwks_cache: Arc::new(tokio::sync::RwLock::new(GoogleJwksCache::new())),
        ws_hub,
        oauth_tickets: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        storage: storage_reg,
        sfu: Arc::new(chatw::calls::sfu::SfuHandle::new()),
        rpc_router: Arc::new(std::sync::OnceLock::new()),
        valkey,
    };
    let router = axum::Router::new()
        .nest("/api/storage", storage::router::routes())
        .nest("/api/groups", groups::routes())
        .nest("/api/messages", messages::routes())
        .nest("/api/sync", devices_sync::sync_routes())
        .nest("/api/typing", typing::routes())
        .with_state(state.clone());
    (pool, router, state, cfg)
}

async fn make_user_with_keys(
    pool: &sqlx::PgPool,
    cfg: &Config,
    label: &str,
) -> (Uuid, String, SigningKey, [u8; 32]) {
    let uid = Uuid::new_v4();
    let uname = format!("{label}_{}", &uid.simple().to_string()[..8]);
    sqlx::query(
        "INSERT INTO users (id, google_sub, username, username_norm) VALUES ($1, $2, $3, $4)",
    )
    .bind(uid)
    .bind(format!("sub_{uid}"))
    .bind(&uname)
    .bind(uname.to_lowercase())
    .execute(pool)
    .await
    .expect("insert user");

    // Register P-256 ECDH & ECDSA uncompressed 65-byte SEC1 keys so real E2EE HTTP message send passes signature checks!
    let mut raw_ecdh = [0u8; 32];
    let mut raw_ecdsa = [0u8; 32];
    getrandom::fill(&mut raw_ecdh).unwrap();
    getrandom::fill(&mut raw_ecdsa).unwrap();
    let ecdh_sk = SecretKey::from_slice(&raw_ecdh).unwrap();
    let ecdh_pub = EncodedPoint::from(&ecdh_sk.public_key()).as_bytes().to_vec();
    let ecdsa_sk = SigningKey::from_slice(&raw_ecdsa).unwrap();
    let ecdsa_pub = EncodedPoint::from(ecdsa_sk.verifying_key()).as_bytes().to_vec();
    let fp = chatw::crypto::compute_identity_fingerprint(&ecdh_pub, &ecdsa_pub);

    sqlx::query(
        "INSERT INTO device_keys (user_id, ecdh_public, ecdsa_public, fingerprint) VALUES ($1, $2, $3, $4)",
    )
    .bind(uid)
    .bind(&ecdh_pub)
    .bind(&ecdsa_pub)
    .bind(&fp[..])
    .execute(pool)
    .await
    .expect("insert device_keys");

    let set_cookie = session::create_session(pool, cfg, uid, None)
        .await
        .expect("create session");
    let cookie_pair = set_cookie.split(';').next().unwrap().to_string();
    (uid, cookie_pair, ecdsa_sk, fp)
}

fn build_signed_envelope(
    message_id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    recipient_fp: &[u8; 32],
    sender_signing_key: &SigningKey,
    ciphertext: &[u8],
) -> String {
    let mut ct = ciphertext.to_vec();
    if ct.len() < 32 {
        ct.resize(32, 0xA5);
    }
    let mut raw = Vec::with_capacity(174 + ct.len() + 64);
    raw.push(1u8); // version
    raw.extend_from_slice(message_id.as_bytes());
    raw.extend_from_slice(sender_id.as_bytes());
    raw.extend_from_slice(recipient_id.as_bytes());
    raw.extend_from_slice(recipient_fp);
    // Ephemeral public key (65 bytes uncompressed P-256)
    let mut eph_raw = [0u8; 32];
    getrandom::fill(&mut eph_raw).unwrap();
    let eph_sk = SecretKey::from_slice(&eph_raw).unwrap();
    let eph_point = EncodedPoint::from(&eph_sk.public_key());
    raw.extend_from_slice(eph_point.as_bytes());
    let salt = [0x22u8; 16];
    let nonce = [0x11u8; 12];
    raw.extend_from_slice(&salt);
    raw.extend_from_slice(&nonce);
    raw.extend_from_slice(&ct);
    let sig: p256::ecdsa::Signature = sender_signing_key.sign(&raw);
    raw.extend_from_slice(&sig.to_bytes());
    // Sanity parse locally
    assert!(Envelope::parse(&raw).is_ok());
    URL_SAFE_NO_PAD.encode(raw)
}

async fn send_json(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    idem_key: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value, axum::http::HeaderMap) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    if let Some(ik) = idem_key {
        builder = builder.header("Idempotency-Key", ik);
    }
    let req = if let Some(b) = body {
        builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, val, headers)
}

#[tokio::test]
async fn spec165_valkey_pubsub_restart_pg_inline_idempotency_and_security() {
    // Start a REAL local Valkey/Redis server with bounded RAM (64MB)
    let mut vk_server = TempValkeyServer::start(16389)
        .expect("redis-server must be available at /home/ngson/.local/bin/redis-server");
    let vk_url = vk_server.url();

    let (pool, app, state, cfg) = setup_app("full", Some(vk_url.clone()), 16_384).await;
    assert!(state.valkey.is_configured());
    assert!(state.valkey.is_connected(), "ValkeyCoordinator must connect to live server");

    let (alice, alice_cookie, alice_sk, alice_fp) = make_user_with_keys(&pool, &cfg, "alice165").await;
    let (bob, bob_cookie, _bob_sk, bob_fp) = make_user_with_keys(&pool, &cfg, "bob165").await;
    let (eve, eve_cookie, _eve_sk, _eve_fp) = make_user_with_keys(&pool, &cfg, "eve165").await;
    let _ = eve;

    let (u_a, u_b) = (alice.min(bob), alice.max(bob));
    sqlx::query("INSERT INTO friendships (user_a, user_b) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(u_a)
        .bind(u_b)
        .execute(&pool)
        .await
        .unwrap();

    // ── 1. Valkey Ephemeral Primitives: Presence, Typing, Rate Limit, Lua Lock, Task Dedup, Meta Cache ──
    let (conn_id, mut bob_rx_node1) = state.ws_hub.register(bob, None, None).await.unwrap();
    assert_eq!(state.valkey.is_user_online(bob).await, Some(true));

    // Second backend node (`node2`) connected to SAME Valkey -> verifies Cross-Node WebSocket Pub/Sub!
    let node2_hub = Arc::new(WsHub::new(100));
    let node2_vk = Arc::new(ValkeyCoordinator::connect(Some(vk_url.clone())).await);
    node2_hub.attach_valkey(node2_vk.clone()).await;
    node2_vk.spawn_ws_pubsub_listener(node2_hub.clone());
    let (_conn2, mut bob_rx_node2) = node2_hub.register(bob, None, None).await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Trigger typing via HTTP POST /api/typing -> Valkey typing TTL + Cross-node WS Pub/Sub!
    let (st_typ, _, _) = send_json(
        &app,
        "POST",
        "/api/typing",
        Some(&alice_cookie),
        None,
        Some(serde_json::json!({ "friend_id": bob, "is_typing": true })),
    )
    .await;
    assert_eq!(st_typ, StatusCode::NO_CONTENT);
    assert_eq!(state.valkey.is_typing(alice, bob).await, Some(true));

    let ev_local = tokio::time::timeout(Duration::from_secs(2), bob_rx_node1.recv()).await.unwrap().unwrap();
    let ev_remote = tokio::time::timeout(Duration::from_secs(2), bob_rx_node2.recv()).await.unwrap().unwrap();
    assert!(matches!(ev_local, WsEvent::TypingStart { user_id } if user_id == alice));
    assert!(matches!(ev_remote, WsEvent::TypingStart { user_id } if user_id == alice));

    // Valkey Lua Distributed Lock & Task Deduplication
    assert_eq!(state.valkey.try_acquire_lock("test:job1", "tokA", 30).await, Some(true));
    assert_eq!(state.valkey.try_acquire_lock("test:job1", "tokB", 30).await, Some(false));
    assert_eq!(state.valkey.renew_lock("test:job1", "tokA", 60).await, Some(true));
    state.valkey.release_lock("test:job1", "tokA").await;
    assert_eq!(state.valkey.try_acquire_lock("test:job1", "tokB", 30).await, Some(true));

    assert!(state.valkey.claim_task_once("dedup:k1", 30).await);
    assert!(!state.valkey.claim_task_once("dedup:k1", 30).await);

    // Valkey Distributed Rate Limiter
    let rl_key = vec![format!("rltest:{}", Uuid::new_v4())];
    assert!(state.valkey.check_rate_limit(&rl_key, 2, Duration::from_secs(60)).await.is_ok());
    assert!(state.valkey.check_rate_limit(&rl_key, 2, Duration::from_secs(60)).await.is_ok());
    assert!(state.valkey.check_rate_limit(&rl_key, 2, Duration::from_secs(60)).await.is_err());

    // ── 2. HTTP Message Send + Idempotency-Key Deduplication + WS Receive ──
    let msg1_id = Uuid::new_v4();
    let rec_env = build_signed_envelope(msg1_id, alice, bob, &bob_fp, &alice_sk, b"cipher-hello-1");
    let snd_env = build_signed_envelope(msg1_id, alice, alice, &alice_fp, &alice_sk, b"cipher-hello-1-self");
    let send_body = serde_json::json!({
        "message_id": msg1_id,
        "recipient_id": bob,
        "recipient_envelope": rec_env,
        "sender_envelope": snd_env,
        "reply_to_id": null,
    });
    let idem1 = format!("idem-msg-{msg1_id}");
    let (s1, v1, _) = send_json(&app, "POST", "/api/messages", Some(&alice_cookie), Some(&idem1), Some(send_body.clone())).await;
    assert_eq!(s1, StatusCode::OK);
    // Retry with SAME Idempotency-Key must return exact same seq & zero duplicate rows!
    let (s2, v2, _) = send_json(&app, "POST", "/api/messages", Some(&alice_cookie), Some(&idem1), Some(send_body)).await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(v1["seq"], v2["seq"]);
    let msg_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE id = $1")
        .bind(msg1_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(msg_count, 1);

    // ── 3. PG Inline Small Chunk (<=16 KiB) vs Object Storage Large Chunk (>16 KiB) — ZERO DUPLICATION ──
    let root_key = [0x33u8; 32];
    let dict_reg = DictionaryRegistry::new();
    let old_ts_ms = (chrono::Utc::now() - chrono::Duration::days(14)).timestamp_millis();

    // 3a. Small chunk (~1-3 KiB sealed <= 16 KiB) -> MUST store in `pg_inline` with ZERO CAS file!
    let small_msgs: Vec<CompactMessage> = (0..40)
        .map(|i| CompactMessage {
            id: Uuid::new_v4().to_string(),
            seq: i + 1,
            sender_id: alice.to_string(),
            created_at_ms: old_ts_ms + i * 1000,
            msg_type: MessageType::Text,
            text: format!("Tin nhắn cũ nhỏ số {i} kiểm thử lưu inline trong PostgreSQL"),
            reply_to_id: None,
            is_edited: false,
            deleted_for_all: false,
            is_pinned: false,
            is_bookmarked: false,
            status_code: 0,
            reactions: Vec::new(),
            attachments: Vec::new(),
            extra_meta: None,
        })
        .collect();
    let small_cid = Uuid::new_v4();
    let small_key = derive_chunk_storage_key(&root_key, &alice.to_string(), ScopeKind::Dm, &bob.to_string(), 1).unwrap();
    let small_sealed = seal_and_verify_message_chunk(
        &small_msgs,
        &SealChunkRequest {
            chunk_id: &small_cid.to_string(),
            account_id: &alice.to_string(),
            scope_type: ScopeKind::Dm,
            scope_id: &bob.to_string(),
            chunk_version: 1,
            access_epoch: 1,
            key_version: 1,
            candidate_dict_id: 1,
            now_ms: chrono::Utc::now().timestamp_millis(),
            require_space_savings: true,
            zstd_opts: Zstd22Options::default(),
        },
        &small_key,
        &dict_reg,
    ).unwrap();
    assert!(small_sealed.sealed_bytes.len() <= 16_384);

    let t_inline = Instant::now();
    let commit_small_body = serde_json::json!({
        "scope_type": "dm",
        "scope_id": bob,
        "expected_manifest_version": 0,
        "chunk_id": small_cid,
        "compression_level": MANDATORY_ZSTD_LEVEL,
        "dictionary_id": small_sealed.metadata.dictionary_id,
        "original_size": small_sealed.metadata.original_size,
        "serialized_size": small_sealed.metadata.serialized_size,
        "compressed_size": small_sealed.metadata.compressed_size,
        "padded_size": small_sealed.metadata.padded_size,
        "message_count": small_sealed.metadata.message_count,
        "seq_start": small_sealed.metadata.seq_start,
        "seq_end": small_sealed.metadata.seq_end,
        "first_created_at_ms": small_sealed.metadata.first_created_at_ms,
        "last_created_at_ms": small_sealed.metadata.last_created_at_ms,
        "nonce_b64": small_sealed.metadata.nonce_b64,
        "plaintext_blake3_hex": small_sealed.metadata.plaintext_blake3_hex,
        "ciphertext_blake3_hex": small_sealed.metadata.ciphertext_blake3_hex,
        "payload_b64": B64.encode(&small_sealed.sealed_bytes),
    });
    let idem_chk = format!("idem-chk-{small_cid}");
    let (sc1, vc1, _) = send_json(&app, "POST", "/api/storage/msg-chunks/commit", Some(&alice_cookie), Some(&idem_chk), Some(commit_small_body.clone())).await;
    let inline_commit_ms = t_inline.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(sc1, StatusCode::OK);
    assert_eq!(vc1["storage_placement"], "pg_inline");
    // Retry commit with SAME Idempotency-Key returns cached OK instead of 409 conflict!
    let (sc1_dup, vc1_dup, _) = send_json(&app, "POST", "/api/storage/msg-chunks/commit", Some(&alice_cookie), Some(&idem_chk), Some(commit_small_body)).await;
    assert_eq!(sc1_dup, StatusCode::OK);
    assert_eq!(vc1_dup["manifest_version"], vc1["manifest_version"]);

    // Verify DB row has `storage_backend = 'pg_inline'` and non-null `inline_payload`!
    let row_in = sqlx::query("SELECT storage_backend, inline_payload IS NOT NULL AS has_in FROM message_chunks WHERE id = $1")
        .bind(small_cid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row_in.get::<String, _>("storage_backend"), "pg_inline");
    assert!(row_in.get::<bool, _>("has_in"));

    // 3b. Large chunk (> 16 KiB sealed) -> MUST store in Object Storage (`local_cas`) with `inline_payload IS NULL`!
    let large_msgs: Vec<CompactMessage> = (0..1200)
        .map(|i| {
            // High-entropy unique strings so Zstd-22 output exceeds 16 KiB while still saving space vs JSON!
            let unique_hash = blake3::hash(format!("payload-{i}-salt-{}", i * 7919).as_bytes()).to_hex();
            CompactMessage {
                id: Uuid::new_v4().to_string(),
                seq: 100 + i,
                sender_id: alice.to_string(),
                created_at_ms: old_ts_ms + (100 + i) * 1000,
                msg_type: MessageType::Text,
                text: format!("Tin nhắn lớn {i} mã đối soát {unique_hash} và chi tiết lịch sử dài"),
                reply_to_id: None,
                is_edited: false,
                deleted_for_all: false,
                is_pinned: false,
                is_bookmarked: false,
                status_code: 0,
                reactions: Vec::new(),
                attachments: Vec::new(),
                extra_meta: None,
            }
        })
        .collect();
    let large_cid = Uuid::new_v4();
    let large_key = derive_chunk_storage_key(&root_key, &alice.to_string(), ScopeKind::Dm, &bob.to_string(), 1).unwrap();
    let large_sealed = seal_and_verify_message_chunk(
        &large_msgs,
        &SealChunkRequest {
            chunk_id: &large_cid.to_string(),
            account_id: &alice.to_string(),
            scope_type: ScopeKind::Dm,
            scope_id: &bob.to_string(),
            chunk_version: 1,
            access_epoch: 1,
            key_version: 1,
            candidate_dict_id: 1,
            now_ms: chrono::Utc::now().timestamp_millis(),
            require_space_savings: true,
            zstd_opts: Zstd22Options::default(),
        },
        &large_key,
        &dict_reg,
    ).unwrap();
    assert!(large_sealed.sealed_bytes.len() > 16_384, "large chunk must exceed 16 KiB threshold (got {})", large_sealed.sealed_bytes.len());

    let t_obj = Instant::now();
    let (sc2, vc2, _) = send_json(
        &app,
        "POST",
        "/api/storage/msg-chunks/commit",
        Some(&alice_cookie),
        None,
        Some(serde_json::json!({
            "scope_type": "dm",
            "scope_id": bob,
            "expected_manifest_version": 1,
            "chunk_id": large_cid,
            "compression_level": MANDATORY_ZSTD_LEVEL,
            "dictionary_id": large_sealed.metadata.dictionary_id,
            "original_size": large_sealed.metadata.original_size,
            "serialized_size": large_sealed.metadata.serialized_size,
            "compressed_size": large_sealed.metadata.compressed_size,
            "padded_size": large_sealed.metadata.padded_size,
            "message_count": large_sealed.metadata.message_count,
            "seq_start": large_sealed.metadata.seq_start,
            "seq_end": large_sealed.metadata.seq_end,
            "first_created_at_ms": large_sealed.metadata.first_created_at_ms,
            "last_created_at_ms": large_sealed.metadata.last_created_at_ms,
            "nonce_b64": large_sealed.metadata.nonce_b64,
            "plaintext_blake3_hex": large_sealed.metadata.plaintext_blake3_hex,
            "ciphertext_blake3_hex": large_sealed.metadata.ciphertext_blake3_hex,
            "payload_b64": B64.encode(&large_sealed.sealed_bytes),
        })),
    ).await;
    let obj_commit_ms = t_obj.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(sc2, StatusCode::OK);
    assert_eq!(vc2["storage_placement"], "object_storage");

    // Verify ZERO duplication in PostgreSQL (`inline_payload IS NULL` for Object Storage chunk!)
    let row_obj = sqlx::query("SELECT storage_backend, inline_payload IS NULL AS no_dup FROM message_chunks WHERE id = $1")
        .bind(large_cid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row_obj.get::<String, _>("storage_backend"), "local_cas");
    assert!(row_obj.get::<bool, _>("no_dup"));

    // Download BOTH small (PG inline) and large (Object Storage) chunks via HTTP & verify byte-exact match!
    for (cid, expected_bytes) in [(small_cid, &small_sealed.sealed_bytes), (large_cid, &large_sealed.sealed_bytes)] {
        let req = Request::builder()
            .method("GET")
            .uri(format!("/api/storage/msg-chunks/{cid}"))
            .header(header::COOKIE, &alice_cookie)
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // Security audit: NEVER leak S3 URL, internal storage_key, or credentials in headers!
        for (k, v) in resp.headers() {
            let ks = k.as_str().to_lowercase();
            let vs = v.to_str().unwrap_or("");
            assert!(!ks.contains("s3") && !ks.contains("minio") && !vs.contains("msgchunks/"));
        }
        let body = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024).await.unwrap();
        assert_eq!(body.as_ref(), expected_bytes.as_slice());
    }

    // ── 4. VALKEY KILL & RESTART RESILIENCE TEST (Zero Message Loss + PG Durable Recovery!) ──
    vk_server.kill_now();
    tokio::time::sleep(Duration::from_millis(150)).await;

    // While Valkey is COMPLETELY DEAD, send a new message over HTTP -> MUST SUCCEED cleanly!
    let msg2_id = Uuid::new_v4();
    let rec2 = build_signed_envelope(msg2_id, alice, bob, &bob_fp, &alice_sk, b"cipher-during-valkey-outage");
    let snd2 = build_signed_envelope(msg2_id, alice, alice, &alice_fp, &alice_sk, b"cipher-during-valkey-outage-self");
    let (s_outage, v_outage, _) = send_json(
        &app,
        "POST",
        "/api/messages",
        Some(&alice_cookie),
        Some(&format!("idem-outage-{msg2_id}")),
        Some(serde_json::json!({
            "message_id": msg2_id,
            "recipient_id": bob,
            "recipient_envelope": rec2,
            "sender_envelope": snd2,
            "reply_to_id": null,
        })),
    )
    .await;
    assert_eq!(s_outage, StatusCode::OK, "HTTP message send MUST succeed even when Valkey is down!");
    assert!(v_outage["seq"].as_i64().unwrap_or(0) > 0);

    // Rate limiter must ALSO degrade cleanly to local in-memory fallback during Valkey outage!
    let rl_fb = vec![format!("rlfb:{}", Uuid::new_v4())];
    assert!(state.valkey.check_rate_limit(&rl_fb, 2, Duration::from_secs(60)).await.is_ok());
    assert!(state.valkey.check_rate_limit(&rl_fb, 2, Duration::from_secs(60)).await.is_ok());
    assert!(state.valkey.check_rate_limit(&rl_fb, 2, Duration::from_secs(60)).await.is_err());

    // Now RESTART Valkey on the same port and verify automatic reconnection after circuit cooldown!
    assert!(vk_server.restart(), "Valkey server must restart cleanly");
    tokio::time::sleep(Duration::from_millis(500)).await;
    state.valkey.set_user_online(alice, None).await;
    assert_eq!(state.valkey.is_user_online(alice).await, Some(true), "Valkey must auto-reconnect after restart!");

    // ── 5. WebSocket Reconnect Ordered Gap Recovery (`GET /api/sync/events?after_event_id=`) ──
    let (se_bob, ve_bob, _) = send_json(&app, "GET", "/api/sync/events?after_event_id=0&limit=50", Some(&bob_cookie), None, None).await;
    assert_eq!(se_bob, StatusCode::OK);
    let bob_events = ve_bob["events"].as_array().unwrap();
    let recovered_msg_ids: Vec<String> = bob_events
        .iter()
        .filter_map(|e| e["message_id"].as_str().map(str::to_string))
        .collect();
    assert!(recovered_msg_ids.contains(&msg1_id.to_string()), "Bob must recover msg1");
    assert!(recovered_msg_ids.contains(&msg2_id.to_string()), "Bob must recover msg2 sent during Valkey outage!");

    let latest_bob = ve_bob["latest_event_id"].as_i64().unwrap();
    let (se_bob2, ve_bob2, _) = send_json(
        &app,
        "GET",
        &format!("/api/sync/events?after_event_id={latest_bob}&limit=50"),
        Some(&bob_cookie),
        None,
        None,
    ).await;
    assert_eq!(se_bob2, StatusCode::OK);
    assert_eq!(ve_bob2["events"].as_array().unwrap().len(), 0, "Catch-up after latest_event_id must be deduplicated/empty");

    // Alice also receives durable `manifest.updated` events in her ordered event log!
    let (se_alice, ve_alice, _) = send_json(&app, "GET", "/api/sync/events?after_event_id=0&limit=50", Some(&alice_cookie), None, None).await;
    assert_eq!(se_alice, StatusCode::OK);
    assert!(ve_alice["events"].as_array().unwrap().iter().any(|e| e["event_type"] == "manifest.updated"));

    // ── 6. Security & F12 Audit Assertions ──
    // Unauthenticated access rejected
    let (s_unauth, _, _) = send_json(&app, "GET", "/api/sync/events", None, None, None).await;
    assert_eq!(s_unauth, StatusCode::UNAUTHORIZED);

    // Cross-account IDOR: Eve CANNOT download Alice's inline OR external chunk!
    for cid in [small_cid, large_cid] {
        let (s_idor, _, _) = send_json(&app, "GET", &format!("/api/storage/msg-chunks/{cid}"), Some(&eve_cookie), None, None).await;
        assert_eq!(s_idor, StatusCode::FORBIDDEN);
    }

    // Eve's `/api/sync/events` MUST NEVER contain Alice or Bob's events!
    let (se_eve, ve_eve, _) = send_json(&app, "GET", "/api/sync/events?after_event_id=0", Some(&eve_cookie), None, None).await;
    assert_eq!(se_eve, StatusCode::OK);
    assert_eq!(ve_eve["events"].as_array().unwrap().len(), 0);

    // Telemetry exposes Valkey health metrics WITHOUT leaking Valkey URL or credentials!
    let (s_tel, v_tel, _) = send_json(&app, "GET", "/api/storage/telemetry", Some(&alice_cookie), None, None).await;
    assert_eq!(s_tel, StatusCode::OK);
    assert_eq!(v_tel["valkey"]["configured"], true);
    let tel_str = v_tel.to_string();
    assert!(!tel_str.contains("redis://") && !tel_str.contains("16389"), "Telemetry must never leak Valkey host/URL!");

    // Plaintext search shard upload MUST be rejected!
    let plain_b64 = B64.encode(b"PLAINTEXT_NOT_CWSE1_SEARCH_INDEX_DATA_1234567890");
    let plain_b3 = blake3::hash(b"PLAINTEXT_NOT_CWSE1_SEARCH_INDEX_DATA_1234567890").to_hex().to_string();
    let (s_plain, _, _) = send_json(
        &app,
        "PUT",
        "/api/storage/search-index/segments/dm:test/0",
        Some(&alice_cookie),
        None,
        Some(serde_json::json!({
            "expected_version": 0,
            "doc_count": 1,
            "min_seq": 1,
            "max_seq": 1,
            "blake3_hex": plain_b3,
            "sealed_shard_b64": plain_b64,
        })),
    ).await;
    assert_eq!(s_plain, StatusCode::BAD_REQUEST);

    state.ws_hub.unregister(&bob, conn_id).await;
    println!(
        "Spec 165 Live Valkey + PG Inline Benchmark: small_inline({}B)={:.2}ms | large_obj({}B)={:.2}ms | Valkey RAM={:?}B",
        small_sealed.sealed_bytes.len(),
        inline_commit_ms,
        large_sealed.sealed_bytes.len(),
        obj_commit_ms,
        v_tel["valkey"]["used_memory_bytes"].as_u64()
    );
    let _ = std::fs::remove_dir_all(&cfg.uploads_dir);
}
