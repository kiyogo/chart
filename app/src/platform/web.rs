use futures_channel::{mpsc::UnboundedSender, oneshot};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestCredentials, RequestInit, Response, WebSocket};

use crate::models::{ApiError, WsEvent};

pub fn api_base() -> String {
    String::new()
}

pub fn initial_window_width() -> u32 {
    web_sys::window()
        .and_then(|w| w.inner_width().ok())
        .and_then(|v| v.as_f64())
        .map(|v| v as u32)
        .unwrap_or(0)
}

pub fn storage_get(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(key)
        .ok()?
}

pub fn storage_set(key: &str, value: &str) {
    if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) {
        let _ = storage.set_item(key, value);
    }
}

pub async fn sleep_ms(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(win) = web_sys::window() {
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32);
        }
    });
    let _ = JsFuture::from(promise).await;
}

fn is_cookie_mutating_auth_path(path: &str) -> bool {
    let bare = path.split('?').next().unwrap_or(path);
    matches!(
        bare,
        "/api/auth/login"
            | "/api/auth/register"
            | "/api/auth/password"
            | "/api/auth/dev-login"
            | "/api/auth/logout"
            | "/api/auth/logout-all"
            | "/api/auth/google"
            | "/api/auth/google/callback"
            | "/api/auth/providers"
            | "/api/auth/oauth-ticket"
            | "/api/auth/2fa/pending"
            | "/api/auth/2fa/verify"
            | "/api/auth/accounts"
            | "/api/auth/accounts/add"
            | "/api/auth/accounts/switch"
    ) || bare.starts_with("/api/auth/oauth-ticket/")
        || bare.starts_with("/api/auth/accounts/")
}

