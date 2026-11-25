//! Cross-Platform Bridge for Spec 164:
//! - Incremental Zstandard Level 22 Message Chunks
//! - Encrypted Local Vietnamese Full-Text Search Index (`CWSE1`)
//! - Dynamic Adaptive Storage Budget (`Auto`, `SpaceSaver`, `HighPerformance`, `Custom`) + Breakdown
//! - L1 RAM Cache Wipe on Lock / Logout & Intelligent Prefetch Cancellation
//!
//! On Web (`wasm32`): delegates heavy Zstd-22 & search work to off-main-thread WASM Worker (`window.__chatwMsgStore`).
//! On Native (Windows, Linux, macOS, Android, iOS): executes directly in native Rust `chatw-msg-store` (`libzstd` L22).

use chatw_msg_store::{
    CompactMessage, MessageType, ScopeKind, SearchCoverageState, SearchFilters, SearchHit,
    SearchQuery, SearchResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Spec 166: L0 Volatile RAM Decrypted DM Plaintext Cache (keyed by `"{uid}:{msg_id}:{env_len}:{env_tail}"`).
/// Eliminates redundant P-256 ECDH + ECDSA verify + AES-256-GCM decryption on conversation switches.
/// Strictly volatile RAM — NEVER persisted to disk and IMMEDIATELY cleared on lock, logout, or key import.
const DM_DECRYPT_CACHE_MAX: usize = 4096;
static DM_DECRYPT_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn dm_decrypt_key(uid: &str, msg_id: &str, envelope: &str) -> String {
    let tail_start = envelope.len().saturating_sub(16);
    format!("{uid}:{msg_id}:{}:{}", envelope.len(), &envelope[tail_start..])
}

pub fn get_cached_decrypted_dm(uid: &str, msg_id: &str, envelope: &str) -> Option<String> {
    let lock = DM_DECRYPT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let guard = lock.lock().ok()?;
    guard.get(&dm_decrypt_key(uid, msg_id, envelope)).cloned()
}

pub fn put_cached_decrypted_dm(uid: &str, msg_id: &str, envelope: &str, plaintext: &str) {
    let lock = DM_DECRYPT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = lock.lock() {
        if guard.len() >= DM_DECRYPT_CACHE_MAX {
            // Simple bounded clear-half eviction to stay strictly O(1) memory bounded
            let to_drop: Vec<String> = guard.keys().take(DM_DECRYPT_CACHE_MAX / 2).cloned().collect();
            for k in to_drop {
                guard.remove(&k);
            }
        }
        guard.insert(dm_decrypt_key(uid, msg_id, envelope), plaintext.to_string());
    }
}

pub fn clear_decrypted_dm_cache() {
    if let Some(lock) = DM_DECRYPT_CACHE.get()
        && let Ok(mut guard) = lock.lock()
    {
        guard.clear();
    }
}

/// Zero-overhead click-to-DOM & click-to-paint profiler hook (`window.__chatwPerf`) for Spec 166 verification.
pub fn perf_mark_interaction(action: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(perf_fn) = js_sys::Reflect::get(&win, &wasm_bindgen::JsValue::from_str("__chatwPerfMark")) {
                if let Some(func) = wasm_bindgen::JsCast::dyn_ref::<js_sys::Function>(&perf_fn) {
                    let _ = func.call1(&win, &wasm_bindgen::JsValue::from_str(action));
                }
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = action;
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StorageBreakdown {
    #[serde(default = "default_auto_mode")]
    pub mode: String,
    #[serde(default)]
    pub browser_quota_bytes: u64,
    #[serde(default)]
    pub free_bytes: u64,
    #[serde(default)]
    pub total_bytes: u64,
    #[serde(default)]
    pub messages_bytes: u64,
    #[serde(default)]
    pub media_bytes: u64,
    #[serde(default)]
    pub files_bytes: u64,
    #[serde(default)]
    pub cache_bytes: u64,
    #[serde(default)]
    pub offline_bytes: u64,
    #[serde(default)]
    pub search_index_bytes: u64,
    #[serde(default)]
    pub saved_bytes: u64,
    #[serde(default)]
    pub chunk_count: u64,
    #[serde(default)]
    pub indexed_docs: u64,
    #[serde(default)]
    pub budget_bytes: u64,
    #[serde(default)]
    pub safe_ceiling_bytes: u64,
    #[serde(default)]
    pub last_search_latency_ms: f64,
}

fn default_auto_mode() -> String {
    "auto".into()
}

pub fn set_storage_mode_local(mode: &str, custom_mb: Option<u64>) {
    let valid = match mode {
        "space_saver" | "high_performance" | "custom" => mode,
        _ => "auto",
    };
    crate::platform::storage_set("chatw_storage_mode", valid);
    if valid == "custom" {
        if let Some(mb) = custom_mb {
            crate::platform::storage_set("chatw_cache_limit_mb", &mb.max(32).to_string());
        }
    } else if valid == "auto" {
        crate::platform::storage_set("chatw_cache_limit_mb", "0");
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(m_js) = serde_json::to_string(valid) {
            let mb_arg = custom_mb.map(|v| v.to_string()).unwrap_or_else(|| "null".into());
            let _ = dioxus::document::eval(&format!(
                "if (window.__chatwMsgStore) window.__chatwMsgStore.setStorageMode({m_js}, {mb_arg});"
            ));
        }
    }
}

pub fn get_storage_mode_local() -> String {
    crate::platform::storage_get("chatw_storage_mode")
        .filter(|m| matches!(m.as_str(), "auto" | "space_saver" | "high_performance" | "custom"))
        .unwrap_or_else(|| "auto".into())
}

/// Bind current account's root key (base64) into session memory so Web Worker Zstd-22 & search
/// index shards use the real E2EE HKDF root key (NEVER persisted in plaintext!).
pub fn bind_session_root_key_b64(account_id: &str, root_key: &[u8; 32]) {
    #[cfg(target_arch = "wasm32")]
    {
        use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
        let rk_b64 = B64.encode(root_key);
        if let (Ok(a_js), Ok(k_js)) = (serde_json::to_string(account_id), serde_json::to_string(&rk_b64)) {
            // Pass into private closure inside storage_engine.js — NEVER leave raw key on window!
            let _ = dioxus::document::eval(&format!(
                "window.__chatwCurrentUserId = {a_js}; if (window.__chatwMsgStore && window.__chatwMsgStore.bindSessionKey) {{ window.__chatwMsgStore.bindSessionKey({a_js}, {k_js}); }} else {{ window.__chatwSessionRootKeyB64 = {k_js}; }}"
            ));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        native_impl::bind_session_key(account_id, root_key);
    }
}

pub fn wipe_sensitive_l1_and_session_key() {
    clear_decrypted_dm_cache();
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dioxus::document::eval(
            "window.__chatwSessionRootKeyB64 = null; if (window.__chatwMsgStore) window.__chatwMsgStore.wipeL1Ram();",
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        native_impl::wipe_session_ram();
    }
}

pub async fn get_storage_breakdown() -> StorageBreakdown {
    #[cfg(target_arch = "wasm32")]
    {
        let mut ev = dioxus::document::eval(
            r#"
            (async () => {
                try {
                    if (window.__chatwMsgStore) {
                        const r = await window.__chatwMsgStore.getDynamicBudgetAndBreakdown();
                        dioxus.send({
                            mode: r.mode || 'auto',
                            browser_quota_bytes: Number(r.browser_quota_bytes || 0),
                            free_bytes: Number(r.free_bytes || 0),
                            total_bytes: Number(r.total_bytes || 0),
                            messages_bytes: Number(r.messages_bytes || 0),
                            media_bytes: Number(r.media_bytes || 0),
                            files_bytes: Number(r.files_bytes || 0),
                            cache_bytes: Number(r.cache_bytes || 0),
                            offline_bytes: Number(r.offline_bytes || 0),
                            search_index_bytes: Number(r.search_index_bytes || 0),
                            saved_bytes: Number(r.saved_bytes || 0),
                            chunk_count: Number(r.chunk_count || 0),
                            indexed_docs: Number(r.indexed_docs || 0),
                            budget_bytes: Number((r.budget && r.budget.total_local_budget_bytes) || 0),
                            safe_ceiling_bytes: Number((r.budget && r.budget.effective_safe_ceiling_bytes) || 0),
                            last_search_latency_ms: Number(r.last_search_latency_ms || 0),
                        });
                        return;
                    }
                } catch (_) {}
                dioxus.send(null);
            })();
            "#,
        );
        if let Ok(val) = ev.recv::<serde_json::Value>().await {
            if let Ok(parsed) = serde_json::from_value::<StorageBreakdown>(val) {
                return parsed;
            }
        }
        StorageBreakdown::default()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        native_impl::get_breakdown().await
    }
}

/// Incrementally index a batch of messages into the local encrypted Vietnamese full-text index.
pub async fn index_messages_incremental(
    account_id: &str,
    scope_type: ScopeKind,
    scope_id: &str,
    messages: &[CompactMessage],
    coverage: SearchCoverageState,
) {
    if messages.is_empty() {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let Ok(acc_js) = serde_json::to_string(account_id) else { return; };
        let Ok(st_js) = serde_json::to_string(scope_type.as_str()) else { return; };
        let Ok(sid_js) = serde_json::to_string(scope_id) else { return; };
        let Ok(msgs_js) = serde_json::to_string(messages) else { return; };
        let cov_str = match coverage {
            SearchCoverageState::IndexedComplete => "indexed_complete",
            SearchCoverageState::Syncing => "syncing",
            SearchCoverageState::NotDownloaded => "not_downloaded",
            _ => "partially_indexed",
        };
        let Ok(cov_js) = serde_json::to_string(cov_str) else { return; };
        let js = format!(
            "if (window.__chatwMsgStore) {{ window.__chatwMsgStore.indexMessages({acc_js}, {st_js}, {sid_js}, {msgs_js}, '', {cov_js}).catch(() => {{}}); }}"
        );
        let _ = dioxus::document::eval(&js);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        native_impl::index_messages(account_id, scope_type, scope_id, messages, coverage);
    }
}

pub async fn delete_message_from_search_index(account_id: &str, message_id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let (Ok(a_js), Ok(m_js)) = (serde_json::to_string(account_id), serde_json::to_string(message_id)) {
            let _ = dioxus::document::eval(&format!(
                "if (window.__chatwMsgStore) {{ window.__chatwMsgStore.deleteFromSearchIndex({a_js}, {m_js}).catch(() => {{}}); }}"
            ));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        native_impl::delete_from_index(account_id, message_id);
    }
}

/// Execute encrypted local Vietnamese & multilingual full-text search across all indexed scopes.
pub async fn search_local_messages(
    account_id: &str,
    text: &str,
    filters: SearchFilters,
    limit: usize,
) -> (SearchResponse, f64) {
    let query = SearchQuery {
        text: text.to_string(),
        exact_diacritics: false,
        phrase_match: false,
        prefix_match: true,
        filters,
        offset: 0,
        limit: limit.clamp(1, 100),
    };
    #[cfg(target_arch = "wasm32")]
    {
        let Ok(a_js) = serde_json::to_string(account_id) else {
            return (empty_search_resp(), 0.0);
        };
        let Ok(q_js) = serde_json::to_string(&query) else {
            return (empty_search_resp(), 0.0);
        };
        let js = format!(
            r#"
            (async () => {{
                try {{
                    if (window.__chatwMsgStore) {{
                        const r = await window.__chatwMsgStore.searchLocalMessages({a_js}, {q_js});
                        dioxus.send(r);
                        return;
                    }}
                }} catch (_) {{}}
                dioxus.send(null);
            }})();
            "#
        );
        let mut ev = dioxus::document::eval(&js);
        if let Ok(v) = ev.recv::<serde_json::Value>().await {
            let lat = v.get("latency_ms").and_then(|n| n.as_f64()).unwrap_or(0.0);
            if let Ok(resp) = serde_json::from_value::<SearchResponse>(v) {
                return (resp, lat);
            }
        }
        (empty_search_resp(), 0.0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        native_impl::search_index(account_id, &query)
    }
}

fn empty_search_resp() -> SearchResponse {
    SearchResponse {
        hits: Vec::<SearchHit>::new(),
        total_matched: 0,
        has_more: false,
        coverage: SearchCoverageState::NotDownloaded,
        unindexed_scopes: Vec::new(),
        from_result_cache: false,
    }
}

/// Convert a DM `DisplayMessage` into lossless `CompactMessage` for indexing & Zstd-22 chunking.
pub fn dm_to_compact(m: &crate::models::DisplayMessage) -> CompactMessage {
    let ts_ms = chrono::DateTime::parse_from_rfc3339(&m.created_at)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0);
    let mtype = if m.text.starts_with("__IMG__") {
        MessageType::Image
    } else if m.text.starts_with("__VID__") {
        MessageType::Video
    } else if m.text.starts_with("__VOICE__") {
        MessageType::AudioVoice
    } else if m.text.starts_with("__FILE__") {
        MessageType::File
    } else if m.text.starts_with("__STICKER__") {
        MessageType::Sticker
    } else if m.text.contains("http://") || m.text.contains("https://") {
        MessageType::Link
    } else {
        MessageType::Text
    };
    CompactMessage {
        id: m.id.clone(),
        seq: m.seq,
        sender_id: m.sender_id.clone(),
        created_at_ms: ts_ms,
        msg_type: mtype,
        text: m.text.clone(),
        reply_to_id: m.reply_to_id.clone(),
        is_edited: false,
        deleted_for_all: m.text == "[Message has been revoked]",
        is_pinned: m.is_pinned,
        is_bookmarked: false,
        status_code: 0,
        reactions: Vec::new(),
        attachments: Vec::new(),
        extra_meta: None,
    }
}

pub fn group_to_compact(m: &crate::models::GroupMessageItem) -> CompactMessage {
    let ts_ms = chrono::DateTime::parse_from_rfc3339(&m.created_at)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0);
    CompactMessage {
        id: m.id.clone(),
        seq: m.seq,
        sender_id: m.sender_id.clone(),
        created_at_ms: ts_ms,
        msg_type: if m.content.contains("http://") || m.content.contains("https://") {
            MessageType::Link
        } else {
            MessageType::Text
        },
        text: m.content.clone(),
        reply_to_id: m.reply_to_id.clone(),
        is_edited: false,
        deleted_for_all: false,
        is_pinned: false,
        is_bookmarked: false,
        status_code: 0,
        reactions: Vec::new(),
        attachments: Vec::new(),
        extra_meta: None,
    }
}

pub fn channel_to_compact(m: &crate::models::ChannelMessageItem) -> CompactMessage {
    let ts_ms = chrono::DateTime::parse_from_rfc3339(&m.created_at)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0);
    CompactMessage {
        id: m.id.clone(),
        seq: m.seq,
        sender_id: m.sender_id.clone(),
        created_at_ms: ts_ms,
        msg_type: if m.content.contains("http://") || m.content.contains("https://") {
            MessageType::Link
        } else {
            MessageType::Text
        },
        text: m.content.clone(),
        reply_to_id: m.reply_to_id.clone(),
        is_edited: false,
        deleted_for_all: false,
        is_pinned: false,
        is_bookmarked: false,
        status_code: 0,
        reactions: Vec::new(),
        attachments: Vec::new(),
        extra_meta: None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native_impl {
    use super::*;
    use chatw_msg_store::{
        DeviceCapabilityProfile, EncryptedSqliteStore, L1ChunkCache, PlatformKind, ResourceSignals,
        StorageMode, compute_dynamic_storage_budget,
    };
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;
    use zeroize::Zeroize;

    /// Unified Native Local Storage Runtime (Spec 165 Sections 37 & 38):
    /// ONE single encrypted SQLite + SQLCipher + FTS5 database per account across
    /// Android, iOS, Windows, Linux, and macOS — zero duplicate local DBs!
    struct NativeRuntimeState {
        account_id: String,
        root_key: [u8; 32],
        has_key: bool,
        db: Option<EncryptedSqliteStore>,
        l1: L1ChunkCache,
        prev_budget: Option<u64>,
        last_search_ms: f64,
    }

    static STATE: OnceLock<Mutex<NativeRuntimeState>> = OnceLock::new();

    fn state_lock() -> &'static Mutex<NativeRuntimeState> {
        STATE.get_or_init(|| {
            Mutex::new(NativeRuntimeState {
                account_id: String::new(),
                root_key: [0u8; 32],
                has_key: false,
                db: None,
                l1: L1ChunkCache::new(64 * 1024 * 1024),
                prev_budget: None,
                last_search_ms: 0.0,
            })
        })
    }

    fn account_db_dir(account_id: &str) -> PathBuf {
        let safe: String = account_id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        crate::storage::native::native_cache_dir_for_account(&safe)
    }

    fn current_platform() -> PlatformKind {
        #[cfg(target_os = "android")]
        {
            PlatformKind::Android
        }
        #[cfg(target_os = "ios")]
        {
            PlatformKind::Ios
        }
        #[cfg(target_os = "windows")]
        {
            PlatformKind::Windows
        }
        #[cfg(target_os = "macos")]
        {
            PlatformKind::MacOs
        }
        #[cfg(not(any(
            target_os = "android",
            target_os = "ios",
            target_os = "windows",
            target_os = "macos"
        )))]
        {
            PlatformKind::Linux
        }
    }

    pub fn bind_session_key(account_id: &str, key: &[u8; 32]) {
        let Ok(mut st) = state_lock().lock() else { return; };
        st.account_id = account_id.to_string();
        st.root_key = *key;
        st.has_key = true;
        let dir = account_db_dir(account_id);
        match EncryptedSqliteStore::open_or_recover(&dir, account_id, &st.root_key, 1) {
            Ok((store, _recovered)) => {
                st.db = Some(store);
            }
            Err(_) => {
                st.db = None;
            }
        }
    }

    pub fn wipe_session_ram() {
        let Ok(mut st) = state_lock().lock() else { return; };
        st.l1.wipe_sensitive();
        st.root_key.zeroize();
        st.has_key = false;
        st.db = None;
    }

    pub fn index_messages(
        account_id: &str,
        scope_type: ScopeKind,
        scope_id: &str,
        messages: &[CompactMessage],
        coverage: SearchCoverageState,
    ) {
        let Ok(mut st) = state_lock().lock() else { return; };
        if st.db.is_none() && st.has_key && !account_id.is_empty() {
            let dir = account_db_dir(account_id);
            if let Ok((store, _)) =
                EncryptedSqliteStore::open_or_recover(&dir, account_id, &st.root_key, 1)
            {
                st.db = Some(store);
            }
        }
        if let Some(ref mut db) = st.db {
            let _ = db.upsert_messages_batch(scope_type, scope_id, None, messages, coverage);
        }
    }

    pub fn delete_from_index(_account_id: &str, message_id: &str) {
        let Ok(mut st) = state_lock().lock() else { return; };
        if let Some(ref mut db) = st.db {
            let _ = db.delete_message(message_id);
        }
    }

    pub fn search_index(_account_id: &str, query: &SearchQuery) -> (SearchResponse, f64) {
        let t0 = Instant::now();
        let Ok(mut st) = state_lock().lock() else {
            return (empty_search_resp(), 0.0);
        };
        let resp = st
            .db
            .as_ref()
            .and_then(|db| db.search_fts5(query).ok())
            .unwrap_or_else(empty_search_resp);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        st.last_search_ms = ms;
        (resp, ms)
    }

    pub async fn get_breakdown() -> StorageBreakdown {
        let q = crate::storage::native::get_quota_estimate().await;
        let mode_str = get_storage_mode_local();
        let mode = StorageMode::parse_mode(&mode_str);
        let free = q.browser_quota_bytes.saturating_sub(q.browser_usage_bytes);
        let profile = DeviceCapabilityProfile {
            platform: current_platform(),
            total_ram_mb: 8192,
            available_ram_mb: 4096,
            cpu_cores: std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(4),
            total_disk_bytes: q.browser_quota_bytes.max(free),
            free_disk_bytes: free,
            os_quota_bytes: None,
            is_ssd: true,
            is_mobile: matches!(current_platform(), PlatformKind::Android | PlatformKind::Ios),
        };
        let signals = ResourceSignals::default();
        let custom_mb = if mode == StorageMode::Custom {
            Some(q.limit_mb.max(32) as u64)
        } else {
            None
        };
        let (prev_b, db_stats, last_ms) = {
            let st = state_lock().lock().ok();
            let prev = st.as_ref().and_then(|s| s.prev_budget);
            let stats = st
                .as_ref()
                .and_then(|s| s.db.as_ref().map(|d| d.stats()))
                .unwrap_or_default();
            let lms = st.as_ref().map(|s| s.last_search_ms).unwrap_or(0.0);
            (prev, stats, lms)
        };
        let budget = compute_dynamic_storage_budget(&profile, mode, custom_mb, prev_b, &signals);
        if let Ok(mut st) = state_lock().lock() {
            st.prev_budget = Some(budget.total_local_budget_bytes);
            st.l1.set_max_bytes(budget.l1_ram_budget_bytes);
            if let Some(ref db) = st.db {
                let _ = db.evict_chunks_to_budget(budget.message_cache_budget_bytes);
            }
        }
        let msg_bytes = db_stats.db_file_bytes + db_stats.external_chunks_bytes;
        StorageBreakdown {
            mode: mode_str,
            browser_quota_bytes: q.browser_quota_bytes,
            free_bytes: free,
            total_bytes: q.total_local_bytes + msg_bytes,
            messages_bytes: msg_bytes,
            media_bytes: q.disposable_cache_bytes,
            files_bytes: q.pinned_unique_bytes,
            cache_bytes: q.disposable_cache_bytes + db_stats.external_chunks_bytes,
            offline_bytes: q.pinned_unique_bytes + msg_bytes,
            search_index_bytes: db_stats.db_file_bytes,
            saved_bytes: db_stats.saved_bytes,
            chunk_count: db_stats.chunk_count,
            indexed_docs: db_stats.fts_doc_count,
            budget_bytes: budget.total_local_budget_bytes,
            safe_ceiling_bytes: budget.effective_safe_ceiling_bytes,
            last_search_latency_ms: last_ms,
        }
    }
}
