//! Official ChatW Rust-to-WASM Plugin SDK (`wasm32-unknown-unknown`)
//!
//! Exports standard memory & length-prefixed JSON ABI understood by `ChatwPluginHost`:
//! - `chatw_alloc(len: usize) -> *mut u8`
//! - `chatw_invoke(ptr: *const u8, len: usize) -> *const u8` (writes `[u32_le_len][utf8_json]`)
//! And imports host functions from `"env"`:
//! - `chatw_host_log(ptr: *const u8, len: usize)`
//! - `chatw_host_emit(ptr: *const u8, len: usize)`

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn chatw_host_log(ptr: *const u8, len: usize);
    fn chatw_host_emit(ptr: *const u8, len: usize);
}

/// Sends a log / toast notification message to the ChatW host sandbox.
pub fn host_log(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        chatw_host_log(msg.as_ptr(), msg.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = msg;
}

/// Emits an RPC command (e.g. `ui.notify`, `storage.set`) to the ChatW host sandbox.
pub fn host_emit(method: &str, args: Value) {
    let payload = serde_json::json!({ "method": method, "args": args }).to_string();
    #[cfg(target_arch = "wasm32")]
    unsafe {
        chatw_host_emit(payload.as_ptr(), payload.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = payload;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginRequest {
    pub kind: String,
    #[serde(default)]
    pub trigger: Option<String>,
    #[serde(default)]
    pub args: Option<String>,
    #[serde(default)]
    pub action_id: Option<String>,
    #[serde(default)]
    pub event_name: Option<String>,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub context: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PluginResponse {
    pub fn ok_text(text: impl Into<String>) -> Self {
        Self { ok: true, text: Some(text.into()), data: None, error: None }
    }
    pub fn ok_data(text: impl Into<String>, data: Value) -> Self {
        Self { ok: true, text: Some(text.into()), data: Some(data), error: None }
    }
    pub fn err(msg: impl Into<String>) -> Self {
        Self { ok: false, text: None, data: None, error: Some(msg.into()) }
    }
}

/// Allocates `len` bytes in linear memory for host-to-WASM input writing.
#[no_mangle]
pub extern "C" fn chatw_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Encodes a JSON response into a length-prefixed `[u32_le_len][bytes]` buffer in WASM memory.
pub fn encode_response(resp: &PluginResponse) -> *const u8 {
    let json = serde_json::to_vec(resp).unwrap_or_else(|_| b"{\"ok\":false,\"error\":\"encode\"}".to_vec());
    let len_bytes = (json.len() as u32).to_le_bytes();
    let mut out = Vec::with_capacity(4 + json.len());
    out.extend_from_slice(&len_bytes);
    out.extend_from_slice(&json);
    let ptr = out.as_ptr();
    std::mem::forget(out);
    ptr
}

/// Helper macro to register a Rust handler function as the exported `chatw_invoke` entrypoint.
#[macro_export]
macro_rules! export_chatw_plugin {
    ($handler:path) => {
        #[no_mangle]
        pub extern "C" fn chatw_invoke(ptr: *const u8, len: usize) -> *const u8 {
            let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
            let req: $crate::PluginRequest = match serde_json::from_slice(slice) {
                Ok(r) => r,
                Err(_) => return $crate::encode_response(&$crate::PluginResponse::err("invalid_request_json")),
            };
            let resp: $crate::PluginResponse = $handler(req);
            $crate::encode_response(&resp)
        }
    };
}
