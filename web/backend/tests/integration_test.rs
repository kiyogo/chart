use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use p256::{
    PublicKey, SecretKey,
    ecdsa::{Signature, SigningKey, signature::Signer},
    elliptic_curve::{rand_core::OsRng, sec1::ToEncodedPoint},
};
use serde_json::{Value, json};
use sha2::Sha256;
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;
use uuid::Uuid;

use chatw::{
    attachments, audit,
    auth::{self, AppState, google::GoogleJwksCache, session::create_session},
    config::Config,
    crypto::{self, Envelope, PROTOCOL_VERSION, compute_fingerprint},
    db, enterprise, feed, friends, messages, servers, typing, users, vico, webrtc,
    websocket::{self, WsHub},
};

fn create_test_config() -> Config {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://chatw:chatw_dev_password@%2Ftmp%2Fkiyo-pg-run/chatw".into()
    });
    Config {
        database_url,
        db_max_connections: 5,
        bind_addr: "127.0.0.1:8080".parse().unwrap(),
        public_origin: "http://localhost:8080".into(),
        google_client_id: "test-client-id".into(),
        google_client_secret: "test-client-secret".into(),
        google_redirect_url: "http://localhost:8080/api/auth/google/callback".into(),
        session_secret: b"test-session-secret-at-least-32-bytes-long!".to_vec(),
        session_ttl: Duration::from_secs(3600),
        cookie_secure: false,
        static_dir: None,
        uploads_dir: std::env::temp_dir().join("chatw_test_uploads"),
        max_ws_connections: 100,
        trust_proxy: false,
        dev_login_enabled: false,
        google_configured: false,
    }
}

async fn build_test_app(pool: sqlx::PgPool, config: Config) -> axum::Router {
    let ws_hub = Arc::new(WsHub::new(config.max_ws_connections));
    let jwks_cache = Arc::new(tokio::sync::RwLock::new(GoogleJwksCache::new()));
    let http_client = reqwest::Client::new();

    let state = AppState {
        config: Arc::new(config.clone()),
        db: pool,
        http_client,
        jwks_cache,
        ws_hub,
        oauth_tickets: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        storage: Arc::new(chatw::storage::StorageRegistry::from_uploads_dir(&config.uploads_dir)),
        sfu: Arc::new(chatw::calls::sfu::SfuHandle::new()),
        rpc_router: Arc::new(std::sync::OnceLock::new()),
    };

    axum::Router::new()
        .nest("/api/auth", auth::routes())
        .nest("/api/users", users::routes())
        .nest("/api/keys", crypto::keys::routes().merge(chatw::devices_sync::key_backup_routes()))
        .nest("/api/devices", chatw::devices_sync::device_routes())
        .nest("/api/sync", chatw::devices_sync::sync_routes())
        .nest("/api/recovery", chatw::devices_sync::recovery_routes())
        .nest("/api/storage", chatw::storage::router::routes())
        .nest("/api/friends", friends::routes())
        .nest("/api/feed", feed::routes())
        .nest("/api/vico", vico::routes())
        .nest("/api/messages", messages::routes())
        .nest("/api/attachments", attachments::routes())
        .nest("/api/audit", audit::routes())
        .nest("/api/servers", servers::server_routes())
        .nest("/api/channels", servers::channel_routes())
        .nest("/api", enterprise::routes())
        .nest("/api/webrtc", webrtc::routes())
        .nest("/api/typing", typing::routes())
        .merge(websocket::routes())
        .route("/robots.txt", axum::routing::get(|| async { ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], "User-agent: *\nAllow: /\n\nSitemap: /sitemap.xml\n") }))
        .route("/sitemap.xml", axum::routing::get(|| async { ([(header::CONTENT_TYPE, "application/xml; charset=utf-8")], "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  <url>\n    <loc>https://github.com/kiyogo/chart</loc>\n  </url>\n</urlset>") }))
        .route("/llms.txt", axum::routing::get(|| async { ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], include_str!("../../frontend/public/llms.txt")) }))
        .with_state(state)
}

struct TestClientKeys {
    ecdh_secret: SecretKey,
    ecdh_pub_bytes: [u8; 65],
    fingerprint: [u8; 32],
    ecdsa_signing: SigningKey,
    ecdsa_pub_bytes: [u8; 65],
}

impl TestClientKeys {
    fn generate() -> Self {
        let ecdh_secret = SecretKey::random(&mut OsRng);
        let ecdh_pub = ecdh_secret.public_key().to_encoded_point(false);
        let mut ecdh_pub_bytes = [0u8; 65];
        ecdh_pub_bytes.copy_from_slice(ecdh_pub.as_bytes());

        let fingerprint = compute_fingerprint(&ecdh_pub_bytes);

        let ecdsa_signing = SigningKey::random(&mut OsRng);
        let ecdsa_verifying = ecdsa_signing.verifying_key();
        let ecdsa_pub = ecdsa_verifying.to_encoded_point(false);
        let mut ecdsa_pub_bytes = [0u8; 65];
        ecdsa_pub_bytes.copy_from_slice(ecdsa_pub.as_bytes());

        Self {
            ecdh_secret,
            ecdh_pub_bytes,
            fingerprint,
            ecdsa_signing,
            ecdsa_pub_bytes,
        }
    }
}

