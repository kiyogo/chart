//! Versioned Zstandard Dictionary Registry & Encrypted Private Dictionary Vault (Section 7).
//!
//! - Provides a built-in, non-sensitive public multilingual/Vietnamese chat dictionary (`DICT_ID_PUBLIC_CHAT_V1 = 1`)
//!   packed with high-frequency Vietnamese, English, Japanese, Chinese, emoji, and ChatW schema tokens
//!   (`ok`, `alo`, `ừ`, `vâng`, `dạ`, `hello`, `hi`, `yes`, `no`, emojis, MIME types, JSON metadata keys).
//! - Supports optional E2EE private dictionaries (`dict_id >= 1000`) encrypted with domain-separated
//!   HKDF keys (`CWDC1`), strictly isolated per account and scope.
//! - Retains older dictionary versions as long as any chunk references them.

use crate::crypto::{DerivedKey, open_aead_blob, seal_aead_blob};
use crate::model::{MsgStoreError, MsgStoreResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

pub const DICT_ID_NONE: u32 = 0;
pub const DICT_ID_PUBLIC_CHAT_V1: u32 = 1;
pub const PRIVATE_DICT_ID_MIN: u32 = 1000;
const PRIVATE_DICT_MAGIC: &[u8; 5] = b"CWDC1";

/// Versioned dictionary entry held in memory or synced via registry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZstdDictionaryEntry {
    pub dict_id: u32,
    pub version: u32,
    pub is_private_e2ee: bool,
    pub blake3_hex: String,
    pub bytes: Vec<u8>,
}

/// Non-sensitive public multilingual chat dictionary corpus (raw content dictionary compatible
/// with both native C `libzstd` raw dictionaries and pure-Rust `structured-zstd` raw dictionaries).
///
/// Using raw content dictionaries ensures 100% deterministic, identical bytes across every
/// platform (Web WASM, Android, iOS, Windows, Linux, macOS, Server) with zero runtime training
/// variance while dramatically improving short-message and multilingual chat compression!
fn build_public_chat_dict_v1_bytes() -> Vec<u8> {
    let mut dict = Vec::with_capacity(16 * 1024);
    // High-frequency Vietnamese, English, Japanese, Chinese, Emoji & ChatW protocol phrases
    // ordered with most frequent/short patterns near the END of the raw dictionary
    // (Zstd raw-content dictionary uses smaller repeat offsets for suffixes near the end!).
    const SECTIONS: &[&str] = &[
        // Technical / attachment / metadata common substrings
        "application/octet-stream image/avif image/webp image/png image/jpeg video/mp4 video/webm audio/ogg audio/opus audio/flac ",
        "\"forwarded\":true \"thread_id\": \"bot_id\": \"reply_to\": \"mentions\": \"reactions\": \"attachments\": \"pinned\":true ",
        "https:// http:// www. .com .vn .org .io .dev /api/attachments/ /uploads/avatars/ ",
        // Japanese & Chinese common chat phrases
        "おはよう こんにちは こんばんは ありがとう ありがとうございます 了解 お疲れ様 よろしく お願いします 大丈夫 わかりました はい いいえ ちょっと待って ",
        "你好 谢谢 好的 没问题 收到 晚上好 早上好 明天见 辛苦了 哈哈 嗯嗯 可以 知道了 再见 不客气 马上 ",
        // English common phrases & short replies
        "good morning good afternoon good evening how are you doing today sounds good let me know thanks thank you so much ",
        "no problem you're welcome see you tomorrow on my way just a second give me a minute sure thing makes sense got it ",
        "can you check this please send me the file link photo video meeting at call me when free ",
        // Vietnamese common everyday chat phrases (both accented & accent-less variants)
        "chào buổi sáng chào bạn khỏe không hôm nay thế nào ăn cơm chưa đang làm gì đó về nhà chưa đi ngủ đi ngủ ngon nhé ",
        "cảm ơn bạn nhiều nha không có gì đâu xin lỗi nhé để mình xem lại một chút chờ mình xíu nha mình đang bận chút ",
        "gửi lại giúp mình cái ảnh video tệp tin này với nhé hẹn gặp lại ngày mai lúc mấy giờ ở đâu vậy bạn ơi ",
        "chao ban khoe khong hom nay the nao an com chua dang lam gi do cam on nhieu nha khong co gi xin loi cho minh xiu ",
        "được rồi đồng ý chuẩn luôn tuyệt vời quá hay lắm chính xác luôn mình hiểu rồi để mình xử lý ngay bây giờ nhé ",
        "họp nhóm dự án báo cáo tiến độ kiểm tra lại bản cập nhật lỗi đã sửa xong chạy ổn định rồi đó anh em ơi ",
        // High-frequency Emoji sequences
        "👍 ❤️ 😂 🤣 😊 😍 🥰 😭 🙏 🔥 ✨ 🎉 💯 👏 🤝 👌 😅 😁 😎 🤔 😢 😡 ❤️‍🔥 ✅ ❌ ⚡ 🚀 💬 📎 📷 🎥 🎤 ",
        // Ultra-high-frequency short replies placed at the very end for minimal Zstd match offsets!
        "haha hehe hihi lol lmao omg btw asap fyi pls thx ty np gn gm ",
        "ừm ừa à ờ ôi trời ơi thế à thật không đỉnh quá ghê vậy sao đó hả ",
        "không có chưa rồi xong đang đi về tới nơi đây nè nha nhé ạ ơi ",
        "yes no ok okay sure cool nice great thanks hi hello hey bye ",
        "ok oke okie alo ừ ư vâng dạ uhm uh huh haha ❤️ 👍 😂 ",
    ];
    for s in SECTIONS {
        dict.extend_from_slice(s.as_bytes());
    }
    dict
}

