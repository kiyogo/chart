use futures_channel::mpsc::UnboundedSender;
use futures_util::StreamExt;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::models::{ApiError, WsEvent};

static COOKIE_STORE: OnceLock<Arc<reqwest::cookie::Jar>> = OnceLock::new();
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static SESSION_TOKEN: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static STORAGE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn api_base() -> String {
    std::env::var("CHATW_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8085".to_string())
        .trim_end_matches('/')
        .to_string()
}

pub fn initial_window_width() -> u32 {
    1280
}

fn cookie_jar() -> Arc<reqwest::cookie::Jar> {
    COOKIE_STORE
        .get_or_init(|| Arc::new(reqwest::cookie::Jar::default()))
        .clone()
}

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
        reqwest::Client::builder()
            .cookie_provider(cookie_jar())
            .timeout(Duration::from_secs(15))
            .build()
            .expect("reqwest client")
    })
}

fn session_token_store() -> &'static Mutex<Option<String>> {
    SESSION_TOKEN.get_or_init(|| {
        let stored = storage_get("chatw_session_token").filter(|s| !s.is_empty());
        if let Some(ref tok) = stored {
            if let Ok(url) = reqwest::Url::parse(&api_base()) {
                cookie_jar().add_cookie_str(&format!("chatw_session={tok}; Path=/"), &url);
            }
        }
        Mutex::new(stored)
    })
}

fn set_session_cookie(token: &str) {
    if let Ok(mut guard) = session_token_store().lock() {
        *guard = Some(token.to_string());
    }
    storage_set("chatw_session_token", token);
    if let Ok(url) = reqwest::Url::parse(&api_base()) {
        cookie_jar().add_cookie_str(&format!("chatw_session={token}; Path=/"), &url);
    }
}

fn capture_session_cookie_from_headers(headers: &reqwest::header::HeaderMap) {
    for val in headers.get_all(reqwest::header::SET_COOKIE) {
        if let Ok(s) = val.to_str()
            && let Some(rest) = s.strip_prefix("chatw_session=") {
                let tok = rest.split(';').next().unwrap_or("").trim();
                if let Ok(mut guard) = session_token_store().lock() {
                    if tok.is_empty() {
                        *guard = None;
                        storage_set("chatw_session_token", "");
                    } else {
                        *guard = Some(tok.to_string());
                        storage_set("chatw_session_token", tok);
                    }
                }
            }
    }
}

fn storage_file_path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .or_else(|_| std::env::var("LOCALAPPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".local").join("share"))
                .unwrap_or_else(|_| std::env::temp_dir())
        });
    base.join("chatw").join("client_storage.json")
}

fn load_storage_map() -> HashMap<String, String> {
    let path = storage_file_path();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_storage_map(map: &HashMap<String, String>) {
    let path = storage_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(map) {
        let _ = std::fs::write(&path, json);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
    }
}

pub fn storage_get(key: &str) -> Option<String> {
    let _g = STORAGE_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    load_storage_map().get(key).cloned()
}

pub fn storage_set(key: &str, value: &str) {
    let _g = STORAGE_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
    let mut map = load_storage_map();
    map.insert(key.to_string(), value.to_string());
    save_storage_map(&map);
}

pub async fn sleep_ms(ms: u32) {
    tokio::time::sleep(Duration::from_millis(ms as u64)).await;
}

pub async fn http_request(
    method: &str,
    path: &str,
    json_body: Option<String>,
    raw_body: Option<(Vec<u8>, &str)>,
) -> Result<(u16, String), ApiError> {
    let url = format!("{}{}", api_base(), path);
    let m = reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| ApiError {
        code: "invalid_method".into(),
        status: 0,
    })?;

    let mut req = http_client().request(m, &url);
    if let Some((bytes, content_type)) = raw_body {
        req = req.header("Content-Type", content_type).body(bytes);
    } else if let Some(body) = json_body {
        req = req.header("Content-Type", "application/json").body(body);
    }

    let resp = req.send().await.map_err(|_| ApiError {
        code: "network_error".into(),
        status: 0,
    })?;

    capture_session_cookie_from_headers(resp.headers());
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();

    if !(200..300).contains(&status) {
        let mut code = "request_failed".to_string();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(e) = v.get("error").and_then(|s| s.as_str()) {
                code = e.to_string();
            }
        return Err(ApiError { code, status });
    }

    Ok((status, text))
}