async fn try_core_worker_rpc(
    method: &str,
    path: &str,
    json_body: Option<&str>,
    raw_body: Option<(&[u8], &str)>,
) -> Option<Result<(u16, String), ApiError>> {
    if is_cookie_mutating_auth_path(path) {
        return None;
    }
    let m_json = serde_json::to_string(method).ok()?;
    let p_json = serde_json::to_string(path).ok()?;
    let j_json = match json_body {
        Some(s) => serde_json::to_string(s).ok()?,
        None => "null".to_string(),
    };
    let (r_b64_json, r_ct_json) = match raw_body {
        Some((bytes, ct)) => {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
            (
                serde_json::to_string(&b64).ok()?,
                serde_json::to_string(ct).ok()?,
            )
        }
        None => ("null".to_string(), "\"\"".to_string()),
    };
    let wait_ms: u32 = if path.starts_with("/api/auth/me") { 550 } else { 2200 };
    let is_static_cacheable = method.eq_ignore_ascii_case("GET")
        && (path.starts_with("/stickers/openmoji/catalog.json") || path.starts_with("/locales/"));
    let static_flag = if is_static_cacheable { "true" } else { "false" };

    let js = format!(
        r#"
        (async () => {{
            try {{
                if ({static_flag} && window.__chatwStorage && window.__chatwStorage.getStaticCache) {{
                    const hit = await window.__chatwStorage.getStaticCache('static:' + {p_json});
                    if (hit && typeof hit.data === 'string') {{
                        dioxus.send({{ handled: true, ok: true, status: 200, text: hit.data }});
                        return;
                    }}
                }}
                const core = window.__chatwCore;
                if (!core || !core.isSupported || !core.isSupported()) {{
                    dioxus.send({{ handled: false }});
                    return;
                }}
                if (core.isEffectivelyOffline ? core.isEffectivelyOffline() : (core.isSimulatedOffline && core.isSimulatedOffline())) {{
                    dioxus.send({{ handled: true, ok: false, status: 0, error: 'network_error', text: '' }});
                    return;
                }}
                if (!core.isConnected() && core.hasAuthHint && !core.hasAuthHint()) {{
                    dioxus.send({{ handled: false }});
                    return;
                }}
                if (!core.isConnected()) {{
                    await core.waitUntilReady({wait_ms});
                }}
                if (!core.isConnected()) {{
                    if (core.isEffectivelyOffline && core.isEffectivelyOffline()) {{
                        dioxus.send({{ handled: true, ok: false, status: 0, error: 'network_error', text: '' }});
                        return;
                    }}
                    dioxus.send({{ handled: false }});
                    return;
                }}
                let res = await core.rpcFromRust({m_json}, {p_json}, {j_json}, {r_b64_json}, {r_ct_json});
                if ((!res || !res.ok || res.status === 0) && !(core.isEffectivelyOffline && core.isEffectivelyOffline())) {{
                    try {{
                        core.reconnectNow();
                        const ready = await core.waitUntilReady(2000);
                        if (ready) {{
                            res = await core.rpcFromRust({m_json}, {p_json}, {j_json}, {r_b64_json}, {r_ct_json});
                        }}
                    }} catch (_) {{}}
                    if (!res || !res.ok || res.status === 0) {{
                        dioxus.send({{ handled: false }});
                        return;
                    }}
                }}
                if ({static_flag} && res && res.ok && res.status === 200 && res.text && window.__chatwStorage && window.__chatwStorage.putStaticCache) {{
                    window.__chatwStorage.putStaticCache('static:' + {p_json}, 'v2', res.text).catch(() => {{}});
                }}
                dioxus.send({{ handled: true, ...res }});
            }} catch (e) {{
                dioxus.send({{ handled: false }});
            }}
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v: serde_json::Value = eval.recv().await.ok()?;
    if v.get("handled").and_then(|b| b.as_bool()) != Some(true) {
        return None;
    }
    let ok = v.get("ok").and_then(|b| b.as_bool()).unwrap_or(false);
    let status = v.get("status").and_then(|n| n.as_u64()).unwrap_or(0) as u16;
    let text = v.get("text").and_then(|s| s.as_str()).unwrap_or("").to_string();
    if !ok || status == 0 {
        let code = v
            .get("error")
            .and_then(|s| s.as_str())
            .unwrap_or("network_error")
            .to_string();
        return Some(Err(ApiError { code, status: 0 }));
    }
    if !(200..300).contains(&status) {
        let mut code = "request_failed".to_string();
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(e) = parsed.get("error").and_then(|s| s.as_str())
        {
            code = e.to_string();
        }
        return Some(Err(ApiError { code, status }));
    }
    Some(Ok((status, text)))
}

pub async fn http_request(
    method: &str,
    path: &str,
    json_body: Option<String>,
    raw_body: Option<(Vec<u8>, &str)>,
) -> Result<(u16, String), ApiError> {
    // 1. Prefer SharedWorker CWBF2 Encrypted Binary RPC Multiplexer (0 Fetch/XHR on F12!)
    let raw_ref = raw_body.as_ref().map(|(b, ct)| (b.as_slice(), *ct));
    if let Some(rpc_res) = try_core_worker_rpc(method, path, json_body.as_deref(), raw_ref).await {
        return rpc_res;
    }

    // 2. HTTP fetch fallback (used ONLY for HttpOnly cookie-setting login/logout or pre-auth boot)
    let window = web_sys::window().ok_or_else(|| ApiError {
        code: "no_window".into(),
        status: 0,
    })?;

    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_credentials(RequestCredentials::Include);

    let headers = Headers::new().map_err(|_| ApiError {
        code: "headers_failed".into(),
        status: 0,
    })?;

    if let Some((bytes, content_type)) = raw_body {
        let _ = headers.set("Content-Type", content_type);
        let u8arr = js_sys::Uint8Array::from(bytes.as_slice());
        opts.set_body(&u8arr.into());
    } else if let Some(body) = json_body {
        let _ = headers.set("Content-Type", "application/json");
        opts.set_body(&JsValue::from_str(&body));
    }
    opts.set_headers(&headers);

    let request = Request::new_with_str_and_init(path, &opts).map_err(|_| ApiError {
        code: "request_init_failed".into(),
        status: 0,
    })?;

    let resp_val = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| ApiError {
            code: "network_error".into(),
            status: 0,
        })?;

    let resp: Response = resp_val.dyn_into().map_err(|_| ApiError {
        code: "invalid_response".into(),
        status: 0,
    })?;

    let status = resp.status();
    let text_promise = resp.text().map_err(|_| ApiError {
        code: "read_body_failed".into(),
        status,
    })?;
    let text_val = JsFuture::from(text_promise).await.map_err(|_| ApiError {
        code: "read_body_failed".into(),
        status,
    })?;
    let text = text_val.as_string().unwrap_or_default();

    if !(200..300).contains(&status) {
        let mut code = "request_failed".to_string();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(e) = v.get("error").and_then(|s| s.as_str())
        {
            code = e.to_string();
        }
        return Err(ApiError { code, status });
    }

    // Immediately connect SharedWorker Binary RPC right after login or auth_me confirms session cookie,
    // so 100% of subsequent app requests have 0 Fetch/XHR on F12!
    if path.starts_with("/api/auth/dev-login") || path.starts_with("/api/auth/me") {
        let mut ev = dioxus::document::eval(
            r#"
            (async () => {
                try {
                    if (window.__chatwCore) {
                        if (window.__chatwCore.setAuthHint) window.__chatwCore.setAuthHint(true);
                        if (!window.__chatwCore.isConnected()) {
                            window.__chatwCore.reconnectNow();
                            await window.__chatwCore.waitUntilReady(2500);
                        }
                        await window.__chatwCore.bridgeServiceWorkerPort();
                    }
                } catch (_) {}
                dioxus.send(true);
            })();
            "#,
        );
        let _ = ev.recv::<bool>().await;
    }

    Ok((status, text))
}

pub async fn start_google_oauth() -> Result<bool, String> {
    if let Some(win) = web_sys::window() {
        let _ = win.location().set_href("/api/auth/google");
        Ok(false)
    } else {
        Err("Cannot open Google sign in page".into())
    }
}

/// WebSocket loop offloaded to Dedicated Web Worker (`/network_worker.js`) with
/// ECDH P-256 + AES-256-GCM encrypted binary protocol (`CWBF1`, `/api/ws?proto=bin`).
/// Gracefully falls back to direct WebSocket only if `Worker` / Web Crypto is unavailable.
pub async fn run_ws_loop(tx: UnboundedSender<WsEvent>, stop: Arc<AtomicBool>) {
    let ws_url = match web_sys::window() {
        Some(w) => {
            let loc = w.location();
            let proto = loc.protocol().unwrap_or_else(|_| "http:".into());
            let host = loc.host().unwrap_or_else(|_| "127.0.0.1:8085".into());
            let ws_proto = if proto == "https:" { "wss:" } else { "ws:" };
            format!("{ws_proto}//{host}/api/ws")
        }
        None => return,
    };

    // Prefer Dedicated Web Worker binary encrypted WebSocket bridge (Spec 069)
    if let Some(win) = web_sys::window()
        && let Ok(bridge) = js_sys::Reflect::get(&win, &JsValue::from_str("__chatwNetBridge"))
        && !bridge.is_undefined()
        && !bridge.is_null()
    {
        let is_sup = js_sys::Reflect::get(&bridge, &JsValue::from_str("isSupported"))
            .ok()
            .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
            .and_then(|f| f.call0(&bridge).ok())
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_sup {
            let tx_worker = tx.clone();
            let cb = Closure::<dyn FnMut(JsValue)>::new(move |val: JsValue| {
                if let Some(txt) = val.as_string()
                    && let Ok(parsed) = serde_json::from_str::<WsEvent>(&txt)
                {
                    let _ = tx_worker.unbounded_send(parsed);
                }
            });

            let unsub_fn = js_sys::Reflect::get(&bridge, &JsValue::from_str("subscribe"))
                .ok()
                .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
                .and_then(|f| f.call1(&bridge, cb.as_ref().unchecked_ref()).ok())
                .and_then(|v| v.dyn_into::<js_sys::Function>().ok());

            let started = js_sys::Reflect::get(&bridge, &JsValue::from_str("start"))
                .ok()
                .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
                .and_then(|f| f.call1(&bridge, &JsValue::from_str(&ws_url)).ok())
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if started {
                while !stop.load(Ordering::Relaxed) {
                    sleep_ms(200).await;
                }
                if let Some(unsub) = unsub_fn {
                    let _ = unsub.call0(&JsValue::NULL);
                }
                if let Ok(stop_f) = js_sys::Reflect::get(&bridge, &JsValue::from_str("stop"))
                    && let Ok(f) = stop_f.dyn_into::<js_sys::Function>()
                {
                    let _ = f.call0(&bridge);
                }
                drop(cb);
                return;
            }
            if let Some(unsub) = unsub_fn {
                let _ = unsub.call0(&JsValue::NULL);
            }
            drop(cb);
        }
    }

    // Legacy fallback if Dedicated Web Worker is unavailable
    let mut retry_delay_ms: f64 = 1000.0;

    while !stop.load(Ordering::Relaxed) {
        let ws = match WebSocket::new(&ws_url) {
            Ok(ws) => ws,
            Err(_) => {
                sleep_ms(retry_delay_ms as u32).await;
                retry_delay_ms = (retry_delay_ms * 1.5).min(10000.0);
                continue;
            }
        };

        let (close_tx, mut close_rx) = oneshot::channel::<()>();
        let close_tx = Arc::new(std::sync::Mutex::new(Some(close_tx)));
        let opened = Arc::new(AtomicBool::new(false));

        let opened_clone = opened.clone();
        let onopen = Closure::<dyn FnMut()>::new(move || {
            opened_clone.store(true, Ordering::Relaxed);
        });
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));

        let tx_clone = tx.clone();
        let onmessage =
            Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
                if let Some(txt) = e.data().as_string()
                    && let Ok(parsed) = serde_json::from_str::<WsEvent>(&txt)
                {
                    let _ = tx_clone.unbounded_send(parsed);
                }
            });
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

        let close_tx_c = close_tx.clone();
        let onclose =
            Closure::<dyn FnMut(web_sys::CloseEvent)>::new(move |_: web_sys::CloseEvent| {
                if let Ok(mut guard) = close_tx_c.lock()
                    && let Some(c) = guard.take()
                {
                    let _ = c.send(());
                }
            });
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));

        let ws_err_ref = ws.clone();
        let onerror =
            Closure::<dyn FnMut(web_sys::ErrorEvent)>::new(move |_: web_sys::ErrorEvent| {
                let _ = ws_err_ref.close();
            });
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));

        // Wait until closed or stopped
        loop {
            if stop.load(Ordering::Relaxed) {
                let _ = ws.close();
                break;
            }
            if close_rx.try_recv().ok().flatten().is_some() {
                break;
            }
            sleep_ms(150).await;
        }

        ws.set_onopen(None);
        ws.set_onmessage(None);
        ws.set_onclose(None);
        ws.set_onerror(None);
        drop(onopen);
        drop(onmessage);
        drop(onclose);
        drop(onerror);

        if stop.load(Ordering::Relaxed) {
            break;
        }
        if opened.load(Ordering::Relaxed) {
            retry_delay_ms = 1000.0;
        }
        sleep_ms(retry_delay_ms as u32).await;
        retry_delay_ms = (retry_delay_ms * 1.5).min(10000.0);
    }
}

