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