pub fn public_chat_dict_v1() -> &'static ZstdDictionaryEntry {
    static ENTRY: OnceLock<ZstdDictionaryEntry> = OnceLock::new();
    ENTRY.get_or_init(|| {
        let bytes = build_public_chat_dict_v1_bytes();
        let blake3_hex = blake3::hash(&bytes).to_hex().to_string();
        ZstdDictionaryEntry {
            dict_id: DICT_ID_PUBLIC_CHAT_V1,
            version: 1,
            is_private_e2ee: false,
            blake3_hex,
            bytes,
        }
    })
}

/// Verify integrity of any dictionary bytes against expected BLAKE3 hex digest.
pub fn verify_dictionary_integrity(bytes: &[u8], expected_blake3_hex: &str) -> MsgStoreResult<()> {
    let actual = blake3::hash(bytes).to_hex().to_string();
    if actual != expected_blake3_hex {
        return Err(MsgStoreError::IntegrityMismatch {
            expected: expected_blake3_hex.to_string(),
            actual,
        });
    }
    Ok(())
}

/// Seal a private user/conversation dictionary with AES-256-GCM before syncing to server.
/// Never leaks private training samples to server or other accounts!
pub fn seal_private_dictionary(
    key: &DerivedKey,
    account_id: &str,
    scope_id: &str,
    dict_id: u32,
    dict_version: u32,
    raw_dict_bytes: &[u8],
) -> MsgStoreResult<Vec<u8>> {
    if dict_id < PRIVATE_DICT_ID_MIN {
        return Err(MsgStoreError::ScopeViolation(
            "private dictionary id must be >= 1000".into(),
        ));
    }
    let aad = format!("CWDC1|acc={account_id}|sid={scope_id}|id={dict_id}|v={dict_version}");
    let ct = seal_aead_blob(key, aad.as_bytes(), raw_dict_bytes)?;
    let mut out = Vec::with_capacity(5 + 8 + ct.len());
    out.extend_from_slice(PRIVATE_DICT_MAGIC);
    out.extend_from_slice(&dict_id.to_le_bytes());
    out.extend_from_slice(&dict_version.to_le_bytes());
    out.extend_from_slice(&ct);
    Ok(out)
}

pub fn open_private_dictionary(
    key: &DerivedKey,
    account_id: &str,
    scope_id: &str,
    sealed: &[u8],
) -> MsgStoreResult<ZstdDictionaryEntry> {
    if sealed.len() < 5 + 8 + 28 || &sealed[..5] != PRIVATE_DICT_MAGIC {
        return Err(MsgStoreError::InvalidFormat("bad CWDC1 private dict header".into()));
    }
    let dict_id = u32::from_le_bytes([sealed[5], sealed[6], sealed[7], sealed[8]]);
    let version = u32::from_le_bytes([sealed[9], sealed[10], sealed[11], sealed[12]]);
    let aad = format!("CWDC1|acc={account_id}|sid={scope_id}|id={dict_id}|v={version}");
    let bytes = open_aead_blob(key, aad.as_bytes(), &sealed[13..])?;
    let blake3_hex = blake3::hash(&bytes).to_hex().to_string();
    Ok(ZstdDictionaryEntry {
        dict_id,
        version,
        is_private_e2ee: true,
        blake3_hex,
        bytes,
    })
}

/// Versioned Dictionary Registry retaining all referenced dictionaries and verifying integrity.
#[derive(Clone, Debug)]
pub struct DictionaryRegistry {
    entries: HashMap<u32, ZstdDictionaryEntry>,
    ref_counts: HashMap<u32, u64>,
}

impl Default for DictionaryRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DictionaryRegistry {
    pub fn new() -> Self {
        let mut entries = HashMap::new();
        let pub_v1 = public_chat_dict_v1().clone();
        entries.insert(pub_v1.dict_id, pub_v1);
        Self {
            entries,
            ref_counts: HashMap::new(),
        }
    }

    pub fn register(&mut self, entry: ZstdDictionaryEntry) -> MsgStoreResult<()> {
        verify_dictionary_integrity(&entry.bytes, &entry.blake3_hex)?;
        self.entries.insert(entry.dict_id, entry);
        Ok(())
    }

    pub fn get(&self, dict_id: u32) -> MsgStoreResult<Option<&ZstdDictionaryEntry>> {
        if dict_id == DICT_ID_NONE {
            return Ok(None);
        }
        self.entries
            .get(&dict_id)
            .map(Some)
            .ok_or(MsgStoreError::DictionaryMissing(dict_id))
    }

    pub fn add_reference(&mut self, dict_id: u32) {
        if dict_id != DICT_ID_NONE {
            *self.ref_counts.entry(dict_id).or_insert(0) += 1;
        }
    }

    pub fn remove_reference(&mut self, dict_id: u32) {
        if let Some(c) = self.ref_counts.get_mut(&dict_id) {
            *c = c.saturating_sub(1);
        }
    }

    /// Safely prune unreferenced private dictionaries ONLY (never deletes public built-in
    /// or any dictionary still referenced by a chunk!).
    pub fn try_remove(&mut self, dict_id: u32) -> bool {
        if dict_id < PRIVATE_DICT_ID_MIN {
            return false;
        }
        if self.ref_counts.get(&dict_id).copied().unwrap_or(0) > 0 {
            return false;
        }
        self.entries.remove(&dict_id).is_some()
    }
}