/// Download raw bytes via SharedWorker Binary WS (`OP_MEDIA_CHUNK_REQ`) with HTTP fallback.
pub async fn fetch_bytes(path: &str) -> Result<Vec<u8>, String> {
    let url_json = serde_json::to_string(path).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            try {{
                if (window.__chatwCore && window.__chatwCore.isSupported && window.__chatwCore.isSupported()) {{
                    if (!window.__chatwCore.isConnected()) await window.__chatwCore.waitUntilReady(1800);
                    if (window.__chatwCore.isConnected()) {{
                        const dl = await window.__chatwCore.downloadBytes({url_json}, {{ priority: 'HIGH' }});
                        if (dl && dl.ok && dl.b64) {{
                            dioxus.send({{ b64: dl.b64 }});
                            return;
                        }}
                    }}
                }}
                const res = await fetch({url_json}, {{ credentials: 'include' }});
                if (!res.ok) {{ dioxus.send({{ error: 'http_' + res.status }}); return; }}
                const buf = new Uint8Array(await res.arrayBuffer());
                let s = '';
                for (let i = 0; i < buf.length; i += 0x8000) s += String.fromCharCode.apply(null, buf.subarray(i, i + 0x8000));
                dioxus.send({{ b64: btoa(s) }});
            }} catch (e) {{ dioxus.send({{ error: 'network_error' }}); }}
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let v = eval.recv::<serde_json::Value>().await.map_err(|_| "eval_failed".to_string())?;
    if let Some(e) = v.get("error").and_then(|e| e.as_str()) {
        return Err(e.to_string());
    }
    use base64::Engine;
    let b64 = v.get("b64").and_then(|b| b.as_str()).ok_or("no_data")?;
    base64::engine::general_purpose::STANDARD.decode(b64).map_err(|_| "decode_failed".to_string())
}

