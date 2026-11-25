//! `chatw-msg-store`: Shared Cross-Platform Rust Core for ChatW Spec 164.
//!
//! Unifies:
//! - Lossless Columnar Binary Message Serialization (`CWMC1`)
//! - Mandatory Zstandard Level 22 Compression (Native `libzstd` + Pure-Rust WASM `structured-zstd`)
//! - Versioned Public & E2EE Private Zstd Dictionaries
//! - AES-256-GCM AEAD Chunk Sealing with Domain-Separated HKDF Keys & CRIME/BREACH Padding
//! - Incremental >= 7-Day Chunking, Checkpoints, Backpressure, & Mutation/Tombstone Overlays
//! - Dynamic Adaptive Storage Budgeting (Hysteresis + Multi-Platform Tier Curve) & L1/L2 Smart Cache
//! - Encrypted Local Full-Text Search Index with Full Vietnamese Diacritic + Multilingual Support
//! - Unified 10-Priority Resource Scheduler & Privacy-Safe Telemetry

pub mod budget;
pub mod codec;
pub mod crypto;
pub mod dict;
pub mod incremental;
pub mod model;
pub mod scheduler;
pub mod search;
#[cfg(all(feature = "sqlite-sqlcipher", not(target_arch = "wasm32")))]
pub mod sqlite_store;
pub mod zstd22;

pub use budget::*;
pub use codec::*;
pub use crypto::*;
pub use dict::*;
pub use incremental::*;
pub use model::*;
pub use scheduler::*;
pub use search::*;
#[cfg(all(feature = "sqlite-sqlcipher", not(target_arch = "wasm32")))]
pub use sqlite_store::*;
pub use zstd22::*;

/// Complete end-to-end verified sealed message chunk output.
#[derive(Clone, Debug)]
pub struct SealedMessageChunk {
    pub metadata: ChunkMetadata,
    pub sealed_bytes: Vec<u8>,
}

/// Parameters for sealing an immutable message chunk (Section 17 & 25.1).
#[derive(Clone, Debug)]
pub struct SealChunkRequest<'a> {
    pub chunk_id: &'a str,
    pub account_id: &'a str,
    pub scope_type: ScopeKind,
    pub scope_id: &'a str,
    pub chunk_version: u32,
    pub access_epoch: u32,
    pub key_version: u32,
    pub candidate_dict_id: u32,
    pub now_ms: i64,
    /// If true, skips sealing when final encrypted chunk does not strictly save bytes vs raw JSON.
    pub require_space_savings: bool,
    pub zstd_opts: Zstd22Options,
}

/// Full Section 17 pipeline:
/// 1. Validate >= 7-day eligibility (optional caller enforcement + scope sanity).
/// 2. Serialize losslessly to `CWMC1` columnar binary.
/// 3. Compress with **Zstandard Level 22** (auto-selecting dictionary only if beneficial).
/// 4. Apply side-channel-resistant quantized padding (`CWPD`).
/// 5. Encrypt with AES-256-GCM + fresh CSPRNG nonce + complete metadata AAD (`CWEC01`).
/// 6. Verify space savings vs baseline JSON (unless `require_space_savings == false`).
/// 7. **Self-verify**: decrypt + unpad + decompress + deserialize + assert exact 100% equality
///    with original messages BEFORE returning!
pub fn seal_and_verify_message_chunk(
    messages: &[CompactMessage],
    req: &SealChunkRequest<'_>,
    key: &DerivedKey,
    dict_registry: &DictionaryRegistry,
) -> MsgStoreResult<SealedMessageChunk> {
    if messages.is_empty() {
        return Err(MsgStoreError::InvalidFormat("cannot seal empty chunk".into()));
    }
    if messages.len() > MAX_MESSAGES_PER_CHUNK {
        return Err(MsgStoreError::InvalidFormat("chunk exceeds max message count".into()));
    }

    // Measure original uncompressed JSON baseline (honest comparison including all metadata)
    let raw_json_baseline = serde_json::to_vec(messages)
        .map_err(|e| MsgStoreError::InvalidFormat(format!("baseline json: {e}")))?;
    let original_size = raw_json_baseline.len();

    // Step 1: Compact lossless columnar binary serialization
    let serialized = serialize_chunk_messages(messages)?;
    let plaintext_blake3_hex = blake3::hash(&serialized).to_hex().to_string();

    // Step 2: Mandatory Zstandard Level 22 (with dictionary only when demonstrably beneficial)
    let (compressed, chosen_dict_id) = compress_zstd22_best_dict(
        &serialized,
        req.candidate_dict_id,
        dict_registry,
        Some(req.zstd_opts),
    )?;

    let seq_start = messages.first().map(|m| m.seq).unwrap_or(0);
    let seq_end = messages.last().map(|m| m.seq).unwrap_or(0);
    let first_created_at_ms = messages.first().map(|m| m.created_at_ms).unwrap_or(req.now_ms);
    let last_created_at_ms = messages.last().map(|m| m.created_at_ms).unwrap_or(req.now_ms);

    let aad = ChunkAadContext {
        chunk_id: req.chunk_id.to_string(),
        account_id: req.account_id.to_string(),
        scope_type: req.scope_type,
        scope_id: req.scope_id.to_string(),
        chunk_version: req.chunk_version,
        access_epoch: req.access_epoch,
        key_version: req.key_version,
        seq_start,
        seq_end,
        dictionary_id: chosen_dict_id,
        plaintext_blake3_hex: plaintext_blake3_hex.clone(),
    };

    // Step 3 & 4: Side-channel bucket padding + AES-256-GCM AEAD encryption
    let (sealed_bytes, nonce_b64, padded_size) = seal_chunk_payload(key, &aad, &compressed)?;

    if req.require_space_savings && sealed_bytes.len() >= original_size {
        return Err(MsgStoreError::NoSpaceSavings {
            original: original_size,
            packed: sealed_bytes.len(),
        });
    }

    let ciphertext_blake3_hex = blake3::hash(&sealed_bytes).to_hex().to_string();

    let metadata = ChunkMetadata {
        chunk_id: req.chunk_id.to_string(),
        account_id: req.account_id.to_string(),
        scope_type: req.scope_type,
        scope_id: req.scope_id.to_string(),
        chunk_version: req.chunk_version,
        format_version: CHUNK_FORMAT_VERSION,
        access_epoch: req.access_epoch,
        compression_algo: "zstd".to_string(),
        compression_level: MANDATORY_ZSTD_LEVEL,
        dictionary_id: chosen_dict_id,
        original_size: original_size as u64,
        serialized_size: serialized.len() as u64,
        compressed_size: compressed.len() as u64,
        padded_size: padded_size as u64,
        encrypted_size: sealed_bytes.len() as u64,
        message_count: messages.len() as u32,
        seq_start,
        seq_end,
        first_created_at_ms,
        last_created_at_ms,
        created_at_ms: req.now_ms,
        updated_at_ms: req.now_ms,
        encryption_version: ENCRYPTION_VERSION_V1,
        key_version: req.key_version,
        nonce_b64,
        plaintext_blake3_hex,
        ciphertext_blake3_hex,
        storage_tier: StorageTier::Warm,
        storage_location: "local_cas".to_string(),
        sync_state: "verified_local".to_string(),
    };

    // Step 5: Mandatory pre-commit round-trip verification (Section 17 #9 & 25.1 #2)
    let verified = open_and_verify_message_chunk(&metadata, &sealed_bytes, key, dict_registry)?;
    if verified != messages {
        return Err(MsgStoreError::IntegrityMismatch {
            expected: "exact round-trip message sequence".into(),
            actual: "deserialized payload diverged from source".into(),
        });
    }

    Ok(SealedMessageChunk {
        metadata,
        sealed_bytes,
    })
}

