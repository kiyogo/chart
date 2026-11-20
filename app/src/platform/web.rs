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

pub async fn http_request(
    method: &str,
    path: &str,
    json_body: Option<String>,
    raw_body: Option<(Vec<u8>, &str)>,
) -> Result<(u16, String), ApiError> {
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

    Ok((status, text))
}

pub async fn start_google_oauth() -> Result<bool, String> {
    if let Some(win) = web_sys::window() {
        let _ = win.location().set_href("/api/auth/google");
        Ok(false)
    } else {
        Err("Không thể mở trang đăng nhập Google".into())
    }
}

/// WebSocket loop with exponential backoff reconnect (`1000ms` -> `*1.5` -> max `10000ms`).
pub async fn run_ws_loop(tx: UnboundedSender<WsEvent>, stop: Arc<AtomicBool>) {
    let mut retry_delay_ms: f64 = 1000.0;

    while !stop.load(Ordering::Relaxed) {
        let ws_url = match web_sys::window() {
            Some(w) => {
                let loc = w.location();
                let proto = loc.protocol().unwrap_or_else(|_| "http:".into());
                let host = loc.host().unwrap_or_else(|_| "127.0.0.1:8085".into());
                let ws_proto = if proto == "https:" { "wss:" } else { "ws:" };
                format!("{ws_proto}//{host}/api/ws")
            }
            None => break,
        };

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

/// Download raw bytes from an API path (used for encrypted attachments).
pub async fn fetch_bytes(path: &str) -> Result<Vec<u8>, String> {
    let url_json = serde_json::to_string(path).unwrap_or_default();
    let js = format!(
        r#"
        (async () => {{
            try {{
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