// Client helper: build envelope
#[allow(deprecated)]
fn client_build_envelope(
    sender_keys: &TestClientKeys,
    recipient_ecdh_pub_bytes: &[u8; 65],
    recipient_fingerprint: &[u8; 32],
    message_id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    plaintext: &str,
) -> Vec<u8> {
    use aes_gcm::{
        Aes256Gcm, KeyInit, Nonce,
        aead::{Aead, Payload},
    };
    use hkdf::Hkdf;

    let ephemeral_secret = SecretKey::random(&mut OsRng);
    let ephemeral_pub = ephemeral_secret.public_key().to_encoded_point(false);
    let recip_point = PublicKey::from_sec1_bytes(recipient_ecdh_pub_bytes).unwrap();
    let shared = p256::ecdh::diffie_hellman(
        ephemeral_secret.to_nonzero_scalar(),
        recip_point.as_affine(),
    );

    let salt = [11u8; 16];
    let nonce_bytes = [22u8; 12];
    let info = format!("chatw-v1-msg:{message_id}");

    let hk = Hkdf::<Sha256>::new(Some(&salt), shared.raw_secret_bytes());
    let mut aes_key = [0u8; 32];
    hk.expand(info.as_bytes(), &mut aes_key).unwrap();

    let cipher = Aes256Gcm::new_from_slice(&aes_key).unwrap();

    let mut aad = Vec::new();
    aad.push(PROTOCOL_VERSION);
    aad.extend_from_slice(message_id.as_bytes());
    aad.extend_from_slice(sender_id.as_bytes());
    aad.extend_from_slice(recipient_id.as_bytes());
    aad.extend_from_slice(recipient_fingerprint);

    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext.as_bytes(),
                aad: &aad,
            },
        )
        .unwrap();

    let mut envelope = Vec::new();
    envelope.extend_from_slice(&aad);
    envelope.extend_from_slice(ephemeral_pub.as_bytes());
    envelope.extend_from_slice(&salt);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ciphertext);

    let sig: Signature = sender_keys.ecdsa_signing.sign(&envelope);
    envelope.extend_from_slice(sig.to_bytes().as_slice());

    envelope
}

// Client helper: decrypt envelope
#[allow(deprecated)]
fn client_decrypt_envelope(
    recipient_keys: &TestClientKeys,
    sender_ecdsa_pub: &[u8; 65],
    envelope_raw: &[u8],
) -> String {
    use aes_gcm::{
        Aes256Gcm, KeyInit, Nonce,
        aead::{Aead, Payload},
    };
    use hkdf::Hkdf;

    let env = Envelope::parse(envelope_raw).expect("parse envelope");
    env.verify_signature(sender_ecdsa_pub)
        .expect("verify signature");

    let ephem_point = PublicKey::from_sec1_bytes(env.ephemeral_public_key).unwrap();
    let shared = p256::ecdh::diffie_hellman(
        recipient_keys.ecdh_secret.to_nonzero_scalar(),
        ephem_point.as_affine(),
    );

    let info = format!("chatw-v1-msg:{}", env.message_id);
    let hk = Hkdf::<Sha256>::new(Some(env.salt), shared.raw_secret_bytes());
    let mut aes_key = [0u8; 32];
    hk.expand(info.as_bytes(), &mut aes_key).unwrap();

    let cipher = Aes256Gcm::new_from_slice(&aes_key).unwrap();
    let plaintext_bytes = cipher
        .decrypt(
            Nonce::from_slice(env.nonce),
            Payload {
                msg: env.ciphertext,
                aad: env.aad(),
            },
        )
        .expect("decrypt aes-gcm");

    String::from_utf8(plaintext_bytes).unwrap()
}

