use base64::{Engine, engine::general_purpose::STANDARD as B64};

use super::{LocalObjectMeta, LocalQuotaInfo};

pub async fn init_local_storage() -> Result<(String, String, String), String> {
    let js = r#"
    (async () => {
        try {
            if (!window.__chatwStorage) {
                dioxus.send({ ok: false });
                return;
            }
            const res = await window.__chatwStorage.init();
            dioxus.send(res);
        } catch (e) {
            dioxus.send({ ok: false, error: String(e) });
        }
    })();
    "#;
    let mut eval = dioxus::document::eval(js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    let cid = v.get("clientDeviceId").and_then(|s| s.as_str()).unwrap_or("web-fallback-01").to_string();
    let dname = v.get("deviceName").and_then(|s| s.as_str()).unwrap_or("Web Browser").to_string();
    Ok((cid, "web".into(), dname))
}

pub async fn get_client_device_info() -> (String, String, String) {
    init_local_storage()
        .await
        .unwrap_or_else(|_| ("web-device-local".into(), "web".into(), "Web Browser".into()))
}

pub async fn put_local_object(meta: &LocalObjectMeta, bytes: &[u8]) -> Result<String, String> {
    let meta_json = serde_json::to_string(meta).unwrap_or_default();
    let b64_json = serde_json::to_string(&B64.encode(bytes)).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            if (!window.__chatwStorage) {{ dioxus.send({{ ok: false, error: 'no_engine' }}); return; }}
            const res = await window.__chatwStorage.putObject({meta_json}, {b64_json});
            dioxus.send(res);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    if v.get("ok").and_then(|b| b.as_bool()) == Some(true) {
        Ok(v.get("local_tier").and_then(|s| s.as_str()).unwrap_or("indexeddb").to_string())
    } else {
        Err(v.get("error").and_then(|s| s.as_str()).unwrap_or("put_failed").to_string())
    }
}

pub async fn get_local_object(ref_id: &str) -> Option<(Vec<u8>, String, String)> {
    let rid_json = serde_json::to_string(ref_id).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            if (!window.__chatwStorage) {{ dioxus.send({{ found: false }}); return; }}
            const res = await window.__chatwStorage.getObject({rid_json});
            dioxus.send(res);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v: serde_json::Value = eval.recv().await.ok()?;
    if v.get("found").and_then(|b| b.as_bool()) != Some(true) {
        return None;
    }
    let b64 = v.get("bytesB64").and_then(|s| s.as_str())?;
    let bytes = B64.decode(b64).ok()?;
    let mime = v.get("mime_type").and_then(|s| s.as_str()).unwrap_or("application/octet-stream").to_string();
    let sha = v.get("sha256_plain").and_then(|s| s.as_str()).unwrap_or("").to_string();
    Some((bytes, mime, sha))
}

pub async fn delete_local_object(ref_id: &str, force_unique: bool) -> Result<u64, String> {
    let rid_json = serde_json::to_string(ref_id).unwrap_or_default();
    let force_str = if force_unique { "true" } else { "false" };
    let js = format!(
        r#"
        (async () => {{
            if (!window.__chatwStorage) {{ dioxus.send({{ deleted: false, reason: 'no_engine' }}); return; }}
            const res = await window.__chatwStorage.deleteObject({rid_json}, {force_str});
            dioxus.send(res);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    if v.get("deleted").and_then(|b| b.as_bool()) == Some(true) {
        Ok(v.get("freed_bytes").and_then(|n| n.as_u64()).unwrap_or(0))
    } else {
        Err(v.get("reason").and_then(|s| s.as_str()).unwrap_or("delete_failed").to_string())
    }
}

pub async fn clear_safe_cache(include_unique: bool) -> Result<(u64, usize), String> {
    let inc = if include_unique { "true" } else { "false" };
    let js = format!(
        r#"
        (async () => {{
            if (!window.__chatwStorage) {{ dioxus.send({{ removed: 0, freed_bytes: 0, protected_unique: 0 }}); return; }}
            const res = await window.__chatwStorage.clearSafeCache({inc});
            dioxus.send(res);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v: serde_json::Value = eval.recv().await.map_err(|e| e.to_string())?;
    Ok((
        v.get("freed_bytes").and_then(|n| n.as_u64()).unwrap_or(0),
        v.get("protected_unique").and_then(|n| n.as_u64()).unwrap_or(0) as usize,
    ))
}

pub async fn get_quota_estimate() -> LocalQuotaInfo {
    let js = r#"
    (async () => {
        if (!window.__chatwStorage) { dioxus.send({}); return; }
        const res = await window.__chatwStorage.getQuotaEstimate();
        dioxus.send(res);
    })();
    "#;
    let mut eval = dioxus::document::eval(js);
    eval.recv::<LocalQuotaInfo>().await.unwrap_or_default()
}

pub async fn list_local_locations() -> Vec<serde_json::Value> {
    let js = r#"
    (async () => {
        if (!window.__chatwStorage) { dioxus.send([]); return; }
        const res = await window.__chatwStorage.listLocalLocations();
        dioxus.send(res);
    })();
    "#;
    let mut eval = dioxus::document::eval(js);
    eval.recv::<Vec<serde_json::Value>>().await.unwrap_or_default()
}

pub async fn load_sync_cursors() -> (i64, i64, i64) {
    let js = r#"
    (async () => {
        if (!window.__chatwStorage) { dioxus.send({ dm_seq: 0, group_seq: 0, channel_seq: 0 }); return; }
        const res = await window.__chatwStorage.loadCursors();
        dioxus.send(res);
    })();
    "#;
    let mut eval = dioxus::document::eval(js);
    let v: serde_json::Value = eval.recv().await.unwrap_or_default();
    (
        v.get("dm_seq").and_then(|n| n.as_i64()).unwrap_or(0),
        v.get("group_seq").and_then(|n| n.as_i64()).unwrap_or(0),
        v.get("channel_seq").and_then(|n| n.as_i64()).unwrap_or(0),
    )
}

pub async fn save_sync_cursors(dm_seq: i64, group_seq: i64, channel_seq: i64) {
    let js = format!(
        r#"
        (async () => {{
            if (window.__chatwStorage) {{
                await window.__chatwStorage.saveCursors({{ dm_seq: {dm_seq}, group_seq: {group_seq}, channel_seq: {channel_seq} }});
            }}
        }})();
        "#
    );
    let _ = dioxus::document::eval(&js);
}
