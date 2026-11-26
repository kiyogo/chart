//! Spec 164 Backend Integration Tests:
//! - Strict 7-day compression age gate & mandatory Zstd Level 22
//! - Distributed leases with fencing tokens & multi-device contention
//! - OCC manifest version CAS (`409 Conflict` on stale version) & atomic commit
//! - Cross-account isolation (DM chunks & encrypted search segments)
//! - Append-only mutation/tombstone log over immutable chunks
//! - Versioned public & encrypted private (`CWDC1`) dictionaries
//! - Encrypted search index (`CWSE1`) multi-device sync with OCC & plaintext rejection
//! - Server incremental >=7d group compaction + Warm->Cold tiering + Cold->Warm promotion
//! - Privacy-safe device telemetry & adaptive `storage_mode` policy persistence

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use chatw::{
    auth::{AppState, google::GoogleJwksCache, session::create_session},
    config::Config,
    db, groups, messages, storage,
    websocket::WsHub,
};
use chatw_msg_store::{
    CompactMessage, DICT_ID_PUBLIC_CHAT_V1, DictionaryRegistry, LocalSearchIndex, MessageType,
    ScopeKind, SealChunkRequest, SearchCoverageState, Zstd22Options, derive_chunk_storage_key,
    derive_private_dict_key, derive_search_index_key, open_and_verify_message_chunk,
    seal_and_verify_message_chunk, seal_private_dictionary,
};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;
use uuid::Uuid;

fn test_config(tag: &str) -> Config {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://chatw:chatw_dev_password@%2Ftmp%2Fkiyo-pg-run/chatw".into()
    });
    Config {
        database_url,
        db_max_connections: 5,
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        public_origin: "http://localhost:8085".into(),
        google_client_id: "test-client".into(),
        google_client_secret: "test-secret".into(),
        google_redirect_url: "http://localhost:8085/api/auth/google/callback".into(),
        session_secret: b"test-session-secret-at-least-32-bytes-long!".to_vec(),
        session_ttl: Duration::from_secs(3600),
        cookie_secure: false,
        static_dir: None,
        uploads_dir: std::env::temp_dir().join(format!("chatw_spec164_{tag}_{}", Uuid::new_v4().simple())),
        max_ws_connections: 100,
        trust_proxy: false,
        cors_allowed_origins: Vec::new(),
        google_configured: false,
        kiyo_issuer: String::new(),
        kiyo_client_id: String::new(),
        kiyo_client_secret: String::new(),
        kiyo_redirect_url: String::new(),
        kiyo_configured: false,
        valkey_url: None,
        small_chunk_inline_threshold_bytes: 16384,
    }
}

async fn setup_app(tag: &str) -> (sqlx::PgPool, axum::Router, Arc<storage::StorageRegistry>, Config) {
    let cfg = test_config(tag);
    let pool = db::connect(&cfg.database_url, 5).await.expect("connect pg");
    db::run_migrations(&pool).await.expect("migrations");
    let storage_reg = Arc::new(storage::StorageRegistry::from_uploads_dir(&cfg.uploads_dir));
    let state = AppState {
        config: Arc::new(cfg.clone()),
        db: pool.clone(),
        http_client: reqwest::Client::new(),
        jwks_cache: Arc::new(tokio::sync::RwLock::new(GoogleJwksCache::new())),
        kiyo_jwks_cache: Arc::new(tokio::sync::RwLock::new(GoogleJwksCache::new())),
        ws_hub: Arc::new(WsHub::new(100)),
        oauth_tickets: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        storage: storage_reg.clone(),
        sfu: Arc::new(chatw::calls::sfu::SfuHandle::new()),
        rpc_router: Arc::new(std::sync::OnceLock::new()),
        valkey: Arc::new(chatw::cache::ValkeyCoordinator::disabled()),
    };
    let router = axum::Router::new()
        .nest("/api/storage", storage::router::routes())
        .nest("/api/groups", groups::routes())
        .nest("/api/messages", messages::routes())
        .with_state(state);
    (pool, router, storage_reg, cfg)
}