/// Full verified open pipeline:
/// 1. Verify ciphertext BLAKE3 integrity BEFORE decryption.
/// 2. Verify AAD + AES-256-GCM AEAD authentication tag & decrypt.
/// 3. Unpad `CWPD` side-channel frame.
/// 4. Decompress with bounded bomb-safe Zstd decoder (+ verified dictionary if referenced).
/// 5. Verify plaintext BLAKE3 digest BEFORE deserializing.
/// 6. Deserialize `CWMC1` columnar binary into exact lossless `Vec<CompactMessage>`.
pub fn open_and_verify_message_chunk(
    meta: &ChunkMetadata,
    sealed_bytes: &[u8],
    key: &DerivedKey,
    dict_registry: &DictionaryRegistry,
) -> MsgStoreResult<Vec<CompactMessage>> {
    let actual_ct_hash = blake3::hash(sealed_bytes).to_hex().to_string();
    if actual_ct_hash != meta.ciphertext_blake3_hex {
        return Err(MsgStoreError::IntegrityMismatch {
            expected: meta.ciphertext_blake3_hex.clone(),
            actual: actual_ct_hash,
        });
    }

    let aad = ChunkAadContext {
        chunk_id: meta.chunk_id.clone(),
        account_id: meta.account_id.clone(),
        scope_type: meta.scope_type,
        scope_id: meta.scope_id.clone(),
        chunk_version: meta.chunk_version,
        access_epoch: meta.access_epoch,
        key_version: meta.key_version,
        seq_start: meta.seq_start,
        seq_end: meta.seq_end,
        dictionary_id: meta.dictionary_id,
        plaintext_blake3_hex: meta.plaintext_blake3_hex.clone(),
    };

    let compressed = open_chunk_payload(key, &aad, sealed_bytes)?;
    let dict_entry = dict_registry.get(meta.dictionary_id)?;
    let dict_bytes = dict_entry.map(|d| d.bytes.as_slice());

    let serialized = decompress_zstd_bounded(&compressed, dict_bytes, Some(DecompressBounds::default()))?;

    let actual_pt_hash = blake3::hash(&serialized).to_hex().to_string();
    if actual_pt_hash != meta.plaintext_blake3_hex {
        return Err(MsgStoreError::IntegrityMismatch {
            expected: meta.plaintext_blake3_hex.clone(),
            actual: actual_pt_hash,
        });
    }

    let messages = deserialize_chunk_messages(&serialized)?;
    if messages.len() as u32 != meta.message_count {
        return Err(MsgStoreError::IntegrityMismatch {
            expected: format!("{} messages", meta.message_count),
            actual: format!("{} messages", messages.len()),
        });
    }
    Ok(messages)
}
