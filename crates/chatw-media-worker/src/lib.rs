//! Dedicated Off-Main-Thread Release WASM Worker Engine (Spec 153 + Spec 164)
//! Compiles with `--release` (`opt-level = 3`) and runs inside `/media_worker.js`
//! so AVIF encoding, PDQ 256-bit hashing, BLAKE3, CWCMP1 compression,
//! **Zstandard Level 22 message chunk sealing/opening**, **Encrypted Vietnamese Full-Text Search**,
//! and **Dynamic Adaptive Storage Budget** NEVER block the Main UI Thread!

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use chatw_media_core::{
    avif::{
        AdaptiveAvifOutput, AvifCompressionMode, AvifEncodeOptions, encode_rgba_adaptive_avif,
        optimize_image_to_avif,
    },
    classifier::ImageClass,
    dedup::{compute_pdq_hash_rgba, hash_bytes_exact},
    files::{CompressionGoal, LosslessCodec, compress_file_smart},
};
use chatw_msg_store::{
    ChunkMetadata, CompactMessage, DICT_ID_PUBLIC_CHAT_V1, DeviceCapabilityProfile,
    DictionaryRegistry, LocalSearchIndex, ResourceSignals, ScopeKind, SealChunkRequest,
    SearchCoverageState, SearchQuery, StorageMode, Zstd22Options, compute_dynamic_storage_budget,
    derive_chunk_storage_key, derive_search_index_key, open_and_verify_message_chunk,
    seal_and_verify_message_chunk,
};
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

fn parse_mode(mode: &str) -> AvifCompressionMode {
    match mode {
        "high" => AvifCompressionMode::HighQuality,
        "balanced" | "fast" => AvifCompressionMode::FastAdaptive,
        "aggressive" | "data_saver" => AvifCompressionMode::UltraAggressive,
        _ => AvifCompressionMode::UltraAdaptive,
    }
}

fn build_avif_result(res: AdaptiveAvifOutput) -> Object {
    let out = Object::new();
    let bytes = Uint8Array::from(res.avif_bytes.as_slice());
    let is_crisp = matches!(
        res.analysis.class,
        ImageClass::CrispTextUi | ImageClass::FlatGraphic | ImageClass::AlphaGraphic
    );
    let super_res = res.width < res.target_width || res.height < res.target_height;
    let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
    let _ = Reflect::set(&out, &"bytes".into(), &bytes.into());
    let _ = Reflect::set(&out, &"width".into(), &JsValue::from(res.width));
    let _ = Reflect::set(&out, &"height".into(), &JsValue::from(res.height));
    let _ = Reflect::set(&out, &"targetWidth".into(), &JsValue::from(res.target_width));
    let _ = Reflect::set(&out, &"targetHeight".into(), &JsValue::from(res.target_height));
    let _ = Reflect::set(&out, &"superResRecommended".into(), &JsValue::from(super_res));
    let _ = Reflect::set(&out, &"isCrispText".into(), &JsValue::from(is_crisp));
    let _ = Reflect::set(
        &out,
        &"aiRecommended".into(),
        &JsValue::from(res.ai_restoration_recommended),
    );
    if let Some(ssim) = res.estimated_ssim {
        let _ = Reflect::set(&out, &"estimatedSsim".into(), &JsValue::from(ssim));
    }
    if let Some(ref prof) = res.orig_profile {
        if let Ok(js_prof) = serde_json::to_string(prof) {
            let _ = Reflect::set(&out, &"origProfileJson".into(), &JsValue::from(js_prof));
        }
    }
    out
}

fn error_obj(msg: &str) -> JsValue {
    let out = Object::new();
    let _ = Reflect::set(&out, &"ok".into(), &JsValue::FALSE);
    let _ = Reflect::set(&out, &"error".into(), &JsValue::from(msg));
    out.into()
}

#[wasm_bindgen(js_name = encodeRgbaAvifAdaptive)]
pub fn encode_rgba_avif_adaptive(
    rgba: &[u8],
    width: u32,
    height: u32,
    original_encoded_size: usize,
    raw_file_bytes: &[u8],
    mode: &str,
) -> JsValue {
    let opts = AvifEncodeOptions {
        mode: parse_mode(mode),
        max_edge_override: None,
        min_ssim_floor: None,
        generate_thumbnail: false,
        threads: Some(1),
    };
    let raw_opt = if raw_file_bytes.is_empty() {
        None
    } else {
        Some(raw_file_bytes)
    };
    match encode_rgba_adaptive_avif(rgba, width, height, original_encoded_size, raw_opt, &opts) {
        Ok(res) => build_avif_result(res).into(),
        Err(e) => error_obj(&e.to_string()),
    }
}