async fn make_user(pool: &sqlx::PgPool, cfg: &Config, label: &str) -> (Uuid, String) {
    let uid = Uuid::new_v4();
    let uname = format!("{label}_{}", &uid.simple().to_string()[..8]);
    sqlx::query(
        "INSERT INTO users (id, google_sub, username, username_norm) VALUES ($1, $2, $3, $4)",
    )
    .bind(uid)
    .bind(format!("sub_{uid}"))
    .bind(&uname)
    .bind(&uname)
    .execute(pool)
    .await
    .unwrap();
    let set_cookie = create_session(pool, cfg, uid, Some("test-dev"))
        .await
        .unwrap();
    let cookie_kv = set_cookie.split(';').next().unwrap().to_string();
    (uid, cookie_kv)
}

async fn req_json(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri).header(header::COOKIE, cookie);
    let req = if let Some(v) = body {
        b = b.header(header::CONTENT_TYPE, "application/json");
        b.body(Body::from(v.to_string())).unwrap()
    } else {
        b.body(Body::empty()).unwrap()
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let st = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(json!({ "raw_len": bytes.len() }));
    (st, val)
}

async fn req_raw(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: &str,
) -> (StatusCode, Vec<u8>, header::HeaderMap) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let st = resp.status();
    let hdrs = resp.headers().clone();
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024).await.unwrap();
    (st, bytes.to_vec(), hdrs)
}

