//! Send an end-to-end encrypted direct message from anywhere in the app
//! (composer, auto-reply, business greeting, invite cards, polls, images).

use crate::{
    crypto::e2ee::{PeerKeys, encrypt_chat_message, get_or_verify_peer_keys, get_user_keys},
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

pub async fn send_encrypted_dm(my_id: &str, partner_id: &str, text: &str, reply_to: Option<&str>) -> Result<SentDm, String> {
    let keys = get_user_keys(Some(my_id)).ok_or("missing_local_keys")?;
    let cached = PEERS.with(|p| p.borrow().get(partner_id).cloned());
    let peer = match cached {
        Some(p) => p,
        None => {
            let pk = api::keys_get(partner_id).await.map_err(|e| e.code)?;
            let peer = get_or_verify_peer_keys(&pk.user_id, &pk.ecdh_public, &pk.ecdsa_public, &pk.fingerprint)?;
            PEERS.with(|p| p.borrow_mut().insert(partner_id.to_string(), peer.clone()));
            peer
        }
    };
    let id = uuid::Uuid::new_v4().to_string();
    let (recipient_env, sender_env) = encrypt_chat_message(&id, my_id, partner_id, &keys, &peer, text)?;
    let resp = api::messages_send_reply(&id, partner_id, &recipient_env, &sender_env, reply_to).await.map_err(|e| e.code)?;
    Ok(SentDm { id, seq: resp.seq, created_at: resp.created_at })
}