pub async fn stream_export_download(
    path: &str,
    json_body: &str,
    default_filename: &str,
) -> Result<super::ExportDownloadResult, ApiError> {
    let url_json = serde_json::to_string(path).unwrap_or_default();
    let body_json = serde_json::to_string(json_body).unwrap_or_default();
    let fname_json = serde_json::to_string(default_filename).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            try {{
                if (!window.__chatwStorage || !window.__chatwStorage.streamDownloadPost) {{
                    dioxus.send({{ ok: false, status: 0, error: 'storage_engine_unavailable' }});
                    return;
                }}
                const out = await window.__chatwStorage.streamDownloadPost({url_json}, {body_json}, {fname_json}, {{}});
                dioxus.send(out);
            }} catch (e) {{
                dioxus.send({{ ok: false, status: 0, error: 'network_error' }});
            }}
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let val = eval.recv::<serde_json::Value>().await.map_err(|_| ApiError {
        code: "eval_failed".into(),
        status: 0,
    })?;
    if !val.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        let status = val.get("status").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
        let code = val
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("export_failed")
            .to_string();
        return Err(ApiError { code, status });
    }
    Ok(super::ExportDownloadResult {
        filename: val
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or(default_filename)
            .to_string(),
        byte_size: val.get("byte_size").and_then(|v| v.as_u64()).unwrap_or(0),
        job_id: val
            .get("job_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

pub async fn selected_file_meta(input_id: &str) -> Option<super::SelectedFileMeta> {
    let id_json = serde_json::to_string(input_id).unwrap_or_default();
    let js = format!(
        r#"
        (() => {{
            const info = window.__chatwStorage && window.__chatwStorage.getSelectedFileInfo
                ? window.__chatwStorage.getSelectedFileInfo({id_json})
                : null;
            dioxus.send(info);
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let val = eval.recv::<serde_json::Value>().await.ok()?;
    if val.is_null() {
        return None;
    }
    Some(super::SelectedFileMeta {
        name: val.get("name")?.as_str()?.to_string(),
        byte_size: val.get("byte_size")?.as_u64()?,
    })
}

pub async fn upload_backup_from_input(
    input_id: &str,
    endpoint_path: &str,
    backup_password: Option<&str>,
) -> Result<super::UploadBackupResult, ApiError> {
    let id_json = serde_json::to_string(input_id).unwrap_or_default();
    let url_json = serde_json::to_string(endpoint_path).unwrap_or_default();
    let pw_json = serde_json::to_string(backup_password.unwrap_or("")).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            try {{
                if (!window.__chatwStorage || !window.__chatwStorage.uploadBackupInput) {{
                    dioxus.send({{ ok: false, status: 0, error: 'storage_engine_unavailable', body: '' }});
                    return;
                }}
                const out = await window.__chatwStorage.uploadBackupInput({id_json}, {url_json}, {pw_json});
                dioxus.send(out);
            }} catch (e) {{
                dioxus.send({{ ok: false, status: 0, error: 'network_error', body: '' }});
            }}
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let val = eval.recv::<serde_json::Value>().await.map_err(|_| ApiError {
        code: "eval_failed".into(),
        status: 0,
    })?;
    let status = val.get("status").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
    let body = val
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
        return Err(ApiError {
            code: err.to_string(),
            status,
        });
    }
    if !(200..300).contains(&status) {
        let mut code = "request_failed".to_string();
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body)
            && let Some(e) = parsed.get("error").and_then(|s| s.as_str())
        {
            code = e.to_string();
        }
        return Err(ApiError { code, status });
    }
    Ok(super::UploadBackupResult {
        status,
        body,
        filename: val
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("backup")
            .to_string(),
        byte_size: val.get("byte_size").and_then(|v| v.as_u64()).unwrap_or(0),
    })
}

pub async fn register_device_passkey_local() -> Result<super::DevicePasskeyCreated, ApiError> {
    let js = r#"
        (async () => {
            try {
                if (!window.__chatwStorage || !window.__chatwStorage.registerDevicePasskey) {
                    dioxus.send({ error: 'storage_engine_unavailable' });
                    return;
                }
                const out = await window.__chatwStorage.registerDevicePasskey();
                dioxus.send(out);
            } catch (e) {
                dioxus.send({ error: 'passkey_keygen_failed' });
            }
        })();
    "#;
    let mut eval = dioxus::document::eval(js);
    let val = eval.recv::<serde_json::Value>().await.map_err(|_| ApiError {
        code: "eval_failed".into(),
        status: 0,
    })?;
    if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
        return Err(ApiError {
            code: err.to_string(),
            status: 400,
        });
    }
    Ok(super::DevicePasskeyCreated {
        credential_id: val
            .get("credential_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        public_key_spki_b64: val
            .get("public_key_spki_b64")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

pub async fn sign_passkey_challenge_local(
    challenge_b64: &str,
) -> Result<super::DevicePasskeySignature, ApiError> {
    let ch_json = serde_json::to_string(challenge_b64).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            try {{
                if (!window.__chatwStorage || !window.__chatwStorage.signPasskeyChallenge) {{
                    dioxus.send({{ error: 'storage_engine_unavailable' }});
                    return;
                }}
                const out = await window.__chatwStorage.signPasskeyChallenge({ch_json});
                dioxus.send(out);
            }} catch (e) {{
                dioxus.send({{ error: 'passkey_sign_failed' }});
            }}
        }})();
        "#
    );
    let mut eval = dioxus::document::eval(&js);
    let val = eval.recv::<serde_json::Value>().await.map_err(|_| ApiError {
        code: "eval_failed".into(),
        status: 0,
    })?;
    if let Some(err) = val.get("error").and_then(|v| v.as_str()) {
        return Err(ApiError {
            code: err.to_string(),
            status: 400,
        });
    }
    Ok(super::DevicePasskeySignature {
        credential_id: val
            .get("credential_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        signature_b64: val
            .get("signature_b64")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}
