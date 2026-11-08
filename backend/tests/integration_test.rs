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
    auth::{self, AppState, google::GoogleJwksCache, session::create_session},
    config::Config,
    crypto::{self, Envelope, PROTOCOL_VERSION, compute_fingerprint},
    db, friends, messages, typing, users,
    websocket::{self, WsHub},
};

fn create_test_config() -> Config {
    Config {
        database_url: "postgres://chatw:chatw_dev_password@127.0.0.1:55432/chatw".into(),
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
        config: Arc::new(config),
        db: pool,
        http_client,
        jwks_cache,
        ws_hub,
        oauth_tickets: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
    };

    axum::Router::new()
        .nest("/api/auth", auth::routes())
        .nest("/api/users", users::routes())
        .nest("/api/keys", crypto::keys::routes())
        .nest("/api/friends", friends::routes())
        .nest("/api/messages", messages::routes())
        .nest("/api/typing", typing::routes())
        .merge(websocket::routes())
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
    let cookie_a = create_session(&pool, &config, user_a_id).await.unwrap();
    let cookie_b = create_session(&pool, &config, user_b_id).await.unwrap();
    let cookie_c = create_session(&pool, &config, user_c_id).await.unwrap();

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
}
