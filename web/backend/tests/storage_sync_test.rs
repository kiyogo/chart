use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use image::{DynamicImage, Rgba, RgbaImage, codecs::png::PngEncoder};
use p256::{
    SecretKey,
    ecdsa::SigningKey,
    elliptic_curve::{rand_core::OsRng, sec1::ToEncodedPoint},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;
use uuid::Uuid;

use chatw::{
    attachments, audit,
    auth::{self, AppState, google::GoogleJwksCache, session::create_session},
    config::Config,
    crypto::{self, compute_fingerprint},
    db, devices_sync, friends, groups, messages, privacy, servers, storage, users,
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
        uploads_dir: std::env::temp_dir().join(format!("chatw_spec022_{tag}_{}", Uuid::new_v4().simple())),
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

async fn build_app(pool: sqlx::PgPool, config: Config) -> (axum::Router, Arc<storage::StorageRegistry>) {
    let storage_reg = Arc::new(storage::StorageRegistry::from_uploads_dir(&config.uploads_dir));
    let state = AppState {
        config: Arc::new(config),
        db: pool,
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
        .nest("/api/auth", auth::routes())
        .nest("/api/users", users::routes())
        .nest("/api/keys", crypto::keys::routes().merge(devices_sync::key_backup_routes()))
        .nest("/api/devices", devices_sync::device_routes())
        .nest("/api/sync", devices_sync::sync_routes())
        .nest("/api/recovery", devices_sync::recovery_routes())
        .nest("/api/storage", storage::router::routes())
        .nest("/api/friends", friends::routes())
        .nest("/api/groups", groups::routes())
        .nest("/api/messages", messages::routes())
        .nest("/api/attachments", attachments::routes())
        .nest("/api/audit", audit::routes())
        .nest("/api/servers", servers::server_routes())
        .nest("/api/channels", servers::channel_routes())
        .nest("/api/stickers", chatw::stickers::routes())
        .nest("/api/calls", chatw::calls::routes())
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
    let cookie = create_session(pool, config, uid, Some("Mozilla/5.0 Linux")).await.expect("session");
    let cookie_kv = cookie.split(';').next().unwrap().to_string();
    (uid, uname, cookie_kv)
}

async fn make_friends(pool: &sqlx::PgPool, a: Uuid, b: Uuid) {
    let (u_a, u_b) = if a < b { (a, b) } else { (b, a) };
    sqlx::query("INSERT INTO friendships (user_a, user_b) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(u_a)
        .bind(u_b)
        .execute(pool)
        .await
        .expect("friendship");
}

async fn req_json(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri).header(header::COOKIE, cookie);
    let body_obj = if let Some(val) = body {
        b = b.header(header::CONTENT_TYPE, "application/json");
        Body::from(val.to_string())
    } else {
        Body::empty()
    };
    let resp = app.clone().oneshot(b.body(body_obj).unwrap()).await.unwrap();
    let st = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 32 * 1024 * 1024).await.unwrap();
    let val = serde_json::from_slice(&bytes).unwrap_or(json!({ "raw": String::from_utf8_lossy(&bytes).to_string() }));
    (st, val)
}

async fn req_bytes(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: &str,
    ct: &str,
    bytes: Vec<u8>,
    extra_headers: &[(&str, &str)],
) -> (StatusCode, Vec<u8>, axum::http::HeaderMap) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, ct);
    for (k, v) in extra_headers {
        b = b.header(*k, *v);
    }
    let resp = app.clone().oneshot(b.body(Body::from(bytes)).unwrap()).await.unwrap();
    let st = resp.status();
    let hdrs = resp.headers().clone();
    let out = axum::body::to_bytes(resp.into_body(), 32 * 1024 * 1024).await.unwrap().to_vec();
    (st, out, hdrs)
}

fn sha_hex(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    hex::encode(h.finalize())
}

fn sample_png_bytes(w: u32, h: u32) -> Vec<u8> {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(x, y, Rgba([((x * 3) % 255) as u8, ((y * 5) % 255) as u8, 200, 255]));
        }
    }
    let mut out = Vec::new();
    DynamicImage::ImageRgba8(img).write_with_encoder(PngEncoder::new(&mut out)).unwrap();
    out
}