#[wasm_bindgen(js_name = encodeBytesAvifAdaptive)]
pub fn encode_bytes_avif_adaptive(raw_bytes: &[u8], mode: &str) -> JsValue {
    let opts = AvifEncodeOptions {
        mode: parse_mode(mode),
        max_edge_override: None,
        min_ssim_floor: None,
        generate_thumbnail: false,
        threads: Some(1),
    };
    match optimize_image_to_avif(raw_bytes, &opts) {
        Ok(res) => build_avif_result(res).into(),
        Err(e) => error_obj(&e.to_string()),
    }
}

#[wasm_bindgen(js_name = computeImagePdq)]
pub fn compute_image_pdq(rgba: &[u8], width: u32, height: u32) -> JsValue {
    let pdq = compute_pdq_hash_rgba(rgba, width, height);
    let out = Object::new();
    let _ = Reflect::set(&out, &"pdqHex".into(), &JsValue::from(pdq.to_hex()));
    let _ = Reflect::set(&out, &"quality".into(), &JsValue::from(pdq.quality));
    let _ = Reflect::set(
        &out,
        &"captionBandHash".into(),
        &JsValue::from(pdq.caption_band_hash.to_string()),
    );
    let _ = Reflect::set(
        &out,
        &"colorMomentHash".into(),
        &JsValue::from(pdq.color_moment_hash.to_string()),
    );
    let _ = Reflect::set(
        &out,
        &"aspectRatioMilli".into(),
        &JsValue::from(pdq.aspect_ratio_milli),
    );
    out.into()
}

#[wasm_bindgen(js_name = blake3Hex)]
pub fn blake3_hex(bytes: &[u8]) -> String {
    hash_bytes_exact(bytes).blake3_hex
}

#[wasm_bindgen(js_name = compressFileSmart)]
pub fn compress_file_smart_worker(bytes: &[u8]) -> JsValue {
    let out = Object::new();
    match compress_file_smart(bytes, CompressionGoal::Balanced) {
        Ok(res) => {
            let out_bytes = Uint8Array::from(res.payload.as_slice());
            let compressed = res.codec != LosslessCodec::None;
            let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
            let _ = Reflect::set(&out, &"compressed".into(), &JsValue::from(compressed));
            let _ = Reflect::set(&out, &"codec".into(), &JsValue::from(res.codec.name()));
            let _ = Reflect::set(&out, &"bytes".into(), &out_bytes.into());
            let _ = Reflect::set(
                &out,
                &"originalSize".into(),
                &JsValue::from(res.original_size as f64),
            );
            let _ = Reflect::set(
                &out,
                &"compressedSize".into(),
                &JsValue::from(res.final_size as f64),
            );
            out.into()
        }
        Err(e) => error_obj(&e.to_string()),
    }
}

// ── Spec 164: Off-Main-Thread Pure-Rust Zstandard Level 22 Message Chunks & Encrypted Search ────

#[derive(serde::Deserialize)]
struct JsSealMeta {
    chunk_id: String,
    account_id: String,
    scope_type: String,
    scope_id: String,
    #[serde(default = "default_u32_1")]
    chunk_version: u32,
    #[serde(default = "default_u32_1")]
    access_epoch: u32,
    #[serde(default = "default_u32_1")]
    key_version: u32,
    #[serde(default = "default_pub_dict")]
    candidate_dict_id: u32,
    now_ms: i64,
    #[serde(default = "default_true_b")]
    require_space_savings: bool,
    #[serde(default)]
    ram_budget_bytes: Option<u64>,
}
fn default_u32_1() -> u32 {
    1
}
fn default_pub_dict() -> u32 {
    DICT_ID_PUBLIC_CHAT_V1
}
fn default_true_b() -> bool {
    true
}

