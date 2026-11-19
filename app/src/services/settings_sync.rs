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

/// On start (and when another device announces a newer version): take the
/// server copy if it is newer, or upload ours when the server has none yet.
pub async fn pull(ctx: PrefsCtx) {
    let local: Prefs = ctx.prefs.peek().clone();
    if !local.sync_settings {
        return;
    }
    let Ok((data, version)) = fetch().await else { return };
    if version == 0 {
        if let Ok(v) = push(local.sync_payload(), 0).await {
            VERSION.with(|c| c.set(v));
        }
        return;
    }
    if version != known_version() {
        VERSION.with(|c| c.set(version));
        let merged = local.merged_with(&data);
        if merged != local && merged.sync_settings {
            ctx.apply_remote(merged);
        }
    }
}

/// Called after every local change; uploads 1.2 s after the last one.
pub fn schedule_push(ctx: &PrefsCtx) {
    let ctx = *ctx;
    let generation = GENERATION.with(|g| {
        g.set(g.get() + 1);
        g.get()
    });
    // Not tied to the component that changed the setting (a picker may close first).
    dioxus::core::spawn_forever(async move {
        platform::sleep_ms(1200).await;
        if GENERATION.with(|g| g.get()) != generation {
            return;
        }
        let payload = ctx.prefs.peek().sync_payload();
        let base = known_version().max(0);
        match push(payload.clone(), base).await {
            Ok(v) => VERSION.with(|c| c.set(v)),
            Err(e) if e.status == 409 => {
                // Someone else wrote first: adopt their version, keep my latest change on top.
                if let Ok((_, v)) = fetch().await {
                    if let Ok(nv) = push(payload, v).await {
                        VERSION.with(|c| c.set(nv));
                    }
                }
            }
            Err(_) => {}
        }
    });
}