fn sample_meme_png(seed: u64, w: u32, h: u32, caption_variant: u8, perturb: u8) -> Vec<u8> {
    let mut img = RgbaImage::new(w, h);
    let freq_x = 0.05 + ((seed % 17) as f32) * 0.009;
    let freq_y = 0.05 + (((seed >> 8) % 17) as f32) * 0.009;
    let phase_x = (((seed >> 16) % 628) as f32) * 0.01;
    let phase_y = (((seed >> 24) % 628) as f32) * 0.01;
    for y in 0..h {
        for x in 0..w {
            let wave = (((x as f32 * freq_x + phase_x).sin() + (y as f32 * freq_y + phase_y).cos()) * 45.0 + 128.0) as u8;
            let r = wave.saturating_add(perturb);
            let g = (((x + y + (seed as u32 % 250)) / 2) % 255) as u8;
            let b = (255u8).saturating_sub(wave / 2);
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    // Caption stripe on top
    for y in 4..(h / 6) {
        for x in 8..(w - 8) {
            let stripe = if caption_variant == 1 { (x / 6) % 2 == 0 } else { ((x + 15) / 11) % 3 == 0 };
            let val = if stripe { 255 } else { 10 };
            img.put_pixel(x, y, Rgba([val, val, val, 255]));
        }
    }
    let mut out = Vec::new();
    DynamicImage::ImageRgba8(img).write_with_encoder(PngEncoder::new(&mut out)).unwrap();
    out
}

fn sample_meme_jpg(seed: u64, w: u32, h: u32, caption_variant: u8, perturb: u8) -> Vec<u8> {
    let mut img = RgbaImage::new(w, h);
    let freq_x = 0.05 + ((seed % 17) as f32) * 0.009;
    let freq_y = 0.05 + (((seed >> 8) % 17) as f32) * 0.009;
    let phase_x = (((seed >> 16) % 628) as f32) * 0.01;
    let phase_y = (((seed >> 24) % 628) as f32) * 0.01;
    for y in 0..h {
        for x in 0..w {
            let wave = (((x as f32 * freq_x + phase_x).sin() + (y as f32 * freq_y + phase_y).cos()) * 45.0 + 128.0) as u8;
            let r = wave.saturating_add(perturb);
            let g = (((x + y + (seed as u32 % 250)) / 2) % 255) as u8;
            let b = (255u8).saturating_sub(wave / 2);
            img.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }
    // Caption stripe on top
    for y in 4..(h / 6) {
        for x in 8..(w - 8) {
            let stripe = if caption_variant == 1 { (x / 6) % 2 == 0 } else { ((x + 15) / 11) % 3 == 0 };
            let val = if stripe { 255 } else { 10 };
            img.put_pixel(x, y, Rgba([val, val, val, 255]));
        }
    }
    let mut out = Vec::new();
    let rgb = DynamicImage::ImageRgba8(img).to_rgb8();
    DynamicImage::ImageRgb8(rgb).write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)).unwrap();
    out
}

#[tokio::test]
async fn scenario_01_to_05_tiers_rbac_idor_and_legacy_compat() {
    let pool = setup_db().await;
    let cfg = test_config("s1");
    let (app, _reg) = build_app(pool.clone(), cfg.clone()).await;

    let (alice, _, c_alice) = create_test_user(&pool, &cfg, "alice").await;
    let (bob, _, c_bob) = create_test_user(&pool, &cfg, "bob").await;
    let (_eve, _, c_eve) = create_test_user(&pool, &cfg, "eve").await;
    make_friends(&pool, alice, bob).await;

    // 1. DM Temp Relay upload & strict IDOR block for Eve
    let dm_payload = b"encrypted_dm_cipher_blob_xyz".to_vec();
    let (st, raw_dm, _) = req_bytes(
        &app,
        "POST",
        &format!("/api/attachments?scope_type=dm&scope_id={bob}&is_e2ee=true&tier=temp_relay&file_name=secret.bin"),
        &c_alice,
        "application/octet-stream",
        dm_payload.clone(),
        &[],
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let dm_json: Value = serde_json::from_slice(&raw_dm).unwrap();
    assert_eq!(dm_json["storage_tier"], "temp_relay");
    let dm_url = dm_json["download_url"].as_str().unwrap();

    // Alice & Bob can download; Eve MUST get 403 Forbidden (IDOR fixed!)
    let (st_bob, got_bob, _) = req_bytes(&app, "GET", dm_url, &c_bob, "", vec![], &[]).await;
    assert_eq!(st_bob, StatusCode::OK);
    assert_eq!(got_bob, dm_payload);
    let (st_eve, _, _) = req_bytes(&app, "GET", dm_url, &c_eve, "", vec![], &[]).await;
    assert_eq!(st_eve, StatusCode::FORBIDDEN);

    // 2. Group chat durable storage + RBAC enforcement
    let (gst, gval) = req_json(&app, "POST", "/api/groups", &c_alice, Some(json!({ "name": "DevGroup" }))).await;
    assert_eq!(gst, StatusCode::OK);
    let gid = gval["id"].as_str().unwrap();
    req_json(&app, "POST", &format!("/api/groups/{gid}/members"), &c_alice, Some(json!({ "user_id": bob }))).await;

    let grp_png = sample_png_bytes(600, 400);
    let (st_gup, gup_raw, _) = req_bytes(
        &app,
        "POST",
        &format!("/api/attachments?scope_type=group&scope_id={gid}&is_e2ee=false&preserve_original=false&file_name=diagram.png"),
        &c_alice,
        "image/png",
        grp_png.clone(),
        &[],
    )
    .await;
    assert_eq!(st_gup, StatusCode::OK);
    let gup_json: Value = serde_json::from_slice(&gup_raw).unwrap();
    assert_eq!(gup_json["storage_tier"], "server_durable");
    let g_url = gup_json["download_url"].as_str().unwrap();

    let (st_gb, got_gb, _) = req_bytes(&app, "GET", g_url, &c_bob, "", vec![], &[]).await;
    assert_eq!(st_gb, StatusCode::OK);
    assert_eq!(got_gb, grp_png); // Original preserved byte-for-byte!
    let (st_ge, _, _) = req_bytes(&app, "GET", g_url, &c_eve, "", vec![], &[]).await;
    assert_eq!(st_ge, StatusCode::FORBIDDEN);

    // 3. Server channel durable storage + strict RBAC & no_media check
    let (sst, sval) = req_json(&app, "POST", "/api/servers", &c_alice, Some(json!({ "name": "OpsServer" }))).await;
    assert_eq!(sst, StatusCode::OK);
    let sid = sval["id"].as_str().unwrap();
    let (_, chans) = req_json(&app, "GET", &format!("/api/servers/{sid}/channels"), &c_alice, None).await;
    let ch_id = chans[0]["id"].as_str().unwrap();

    let (st_ch_up, ch_raw, _) = req_bytes(
        &app,
        "POST",
        &format!("/api/attachments?scope_type=channel&scope_id={ch_id}&is_e2ee=false&file_name=spec.png"),
        &c_alice,
        "image/png",
        grp_png.clone(),
        &[],
    )
    .await;
    assert_eq!(st_ch_up, StatusCode::OK);
    let ch_json: Value = serde_json::from_slice(&ch_raw).unwrap();
    assert_eq!(ch_json["storage_tier"], "server_durable");
    let ch_url = ch_json["download_url"].as_str().unwrap();
    let (st_ch_eve, _, _) = req_bytes(&app, "GET", ch_url, &c_eve, "", vec![], &[]).await;
    assert_eq!(st_ch_eve, StatusCode::FORBIDDEN);

    // 4. Legacy attachments backward compatibility & unacquainted block
    let legacy_id = Uuid::new_v4();
    let legacy_dir = cfg.uploads_dir.join("attachments");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    let legacy_file = legacy_dir.join(format!("{legacy_id}.bin"));
    std::fs::write(&legacy_file, b"legacy_hello").unwrap();
    sqlx::query(
        "INSERT INTO attachments (id, uploader_id, file_path, file_name, file_size, mime_type) VALUES ($1, $2, $3, 'old.bin', 12, 'application/octet-stream')",
    )
    .bind(legacy_id)
    .bind(alice)
    .bind(legacy_file.to_string_lossy().to_string())
    .execute(&pool)
    .await
    .unwrap();
    let (st_leg_bob, leg_bytes, _) = req_bytes(&app, "GET", &format!("/api/attachments/{legacy_id}"), &c_bob, "", vec![], &[]).await;
    assert_eq!(st_leg_bob, StatusCode::OK);
    assert_eq!(leg_bytes, b"legacy_hello");
    let (st_leg_eve, _, _) = req_bytes(&app, "GET", &format!("/api/attachments/{legacy_id}"), &c_eve, "", vec![], &[]).await;
    assert_eq!(st_leg_eve, StatusCode::FORBIDDEN);

    let _ = std::fs::remove_dir_all(&cfg.uploads_dir);
}

#[tokio::test]
async fn scenario_06_to_14_avatar_honesty_dedup_forward_chunking_and_relay_safety() {
    let pool = setup_db().await;
    let cfg = test_config("s6");
    let (app, storage_reg) = build_app(pool.clone(), cfg.clone()).await;

    let (alice, _, c_alice) = create_test_user(&pool, &cfg, "av_alice").await;
    let (bob, _, c_bob) = create_test_user(&pool, &cfg, "av_bob").await;
    make_friends(&pool, alice, bob).await;

    // 6. Avatar upload format honesty (PNG uploaded -> genuine 512x512 matching extension & magic bytes)
    let png = sample_png_bytes(700, 500);
    let (st_av, av_raw, _) = req_bytes(&app, "POST", "/api/users/avatar", &c_alice, "image/png", png.clone(), &[]).await;
    assert_eq!(st_av, StatusCode::OK);
    let av_json: Value = serde_json::from_slice(&av_raw).unwrap();
    let av_url = av_json["avatar_url"].as_str().unwrap();
    let rel_path = av_url.split('?').next().unwrap().trim_start_matches("/uploads/");
    let disk_bytes = std::fs::read(cfg.uploads_dir.join(rel_path)).unwrap();
    let sniffed = chatw::media_opt::sniff_media_bytes(&disk_bytes);
    assert!(av_url.contains(&format!(".{}", sniffed.ext)), "extension must match real magic bytes ({av_url} vs {})", sniffed.ext);

    // 7. Dedup & refcount GC in Group scope
    let (_, gval) = req_json(&app, "POST", "/api/groups", &c_alice, Some(json!({ "name": "DedupGrp" }))).await;
    let gid = gval["id"].as_str().unwrap();
    req_json(&app, "POST", &format!("/api/groups/{gid}/members"), &c_alice, Some(json!({ "user_id": bob }))).await;
    let blob = b"repeatable-group-document-payload-for-dedup-test-12345".to_vec();
    let (_, r1_raw, _) = req_bytes(&app, "POST", &format!("/api/attachments?scope_type=group&scope_id={gid}&file_name=doc1.bin"), &c_alice, "application/octet-stream", blob.clone(), &[]).await;
    let (_, r2_raw, _) = req_bytes(&app, "POST", &format!("/api/attachments?scope_type=group&scope_id={gid}&file_name=doc2.bin"), &c_alice, "application/octet-stream", blob.clone(), &[]).await;
    let r1: Value = serde_json::from_slice(&r1_raw).unwrap();
    let r2: Value = serde_json::from_slice(&r2_raw).unwrap();
    assert_eq!(r1["deduplicated"], false);
    assert_eq!(r2["deduplicated"], true);
    let ref1 = r1["ref_id"].as_str().unwrap();
    let ref2 = r2["ref_id"].as_str().unwrap();

    // Forward without physical duplication
    let (fst, fval) = req_json(&app, "POST", "/api/storage/forward", &c_alice, Some(json!({
        "from_ref_id": ref1,
        "target_scope_type": "group",
        "target_scope_id": gid,
    }))).await;
    assert_eq!(fst, StatusCode::OK);
    assert_eq!(fval["reused_object"], true);
    let ref3 = fval["ref_id"].as_str().unwrap();

    // Check ref_count == 3, delete all 3 -> physical file deleted ONLY on last delete!
    let rc3: i32 = sqlx::query_scalar("SELECT o.ref_count FROM media_references r JOIN media_objects o ON o.id = r.object_id WHERE r.id = $1::uuid")
        .bind(ref1).fetch_one(&pool).await.unwrap();
    assert_eq!(rc3, 3);
    req_json(&app, "DELETE", &format!("/api/storage/references/{ref1}?scope=everyone"), &c_alice, None).await;
    req_json(&app, "DELETE", &format!("/api/storage/references/{ref2}?scope=everyone"), &c_alice, None).await;
    let rc1: i32 = sqlx::query_scalar("SELECT o.ref_count FROM media_references r JOIN media_objects o ON o.id = r.object_id WHERE r.id = $1::uuid")
        .bind(ref3).fetch_one(&pool).await.unwrap();
    assert_eq!(rc1, 1);
    req_json(&app, "DELETE", &format!("/api/storage/references/{ref3}?scope=everyone"), &c_alice, None).await;
    let remaining_objs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media_objects WHERE sha256 = $1")
        .bind(&storage::sha256_bytes(&blob)[..]).fetch_one(&pool).await.unwrap();
    assert_eq!(remaining_objs, 0);

    // 8. Perceptual Meme Deduplication with Meta PDQ 256-bit DCT & Caption-Band Guard
    let meme_seed = u64::from_le_bytes(Uuid::new_v4().as_bytes()[0..8].try_into().unwrap());
    let meme1_png = sample_meme_png(meme_seed, 128, 128, 1, 0);
    let meme2_reenc_jpg = sample_meme_jpg(meme_seed, 128, 128, 1, 4);
    let meme3_diff_caption_jpg = sample_meme_jpg(meme_seed, 128, 128, 2, 0);

    // Alice uploads master meme 1 (PNG)
    let (_, m1_raw, _) = req_bytes(&app, "POST", &format!("/api/attachments?scope_type=group&scope_id={gid}&file_name=meme1.png"), &c_alice, "image/png", meme1_png.clone(), &[]).await;
    let m1_val: Value = serde_json::from_slice(&m1_raw).unwrap();
    assert_eq!(m1_val["deduplicated"], false);
    let m1_ref = m1_val["ref_id"].as_str().unwrap();

    // Bob uploads meme 2 (JPEG re-encode with JPEG artifacts & distinct hashes, but visually identical meme & caption)
    let (_, m2_raw, _) = req_bytes(&app, "POST", &format!("/api/attachments?scope_type=group&scope_id={gid}&file_name=meme2.jpg"), &c_bob, "image/jpeg", meme2_reenc_jpg.clone(), &[]).await;
    let m2_val: Value = serde_json::from_slice(&m2_raw).unwrap();
    assert_ne!(sha_hex(&meme1_png), sha_hex(&meme2_reenc_jpg), "Hashes MUST be distinct to test perceptual deduplication!");
    assert_eq!(m2_val["deduplicated"], true);
    assert_eq!(m2_val["dedup_match_kind"], "perceptual_pdq_mih_match");
    let m2_ref = m2_val["ref_id"].as_str().unwrap();

    // Bob can download and gets the high quality master file
    let (st_m2_get, m2_got, _) = req_bytes(&app, "GET", m2_val["download_url"].as_str().unwrap(), &c_bob, "", vec![], &[]).await;
    assert_eq!(st_m2_get, StatusCode::OK);
    assert_eq!(m2_got, meme1_png);

    // Bob uploads meme 3 (same background image BUT different caption text!)
    let (_, m3_raw, _) = req_bytes(&app, "POST", &format!("/api/attachments?scope_type=group&scope_id={gid}&file_name=meme3.jpg"), &c_bob, "image/jpeg", meme3_diff_caption_jpg.clone(), &[]).await;
    let m3_val: Value = serde_json::from_slice(&m3_raw).unwrap();
    // Must NOT merge because caption text differs!
    assert_eq!(m3_val["deduplicated"], false, "Different meme caption must NEVER be falsely deduplicated!");

    // Instant Multipart Init Deduplication Check
    let meme1_sha = sha_hex(&meme1_png);
    let (st_init_dedup, init_dedup_val) = req_json(&app, "POST", "/api/storage/uploads/init", &c_alice, Some(json!({
        "scope_type": "group",
        "scope_id": gid,
        "file_name": "instant_meme.png",
        "mime_type": "image/png",
        "total_size": meme1_png.len(),
        "sha256_cipher": meme1_sha,
    }))).await;
    assert_eq!(st_init_dedup, StatusCode::OK);
    assert_eq!(init_dedup_val["dedup_instant"], true);
    assert!(init_dedup_val["download_url"].as_str().unwrap().starts_with("/api/attachments/"));

    // Clean up test memes
    req_json(&app, "DELETE", &format!("/api/storage/references/{m1_ref}?scope=everyone"), &c_alice, None).await;
    req_json(&app, "DELETE", &format!("/api/storage/references/{m2_ref}?scope=everyone"), &c_bob, None).await;

    // 11. Chunked multipart resumable upload + Range download
    let mut big = vec![0x42u8; 10_000];
    big[0..16].copy_from_slice(Uuid::new_v4().as_bytes());
    big[5000..5010].copy_from_slice(b"HELLO_RNG!");
    let total_sha = sha_hex(&big);
    let (ist, ival) = req_json(&app, "POST", "/api/storage/uploads/init", &c_alice, Some(json!({
        "scope_type": "group",
        "scope_id": gid,
        "file_name": "chunked.bin",
        "mime_type": "application/octet-stream",
        "total_size": big.len(),
        "chunk_size": 5000,
        "sha256_cipher": total_sha,
    }))).await;
    assert_eq!(ist, StatusCode::OK);
    let tid = ival["transfer_id"].as_str().unwrap();

    // Chunk 0 valid
    let c0 = big[0..5000].to_vec();
    let (st_c0, _, _) = req_bytes(&app, "PUT", &format!("/api/storage/uploads/{tid}/chunks/0"), &c_alice, "application/octet-stream", c0, &[("x-chunk-sha256", &sha_hex(&big[0..5000]))]).await;
    assert_eq!(st_c0, StatusCode::OK);
    // Resume status reports missing chunk [1]
    let (_, st_val) = req_json(&app, "GET", &format!("/api/storage/uploads/{tid}"), &c_alice, None).await;
    assert_eq!(st_val["missing_chunks"], json!([1]));
    // Corrupted chunk SHA rejected
    let (st_bad, _, _) = req_bytes(&app, "PUT", &format!("/api/storage/uploads/{tid}/chunks/1"), &c_alice, "application/octet-stream", big[5000..].to_vec(), &[("x-chunk-sha256", &"0".repeat(64))]).await;
    assert_eq!(st_bad, StatusCode::BAD_REQUEST);
    // Correct chunk 1 accepted & assembled
    let (st_c1, _, _) = req_bytes(&app, "PUT", &format!("/api/storage/uploads/{tid}/chunks/1"), &c_alice, "application/octet-stream", big[5000..].to_vec(), &[]).await;
    assert_eq!(st_c1, StatusCode::OK);
    let (st_comp, comp_val) = req_json(&app, "POST", &format!("/api/storage/uploads/{tid}/complete"), &c_alice, Some(json!({}))).await;
    assert_eq!(st_comp, StatusCode::OK);
    let chunk_ref = comp_val["ref_id"].as_str().unwrap();

    // Byte Range check
    let (st_rng, rng_bytes, _) = req_bytes(&app, "GET", &format!("/api/attachments/{chunk_ref}"), &c_bob, "", vec![], &[("Range", "bytes=5000-5009")]).await;
    assert_eq!(st_rng, StatusCode::PARTIAL_CONTENT);
    assert_eq!(rng_bytes, b"HELLO_RNG!");

    // 13. Temp relay safe deletion guard: force expired timestamp, verify unacked stays alive, after both ack -> cleaned!
    // Register Alice & Bob devices first so ack tracks devices cleanly.
    req_json(&app, "POST", "/api/devices/register", &c_alice, Some(json!({ "client_device_id": format!("dev-a-{}", Uuid::new_v4().simple()), "platform": "web" }))).await;
    req_json(&app, "POST", "/api/devices/register", &c_bob, Some(json!({ "client_device_id": format!("dev-b-{}", Uuid::new_v4().simple()), "platform": "web" }))).await;
    let (_, dm_r_raw, _) = req_bytes(&app, "POST", &format!("/api/attachments?scope_type=dm&scope_id={bob}&is_e2ee=true&tier=temp_relay"), &c_alice, "application/octet-stream", b"relay_protected".to_vec(), &[]).await;
    let dm_r_id = serde_json::from_slice::<Value>(&dm_r_raw).unwrap()["ref_id"].as_str().unwrap().to_string();
    sqlx::query("UPDATE media_references SET relay_expires_at = now() - interval '1 hour' WHERE id = $1::uuid").bind(&dm_r_id).execute(&pool).await.unwrap();
    let cl1 = storage::migration::cleanup_relays_safely(&pool, &storage_reg).await.unwrap();
    assert!(cl1.expiring_without_safe_copies >= 1);
    let still_exists: Option<Uuid> = sqlx::query_scalar("SELECT object_id FROM media_references WHERE id = $1::uuid").bind(&dm_r_id).fetch_one(&pool).await.unwrap();
    assert!(still_exists.is_some(), "unacked relay must NOT be purged prematurely!");

    // Both Alice & Bob ack copies -> safe purge executes!
    req_json(&app, "POST", &format!("/api/storage/relay/{dm_r_id}/ack"), &c_alice, Some(json!({ "local_tier": "opfs", "sha256_hex": sha_hex(b"relay_protected") }))).await;
    let (_, ack_b) = req_json(&app, "POST", &format!("/api/storage/relay/{dm_r_id}/ack"), &c_bob, Some(json!({ "local_tier": "indexeddb", "sha256_hex": sha_hex(b"relay_protected") }))).await;
    assert_eq!(ack_b["relay_cleaned"], true);

    let _ = std::fs::remove_dir_all(&cfg.uploads_dir);
}

#[tokio::test]
async fn scenario_15_to_25_devices_approval_sync_and_peer_recovery() {
    let pool = setup_db().await;
    let cfg = test_config("s15");
    let (app, _) = build_app(pool.clone(), cfg.clone()).await;

    let (alice, _, c_alice_dev1) = create_test_user(&pool, &cfg, "sync_alice").await;
    let (bob, _, c_bob_dev1) = create_test_user(&pool, &cfg, "sync_bob").await;
    make_friends(&pool, alice, bob).await;

    // 15. Device 1 auto-trusted; second session / device enters 'pending' and cannot delta sync until approved!
    let (st_d1, d1) = req_json(&app, "POST", "/api/devices/register", &c_alice_dev1, Some(json!({
        "client_device_id": format!("alice-primary-{}", Uuid::new_v4().simple()),
        "platform": "desktop",
        "device_name": "Linux Desktop",
    }))).await;
    assert_eq!(st_d1, StatusCode::OK);
    assert_eq!(d1["trust_state"], "trusted");

    let s_alice2 = create_session(&pool, &cfg, alice, Some("Mozilla/5.0 Android")).await.unwrap();
    let c_alice_dev2 = s_alice2.split(';').next().unwrap().to_string();
    let (_, d2) = req_json(&app, "POST", "/api/devices/register", &c_alice_dev2, Some(json!({
        "client_device_id": format!("alice-phone-{}", Uuid::new_v4().simple()),
        "platform": "mobile",
        "device_name": "Android Phone",
    }))).await;
    assert_eq!(d2["trust_state"], "pending");
    let code = d2["verification_code"].as_str().unwrap();
    let req_id = d2["request_id"].as_str().unwrap();

    // Pending device blocked from delta sync!
    let (st_block, _) = req_json(&app, "GET", "/api/sync/delta?dm_since=0", &c_alice_dev2, None).await;
    assert_eq!(st_block, StatusCode::FORBIDDEN);

    // Wrong code fails; right code unlocks delta sync
    let (st_wc, _) = req_json(&app, "POST", &format!("/api/devices/requests/{req_id}/approve"), &c_alice_dev1, Some(json!({ "verification_code": "999999" }))).await;
    assert_eq!(st_wc, StatusCode::BAD_REQUEST);
    let (st_ok, _) = req_json(&app, "POST", &format!("/api/devices/requests/{req_id}/approve"), &c_alice_dev1, Some(json!({ "verification_code": code }))).await;
    assert_eq!(st_ok, StatusCode::OK);
    let (st_sync, _) = req_json(&app, "GET", "/api/sync/delta?dm_since=0", &c_alice_dev2, None).await;
    assert_eq!(st_sync, StatusCode::OK);

    // 16 & 17. Third device rejected + revoked immediately loses auth
    let s_alice3 = create_session(&pool, &cfg, alice, Some("Unknown Browser")).await.unwrap();
    let c_alice_dev3 = s_alice3.split(';').next().unwrap().to_string();
    let (_, d3) = req_json(&app, "POST", "/api/devices/register", &c_alice_dev3, Some(json!({
        "client_device_id": format!("alice-rogue-{}", Uuid::new_v4().simple()),
        "platform": "web",
    }))).await;
    let req3 = d3["request_id"].as_str().unwrap();
    let (st_rej, _) = req_json(&app, "POST", &format!("/api/devices/requests/{req3}/reject"), &c_alice_dev1, Some(json!({}))).await;
    assert_eq!(st_rej, StatusCode::OK);
    let (st_after_rej, _) = req_json(&app, "GET", "/api/auth/me", &c_alice_dev3, None).await;
    assert_eq!(st_after_rej, StatusCode::UNAUTHORIZED);

    // 18. Encrypted key backup + Lost-all-device fallback recovery
    let ecdh = SecretKey::random(&mut OsRng);
    let ecdh_pub = ecdh.public_key().to_encoded_point(false);
    let fp = compute_fingerprint(ecdh_pub.as_bytes());
    let fp_b64 = URL_SAFE_NO_PAD.encode(fp);
    let ecdsa = SigningKey::random(&mut OsRng);
    let (st_keys, _) = req_json(&app, "POST", "/api/keys", &c_alice_dev1, Some(json!({
        "ecdh_public": URL_SAFE_NO_PAD.encode(ecdh_pub.as_bytes()),
        "ecdsa_public": URL_SAFE_NO_PAD.encode(ecdsa.verifying_key().to_encoded_point(false).as_bytes()),
    }))).await;
    assert_eq!(st_keys, StatusCode::OK);
    let (st_kb, _) = req_json(&app, "PUT", "/api/keys/backup", &c_alice_dev1, Some(json!({
        "wrapped_keys_b64": "dGVzdF93cmFwcGVkX2Jsb2I",
        "kdf_salt_b64": "dGVzdF9zYWx0",
        "kdf_iterations": 50000,
        "key_fingerprint_b64": fp_b64,
    }))).await;
    assert_eq!(st_kb, StatusCode::OK);

    let s_alice4 = create_session(&pool, &cfg, alice, Some("Fresh Laptop")).await.unwrap();
    let c_alice_dev4 = s_alice4.split(';').next().unwrap().to_string();
    let (_, reg4_val) = req_json(&app, "POST", "/api/devices/register", &c_alice_dev4, Some(json!({ "client_device_id": format!("alice-lost-{}", Uuid::new_v4().simple()), "platform": "desktop" }))).await;
    let did4_str = reg4_val["device_id"].as_str().unwrap();
    let did4 = Uuid::parse_str(did4_str).unwrap();
    use p256::ecdsa::signature::Signer;
    let mut rec_msg = Vec::new();
    rec_msg.extend_from_slice(b"chatw-recover-device-v1:");
    rec_msg.extend_from_slice(did4.as_bytes());
    let sig: p256::ecdsa::Signature = ecdsa.sign(&rec_msg);
    let proof_b64 = URL_SAFE_NO_PAD.encode(sig.to_bytes());
    let (st_rec, rec_val) = req_json(&app, "POST", "/api/devices/recover-lost", &c_alice_dev4, Some(json!({
        "revoke_unreachable_devices": false,
        "recovery_proof_b64": proof_b64,
    }))).await;
    assert_eq!(st_rec, StatusCode::OK);
    assert_eq!(rec_val["trust_state"], "trusted");
    assert_eq!(rec_val["key_backup"]["wrapped_keys_b64"], "dGVzdF93cmFwcGVkX2Jsb2I");

    // 21-23. Two-Way DM Peer Recovery: happy path + SHA verify + denied by peer + no remaining copies honesty
    req_json(&app, "POST", "/api/devices/register", &c_bob_dev1, Some(json!({ "client_device_id": format!("bob-dev-{}", Uuid::new_v4().simple()), "platform": "web" }))).await;
    let file_bytes = b"precious_dm_photo_bytes_to_recover".to_vec();
    let file_sha = sha_hex(&file_bytes);
    let (_, fraw, _) = req_bytes(&app, "POST", &format!("/api/attachments?scope_type=dm&scope_id={bob}&is_e2ee=true&sha256_plain={file_sha}"), &c_alice_dev1, "application/octet-stream", file_bytes.clone(), &[]).await;
    let fref = serde_json::from_slice::<Value>(&fraw).unwrap()["ref_id"].as_str().unwrap().to_string();
    // Both ack local copy -> server deletes relay -> Bob deletes local cache!
    req_json(&app, "POST", &format!("/api/storage/relay/{fref}/ack"), &c_alice_dev1, Some(json!({ "local_tier": "opfs", "sha256_hex": file_sha }))).await;
    req_json(&app, "POST", &format!("/api/storage/relay/{fref}/ack"), &c_bob_dev1, Some(json!({ "local_tier": "indexeddb", "sha256_hex": file_sha }))).await;
    // Wait simulate relay cleanup removing object_id so server has zero copies:
    sqlx::query("UPDATE media_references SET object_id = NULL, storage_tier = 'dm_client_preferred' WHERE id = $1::uuid").bind(&fref).execute(&pool).await.unwrap();
    // Deleting Bob's local cache removes Bob's media_device_locations entry; Alice still has copy!
    let (st_lc, _) = req_json(&app, "DELETE", &format!("/api/storage/references/{fref}?scope=local_cache"), &c_bob_dev1, None).await;
    assert_eq!(st_lc, StatusCode::OK);

    // Bob requests recovery from Alice -> Alice accepts -> Bob completes with verified SHA-256!
    let (st_rr, rr_val) = req_json(&app, "POST", "/api/recovery/requests", &c_bob_dev1, Some(json!({ "ref_id": fref, "preferred_transport": "webrtc_p2p" }))).await;
    assert_eq!(st_rr, StatusCode::OK);
    let rr_id = rr_val["request_id"].as_str().unwrap();
    let (st_resp, _) = req_json(&app, "POST", &format!("/api/recovery/requests/{rr_id}/respond"), &c_alice_dev1, Some(json!({ "action": "accept", "transport_mode": "webrtc_p2p" }))).await;
    assert_eq!(st_resp, StatusCode::OK);
    let (st_done, done_val) = req_json(&app, "POST", &format!("/api/recovery/requests/{rr_id}/complete"), &c_bob_dev1, Some(json!({ "sha256_hex": file_sha, "local_tier": "opfs" }))).await;
    assert_eq!(st_done, StatusCode::OK);
    assert_eq!(done_val["availability_state"], "restored_from_peer");

    // 22. Per-conversation peer recovery policy disable blocks future peer requests
    req_json(&app, "PUT", &format!("/api/storage/conversations/{bob}"), &c_alice_dev1, Some(json!({ "allow_peer_recovery": false, "dm_encrypted_backup": false }))).await;
    let (_, denied_val) = req_json(&app, "POST", "/api/recovery/requests", &c_bob_dev1, Some(json!({ "ref_id": fref }))).await;
    assert_eq!(denied_val["status"], "denied");
    assert_eq!(denied_val["reason"], "peer_recovery_disabled");

    // 23. Zero copies anywhere & peer offline -> honestly returns no_remaining_copies
    sqlx::query("DELETE FROM media_device_locations WHERE ref_id = $1::uuid").bind(&fref).execute(&pool).await.unwrap();
    req_json(&app, "PUT", &format!("/api/storage/conversations/{bob}"), &c_alice_dev1, Some(json!({ "allow_peer_recovery": true, "dm_encrypted_backup": false }))).await;
    let (_, zero_val) = req_json(&app, "POST", "/api/recovery/requests", &c_bob_dev1, Some(json!({ "ref_id": fref }))).await;
    assert_eq!(zero_val["status"], "no_remaining_copies");

    // 24 & 25. Metrics & S3 Migration dry-run verification
    // Spec 161: storage metrics/migration are admin-only.
    let (st_met_forbidden, _) = req_json(&app, "GET", "/api/storage/metrics", &c_bob_dev1, None).await;
    assert_eq!(st_met_forbidden, StatusCode::FORBIDDEN);
    sqlx::query("UPDATE users SET is_admin = TRUE WHERE id = $1").bind(alice).execute(&pool).await.unwrap();
    let (st_met, met_val) = req_json(&app, "GET", "/api/storage/metrics", &c_alice_dev1, None).await;
    assert_eq!(st_met, StatusCode::OK);
    assert!(met_val["server"]["physical_bytes"].as_i64().is_some());
    let (st_mig, mig_val) = req_json(&app, "POST", "/api/storage/migration/run", &c_alice_dev1, Some(json!({ "dry_run": true }))).await;
    assert_eq!(st_mig, StatusCode::OK);
    assert_eq!(mig_val["dry_run"], true);
    assert_eq!(mig_val["status"], "completed");

    let _ = std::fs::remove_dir_all(&cfg.uploads_dir);
}

#[tokio::test]
async fn spec158_cold_image_compression_after_15_days() {
    let pool = setup_db().await;
    let cfg = test_config("s158");
    let (app, storage_reg) = build_app(pool.clone(), cfg.clone()).await;

    let (alice, _, c_alice) = create_test_user(&pool, &cfg, "cold_alice").await;
    let (bob, _, c_bob) = create_test_user(&pool, &cfg, "cold_bob").await;
    make_friends(&pool, alice, bob).await;

    let (gst, gval) = req_json(&app, "POST", "/api/groups", &c_alice, Some(json!({ "name": "Cold15dGroup" }))).await;
    assert_eq!(gst, StatusCode::OK);
    let gid = gval["id"].as_str().unwrap();
    req_json(&app, "POST", &format!("/api/groups/{gid}/members"), &c_alice, Some(json!({ "user_id": bob }))).await;

    let active_png = sample_meme_png(4101, 480, 360, 1, 0);
    let cold_png = sample_meme_png(8202, 520, 390, 1, 0);
    let (st_act, act_raw, _) = req_bytes(
        &app,
        "POST",
        &format!("/api/attachments?scope_type=group&scope_id={gid}&is_e2ee=false&preserve_original=false&file_name=active15d.png"),
        &c_alice,
        "image/png",
        active_png.clone(),
        &[],
    )
    .await;
    assert_eq!(st_act, StatusCode::OK);
    let act_json: Value = serde_json::from_slice(&act_raw).unwrap();
    let act_ref = act_json["ref_id"].as_str().unwrap();

    let (st_cold, cold_raw, _) = req_bytes(
        &app,
        "POST",
        &format!("/api/attachments?scope_type=group&scope_id={gid}&is_e2ee=false&preserve_original=false&file_name=cold15d.png"),
        &c_alice,
        "image/png",
        cold_png.clone(),
        &[],
    )
    .await;
    assert_eq!(st_cold, StatusCode::OK);
    let cold_json: Value = serde_json::from_slice(&cold_raw).unwrap();
    let cold_ref = cold_json["ref_id"].as_str().unwrap();

    // Backdate ONLY `cold_ref`'s object `last_accessed_at` by 16 days; keep `act_ref` at 5 days ago
    sqlx::query(
        "UPDATE media_objects SET last_accessed_at = now() - interval '5 days' WHERE id = (SELECT object_id FROM media_references WHERE id = $1::uuid)",
    )
    .bind(act_ref)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE media_objects SET last_accessed_at = now() - interval '16 days' WHERE id = (SELECT object_id FROM media_references WHERE id = $1::uuid)",
    )
    .bind(cold_ref)
    .execute(&pool)
    .await
    .unwrap();

    let cold_report = storage::migration::compress_inactive_images_15d(&pool, &storage_reg, 15)
        .await
        .expect("cold compression ok");
    assert_eq!(cold_report.compressed, 1);
    assert!(cold_report.freed_bytes > 0);

    // Active image (< 15d) MUST remain untouched byte-for-byte original PNG and NOT flagged cold
    let (st_get_act, got_act, hdrs_act) = req_bytes(&app, "GET", &format!("/api/attachments/{act_ref}"), &c_bob, "", vec![], &[]).await;
    assert_eq!(st_get_act, StatusCode::OK);
    assert_eq!(got_act, active_png);
    assert_eq!(
        hdrs_act.get("x-chatw-cold-compressed").and_then(|v| v.to_str().ok()),
        Some("0")
    );

    // Cold image (>= 15d) MUST be compressed to smaller AVIF (`ftypavif`), flagged `avif_cold_15d`,
    // and return `x-chatw-cold-compressed: 1` so clients auto-run AI sharpening on cache miss / new device
    let (st_get_cold, got_cold, hdrs_cold) = req_bytes(&app, "GET", &format!("/api/attachments/{cold_ref}"), &c_bob, "", vec![], &[]).await;
    assert_eq!(st_get_cold, StatusCode::OK);
    assert!(got_cold.len() < cold_png.len());
    assert_eq!(&got_cold[4..8], b"ftyp");
    assert_eq!(
        hdrs_cold.get("x-chatw-cold-compressed").and_then(|v| v.to_str().ok()),
        Some("1")
    );
    let codec_row: String = sqlx::query_scalar(
        "SELECT o.compression_codec FROM media_objects o JOIN media_references r ON r.object_id = o.id WHERE r.id = $1::uuid",
    )
    .bind(cold_ref)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(codec_row, "avif_cold_15d");

    let _ = std::fs::remove_dir_all(&cfg.uploads_dir);
}

#[tokio::test]
async fn spec160_password_auth_register_login_and_change_works() {
    let pool = setup_db().await;
    let cfg = test_config("pw_auth");
    let (app, _) = build_app(pool.clone(), cfg.clone()).await;

    let extract_cookie = |hdrs: &header::HeaderMap| -> String {
        hdrs.get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(';').next())
            .unwrap_or("")
            .to_string()
    };

    let uname = format!(
        "pwuser_{}",
        Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(10)
            .collect::<String>()
    );
    let pw1 = "StrongPass!2026";
    let pw2 = "NewStrongPass!2027";

    // Providers report password enabled
    let (st_prov, prov) = req_json(&app, "GET", "/api/auth/providers", "", None).await;
    assert_eq!(st_prov, StatusCode::OK);
    assert_eq!(prov["password"], true);

    // 1. Public self-registration is disabled (403); pre-provisioned user logs in cleanly
    let (st_reg_disabled, _) = req_json(
        &app,
        "POST",
        "/api/auth/register",
        "",
        Some(json!({ "username": uname, "password": pw1 })),
    )
    .await;
    assert_eq!(st_reg_disabled, StatusCode::FORBIDDEN);

    let pw_hash = chatw::backup::reauth::hash_argon2_phc(pw1).unwrap();
    let uid = Uuid::new_v4();
    let uname_norm = uname.to_lowercase();
    let sub = format!("pw:{uid}");
    sqlx::query(
        "INSERT INTO users (id, google_sub, username, username_norm, password_hash) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(uid)
    .bind(&sub)
    .bind(&uname)
    .bind(&uname_norm)
    .bind(&pw_hash)
    .execute(&pool)
    .await
    .unwrap();

    let (st_prelogin, _, reg_hdrs) = req_bytes(
        &app,
        "POST",
        "/api/auth/login",
        "",
        "application/json",
        serde_json::to_vec(&json!({ "username": uname, "password": pw1 })).unwrap(),
        &[],
    )
    .await;
    assert_eq!(st_prelogin, StatusCode::OK);
    let reg_cookie = extract_cookie(&reg_hdrs);
    assert!(!reg_cookie.is_empty());

    // /api/auth/me reports has_password = true
    let (st_me, me) = req_json(&app, "GET", "/api/auth/me", &reg_cookie, None).await;
    assert_eq!(st_me, StatusCode::OK);
    assert_eq!(me["has_password"], true);

    // 2. Wrong password MUST be rejected; right password logs in cleanly
    let (st_bad, _) = req_json(
        &app,
        "POST",
        "/api/auth/login",
        "",
        Some(json!({ "username": uname, "password": "wrong-password" })),
    )
    .await;
    assert_eq!(st_bad, StatusCode::BAD_REQUEST);

    let (st_ok, _, login_hdrs) = req_bytes(
        &app,
        "POST",
        "/api/auth/login",
        "",
        "application/json",
        serde_json::to_vec(&json!({ "username": uname, "password": pw1 })).unwrap(),
        &[],
    )
    .await;
    assert_eq!(st_ok, StatusCode::OK);
    let login_cookie = extract_cookie(&login_hdrs);
    assert!(!login_cookie.is_empty());

    // 3. Change password requires correct current_password when one is already set
    let (st_ch_bad, _) = req_json(
        &app,
        "POST",
        "/api/auth/password",
        &login_cookie,
        Some(json!({ "current_password": "wrong", "new_password": pw2 })),
    )
    .await;
    assert_eq!(st_ch_bad, StatusCode::BAD_REQUEST);

    let (st_ch_ok, _) = req_json(
        &app,
        "POST",
        "/api/auth/password",
        &login_cookie,
        Some(json!({ "current_password": pw1, "new_password": pw2 })),
    )
    .await;
    assert_eq!(st_ch_ok, StatusCode::OK);

    // New password logs in; old password no longer works
    let (st_old, _) = req_json(
        &app,
        "POST",
        "/api/auth/login",
        "",
        Some(json!({ "username": uname, "password": pw1 })),
    )
    .await;
    assert_eq!(st_old, StatusCode::BAD_REQUEST);

    let (st_new, _) = req_json(
        &app,
        "POST",
        "/api/auth/login",
        "",
        Some(json!({ "username": uname, "password": pw2 })),
    )
    .await;
    assert_eq!(st_new, StatusCode::OK);

    let _ = std::fs::remove_dir_all(&cfg.uploads_dir);
}

#[tokio::test]
async fn spec161_security_regressions() {
    let pool = setup_db().await;
    let cfg = test_config("spec161");
    let (app, _) = build_app(pool.clone(), cfg.clone()).await;

    let (alice, _, c_alice) = create_test_user(&pool, &cfg, "s161_alice").await;
    let (bob, _, c_bob) = create_test_user(&pool, &cfg, "s161_bob").await;

    // 1. OAuth ticket polling requires X-Ticket-Secret header (prevents ticket takeover / CSRF fixation)
    let (st_tk, tk_val) = req_json(&app, "POST", "/api/auth/oauth-ticket", "", None).await;
    assert_eq!(st_tk, StatusCode::OK);
    let ticket = tk_val["ticket"].as_str().unwrap();
    let secret = tk_val["ticket_secret"].as_str().unwrap();
    assert!(!secret.is_empty());

    let (st_no_sec, _) = req_json(&app, "GET", &format!("/api/auth/oauth-ticket/{ticket}"), "", None).await;
    assert_eq!(st_no_sec, StatusCode::FORBIDDEN);

    let (st_bad_sec, _, _) = req_bytes(
        &app,
        "GET",
        &format!("/api/auth/oauth-ticket/{ticket}"),
        "",
        "",
        vec![],
        &[("x-ticket-secret", "wrong-secret")],
    )
    .await;
    assert_eq!(st_bad_sec, StatusCode::FORBIDDEN);

    let (st_ok_sec, _, _) = req_bytes(
        &app,
        "GET",
        &format!("/api/auth/oauth-ticket/{ticket}"),
        "",
        "",
        vec![],
        &[("x-ticket-secret", secret)],
    )
    .await;
    assert_eq!(st_ok_sec, StatusCode::OK);

    // 2. Private sticker pack GET is restricted to pack author (closes IDOR)
    let (st_pk, pk_val) = req_json(
        &app,
        "POST",
        "/api/stickers/packs",
        &c_alice,
        Some(json!({ "name": "Alice Secret Pack" })),
    )
    .await;
    assert_eq!(st_pk, StatusCode::OK);
    let pack_id = pk_val["id"].as_str().unwrap();
    let (st_bob_pk, _) = req_json(&app, "GET", &format!("/api/stickers/packs/{pack_id}"), &c_bob, None).await;
    assert_eq!(st_bob_pk, StatusCode::FORBIDDEN);

    // 3. Delta sync must NEVER leak private server channel messages to non-admin members
    let (st_srv, srv_val) = req_json(&app, "POST", "/api/servers", &c_alice, Some(json!({ "name": "SecSrv" }))).await;
    assert_eq!(st_srv, StatusCode::OK);
    let srv_id = srv_val["id"].as_str().unwrap();
    sqlx::query("INSERT INTO server_members (server_id, user_id) VALUES ($1::uuid, $2) ON CONFLICT DO NOTHING")
        .bind(srv_id)
        .bind(bob)
        .execute(&pool)
        .await
        .unwrap();
    let (st_ch, ch_val) = req_json(
        &app,
        "POST",
        &format!("/api/servers/{srv_id}/channels"),
        &c_alice,
        Some(json!({ "name": "exec-only", "channel_type": "text", "is_private": true })),
    )
    .await;
    assert_eq!(st_ch, StatusCode::OK);
    let priv_ch_id = ch_val["id"].as_str().unwrap();
    let (st_msg, _) = req_json(
        &app,
        "POST",
        &format!("/api/channels/{priv_ch_id}/messages"),
        &c_alice,
        Some(json!({ "content": "TOP_SECRET_EXEC_MEMO" })),
    )
    .await;
    assert_eq!(st_msg, StatusCode::OK);

    let (st_delta, delta_val) = req_json(&app, "GET", "/api/sync/delta?channel_since_seq=0", &c_bob, None).await;
    assert_eq!(st_delta, StatusCode::OK);
    let leaked = delta_val["channel_messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["content"].as_str() == Some("TOP_SECRET_EXEC_MEMO"));
    assert!(!leaked, "Private channel message leaked to non-admin via /api/sync/delta!");

    // 4. Meet join must reject joining an arbitrary non-meet call UUID
    let random_call_id = Uuid::new_v4();
    let (st_meet, _) = req_json(
        &app,
        "POST",
        "/api/calls/meet/join",
        &c_bob,
        Some(json!({ "code": random_call_id.to_string() })),
    )
    .await;
    assert_eq!(st_meet, StatusCode::NOT_FOUND);
    let _ = alice;
}