#[tokio::test]
async fn test_full_e2ee_chat_flow() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = create_test_config();
    let pool = db::connect(&config.database_url, config.db_max_connections)
        .await
        .expect("connect to postgres");
    db::run_migrations(&pool).await.expect("migrations");

    let app = build_test_app(pool.clone(), config.clone()).await;

    // 1. Create two test users in database
    let user_a_id = Uuid::new_v4();
    let user_b_id = Uuid::new_v4();
    let user_c_id = Uuid::new_v4(); // Third party

    sqlx::query!(
        "INSERT INTO users (id, google_sub) VALUES ($1, $2)",
        user_a_id,
        format!("sub_a_{user_a_id}")
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO users (id, google_sub) VALUES ($1, $2)",
        user_b_id,
        format!("sub_b_{user_b_id}")
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO users (id, google_sub) VALUES ($1, $2)",
        user_c_id,
        format!("sub_c_{user_c_id}")
    )
    .execute(&pool)
    .await
    .unwrap();

    // 2. Create sessions for User A, User B, and User C
    let cookie_a = create_session(&pool, &config, user_a_id, Some("Mozilla/5.0 (X11; Linux x86_64)")).await.unwrap();
    let cookie_b = create_session(&pool, &config, user_b_id, None).await.unwrap();
    let cookie_c = create_session(&pool, &config, user_c_id, None).await.unwrap();

    let session_header_a = cookie_a.split(';').next().unwrap();
    let session_header_b = cookie_b.split(';').next().unwrap();
    let session_header_c = cookie_c.split(';').next().unwrap();

    // 3. Set names for users
    let name_a = format!("Alice{}", &user_a_id.to_string()[..6]);
    let name_b = format!("Bob{}", &user_b_id.to_string()[..6]);

    let req = Request::post("/api/users/name")
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "name": name_a }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::post("/api/users/name")
        .header(header::COOKIE, session_header_b)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "name": name_b }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Client key generation & registration
    let keys_a = TestClientKeys::generate();
    let keys_b = TestClientKeys::generate();

    let req = Request::post("/api/keys")
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "ecdh_public": URL_SAFE_NO_PAD.encode(keys_a.ecdh_pub_bytes),
                "ecdsa_public": URL_SAFE_NO_PAD.encode(keys_a.ecdsa_pub_bytes)
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::post("/api/keys")
        .header(header::COOKIE, session_header_b)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "ecdh_public": URL_SAFE_NO_PAD.encode(keys_b.ecdh_pub_bytes),
                "ecdsa_public": URL_SAFE_NO_PAD.encode(keys_b.ecdsa_pub_bytes)
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 5. Send Friend Request: Alice -> Bob
    let req = Request::post("/api/friends/requests")
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "receiver_id": user_b_id }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let req_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let request_id: Uuid = req_json["id"].as_str().unwrap().parse().unwrap();

    // 6. Bob accepts Friend Request
    let req = Request::post(format!("/api/friends/requests/{request_id}/accept"))
        .header(header::COOKIE, session_header_b)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 7. Alice encrypts "Xin chào ❤️" for Bob + self-envelope for Alice
    let msg_id = Uuid::new_v4();
    let message_text = "Xin chào ❤️";

    let recip_env = client_build_envelope(
        &keys_a,
        &keys_b.ecdh_pub_bytes,
        &keys_b.fingerprint,
        msg_id,
        user_a_id,
        user_b_id,
        message_text,
    );

    let sender_env = client_build_envelope(
        &keys_a,
        &keys_a.ecdh_pub_bytes,
        &keys_a.fingerprint,
        msg_id,
        user_a_id,
        user_a_id,
        message_text,
    );

    // 8. Alice sends message via HTTP POST
    let req = Request::post("/api/messages")
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "message_id": msg_id,
                "recipient_id": user_b_id,
                "recipient_envelope": URL_SAFE_NO_PAD.encode(&recip_env),
                "sender_envelope": URL_SAFE_NO_PAD.encode(&sender_env)
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 9. SECURITY VERIFICATION 1: Database inspection (Plaintext MUST NOT exist in PostgreSQL)
    let db_msg = sqlx::query!(
        "SELECT recipient_envelope, sender_envelope FROM messages WHERE id = $1",
        msg_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        !db_msg
            .recipient_envelope
            .windows(message_text.len())
            .any(|w| w == message_text.as_bytes())
    );
    assert!(
        !db_msg
            .sender_envelope
            .windows(message_text.len())
            .any(|w| w == message_text.as_bytes())
    );

    // 10. Bob fetches messages and decrypts
    let req = Request::get(format!("/api/messages/{user_a_id}"))
        .header(header::COOKIE, session_header_b)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let msg_list: Vec<Value> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(msg_list.len(), 1);

    let bob_received_env = URL_SAFE_NO_PAD
        .decode(msg_list[0]["envelope"].as_str().unwrap())
        .unwrap();
    let bob_decrypted_text =
        client_decrypt_envelope(&keys_b, &keys_a.ecdsa_pub_bytes, &bob_received_env);
    assert_eq!(bob_decrypted_text, message_text);

    // 11. Alice fetches messages and decrypts her own message
    let req = Request::get(format!("/api/messages/{user_b_id}"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let alice_list: Vec<Value> = serde_json::from_slice(&body_bytes).unwrap();
    let alice_received_env = URL_SAFE_NO_PAD
        .decode(alice_list[0]["envelope"].as_str().unwrap())
        .unwrap();
    let alice_decrypted_text =
        client_decrypt_envelope(&keys_a, &keys_a.ecdsa_pub_bytes, &alice_received_env);
    assert_eq!(alice_decrypted_text, message_text);

    // 12. SECURITY VERIFICATION 2: Unauthorized third party (User C) CANNOT access history
    let req = Request::get(format!("/api/messages/{user_a_id}"))
        .header(header::COOKIE, session_header_c)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 13. SECURITY VERIFICATION 3: Tampering with ciphertext is rejected by backend signature check
    let mut tampered_env = recip_env.clone();
    tampered_env[180] ^= 0x01; // flip byte
    let req = Request::post("/api/messages")
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "message_id": Uuid::new_v4(),
                "recipient_id": user_b_id,
                "recipient_envelope": URL_SAFE_NO_PAD.encode(&tampered_env),
                "sender_envelope": URL_SAFE_NO_PAD.encode(&sender_env)
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 14. SECURITY VERIFICATION 4: Cannot send message to non-friend
    let req = Request::post("/api/messages")
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "message_id": Uuid::new_v4(),
                "recipient_id": user_c_id,
                "recipient_envelope": URL_SAFE_NO_PAD.encode(&recip_env),
                "sender_envelope": URL_SAFE_NO_PAD.encode(&sender_env)
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 15. ADVANCED MESSAGING: Emoji Reaction System
    let req = Request::post(format!("/api/messages/{msg_id}/reactions"))
        .header(header::COOKIE, session_header_b)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "emoji": "❤️" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let reactions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0]["emoji"], "❤️");
    assert_eq!(reactions[0]["count"], 1);

    // 16. ADVANCED MESSAGING: Pinning System
    let req = Request::post(format!("/api/messages/{msg_id}/pin"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::get(format!("/api/messages/pinned/{user_b_id}"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let pinned: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0]["id"], msg_id.to_string());
    assert_eq!(pinned[0]["is_pinned"], true);

    // 17. SESSION MANAGEMENT: List active sessions
    let req = Request::get("/api/auth/sessions")
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let sessions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!sessions.is_empty());
    assert_eq!(sessions[0]["is_current"], true);

    // 18. DISCORD-STYLE SERVERS: Create server & list
    let req = Request::post("/api/servers")
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({
            "name": "Rust Community",
            "description": "High performance Rust discussions"
        }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let server: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let server_id = server["id"].as_str().unwrap().to_string();
    assert_eq!(server["name"], "Rust Community");

    // Verify default General category and general channel were auto-created
    let req = Request::get(format!("/api/servers/{server_id}/channels"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let channels: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    // Spec 003: a new server gets a default "general" text channel and a default voice channel.
    assert_eq!(channels.len(), 2);
    let general = channels
        .iter()
        .find(|c| c["name"] == "general" && c["channel_type"] == "text")
        .expect("default general text channel");
    assert!(channels.iter().any(|c| c["channel_type"] == "voice"), "default voice channel");
    let general_ch_id = general["id"].as_str().unwrap().to_string();

    // 19. DISCORD-STYLE SERVERS: Create custom category and voice channel
    let req = Request::post(format!("/api/servers/{server_id}/categories"))
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "name": "Voice Rooms" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let voice_cat: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let voice_cat_id = voice_cat["id"].as_str().unwrap().to_string();

    let req = Request::post(format!("/api/servers/{server_id}/channels"))
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({
            "name": "Lounge",
            "category_id": voice_cat_id,
            "channel_type": "voice"
        }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let voice_ch: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let voice_ch_id = voice_ch["id"].as_str().unwrap().to_string();

    // 20. DISCORD-STYLE SERVERS: Create invite and have User B join
    let req = Request::post(format!("/api/servers/{server_id}/invites"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let invite_obj: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let invite_code = invite_obj["code"].as_str().unwrap();

    let req = Request::post(format!("/api/servers/join/{invite_code}"))
        .header(header::COOKIE, session_header_b)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verify User B is in server members list
    let req = Request::get(format!("/api/servers/{server_id}/members"))
        .header(header::COOKIE, session_header_b)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let members: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(members.len(), 2);

    // 21. DISCORD-STYLE CHANNELS: Post channel message and fetch history
    let req = Request::post(format!("/api/channels/{general_ch_id}/messages"))
        .header(header::COOKIE, session_header_b)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "content": "Hello Rust server!" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let post_msg: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(post_msg["content"], "Hello Rust server!");

    let req = Request::get(format!("/api/channels/{general_ch_id}/messages"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let ch_msgs: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(ch_msgs.len(), 1);
    assert_eq!(ch_msgs[0]["content"], "Hello Rust server!");

    // 22. RBAC PERMISSIONS: Non-member cannot access channel messages
    let req = Request::get(format!("/api/channels/{general_ch_id}/messages"))
        .header(header::COOKIE, session_header_c)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 23. PROFILE CUSTOMIZATION: Update bio, status & presence
    let req = Request::put("/api/users/profile")
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({
            "bio": "Principal Rust Engineer & Cryptography Architect",
            "status_text": "Building E2EE chat",
            "status_emoji": "⚡",
            "presence": "dnd"
        }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let prof: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(prof["bio"], "Principal Rust Engineer & Cryptography Architect");
    assert_eq!(prof["status_text"], "Building E2EE chat");
    assert_eq!(prof["presence"], "dnd");

    // Fetch User A's profile via GET
    let req = Request::get(format!("/api/users/{user_a_id}/profile"))
        .header(header::COOKIE, session_header_b)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let prof_get: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(prof_get["bio"], "Principal Rust Engineer & Cryptography Architect");

    // 24. WEBRTC SIGNALING: Relay P2P WebRTC signal
    let req = Request::post("/api/webrtc/signal")
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({
            "target_user_id": user_b_id,
            "signal_type": "offer",
            "data": { "type": "offer", "sdp": "v=0\r\no=- 123456 2 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\n" }
        }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 25. WEBRTC VOICE CHANNELS: Join voice, state update, and leave
    let req = Request::post(format!("/api/webrtc/voice/{voice_ch_id}/join"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let participants: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0]["user_id"], user_a_id.to_string());
    assert_eq!(participants[0]["self_mute"], false);

    // User A mutes self
    let req = Request::post(format!("/api/webrtc/voice/{voice_ch_id}/state"))
        .header(header::COOKIE, session_header_a)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "self_mute": true }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let state_res: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(state_res["self_mute"], true);

    // User A leaves voice channel
    let req = Request::post(format!("/api/webrtc/voice/{voice_ch_id}/leave"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Check participants now empty
    let req = Request::get(format!("/api/webrtc/voice/{voice_ch_id}/participants"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let participants_after: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(participants_after.len(), 0);

    // 26. E2EE ATTACHMENTS: Upload encrypted media/audio note, download, and delete
    let fake_encrypted_blob = b"encrypted_audio_voice_note_aes_gcm_payload_bytes_999";
    let req = Request::post("/api/attachments")
        .header(header::COOKIE, session_header_a)
        .header("x-file-name", "voice_note.ogg.enc")
        .header(header::CONTENT_TYPE, "audio/ogg")
        .body(Body::from(fake_encrypted_blob.to_vec()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let attach_res: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let attach_id = attach_res["id"].as_str().unwrap().to_string();
    assert_eq!(attach_res["file_name"], "voice_note.ogg.enc");
    assert_eq!(attach_res["file_size"], fake_encrypted_blob.len() as i64);
    assert_eq!(attach_res["mime_type"], "audio/ogg");

    // User B downloads the encrypted attachment
    let req = Request::get(format!("/api/attachments/{attach_id}"))
        .header(header::COOKIE, session_header_b)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let downloaded_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    assert_eq!(downloaded_bytes.as_ref(), fake_encrypted_blob);

    // User A deletes the attachment
    let req = Request::delete(format!("/api/attachments/{attach_id}"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // Verifying it is gone (404)
    let req = Request::get(format!("/api/attachments/{attach_id}"))
        .header(header::COOKIE, session_header_b)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 27. SECURITY HARDENING & AUDIT LOGGING: Session revocation creates audit log
    let _cookie_a2 = create_session(&pool, &config, user_a_id, None).await.unwrap();

    let req = Request::get("/api/auth/sessions")
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let user_a_sessions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    let session_to_delete = user_a_sessions
        .iter()
        .find(|s| s["is_current"] == false)
        .expect("find non-current session")["id"]
        .as_str()
        .unwrap();

    let req = Request::delete(format!("/api/auth/sessions/{session_to_delete}"))
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::get("/api/audit/logs")
        .header(header::COOKIE, session_header_a)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let audit_logs: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!audit_logs.is_empty());
    assert_eq!(audit_logs[0]["action"], "session_revoked");
    assert_eq!(audit_logs[0]["target_id"], session_to_delete);

    // 28. SEO & MARKETING COMPLIANCE: robots.txt, sitemap.xml, llms.txt endpoints
    let req = Request::get("/robots.txt").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("User-agent: *"));

    let req = Request::get("/sitemap.xml").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("<urlset"));

    let req = Request::get("/llms.txt").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("chatw"));
}

#[tokio::test]
async fn test_enterprise_server_lifecycle() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = create_test_config();
    let pool = db::connect(&config.database_url, config.db_max_connections)
        .await
        .expect("connect to postgres");
    db::run_migrations(&pool).await.expect("migrations");

    let app = build_test_app(pool.clone(), config.clone()).await;

    let owner_id = Uuid::new_v4();
    let member_id = Uuid::new_v4();
    for (uid, prefix) in [(owner_id, "ent_own"), (member_id, "ent_mem")] {
        sqlx::query("INSERT INTO users (id, google_sub) VALUES ($1, $2)")
            .bind(uid)
            .bind(format!("{prefix}_{uid}"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let owner_cookie = create_session(&pool, &config, owner_id, None).await.unwrap();
    let member_cookie = create_session(&pool, &config, member_id, None).await.unwrap();
    let owner_hdr = owner_cookie.split(';').next().unwrap().to_string();
    let member_hdr = member_cookie.split(';').next().unwrap().to_string();

    for (hdr, uname) in [
        (&owner_hdr, format!("Owner{}", &owner_id.to_string()[..6])),
        (&member_hdr, format!("Mem{}", &member_id.to_string()[..6])),
    ] {
        let req = Request::post("/api/users/name")
            .header(header::COOKIE, hdr.as_str())
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "name": uname }).to_string()))
            .unwrap();
        assert_eq!(app.clone().oneshot(req).await.unwrap().status(), StatusCode::OK);
    }

    // 1. Default server creation keeps Enterprise OFF and behaves identically to standard server
    let res = app
        .clone()
        .oneshot(
            Request::post("/api/servers")
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "name": "Community Std" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let std_srv: Value = serde_json::from_slice(&axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(std_srv["is_enterprise"], false);
    let std_id = std_srv["id"].as_str().unwrap().to_string();

    // Calling enterprise config on regular server must fail cleanly
    let res = app
        .clone()
        .oneshot(
            Request::get(format!("/api/servers/{std_id}/enterprise/config"))
                .header(header::COOKIE, owner_hdr.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Create a channel message & join member before converting existing server to Enterprise
    let chs_res = app
        .clone()
        .oneshot(
            Request::get(format!("/api/servers/{std_id}/channels"))
                .header(header::COOKIE, owner_hdr.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let chs: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(chs_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    let general_ch_id = chs[0]["id"].as_str().unwrap().to_string();

    let msg_res = app
        .clone()
        .oneshot(
            Request::post(format!("/api/channels/{general_ch_id}/messages"))
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "content": "Legacy channel message stays intact" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(msg_res.status(), StatusCode::OK);
    let legacy_msg: Value = serde_json::from_slice(&axum::body::to_bytes(msg_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    let legacy_msg_id = legacy_msg["id"].as_str().unwrap().to_string();

    // Add member via DB invite simulation (existing server member prior to enterprise conversion)
    sqlx::query("INSERT INTO server_members (server_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(Uuid::parse_str(&std_id).unwrap())
        .bind(member_id)
        .execute(&pool)
        .await
        .unwrap();

    // 2. Owner enables Enterprise Mode on existing server -> preserves ID/channels/messages & does not force member into employees
    let conv_res = app
        .clone()
        .oneshot(
            Request::put(format!("/api/servers/{std_id}/settings"))
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "is_enterprise": true }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(conv_res.status(), StatusCode::OK);
    let conv_settings: Value = serde_json::from_slice(&axum::body::to_bytes(conv_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(conv_settings["is_enterprise"], true);

    // Check legacy message is still there
    let ch_msgs_res = app
        .clone()
        .oneshot(
            Request::get(format!("/api/channels/{general_ch_id}/messages"))
                .header(header::COOKIE, owner_hdr.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let ch_msgs: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(ch_msgs_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(ch_msgs.iter().any(|m| m["id"] == legacy_msg_id));

    // Only owner was seeded into enterprise_employees, existing member was NOT auto-converted to employee
    let emp_res = app
        .clone()
        .oneshot(
            Request::get(format!("/api/servers/{std_id}/enterprise/employees"))
                .header(header::COOKIE, owner_hdr.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(emp_res.status(), StatusCode::OK);
    let emps: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(emp_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(emps.len(), 1);
    assert_eq!(emps[0]["user_id"], owner_id.to_string());

    // 3. Direct Enterprise creation initializes roles, templates, config cleanly + rapid duplicate protection
    let direct_res = app
        .clone()
        .oneshot(
            Request::post("/api/servers")
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "name": "Acme Corp", "is_enterprise": true }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(direct_res.status(), StatusCode::OK);
    let ent_srv: Value = serde_json::from_slice(&axum::body::to_bytes(direct_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(ent_srv["is_enterprise"], true);
    let acme_id = ent_srv["id"].as_str().unwrap().to_string();

    // Rapid duplicate click within 5s returns the same server_id
    let dup_res = app
        .clone()
        .oneshot(
            Request::post("/api/servers")
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "name": "Acme Corp", "is_enterprise": true }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dup_res.status(), StatusCode::OK);
    let dup_srv: Value = serde_json::from_slice(&axum::body::to_bytes(dup_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(dup_srv["id"].as_str().unwrap(), acme_id);

    // 4. HR sensitive field masking + offboarding revocation + RBAC enforcement
    // Assign member "Nhân viên" role on std_id so member has VIEW_HR | CREATE_TASKS | VIEW_ATTENDANCE but NOT VIEW_SENSITIVE_HR
    let roles_res = app
        .clone()
        .oneshot(
            Request::get(format!("/api/servers/{std_id}/roles"))
                .header(header::COOKIE, owner_hdr.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let roles_list: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(roles_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    let staff_role_id = roles_list.iter().find(|r| r["name"] == "Nhân viên" || r["name"] == "Staff").unwrap()["id"].as_str().unwrap().to_string();

    assert_eq!(
        app.clone()
            .oneshot(
                Request::put(format!("/api/servers/{std_id}/members/{member_id}/roles/{staff_role_id}"))
                    .header(header::COOKIE, owner_hdr.as_str())
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    // Owner updates own sensitive fields (work_email, work_phone, internal_notes)
    assert_eq!(
        app.clone()
            .oneshot(
                Request::post(format!("/api/servers/{std_id}/enterprise/employees"))
                    .header(header::COOKIE, owner_hdr.as_str())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({
                        "user_id": owner_id,
                        "full_name": "Boss Sensitive",
                        "work_email": "boss@acme.local",
                        "work_phone": "0900111222",
                        "internal_notes": "Confidential review"
                    }).to_string()))
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    // Member lists employees -> CANNOT see owner's sensitive fields!
    let mem_emp_res = app
        .clone()
        .oneshot(
            Request::get(format!("/api/servers/{std_id}/enterprise/employees"))
                .header(header::COOKIE, member_hdr.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mem_emp_res.status(), StatusCode::OK);
    let mem_emps: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(mem_emp_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(mem_emps[0]["work_email"].is_null());
    assert!(mem_emps[0]["work_phone"].is_null());
    assert!(mem_emps[0]["internal_notes"].is_null());

    // Member cannot create department (lacks MANAGE_DEPARTMENTS) -> 403 Forbidden
    assert_eq!(
        app.clone()
            .oneshot(
                Request::post(format!("/api/servers/{std_id}/enterprise/departments"))
                    .header(header::COOKIE, member_hdr.as_str())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "name": "Unauthorized Dept" }).to_string()))
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );

    // 5. Private channel message task-creation does NOT leak private content preview
    let priv_ch_id = Uuid::new_v4();
    let priv_msg_id = Uuid::new_v4();
    sqlx::query("INSERT INTO channels (id, server_id, name, channel_type, is_private) VALUES ($1, $2, 'secret-exec', 'text', TRUE)")
        .bind(priv_ch_id)
        .bind(Uuid::parse_str(&std_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO channel_messages (id, channel_id, sender_id, content) VALUES ($1, $2, $3, 'TOP SECRET BUDGET')")
        .bind(priv_msg_id)
        .bind(priv_ch_id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        app.clone()
            .oneshot(
                Request::post(format!("/api/servers/{std_id}/enterprise/tasks"))
                    .header(header::COOKIE, owner_hdr.as_str())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({
                        "title": "Follow up secret",
                        "source_channel_id": priv_ch_id,
                        "source_message_id": priv_msg_id
                    }).to_string()))
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let tasks_res = app
        .clone()
        .oneshot(
            Request::get(format!("/api/servers/{std_id}/enterprise/tasks"))
                .header(header::COOKIE, owner_hdr.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let tasks: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(tasks_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    let secret_task = tasks.iter().find(|t| t["title"] == "Follow up secret").unwrap();
    assert!(secret_task["source_message_preview"].is_null());

    // 6. Server-clock Attendance Check-in / Check-out & Leave Request + Timed Doc Share
    assert_eq!(
        app.clone()
            .oneshot(
                Request::post(format!("/api/servers/{std_id}/enterprise/attendance/check-in"))
                    .header(header::COOKIE, member_hdr.as_str())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "work_mode": "remote", "note": "WFH" }).to_string()))
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    assert_eq!(
        app.clone()
            .oneshot(
                Request::post(format!("/api/servers/{std_id}/enterprise/attendance/check-out"))
                    .header(header::COOKIE, member_hdr.as_str())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "note": "Done" }).to_string()))
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let leave_create = app
        .clone()
        .oneshot(
            Request::post(format!("/api/servers/{std_id}/enterprise/leave"))
                .header(header::COOKIE, member_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({
                    "leave_type": "annual",
                    "start_date": "2026-12-01",
                    "end_date": "2026-12-02",
                    "half_day": "full",
                    "reason": "Family trip"
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(leave_create.status(), StatusCode::OK);
    let lv: Value = serde_json::from_slice(&axum::body::to_bytes(leave_create.into_body(), usize::MAX).await.unwrap()).unwrap();
    let leave_id = lv["id"].as_str().unwrap();

    assert_eq!(
        app.clone()
            .oneshot(
                Request::post(format!("/api/servers/{std_id}/enterprise/leave/{leave_id}/decide"))
                    .header(header::COOKIE, owner_hdr.as_str())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "status": "approved", "note": "Approved" }).to_string()))
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    // 7. Timed Document Share & Resolution
    let doc_create = app
        .clone()
        .oneshot(
            Request::post(format!("/api/servers/{std_id}/enterprise/documents"))
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({
                    "title": "Handbook",
                    "scope": "company",
                    "file_url": "/uploads/media/handbook.pdf",
                    "file_name": "handbook.pdf",
                    "file_size": 2048
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(doc_create.status(), StatusCode::OK);
    let dc: Value = serde_json::from_slice(&axum::body::to_bytes(doc_create.into_body(), usize::MAX).await.unwrap()).unwrap();
    let doc_id = dc["id"].as_str().unwrap();

    let share_res = app
        .clone()
        .oneshot(
            Request::patch(format!("/api/servers/{std_id}/enterprise/documents/{doc_id}"))
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "action": "share", "expires_hours": 12 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(share_res.status(), StatusCode::OK);
    let share_tok = serde_json::from_slice::<Value>(&axum::body::to_bytes(share_res.into_body(), usize::MAX).await.unwrap()).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    let resolve_res = app
        .clone()
        .oneshot(Request::get(format!("/api/enterprise/shared/{share_tok}")).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resolve_res.status(), StatusCode::OK);

    // 7b. Document version bump + Batch employee import + Multi-step approval template & revision
    let bump_res = app
        .clone()
        .oneshot(
            Request::patch(format!("/api/servers/{std_id}/enterprise/documents/{doc_id}"))
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "action": "bump_version" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bump_res.status(), StatusCode::OK);

    let member_uname = format!("Mem{}", &member_id.to_string()[..6]);
    let import_res = app
        .clone()
        .oneshot(
            Request::post(format!("/api/servers/{std_id}/enterprise/employees/import"))
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({
                    "rows": [
                        { "username": member_uname, "full_name": "Imported Member", "job_title": "Engineer", "department_code": "ENG" }
                    ]
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_res.status(), StatusCode::OK);
    let imp_val: Value = serde_json::from_slice(&axum::body::to_bytes(import_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(imp_val["imported"], 1);

    let tpl_res = app
        .clone()
        .oneshot(
            Request::post(format!("/api/servers/{std_id}/enterprise/approvals/templates"))
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({
                    "name": "Cap quyen san xuat",
                    "category": "access_request",
                    "fields": [{ "key": "summary", "label": "Chi tiet", "required": true }],
                    "workflow_steps": [{ "label": "Buoc 1" }, { "label": "Buoc 2" }]
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tpl_res.status(), StatusCode::OK);
    let tpl_id = serde_json::from_slice::<Value>(&axum::body::to_bytes(tpl_res.into_body(), usize::MAX).await.unwrap()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let sub_res = app
        .clone()
        .oneshot(
            Request::post(format!("/api/servers/{std_id}/enterprise/approvals"))
                .header(header::COOKIE, member_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({
                    "template_id": tpl_id,
                    "title": "Xin quyen DB",
                    "category": "access_request",
                    "form_data": { "summary": "Read replica" }
                }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sub_res.status(), StatusCode::OK);
    let app_req_id = serde_json::from_slice::<Value>(&axum::body::to_bytes(sub_res.into_body(), usize::MAX).await.unwrap()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let rev_res = app
        .clone()
        .oneshot(
            Request::post(format!("/api/servers/{std_id}/enterprise/approvals/{app_req_id}/decide"))
                .header(header::COOKIE, owner_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "decision": "revision", "comment": "Bo sung ly do" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rev_res.status(), StatusCode::OK);
    let rev_val: Value = serde_json::from_slice(&axum::body::to_bytes(rev_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(rev_val["status"], "needs_revision");

    // 8. Retention lock prevents disabling enterprise mode; disabling preserves data & export
    assert_eq!(
        app.clone()
            .oneshot(
                Request::patch(format!("/api/servers/{std_id}/enterprise/config"))
                    .header(header::COOKIE, owner_hdr.as_str())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "retention_locked": true }).to_string()))
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone()
            .oneshot(
                Request::put(format!("/api/servers/{std_id}/settings"))
                    .header(header::COOKIE, owner_hdr.as_str())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "is_enterprise": false }).to_string()))
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );

    // Unlock retention -> disable succeeds while preserving export-all JSON
    assert_eq!(
        app.clone()
            .oneshot(
                Request::patch(format!("/api/servers/{std_id}/enterprise/config"))
                    .header(header::COOKIE, owner_hdr.as_str())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "retention_locked": false }).to_string()))
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone()
            .oneshot(
                Request::put(format!("/api/servers/{std_id}/settings"))
                    .header(header::COOKIE, owner_hdr.as_str())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "is_enterprise": false }).to_string()))
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let export_res = app
        .clone()
        .oneshot(
            Request::get(format!("/api/servers/{std_id}/enterprise/export-all"))
                .header(header::COOKIE, owner_hdr.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export_res.status(), StatusCode::OK);
    let exported: Value = serde_json::from_slice(&axum::body::to_bytes(export_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(!exported["tasks"].as_array().unwrap().is_empty());
    assert!(!exported["documents"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_social_feed_lifecycle() {
    let config = create_test_config();
    let pool = db::connect(&config.database_url, 5).await.unwrap();
    db::run_migrations(&pool).await.unwrap();

    let suffix = Uuid::new_v4().to_string()[..8].to_string();
    let a_id = Uuid::new_v4();
    let b_id = Uuid::new_v4();
    let a_uname = format!("feed_a_{suffix}");
    let b_uname = format!("feed_b_{suffix}");

    for (id, uname) in [(a_id, &a_uname), (b_id, &b_uname)] {
        sqlx::query("INSERT INTO users (id, google_sub, username, username_norm) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(format!("sub_{id}"))
            .bind(uname)
            .bind(uname.to_lowercase())
            .execute(&pool)
            .await
            .unwrap();
    }

    let u_a = a_id.min(b_id);
    let u_b = a_id.max(b_id);
    sqlx::query("INSERT INTO friendships (user_a, user_b) VALUES ($1, $2)")
        .bind(u_a)
        .bind(u_b)
        .execute(&pool)
        .await
        .unwrap();

    let a_cookie = create_session(&pool, &config, a_id, None).await.unwrap();
    let b_cookie = create_session(&pool, &config, b_id, None).await.unwrap();
    let a_hdr = a_cookie.split(';').next().unwrap().to_string();
    let b_hdr = b_cookie.split(';').next().unwrap().to_string();
    let app = build_test_app(pool.clone(), config).await;

    // 1. Author creates a friends post + private post
    let c_res = app
        .clone()
        .oneshot(
            Request::post("/api/feed")
                .header(header::COOKIE, a_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "content": "Hello Threads feed!", "visibility": "friends" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(c_res.status(), StatusCode::OK);
    let post_id = serde_json::from_slice::<Value>(&axum::body::to_bytes(c_res.into_body(), usize::MAX).await.unwrap()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let priv_res = app
        .clone()
        .oneshot(
            Request::post("/api/feed")
                .header(header::COOKIE, a_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "content": "Secret diary note", "visibility": "self" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(priv_res.status(), StatusCode::OK);
    let priv_id = serde_json::from_slice::<Value>(&axum::body::to_bytes(priv_res.into_body(), usize::MAX).await.unwrap()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Friend sees the friends post, does NOT see self-only post
    let feed_b = app
        .clone()
        .oneshot(Request::get("/api/feed?filter=friends").header(header::COOKIE, b_hdr.as_str()).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(feed_b.status(), StatusCode::OK);
    let list_b: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(feed_b.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(list_b.iter().any(|p| p["id"] == post_id));
    assert!(!list_b.iter().any(|p| p["id"] == priv_id));

    // 3. Friend replies + likes
    let reply_res = app
        .clone()
        .oneshot(
            Request::post("/api/feed")
                .header(header::COOKIE, b_hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "content": "Nice thread!", "parent_id": post_id }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reply_res.status(), StatusCode::OK);

    let like_res = app
        .clone()
        .oneshot(Request::post(format!("/api/feed/{post_id}/like")).header(header::COOKIE, b_hdr.as_str()).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(like_res.status(), StatusCode::OK);
    let like_val: Value = serde_json::from_slice(&axum::body::to_bytes(like_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(like_val["liked"], true);
    assert_eq!(like_val["like_count"], 1);

    // 4. Replies list contains the reply; non-author cannot delete author's post
    let rep_get = app
        .clone()
        .oneshot(Request::get(format!("/api/feed/{post_id}/replies")).header(header::COOKIE, a_hdr.as_str()).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(rep_get.status(), StatusCode::OK);
    let reps: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(rep_get.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(reps.len(), 1);
    assert_eq!(reps[0]["content"], "Nice thread!");

    assert_eq!(
        app.clone()
            .oneshot(Request::delete(format!("/api/feed/{post_id}")).header(header::COOKIE, b_hdr.as_str()).body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );

    assert_eq!(
        app.clone()
            .oneshot(Request::delete(format!("/api/feed/{post_id}")).header(header::COOKIE, a_hdr.as_str()).body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn test_vico_ai_chat_lifecycle() {
    let config = create_test_config();
    let pool = db::connect(&config.database_url, 5).await.unwrap();
    db::run_migrations(&pool).await.unwrap();

    let uid = Uuid::new_v4();
    let uname = format!("vico_{}", &uid.to_string()[..6]);
    sqlx::query("INSERT INTO users (id, google_sub, username, username_norm) VALUES ($1, $2, $3, $4)")
        .bind(uid)
        .bind(format!("sub_{uid}"))
        .bind(&uname)
        .bind(uname.to_lowercase())
        .execute(&pool)
        .await
        .unwrap();

    let cookie = create_session(&pool, &config, uid, None).await.unwrap();
    let hdr = cookie.split(';').next().unwrap().to_string();
    let app = build_test_app(pool.clone(), config).await;

    let conv_res = app
        .clone()
        .oneshot(Request::get("/api/vico/conversations").header(header::COOKIE, hdr.as_str()).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(conv_res.status(), StatusCode::OK);
    let convs: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(conv_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(convs.len(), 1);
    let cid = convs[0]["id"].as_str().unwrap().to_string();

    let msg_res = app
        .clone()
        .oneshot(
            Request::post(format!("/api/vico/conversations/{cid}/messages"))
                .header(header::COOKIE, hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "content": "25 * 4 + 10" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(msg_res.status(), StatusCode::OK);
    let msg_val: Value = serde_json::from_slice(&axum::body::to_bytes(msg_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert!(msg_val["assistant_message"]["content"].as_str().unwrap().contains("110"));
}

#[tokio::test]
async fn test_server_members_and_search() {
    let config = create_test_config();
    let pool = db::connect(&config.database_url, 5).await.unwrap();
    db::run_migrations(&pool).await.unwrap();

    let uid = Uuid::new_v4();
    let uname = format!("srv_{}", &uid.to_string()[..6]);
    sqlx::query(
        "INSERT INTO users (id, google_sub, username, username_norm, presence, status_text, status_emoji) VALUES ($1, $2, $3, $4, 'online', 'Đang trực', '🚀')",
    )
    .bind(uid)
    .bind(format!("sub_{uid}"))
    .bind(&uname)
    .bind(uname.to_lowercase())
    .execute(&pool)
    .await
    .unwrap();

    let cookie = create_session(&pool, &config, uid, None).await.unwrap();
    let hdr = cookie.split(';').next().unwrap().to_string();
    let app = build_test_app(pool.clone(), config).await;

    let srv_res = app
        .clone()
        .oneshot(
            Request::post("/api/servers")
                .header(header::COOKIE, hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "name": "KiyoMC" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(srv_res.status(), StatusCode::OK);
    let srv_val: Value = serde_json::from_slice(&axum::body::to_bytes(srv_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    let sid = srv_val["id"].as_str().unwrap().to_string();

    let ch_res = app
        .clone()
        .oneshot(Request::get(format!("/api/servers/{sid}/channels")).header(header::COOKIE, hdr.as_str()).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let chs: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(ch_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    let cid = chs[0]["id"].as_str().unwrap().to_string();

    let post_res = app
        .clone()
        .oneshot(
            Request::post(format!("/api/channels/{cid}/messages"))
                .header(header::COOKIE, hdr.as_str())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "content": "Thử tìm kiếm discord" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(post_res.status(), StatusCode::OK);

    let mem_res = app
        .clone()
        .oneshot(Request::get(format!("/api/servers/{sid}/members")).header(header::COOKIE, hdr.as_str()).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(mem_res.status(), StatusCode::OK);
    let mems: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(mem_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(mems.len(), 1);
    assert_eq!(mems[0]["presence"], "online");
    assert_eq!(mems[0]["status_text"], "Đang trực");
    assert_eq!(mems[0]["status_emoji"], "🚀");

    let search_res = app
        .clone()
        .oneshot(Request::get(format!("/api/servers/{sid}/search?q=discord")).header(header::COOKIE, hdr.as_str()).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(search_res.status(), StatusCode::OK);
    let hits: Vec<Value> = serde_json::from_slice(&axum::body::to_bytes(search_res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["content"], "Thử tìm kiếm discord");
}