#[wasm_bindgen(js_name = sealMessageChunkZstd22)]
pub fn seal_message_chunk_zstd22(
    messages_json: &str,
    req_json: &str,
    root_key_b64: &str,
) -> JsValue {
    let msgs: Vec<CompactMessage> = match serde_json::from_str(messages_json) {
        Ok(v) => v,
        Err(e) => return error_obj(&format!("messages_json: {e}")),
    };
    let meta: JsSealMeta = match serde_json::from_str(req_json) {
        Ok(v) => v,
        Err(e) => return error_obj(&format!("req_json: {e}")),
    };
    let Some(scope_type) = ScopeKind::from_str_checked(&meta.scope_type) else {
        return error_obj("invalid_scope_type");
    };
    let root_key = match B64.decode(root_key_b64.trim()) {
        Ok(k) if k.len() >= 16 => k,
        _ => return error_obj("invalid_root_key_b64"),
    };
    let key = match derive_chunk_storage_key(
        &root_key,
        &meta.account_id,
        scope_type,
        &meta.scope_id,
        meta.key_version,
    ) {
        Ok(k) => k,
        Err(e) => return error_obj(&e.to_string()),
    };
    let registry = DictionaryRegistry::new();
    let req = SealChunkRequest {
        chunk_id: &meta.chunk_id,
        account_id: &meta.account_id,
        scope_type,
        scope_id: &meta.scope_id,
        chunk_version: meta.chunk_version,
        access_epoch: meta.access_epoch,
        key_version: meta.key_version,
        candidate_dict_id: meta.candidate_dict_id,
        now_ms: meta.now_ms,
        require_space_savings: meta.require_space_savings,
        zstd_opts: Zstd22Options {
            max_window_log: 22,
            ram_budget_bytes: meta.ram_budget_bytes,
            content_checksum: true,
        },
    };
    match seal_and_verify_message_chunk(&msgs, &req, &key, &registry) {
        Ok(sealed) => {
            let out = Object::new();
            let bytes = Uint8Array::from(sealed.sealed_bytes.as_slice());
            let meta_str = serde_json::to_string(&sealed.metadata).unwrap_or_default();
            let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
            let _ = Reflect::set(&out, &"metadataJson".into(), &JsValue::from(meta_str));
            let _ = Reflect::set(&out, &"bytes".into(), &bytes.into());
            out.into()
        }
        Err(e) => error_obj(&e.to_string()),
    }
}

#[wasm_bindgen(js_name = openMessageChunkZstd22)]
pub fn open_message_chunk_zstd22(
    meta_json: &str,
    sealed_bytes: &[u8],
    root_key_b64: &str,
) -> JsValue {
    let meta: ChunkMetadata = match serde_json::from_str(meta_json) {
        Ok(v) => v,
        Err(e) => return error_obj(&format!("meta_json: {e}")),
    };
    let root_key = match B64.decode(root_key_b64.trim()) {
        Ok(k) if k.len() >= 16 => k,
        _ => return error_obj("invalid_root_key_b64"),
    };
    let key = match derive_chunk_storage_key(
        &root_key,
        &meta.account_id,
        meta.scope_type,
        &meta.scope_id,
        meta.key_version,
    ) {
        Ok(k) => k,
        Err(e) => return error_obj(&e.to_string()),
    };
    let registry = DictionaryRegistry::new();
    match open_and_verify_message_chunk(&meta, sealed_bytes, &key, &registry) {
        Ok(msgs) => {
            let out = Object::new();
            let json = serde_json::to_string(&msgs).unwrap_or_else(|_| "[]".into());
            let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
            let _ = Reflect::set(&out, &"messagesJson".into(), &JsValue::from(json));
            out.into()
        }
        Err(e) => error_obj(&e.to_string()),
    }
}

#[wasm_bindgen(js_name = searchIndexUpsertBatch)]
pub fn search_index_upsert_batch(
    existing_sealed: &[u8],
    messages_json: &str,
    scope_type_str: &str,
    scope_id: &str,
    chunk_id: &str,
    root_key_b64: &str,
    account_id: &str,
    scope_key: &str,
    coverage_str: &str,
) -> JsValue {
    let Some(scope_type) = ScopeKind::from_str_checked(scope_type_str) else {
        return error_obj("invalid_scope_type");
    };
    let root_key = match B64.decode(root_key_b64.trim()) {
        Ok(k) if k.len() >= 16 => k,
        _ => return error_obj("invalid_root_key_b64"),
    };
    let key = match derive_search_index_key(&root_key, account_id, 1) {
        Ok(k) => k,
        Err(e) => return error_obj(&e.to_string()),
    };
    let msgs: Vec<CompactMessage> = match serde_json::from_str(messages_json) {
        Ok(v) => v,
        Err(e) => return error_obj(&format!("messages_json: {e}")),
    };
    let mut index = if existing_sealed.is_empty() {
        LocalSearchIndex::new()
    } else {
        match LocalSearchIndex::open_encrypted_shard(&key, account_id, scope_key, 1, existing_sealed)
        {
            Ok(i) => i,
            Err(_) => LocalSearchIndex::new(),
        }
    };
    let cid_opt = if chunk_id.is_empty() { None } else { Some(chunk_id) };
    for (i, m) in msgs.iter().enumerate() {
        index.upsert_message(scope_type, scope_id, None, cid_opt, i as u32, m);
    }
    let cov = match coverage_str {
        "indexed_complete" => SearchCoverageState::IndexedComplete,
        "syncing" => SearchCoverageState::Syncing,
        "not_downloaded" => SearchCoverageState::NotDownloaded,
        _ => SearchCoverageState::PartiallyIndexed,
    };
    index.set_scope_coverage(&format!("{}:{scope_id}", scope_type.as_str()), cov);

    match index.seal_encrypted_shard(&key, account_id, scope_key, 1) {
        Ok(sealed) => {
            let b3 = blake3::hash(&sealed).to_hex().to_string();
            let out = Object::new();
            let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
            let _ = Reflect::set(&out, &"bytes".into(), &Uint8Array::from(sealed.as_slice()).into());
            let _ = Reflect::set(&out, &"docCount".into(), &JsValue::from(index.doc_count() as u32));
            let _ = Reflect::set(&out, &"termCount".into(), &JsValue::from(index.term_count() as u32));
            let _ = Reflect::set(&out, &"indexVersion".into(), &JsValue::from(index.index_version as f64));
            let _ = Reflect::set(&out, &"blake3Hex".into(), &JsValue::from(b3));
            out.into()
        }
        Err(e) => error_obj(&e.to_string()),
    }
}

