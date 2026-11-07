use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chatw_core::ClientKeys;

#[derive(Serialize, Deserialize, Default)]
pub struct LocalStore {
    pub ecdh_hex: Option<String>,
    pub ecdsa_hex: Option<String>,
    pub last_user_id: Option<Uuid>,
    pub last_username: Option<String>,
}

fn get_storage_path() -> PathBuf {
    if let Some(app_data) = std::env::var_os("APPDATA") {
        let dir = PathBuf::from(app_data).join("chatw");
        let _ = fs::create_dir_all(&dir);
        dir.join("storage.json")
    } else {
        let dir = PathBuf::from("chatw_data");
        let _ = fs::create_dir_all(&dir);
        dir.join("storage.json")
    }
}

pub fn load_or_create_keys() -> ClientKeys {
    let path = get_storage_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(store) = serde_json::from_str::<LocalStore>(&content) {
            if let (Some(ecdh), Some(ecdsa)) = (store.ecdh_hex, store.ecdsa_hex) {
                if let Ok(keys) = ClientKeys::import_raw(&ecdh, &ecdsa) {
                    return keys;
                }
            }
        }
    }

    // Generate new keys and save
    let keys = ClientKeys::generate();
    let (ecdh_hex, ecdsa_hex) = keys.export_raw();
    let store = LocalStore {
        ecdh_hex: Some(ecdh_hex),
        ecdsa_hex: Some(ecdsa_hex),
        ..Default::default()
    };
    if let Ok(json) = serde_json::to_string_pretty(&store) {
        let _ = fs::write(path, json);
    }
    keys
}

pub fn save_user_session(user_id: Uuid, username: Option<String>) {
    let path = get_storage_path();
    let mut store = if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str::<LocalStore>(&content).unwrap_or_default()
    } else {
        LocalStore::default()
    };
    store.last_user_id = Some(user_id);
    store.last_username = username;
    if let Ok(json) = serde_json::to_string_pretty(&store) {
        let _ = fs::write(path, json);
    }
}

pub fn clear_user_session() {
    let path = get_storage_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(mut store) = serde_json::from_str::<LocalStore>(&content) {
            store.last_user_id = None;
            store.last_username = None;
            if let Ok(json) = serde_json::to_string_pretty(&store) {
                let _ = fs::write(path, json);
            }
        }
    }
}