#[tokio::test]
async fn spec164_leases_occ_7day_gate_e2ee_chunks_dicts_search_sync_and_tiering() {
    let (pool, app, storage_reg, cfg) = setup_app("full").await;
    let (alice, c_alice) = make_user(&pool, &cfg, "alice").await;
    let (bob, c_bob) = make_user(&pool, &cfg, "bob").await;

    // Make Alice & Bob friends
    let (ua, ub) = (alice.min(bob), alice.max(bob));
    sqlx::query("INSERT INTO friendships (user_a, user_b) VALUES ($1, $2)")
        .bind(ua)
        .bind(ub)
        .execute(&pool)
        .await
        .unwrap();

    // 1. Policy storage_mode persistence
    let (st_pol, mut pol) = req_json(&app, "GET", "/api/storage/policy", &c_alice, None).await;
    assert_eq!(st_pol, StatusCode::OK);
    assert_eq!(pol["storage_mode"], "auto");
    pol["storage_mode"] = json!("high_performance");
    let (st_upd, pol2) = req_json(&app, "PUT", "/api/storage/policy", &c_alice, Some(pol)).await;
    assert_eq!(st_upd, StatusCode::OK);
    assert_eq!(pol2["storage_mode"], "high_performance");

    // 2. Distributed Lease acquisition, multi-device contention, renewal, fencing
    let (st_l1, l1) = req_json(
        &app,
        "POST",
        "/api/storage/msg-chunks/lease/acquire",
        &c_alice,
        Some(json!({ "scope_type": "dm", "scope_id": bob, "device_id": "dev-A", "ttl_secs": 120 })),
    )
    .await;
    assert_eq!(st_l1, StatusCode::OK);
    assert_eq!(l1["acquired"], true);
    let lease_key = l1["lease_key"].as_str().unwrap().to_string();
    let token_a = l1["fencing_token"].as_i64().unwrap();

    // Second device of Alice CANNOT hijack active lease!
    let (st_l2, l2) = req_json(
        &app,
        "POST",
        "/api/storage/msg-chunks/lease/acquire",
        &c_alice,
        Some(json!({ "scope_type": "dm", "scope_id": bob, "device_id": "dev-B", "ttl_secs": 120 })),
    )
    .await;
    assert_eq!(st_l2, StatusCode::OK);
    assert_eq!(l2["acquired"], false);

    // Renew with checkpoint succeeds for dev-A; stale token fails with 409!
    let (st_rn, _) = req_json(
        &app,
        "POST",
        "/api/storage/msg-chunks/lease/renew",
        &c_alice,
        Some(json!({ "lease_key": lease_key, "device_id": "dev-A", "fencing_token": token_a, "checkpoint": { "last_seq": 50 } })),
    )
    .await;
    assert_eq!(st_rn, StatusCode::OK);

    let (st_stale, _) = req_json(
        &app,
        "POST",
        "/api/storage/msg-chunks/lease/renew",
        &c_alice,
        Some(json!({ "lease_key": lease_key, "device_id": "dev-A", "fencing_token": token_a + 99 })),
    )
    .await;
    assert_eq!(st_stale, StatusCode::CONFLICT);

    // 3. Seal real Zstd-22 E2EE chunk on client side
    let now_ms = chrono::Utc::now().timestamp_millis();
    let old_base = now_ms - 10 * 86_400 * 1000;
    let msgs: Vec<CompactMessage> = (0..120)
        .map(|i| CompactMessage {
            id: Uuid::new_v4().to_string(),
            seq: i + 1,
            sender_id: if i % 2 == 0 { alice } else { bob }.to_string(),
            created_at_ms: old_base + i * 10_000,
            msg_type: MessageType::Text,
            text: format!("Xin chào Nguyễn Văn An! Tin nhắn lịch sử số {i} để nén Zstd Level 22 🔥"),
            reply_to_id: None,
            is_edited: false,
            deleted_for_all: false,
            is_pinned: i == 0,
            is_bookmarked: false,
            status_code: 0,
            reactions: Vec::new(),
            attachments: Vec::new(),
            extra_meta: None,
        })
        .collect();

    let root_a = [0x33u8; 32];
    let norm_dm = format!("{ua}:{ub}");
    let key_a = derive_chunk_storage_key(&root_a, &alice.to_string(), ScopeKind::Dm, &norm_dm, 1).unwrap();
    let dict_reg = DictionaryRegistry::new();
    let cid = Uuid::new_v4();
    let sealed = seal_and_verify_message_chunk(
        &msgs,
        &SealChunkRequest {
            chunk_id: &cid.to_string(),
            account_id: &alice.to_string(),
            scope_type: ScopeKind::Dm,
            scope_id: &norm_dm,
            chunk_version: 1,
            access_epoch: 1,
            key_version: 1,
            candidate_dict_id: DICT_ID_PUBLIC_CHAT_V1,
            now_ms,
            require_space_savings: true,
            zstd_opts: Zstd22Options::default(),
        },
        &key_a,
        &dict_reg,
    )
    .unwrap();

    // 3a. Reject compression level != 22
    let (st_bad_lvl, _) = req_json(
        &app,
        "POST",
        "/api/storage/msg-chunks/commit",
        &c_alice,
        Some(json!({
            "scope_type": "dm", "scope_id": bob, "expected_manifest_version": 0,
            "chunk_id": cid, "compression_level": 19, "dictionary_id": sealed.metadata.dictionary_id,
            "original_size": sealed.metadata.original_size, "serialized_size": sealed.metadata.serialized_size,
            "compressed_size": sealed.metadata.compressed_size, "padded_size": sealed.metadata.padded_size,
            "message_count": sealed.metadata.message_count, "seq_start": sealed.metadata.seq_start,
            "seq_end": sealed.metadata.seq_end, "first_created_at_ms": sealed.metadata.first_created_at_ms,
            "last_created_at_ms": sealed.metadata.last_created_at_ms, "nonce_b64": sealed.metadata.nonce_b64,
            "plaintext_blake3_hex": sealed.metadata.plaintext_blake3_hex,
            "ciphertext_blake3_hex": sealed.metadata.ciphertext_blake3_hex,
            "payload_b64": B64.encode(&sealed.sealed_bytes),
        })),
    )
    .await;
    assert_eq!(st_bad_lvl, StatusCode::BAD_REQUEST);

    // 3b. Reject messages younger than 7 days
    let (st_young, _) = req_json(
        &app,
        "POST",
        "/api/storage/msg-chunks/commit",
        &c_alice,
        Some(json!({
            "scope_type": "dm", "scope_id": bob, "expected_manifest_version": 0,
            "chunk_id": cid, "compression_level": 22, "dictionary_id": sealed.metadata.dictionary_id,
            "original_size": sealed.metadata.original_size, "serialized_size": sealed.metadata.serialized_size,
            "compressed_size": sealed.metadata.compressed_size, "padded_size": sealed.metadata.padded_size,
            "message_count": sealed.metadata.message_count, "seq_start": sealed.metadata.seq_start,
            "seq_end": sealed.metadata.seq_end, "first_created_at_ms": now_ms - 86_400_000,
            "last_created_at_ms": now_ms - 3_600_000, "nonce_b64": sealed.metadata.nonce_b64,
            "plaintext_blake3_hex": sealed.metadata.plaintext_blake3_hex,
            "ciphertext_blake3_hex": sealed.metadata.ciphertext_blake3_hex,
            "payload_b64": B64.encode(&sealed.sealed_bytes),
        })),
    )
    .await;
    assert_eq!(st_young, StatusCode::BAD_REQUEST);

    // 3c. Reject stale OCC manifest_version (409 Conflict)
    let commit_body = json!({
        "scope_type": "dm", "scope_id": bob, "expected_manifest_version": 5,
        "lease_key": lease_key, "fencing_token": token_a,
        "chunk_id": cid, "compression_level": 22, "dictionary_id": sealed.metadata.dictionary_id,
        "original_size": sealed.metadata.original_size, "serialized_size": sealed.metadata.serialized_size,
        "compressed_size": sealed.metadata.compressed_size, "padded_size": sealed.metadata.padded_size,
        "message_count": sealed.metadata.message_count, "seq_start": sealed.metadata.seq_start,
        "seq_end": sealed.metadata.seq_end, "first_created_at_ms": sealed.metadata.first_created_at_ms,
        "last_created_at_ms": sealed.metadata.last_created_at_ms, "nonce_b64": sealed.metadata.nonce_b64,
        "plaintext_blake3_hex": sealed.metadata.plaintext_blake3_hex,
        "ciphertext_blake3_hex": sealed.metadata.ciphertext_blake3_hex,
        "payload_b64": B64.encode(&sealed.sealed_bytes),
    });
    let (st_occ, _) = req_json(&app, "POST", "/api/storage/msg-chunks/commit", &c_alice, Some(commit_body.clone())).await;
    assert_eq!(st_occ, StatusCode::CONFLICT);

    // 3d. Valid OCC commit (expected_manifest_version = 0 -> 1)
    let mut ok_body = commit_body;
    ok_body["expected_manifest_version"] = json!(0);
    let (st_ok, c_res) = req_json(&app, "POST", "/api/storage/msg-chunks/commit", &c_alice, Some(ok_body)).await;
    assert_eq!(st_ok, StatusCode::OK);
    assert_eq!(c_res["manifest_version"], 1);
    assert!(c_res["saved_bytes"].as_i64().unwrap() > 0);

    // 4. Cross-account isolation: Bob CANNOT download Alice's E2EE DM chunk!
    let (st_bob_deny, _, _) = req_raw(&app, "GET", &format!("/api/storage/msg-chunks/{cid}"), &c_bob).await;
    assert_eq!(st_bob_deny, StatusCode::FORBIDDEN);

    // Alice downloads, verifies, decrypts, and matches original 120 messages 100%!
    let (st_alice_dl, dl_bytes, dl_hdrs) = req_raw(&app, "GET", &format!("/api/storage/msg-chunks/{cid}"), &c_alice).await;
    assert_eq!(st_alice_dl, StatusCode::OK);
    assert_eq!(dl_hdrs.get("x-chatw-chunk-tier").unwrap(), "warm");
    let opened = open_and_verify_message_chunk(&sealed.metadata, &dl_bytes, &key_a, &dict_reg).unwrap();
    assert_eq!(opened, msgs);

    // 5. Append mutation/tombstone & read manifest
    let (st_mut, m_res) = req_json(
        &app,
        "POST",
        "/api/storage/msg-chunks/mutations",
        &c_alice,
        Some(json!({
            "scope_type": "dm", "scope_id": bob, "message_id": msgs[0].id,
            "chunk_id": cid, "op_type": "recall"
        })),
    )
    .await;
    assert_eq!(st_mut, StatusCode::OK);
    assert!(m_res["mutation_seq"].as_i64().unwrap() > 0);

    let (st_man, man) = req_json(&app, "GET", &format!("/api/storage/msg-chunks/manifest?scope_type=dm&scope_id={bob}"), &c_alice, None).await;
    assert_eq!(st_man, StatusCode::OK);
    assert_eq!(man["chunk_count"], 1);
    assert_eq!(man["mutations"].as_array().unwrap().len(), 1);

    // 6. Dictionary distribution (built-in public + E2EE private dict rejecting plaintext!)
    let (st_pub_dict, pub_bytes, _) = req_raw(&app, "GET", "/api/storage/msg-chunks/dictionaries/1", &c_alice).await;
    assert_eq!(st_pub_dict, StatusCode::OK);
    assert!(!pub_bytes.is_empty());

    let (st_plain_dict, _) = req_json(
        &app,
        "POST",
        "/api/storage/msg-chunks/dictionaries",
        &c_alice,
        Some(json!({ "dict_id": 1001, "scope_id": norm_dm, "version": 1, "sealed_dict_b64": B64.encode(b"plaintext_dict_forbidden") })),
    )
    .await;
    assert_eq!(st_plain_dict, StatusCode::BAD_REQUEST);

    let pdict_key = derive_private_dict_key(&root_a, &alice.to_string(), &norm_dm, 1).unwrap();
    let sealed_dict = seal_private_dictionary(&pdict_key, &alice.to_string(), &norm_dm, 1001, 1, b"custom dict bytes").unwrap();
    let (st_e2ee_dict, _) = req_json(
        &app,
        "POST",
        "/api/storage/msg-chunks/dictionaries",
        &c_alice,
        Some(json!({ "dict_id": 1001, "scope_id": norm_dm, "version": 1, "sealed_dict_b64": B64.encode(&sealed_dict) })),
    )
    .await;
    assert_eq!(st_e2ee_dict, StatusCode::OK);

    // 7. Encrypted Search Index (`CWSE1`) multi-device sync + plaintext rejection + cross-account isolation
    let search_key = derive_search_index_key(&root_a, &alice.to_string(), 1).unwrap();
    let mut s_idx = LocalSearchIndex::new();
    s_idx.set_scope_coverage(&format!("dm:{bob}"), SearchCoverageState::IndexedComplete);
    for (i, m) in msgs.iter().enumerate() {
        s_idx.upsert_message(ScopeKind::Dm, &bob.to_string(), None, Some(&cid.to_string()), i as u32, m);
    }
    let sealed_shard = s_idx.seal_encrypted_shard(&search_key, &alice.to_string(), "dm_all", 1).unwrap();
    let shard_b3 = blake3::hash(&sealed_shard).to_hex().to_string();

    // Plaintext search upload MUST be rejected!
    let (st_plain_idx, _) = req_json(
        &app,
        "PUT",
        "/api/storage/search-index/segments/dm_all/0",
        &c_alice,
        Some(json!({
            "expected_version": 0, "doc_count": 10, "min_seq": 1, "max_seq": 10,
            "blake3_hex": blake3::hash(b"plaintext index data longer than 32 bytes!!").to_hex().to_string(),
            "sealed_shard_b64": B64.encode(b"plaintext index data longer than 32 bytes!!")
        })),
    )
    .await;
    assert_eq!(st_plain_idx, StatusCode::BAD_REQUEST);

    // Encrypted `CWSE1` upload succeeds with OCC
    let (st_idx_ok, idx_res) = req_json(
        &app,
        "PUT",
        "/api/storage/search-index/segments/dm_all/0",
        &c_alice,
        Some(json!({
            "expected_version": 0, "key_version": 1, "doc_count": msgs.len(),
            "min_seq": 1, "max_seq": 120, "blake3_hex": shard_b3,
            "sealed_shard_b64": B64.encode(&sealed_shard)
        })),
    )
    .await;
    assert_eq!(st_idx_ok, StatusCode::OK);
    assert_eq!(idx_res["segment_version"], 1);

    // Bob CANNOT read Alice's search shard!
    let (st_bob_idx, _, _) = req_raw(&app, "GET", "/api/storage/search-index/segments/dm_all/0", &c_bob).await;
    assert_eq!(st_bob_idx, StatusCode::NOT_FOUND);

    // Alice downloads and decrypts search shard cleanly!
    let (st_alice_idx, got_shard, _) = req_raw(&app, "GET", "/api/storage/search-index/segments/dm_all/0", &c_alice).await;
    assert_eq!(st_alice_idx, StatusCode::OK);
    let restored_idx = LocalSearchIndex::open_encrypted_shard(&search_key, &alice.to_string(), "dm_all", 1, &got_shard).unwrap();
    assert_eq!(restored_idx.doc_count(), msgs.len());

    // 8. Server Incremental >=7d Plaintext Group Compaction + Warm->Cold + Cold->Warm Promotion
    let gid = Uuid::new_v4();
    sqlx::query("INSERT INTO groups (id, name, owner_id) VALUES ($1, 'Spec164 Group', $2)")
        .bind(gid)
        .bind(alice)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO group_members (group_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(gid)
        .bind(alice)
        .execute(&pool)
        .await
        .unwrap();

    // Insert 24 old (>= 10d) messages and 5 fresh (1d) messages
    for i in 0..24 {
        sqlx::query(
            "INSERT INTO group_messages (id, group_id, sender_id, content, created_at) VALUES ($1, $2, $3, $4, now() - interval '10 days')",
        )
        .bind(Uuid::new_v4())
        .bind(gid)
        .bind(alice)
        .bind(format!("Tin nhắn nhóm lịch sử #{i}: báo cáo tiến độ ChatW Zstd Level 22 thành công!"))
        .execute(&pool)
        .await
        .unwrap();
    }
    for i in 0..5 {
        sqlx::query(
            "INSERT INTO group_messages (id, group_id, sender_id, content, created_at) VALUES ($1, $2, $3, $4, now() - interval '1 day')",
        )
        .bind(Uuid::new_v4())
        .bind(gid)
        .bind(alice)
        .bind(format!("Fresh message #{i}"))
        .execute(&pool)
        .await
        .unwrap();
    }

    // Backdate Alice's DM chunk to 35 days ago so tiering moves it Warm -> Cold!
    sqlx::query("UPDATE message_chunks SET last_accessed_at = now() - interval '35 days' WHERE id = $1")
        .bind(cid)
        .execute(&pool)
        .await
        .unwrap();

    let report = storage::message_chunks::run_incremental_server_tiering_tick(&pool, &storage_reg, 2)
        .await
        .unwrap();
    assert_eq!(report.compacted_chunks, 1);
    assert_eq!(report.compacted_messages, 24, "only >=7d messages compacted; 5 fresh messages untouched!");
    assert!(report.bytes_saved > 0);
    assert!(report.moved_to_cold >= 1);

    // Accessing the Cold chunk promotes it back Cold -> Warm!
    let (st_cold_get, _, hdrs_cold) = req_raw(&app, "GET", &format!("/api/storage/msg-chunks/{cid}"), &c_alice).await;
    assert_eq!(st_cold_get, StatusCode::OK);
    assert_eq!(hdrs_cold.get("x-chatw-chunk-tier").unwrap(), "cold");
    let (_, _, hdrs_warm_again) = req_raw(&app, "GET", &format!("/api/storage/msg-chunks/{cid}"), &c_alice).await;
    assert_eq!(hdrs_warm_again.get("x-chatw-chunk-tier").unwrap(), "warm");

    // 9. Privacy-safe telemetry round-trip
    let (st_tel, _) = req_json(
        &app,
        "POST",
        "/api/storage/telemetry",
        &c_alice,
        Some(json!({
            "device_id": "dev-A", "platform": "linux", "storage_mode": "high_performance",
            "compression_ratio": 0.12, "cache_hit_rate": 0.87, "storage_usage_bytes": 12_000_000,
            "storage_budget_bytes": 1_000_000_000, "total_saved_bytes": 80_000_000,
            "indexed_message_count": 120, "last_search_latency_ms": 2.4, "chunk_count": 1
        })),
    )
    .await;
    assert_eq!(st_tel, StatusCode::OK);
    let (st_tel_g, tel_g) = req_json(&app, "GET", "/api/storage/telemetry", &c_alice, None).await;
    assert_eq!(st_tel_g, StatusCode::OK);
    assert_eq!(tel_g["devices"].as_array().unwrap().len(), 1);

    let _ = std::fs::remove_dir_all(&cfg.uploads_dir);
}
