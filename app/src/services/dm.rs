//! Send an end-to-end encrypted direct message from anywhere in the app
//! (composer, auto-reply, business greeting, invite cards, polls, images).

use crate::{
    crypto::e2ee::{PeerKeys, encrypt_chat_message, get_or_verify_peer_keys_scoped, get_user_keys},
    services::api,
};

pub struct SentDm {
    pub id: String,
    pub seq: i64,
    pub created_at: String,
}

thread_local! {
    /// Verified peer keys for this session (TOFU-checked on first fetch).
    static PEERS: std::cell::RefCell<std::collections::HashMap<String, PeerKeys>> = Default::default();
}

pub fn invalidate_peer_cache(my_id: &str, partner_id: &str) {
    let cache_key = format!("{my_id}:{partner_id}");
    PEERS.with(|p| {
        p.borrow_mut().remove(&cache_key);
    });
}

pub fn clear_peer_cache() {
    PEERS.with(|p| {
        p.borrow_mut().clear();
    });
}

pub async fn send_encrypted_dm(my_id: &str, partner_id: &str, text: &str, reply_to: Option<&str>) -> Result<SentDm, String> {
    send_encrypted_dm_with_id(my_id, partner_id, text, reply_to, None).await
}

/// Idempotent E2EE DM send using a caller-supplied deterministic `client_msg_id` (`random_id`),
/// ensuring network retries / offline outbox flushes NEVER create duplicate messages in PostgreSQL.
pub async fn send_encrypted_dm_with_id(
    my_id: &str,
    partner_id: &str,
    text: &str,
    reply_to: Option<&str>,
    client_msg_id: Option<&str>,
) -> Result<SentDm, String> {
    let keys = get_user_keys(Some(my_id)).ok_or("missing_local_keys")?;
    let cache_key = format!("{my_id}:{partner_id}");
    let cached = PEERS.with(|p| p.borrow().get(&cache_key).cloned());
    let peer = match cached {
        Some(p) => p,
        None => {
            let pk = api::keys_get(partner_id)
                .await
                .map_err(|e| if e.status == 0 { "network_error".to_string() } else { e.code })?;
            let peer = get_or_verify_peer_keys_scoped(
                Some(my_id),
                &pk.user_id,
                &pk.ecdh_public,
                &pk.ecdsa_public,
                &pk.fingerprint,
            )?;
            PEERS.with(|p| p.borrow_mut().insert(cache_key, peer.clone()));
            peer
        }
    };
    let id = client_msg_id
        .filter(|s| uuid::Uuid::parse_str(s).is_ok())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let (recipient_env, sender_env) = encrypt_chat_message(&id, my_id, partner_id, &keys, &peer, text)?;
    let resp = api::messages_send_reply(&id, partner_id, &recipient_env, &sender_env, reply_to)
        .await
        .map_err(|e| if e.status == 0 { "network_error".to_string() } else { e.code })?;
    Ok(SentDm { id, seq: resp.seq, created_at: resp.created_at })
}