/// Native Google OAuth via backend ticket flow (`POST /api/auth/oauth-ticket`).
pub async fn start_google_oauth() -> Result<bool, String> {
    let (_, body) = http_request("POST", "/api/auth/oauth-ticket", None, None)
        .await
        .map_err(|e| e.code)?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|_| "invalid_ticket")?;
    let ticket = v
        .get("ticket")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "missing_ticket".to_string())?
        .to_string();

    let auth_url = format!("{}/api/auth/google?ticket={ticket}", api_base());
    open_external_url(&auth_url);

    // Poll up to 120 seconds
    for _ in 0..120 {
        sleep_ms(1000).await;
        if let Ok((_, resp_body)) =
            http_request("GET", &format!("/api/auth/oauth-ticket/{ticket}"), None, None).await
            && let Ok(check) = serde_json::from_str::<serde_json::Value>(&resp_body)
                && check.get("ready").and_then(|b| b.as_bool()).unwrap_or(false)
                    && let Some(tok) = check.get("token").and_then(|t| t.as_str()) {
                        set_session_cookie(tok);
                        return Ok(true);
                    }
    }
    Err("Hết thời gian chờ đăng nhập Google".into())
}

fn open_external_url(url: &str) {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = url;
    }
}

pub async fn run_ws_loop(tx: UnboundedSender<WsEvent>, stop: Arc<AtomicBool>) {
    let mut retry_delay_ms: f64 = 1000.0;

    while !stop.load(Ordering::Relaxed) {
        let base = api_base();
        let ws_url = if let Some(rest) = base.strip_prefix("https://") {
            format!("wss://{rest}/api/ws")
        } else {
            format!("ws://{}/api/ws", base.trim_start_matches("http://"))
        };

        let token = session_token_store()
            .lock()
            .ok()
            .and_then(|g| g.clone());

        let mut req = match ws_url.into_client_request() {
            Ok(r) => r,
            Err(_) => {
                sleep_ms(retry_delay_ms as u32).await;
                continue;
            }
        };

        if let Ok(origin_val) = base.parse() {
            req.headers_mut().insert("Origin", origin_val);
        }
        if let Some(tok) = token
            && let Ok(cookie_val) = format!("chatw_session={tok}").parse() {
                req.headers_mut().insert("Cookie", cookie_val);
            }

        if let Ok((ws_stream, _)) = tokio_tungstenite::connect_async(req).await {
            retry_delay_ms = 1000.0;
            let (_, mut read) = ws_stream.split();
            while !stop.load(Ordering::Relaxed) {
                tokio::select! {
                    msg = read.next() => {
                        match msg {
                            Some(Ok(tokio_tungstenite::tungstenite::Message::Text(txt))) => {
                                if let Ok(parsed) = serde_json::from_str::<WsEvent>(&txt) {
                                    let _ = tx.unbounded_send(parsed);
                                }
                            }
                            Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None | Some(Err(_)) => {
                                break;
                            }
                            _ => {}
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(250)) => {}
                }
            }
        }

        if stop.load(Ordering::Relaxed) {
            break;
        }
        sleep_ms(retry_delay_ms as u32).await;
        retry_delay_ms = (retry_delay_ms * 1.5).min(10000.0);
    }
}

/// Download raw bytes from an API path (used for encrypted attachments).
pub async fn fetch_bytes(path: &str) -> Result<Vec<u8>, String> {
    let _ = session_token_store();
    let url = if path.starts_with("http") { path.to_string() } else { format!("{}{}", api_base(), path) };
    let resp = http_client().get(&url).send().await.map_err(|_| "network_error".to_string())?;
    if !resp.status().is_success() {
        return Err(format!("http_{}", resp.status().as_u16()));
    }
    resp.bytes().await.map(|b| b.to_vec()).map_err(|_| "read_failed".to_string())
}
