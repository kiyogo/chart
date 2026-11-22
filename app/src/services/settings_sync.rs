//! Sync app preferences between my devices through `/api/settings-sync`.
//!
//! Writes are debounced and versioned: a write based on an old version gets a
//! 409, the client then pulls the newest copy, re-applies its own latest
//! preferences on top and retries once. The PIN lock never leaves the device
//! (stripped here and again by the server).

use std::cell::Cell;

use serde_json::{Map, Value, json};

use dioxus::prelude::*;

use crate::{
    models::ApiError,
    platform,
    prefs::{Prefs, PrefsCtx},
};

thread_local! {
    static VERSION: Cell<i64> = const { Cell::new(-1) };
    static GENERATION: Cell<u64> = const { Cell::new(0) };
}

pub fn known_version() -> i64 {
    VERSION.with(|v| v.get())
}

async fn fetch() -> Result<(Map<String, Value>, i64), ApiError> {
    let (_, body) = platform::http_request("GET", "/api/settings-sync", None, None).await?;
    let v: Value = serde_json::from_str(&body).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })?;
    let data = v.get("data").and_then(|d| d.as_object()).cloned().unwrap_or_default();
    Ok((data, v.get("version").and_then(|x| x.as_i64()).unwrap_or(0)))
}

async fn push(payload: Map<String, Value>, base: i64) -> Result<i64, ApiError> {
    let body = json!({ "data": payload, "base_version": base }).to_string();
    let (_, resp) = platform::http_request("PUT", "/api/settings-sync", Some(body), None).await?;
    let v: Value = serde_json::from_str(&resp).map_err(|_| ApiError { code: "invalid_json".into(), status: 200 })?;
    Ok(v.get("version").and_then(|x| x.as_i64()).unwrap_or(base + 1))
}

fn ver_key(uid: &str) -> String {
    format!("chatw_sync_ver_{uid}")
}

fn dirty_key(uid: &str) -> String {
    format!("chatw_sync_dirty_{uid}")
}

fn load_persisted_ver(uid: &str) -> i64 {
    platform::storage_get(&ver_key(uid))
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(-1)
}

fn save_persisted_ver(uid: &str, v: i64) {
    VERSION.with(|c| c.set(v));
    platform::storage_set(&ver_key(uid), &v.to_string());
    platform::storage_set(&dirty_key(uid), "0");
}

fn is_dirty(uid: &str) -> bool {
    platform::storage_get(&dirty_key(uid)).as_deref() == Some("1")
}

/// On start (and when another device announces a newer version): take the
/// server copy if it is newer, or upload ours when the server has none yet
/// or when this device has unpushed local changes (e.g. reload before debounce).
pub async fn pull(ctx: PrefsCtx) {
    let uid = ctx.user_id.peek().clone();
    let local: Prefs = ctx.prefs.peek().clone();
    if !local.sync_settings {
        return;
    }
    if known_version() < 0 {
        let pv = load_persisted_ver(&uid);
        if pv >= 0 {
            VERSION.with(|c| c.set(pv));
        }
    }
    let Ok((data, version)) = fetch().await else { return };
    if version == 0 || is_dirty(&uid) {
        if let Ok(v) = push(local.sync_payload(), version).await {
            save_persisted_ver(&uid, v);
        }
        return;
    }
    if version != known_version() {
        save_persisted_ver(&uid, version);
        let merged = local.merged_with(&data);
        if merged != local && merged.sync_settings {
            ctx.apply_remote(merged);
        }
    }
}

/// Called after every local change; marks dirty immediately and uploads 350 ms after the last change.
pub fn schedule_push(ctx: &PrefsCtx) {
    let ctx = *ctx;
    let uid = ctx.user_id.peek().clone();
    platform::storage_set(&dirty_key(&uid), "1");
    let generation = GENERATION.with(|g| {
        g.set(g.get() + 1);
        g.get()
    });
    // Not tied to the component that changed the setting (a picker may close first).
    dioxus::core::spawn_forever(async move {
        platform::sleep_ms(350).await;
        if GENERATION.with(|g| g.get()) != generation {
            return;
        }
        let payload = ctx.prefs.peek().sync_payload();
        let base = known_version().max(load_persisted_ver(&uid)).max(0);
        match push(payload.clone(), base).await {
            Ok(v) => save_persisted_ver(&uid, v),
            Err(e) if e.status == 409 => {
                // Someone else wrote first: adopt their version, keep my latest change on top.
                if let Ok((_, v)) = fetch().await {
                    if let Ok(nv) = push(payload, v).await {
                        save_persisted_ver(&uid, nv);
                    }
                }
            }
            Err(_) => {}
        }
    });
}