#[wasm_bindgen(js_name = searchIndexDeleteMessage)]
pub fn search_index_delete_message(
    existing_sealed: &[u8],
    message_id: &str,
    root_key_b64: &str,
    account_id: &str,
    scope_key: &str,
) -> JsValue {
    let root_key = match B64.decode(root_key_b64.trim()) {
        Ok(k) if k.len() >= 16 => k,
        _ => return error_obj("invalid_root_key_b64"),
    };
    let key = match derive_search_index_key(&root_key, account_id, 1) {
        Ok(k) => k,
        Err(e) => return error_obj(&e.to_string()),
    };
    let mut index = if existing_sealed.is_empty() {
        LocalSearchIndex::new()
    } else {
        match LocalSearchIndex::open_encrypted_shard(&key, account_id, scope_key, 1, existing_sealed)
        {
            Ok(i) => i,
            Err(e) => return error_obj(&e.to_string()),
        }
    };
    index.delete_message(message_id);
    match index.seal_encrypted_shard(&key, account_id, scope_key, 1) {
        Ok(sealed) => {
            let out = Object::new();
            let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
            let _ = Reflect::set(&out, &"bytes".into(), &Uint8Array::from(sealed.as_slice()).into());
            let _ = Reflect::set(&out, &"docCount".into(), &JsValue::from(index.doc_count() as u32));
            out.into()
        }
        Err(e) => error_obj(&e.to_string()),
    }
}

#[wasm_bindgen(js_name = searchIndexQuery)]
pub fn search_index_query(
    existing_sealed: &[u8],
    query_json: &str,
    root_key_b64: &str,
    account_id: &str,
    scope_key: &str,
) -> JsValue {
    if existing_sealed.is_empty() {
        let out = Object::new();
        let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
        let _ = Reflect::set(
            &out,
            &"responseJson".into(),
            &JsValue::from(r#"{"hits":[],"total_matched":0,"has_more":false,"coverage":"not_downloaded","unindexed_scopes":[],"from_result_cache":false}"#),
        );
        return out.into();
    }
    let root_key = match B64.decode(root_key_b64.trim()) {
        Ok(k) if k.len() >= 16 => k,
        _ => return error_obj("invalid_root_key_b64"),
    };
    let key = match derive_search_index_key(&root_key, account_id, 1) {
        Ok(k) => k,
        Err(e) => return error_obj(&e.to_string()),
    };
    let query: SearchQuery = match serde_json::from_str(query_json) {
        Ok(v) => v,
        Err(e) => return error_obj(&format!("query_json: {e}")),
    };
    let mut index = match LocalSearchIndex::open_encrypted_shard(
        &key,
        account_id,
        scope_key,
        1,
        existing_sealed,
    ) {
        Ok(i) => i,
        Err(e) => return error_obj(&e.to_string()),
    };
    match index.search(&query, None) {
        Ok(resp) => {
            let out = Object::new();
            let json = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());
            let _ = Reflect::set(&out, &"ok".into(), &JsValue::TRUE);
            let _ = Reflect::set(&out, &"responseJson".into(), &JsValue::from(json));
            out.into()
        }
        Err(e) => error_obj(&e.to_string()),
    }
}

#[wasm_bindgen(js_name = computeDynamicBudgetWasm)]
pub fn compute_dynamic_budget_wasm(
    profile_json: &str,
    mode_str: &str,
    custom_mb: f64,
    prev_budget: f64,
    signals_json: &str,
) -> JsValue {
    let profile: DeviceCapabilityProfile = serde_json::from_str(profile_json).unwrap_or_default();
    let signals: ResourceSignals = serde_json::from_str(signals_json).unwrap_or_default();
    let mode = StorageMode::parse_mode(mode_str);
    let custom = if custom_mb > 0.0 {
        Some(custom_mb as u64)
    } else {
        None
    };
    let prev = if prev_budget > 0.0 {
        Some(prev_budget as u64)
    } else {
        None
    };
    let b = compute_dynamic_storage_budget(&profile, mode, custom, prev, &signals);
    JsValue::from_str(&serde_json::to_string(&b).unwrap_or_else(|_| "{}".into()))
}
