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

pub async fn stream_export_download(
    path: &str,
    json_body: &str,
    default_filename: &str,
) -> Result<super::ExportDownloadResult, ApiError> {
    let url = format!("{}{}", api_base(), path);
    let resp = http_client()
        .post(&url)
        .header("Content-Type", "application/json")
        .body(json_body.to_string())
        .send()
        .await
        .map_err(|_| ApiError {
            code: "network_error".into(),
            status: 0,
        })?;
    capture_session_cookie_from_headers(resp.headers());
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        let text = resp.text().await.unwrap_or_default();
        let mut code = "request_failed".to_string();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(e) = v.get("error").and_then(|s| s.as_str())
        {
            code = e.to_string();
        }
        return Err(ApiError { code, status });
    }
    let job_id = resp
        .headers()
        .get("x-backup-job-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let filename = resp
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(|disp| {
            let idx = disp.find("filename=")?;
            let rest = disp[idx + 9..].trim().trim_matches('"');
            let clean = rest.split(';').next()?.trim().trim_matches('"');
            if clean.is_empty() { None } else { Some(clean.to_string()) }
        })
        .unwrap_or_else(|| default_filename.to_string());

    let bytes = resp.bytes().await.map_err(|_| ApiError {
        code: "read_failed".into(),
        status,
    })?;

    let dl_dir = std::env::var("XDG_DOWNLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join("Downloads"))
                .unwrap_or_else(|_| std::env::temp_dir())
        });
    let _ = std::fs::create_dir_all(&dl_dir);
    let safe_name = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(default_filename);
    let target = dl_dir.join(safe_name);
    let _ = std::fs::write(&target, &bytes);

    Ok(super::ExportDownloadResult {
        filename: target.display().to_string(),
        byte_size: bytes.len() as u64,
        job_id,
    })
}

pub async fn selected_file_meta(input_id: &str) -> Option<super::SelectedFileMeta> {
    let id_json = serde_json::to_string(input_id).unwrap_or_default();
    let js = format!(
        r#"
        (() => {{
            const input = document.getElementById({id_json});
            if (!input || !input.files || input.files.length === 0) {{ dioxus.send(null); return; }}
            const f = input.files[0];
            dioxus.send({{ name: f.name, byte_size: f.size }});
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
    let js = format!(
        r#"
        (async () => {{
            const input = document.getElementById({id_json});
            if (!input || !input.files || input.files.length === 0) {{ dioxus.send({{ error: 'no_file' }}); return; }}
            const file = input.files[0];
            if (file.size > 128 * 1024 * 1024) {{ dioxus.send({{ error: 'backup_too_large' }}); return; }}
            const buf = new Uint8Array(await file.arrayBuffer());
            let s = '';
            for (let i = 0; i < buf.length; i += 0x8000) s += String.fromCharCode.apply(null, buf.subarray(i, i + 0x8000));
            dioxus.send({{ b64: btoa(s), name: file.name, byte_size: file.size }});
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
    use base64::Engine;
    let b64 = val.get("b64").and_then(|v| v.as_str()).ok_or(ApiError {
        code: "no_data".into(),
        status: 400,
    })?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| ApiError {
            code: "decode_failed".into(),
            status: 400,
        })?;
    let filename = val
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("backup")
        .to_string();
    let byte_size = val
        .get("byte_size")
        .and_then(|v| v.as_u64())
        .unwrap_or(bytes.len() as u64);

    let url = format!("{}{}", api_base(), endpoint_path);
    let mut req = http_client()
        .post(&url)
        .header("Content-Type", "application/octet-stream")
        .body(bytes);
    if let Some(pw) = backup_password
        && !pw.is_empty()
    {
        req = req.header("x-backup-password", pw);
    }
    let resp = req.send().await.map_err(|_| ApiError {
        code: "network_error".into(),
        status: 0,
    })?;
    capture_session_cookie_from_headers(resp.headers());
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        let mut code = "request_failed".to_string();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body)
            && let Some(e) = v.get("error").and_then(|s| s.as_str())
        {
            code = e.to_string();
        }
        return Err(ApiError { code, status });
    }
    Ok(super::UploadBackupResult {
        status,
        body,
        filename,
        byte_size,
    })
}

pub async fn register_device_passkey_local() -> Result<super::DevicePasskeyCreated, ApiError> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use p256::ecdsa::SigningKey;
    use p256::elliptic_curve::rand_core::{OsRng, RngCore};

    let mut sk_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut sk_bytes);
    let sk = SigningKey::from_slice(&sk_bytes).map_err(|_| ApiError {
        code: "passkey_keygen_failed".into(),
        status: 400,
    })?;
    let mut cred_raw = [0u8; 16];
    OsRng.fill_bytes(&mut cred_raw);
    let credential_id = URL_SAFE_NO_PAD.encode(cred_raw);
    let vk_bytes = sk.verifying_key().to_encoded_point(false).as_bytes().to_vec();
    let public_key_spki_b64 = URL_SAFE_NO_PAD.encode(&vk_bytes);

    let rec = serde_json::json!({
        "credential_id": credential_id,
        "sk_b64": URL_SAFE_NO_PAD.encode(sk.to_bytes()),
    });
    storage_set("chatw_device_passkey_v1", &rec.to_string());

    Ok(super::DevicePasskeyCreated {
        credential_id,
        public_key_spki_b64,
    })
}

pub async fn sign_passkey_challenge_local(
    challenge_b64: &str,
) -> Result<super::DevicePasskeySignature, ApiError> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use p256::ecdsa::{Signature, SigningKey, signature::Signer};

    let raw = storage_get("chatw_device_passkey_v1").ok_or(ApiError {
        code: "no_local_passkey".into(),
        status: 400,
    })?;
    let val: serde_json::Value = serde_json::from_str(&raw).map_err(|_| ApiError {
        code: "no_local_passkey".into(),
        status: 400,
    })?;
    let cred_id = val
        .get("credential_id")
        .and_then(|v| v.as_str())
        .ok_or(ApiError {
            code: "no_local_passkey".into(),
            status: 400,
        })?
        .to_string();
    let sk_b64 = val
        .get("sk_b64")
        .and_then(|v| v.as_str())
        .ok_or(ApiError {
            code: "no_local_passkey".into(),
            status: 400,
        })?;
    let sk_bytes = URL_SAFE_NO_PAD.decode(sk_b64).map_err(|_| ApiError {
        code: "no_local_passkey".into(),
        status: 400,
    })?;
    let sk = SigningKey::from_slice(&sk_bytes).map_err(|_| ApiError {
        code: "passkey_sign_failed".into(),
        status: 400,
    })?;
    let sig: Signature = sk.sign(challenge_b64.as_bytes());
    Ok(super::DevicePasskeySignature {
        credential_id: cred_id,
        signature_b64: URL_SAFE_NO_PAD.encode(sig.to_bytes()),
    })
}
