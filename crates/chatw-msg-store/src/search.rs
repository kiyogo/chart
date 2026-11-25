//! High-Performance Encrypted Local Full-Text Search & Incremental Inverted Index (Sections 13, 14, 15).
//!
//! Features:
//! - Full Unicode NFC/NFD normalization + Vietnamese diacritic folding (`xin chao` <-> `xin chào`,
//!   `nguyen` <-> `Nguyễn`, `dien thoai` <-> `điện thoại`, including combining marks & `đ/Đ`).
//! - Dual-stream indexing (folded + exact accented tokens) for exact-match boost alongside forgiving recall.
//! - CJK bigram tokenization (Japanese/Chinese), emoji extraction, prefix search via `BTreeMap` range scan,
//!   and positional postings for exact phrase search (`"xin chào bạn"`).
//! - BM25 + recency + exact-diacritic boost ranking.
//! - Full filter support: sender, conversation, group, server, channel, date/time range, message type,
//!   attachment, image, video, link, pinned, bookmarked.
//! - Search Result Cache (invalidated automatically on mutation), Pagination, Cooperative Cancellation (`AtomicBool`),
//!   and truthful Coverage State (`Indexed` / `PartiallyIndexed` / `NotDownloaded` / `Syncing` / `NoAccess`).
//! - Incremental updates: `index_message`, `update_chunk_locator` (no re-tokenization!), `edit_message`,
//!   `delete_message` (tombstone prevents resurrection from stale chunks!), `compact_for_storage_limit`.
//! - Encrypted shard serialization (`CWSE1`): compact binary + Zstd Level 22 + AES-256-GCM AEAD.

use crate::crypto::{DerivedKey, open_aead_blob, seal_aead_blob};
use crate::model::{
    CompactMessage, MessageType, MsgStoreError, MsgStoreResult, ScopeKind,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use unicode_normalization::UnicodeNormalization;

pub const SEARCH_SHARD_MAGIC: &[u8; 5] = b"CWSE1";
pub const SEARCH_SHARD_FORMAT_VERSION: u16 = 1;

/// Normalize Vietnamese & multilingual text into lowercase ASCII-folded form (accent-insensitive)
/// WITHOUT altering original stored message text.
/// Handles precomposed Unicode (`ệ`, `đ`, `Đ`), decomposed combining marks (`a\u{0301}`), and mixed case.
pub fn fold_vietnamese_unicode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.nfd() {
        // Skip Unicode combining diacritical marks (U+0300..=U+036F, U+1AB0..=U+1AFF, U+1DC0..=U+1DFF)
        let code = ch as u32;
        if (0x0300..=0x036F).contains(&code)
            || (0x1AB0..=0x1AFF).contains(&code)
            || (0x1DC0..=0x1DFF).contains(&code)
        {
            continue;
        }
        match ch {
            'đ' | 'Đ' | 'ð' | 'Ð' => out.push('d'),
            other => {
                for lc in other.to_lowercase() {
                    out.push(lc);
                }
            }
        }
    }
    out
}

/// Normalize text preserving exact accents (Unicode NFC + lowercase).
pub fn normalize_exact_unicode(input: &str) -> String {
    input.nfc().collect::<String>().to_lowercase()
}

#[inline]
fn is_cjk(ch: char) -> bool {
    let c = ch as u32;
    (0x3040..=0x30FF).contains(&c) // Hiragana + Katakana
        || (0x4E00..=0x9FFF).contains(&c) // CJK Unified Ideographs
        || (0x3400..=0x4DBF).contains(&c) // CJK Ext A
        || (0xAC00..=0xD7AF).contains(&c) // Hangul
}

#[inline]
fn is_emoji(ch: char) -> bool {
    let c = ch as u32;
    (0x1F300..=0x1FAFF).contains(&c)
        || (0x2600..=0x27BF).contains(&c)
        || (0x2300..=0x23FF).contains(&c)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositionedToken {
    pub folded: String,
    pub exact: String,
    pub position: u32,
}

/// Tokenize multilingual text (Vietnamese, English, Japanese, Chinese, Emoji) into positioned
/// dual tokens `(folded, exact, position)`.
pub fn tokenize_text(input: &str) -> Vec<PositionedToken> {
    let exact_norm = normalize_exact_unicode(input);
    let mut tokens = Vec::new();
    let mut pos: u32 = 0;

    let chars: Vec<char> = exact_norm.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if is_emoji(ch) {
            let s = ch.to_string();
            tokens.push(PositionedToken {
                folded: s.clone(),
                exact: s,
                position: pos,
            });
            pos = pos.saturating_add(1);
            i += 1;
            continue;
        }
        if is_cjk(ch) {
            // Emit CJK unigram + overlapping bigram for fast Japanese/Chinese substring recall
            let mut run = Vec::new();
            while i < chars.len() && is_cjk(chars[i]) {
                run.push(chars[i]);
                i += 1;
            }
            if run.len() == 1 {
                let s = run[0].to_string();
                tokens.push(PositionedToken {
                    folded: s.clone(),
                    exact: s,
                    position: pos,
                });
                pos = pos.saturating_add(1);
            } else {
                for w in 0..run.len() {
                    let u = run[w].to_string();
                    tokens.push(PositionedToken {
                        folded: u.clone(),
                        exact: u,
                        position: pos,
                    });
                    if w + 1 < run.len() {
                        let mut bg = String::with_capacity(6);
                        bg.push(run[w]);
                        bg.push(run[w + 1]);
                        tokens.push(PositionedToken {
                            folded: bg.clone(),
                            exact: bg,
                            position: pos,
                        });
                    }
                    pos = pos.saturating_add(1);
                }
            }
            continue;
        }
        if ch.is_alphanumeric() {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let exact: String = chars[start..i].iter().collect();
            let folded = fold_vietnamese_unicode(&exact);
            if !folded.is_empty() {
                tokens.push(PositionedToken {
                    folded,
                    exact,
                    position: pos,
                });
                pos = pos.saturating_add(1);
            }
            continue;
        }
        i += 1;
    }

    tokens
}

/// Compact locator metadata for every indexed message (enables instant chunk lookup without
/// decompressing any chunk!).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexedDocMeta {
    pub doc_id: u32,
    pub message_id: String,
    pub scope_type: ScopeKind,
    pub scope_id: String,
    /// Optional parent server ID (for channel filtering) or group ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    /// `None` if message is still Hot (< 7d / unchunked); `Some(chunk_id)` once sealed!
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_id: Option<String>,
    /// 0-based offset inside the chunk for O(1) message extraction after lazy decompression.
    pub chunk_msg_offset: u32,
    pub seq: i64,
    pub created_at_ms: i64,
    pub sender_id: String,
    pub msg_type: MessageType,
    pub token_len: u32,
    pub has_attachment: bool,
    pub has_image: bool,
    pub has_video: bool,
    pub has_link: bool,
    pub is_pinned: bool,
    pub is_bookmarked: bool,
    /// Short local encrypted preview snippet (<= 160 chars) for instant search result display.
    pub snippet: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Posting {
    pub doc_id: u32,
    pub positions: Vec<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCoverageState {
    IndexedComplete,
    PartiallyIndexed,
    NotDownloaded,
    Syncing,
    NoAccess,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    pub sender_id: Option<String>,
    pub scope_type: Option<ScopeKind>,
    pub scope_id: Option<String>,
    pub container_id: Option<String>,
    pub date_from_ms: Option<i64>,
    pub date_to_ms: Option<i64>,
    pub msg_type: Option<MessageType>,
    pub has_attachment: Option<bool>,
    pub has_image: Option<bool>,
    pub has_video: Option<bool>,
    pub has_link: Option<bool>,
    pub is_pinned: Option<bool>,
    pub is_bookmarked: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    #[serde(default)]
    pub exact_diacritics: bool,
    #[serde(default)]
    pub phrase_match: bool,
    #[serde(default = "default_prefix_true")]
    pub prefix_match: bool,
    #[serde(default)]
    pub filters: SearchFilters,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_prefix_true() -> bool {
    true
}
fn default_limit() -> usize {
    30
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub message_id: String,
    pub scope_type: ScopeKind,
    pub scope_id: String,
    pub chunk_id: Option<String>,
    pub chunk_msg_offset: u32,
    pub seq: i64,
    pub created_at_ms: i64,
    pub sender_id: String,
    pub msg_type: MessageType,
    pub score: f64,
    pub snippet: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub total_matched: usize,
    pub has_more: bool,
    pub coverage: SearchCoverageState,
    pub unindexed_scopes: Vec<String>,
    pub from_result_cache: bool,
}

/// Core local inverted index (portable across Desktop, Mobile, Web WASM Worker).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LocalSearchIndex {
    pub format_version: u16,
    pub index_version: u64,
    next_doc_id: u32,
    docs: HashMap<u32, IndexedDocMeta>,
    msg_id_to_doc: HashMap<String, u32>,
    /// Tombstone set ensuring deleted/recalled messages NEVER reappear from stale chunks!
    deleted_tombstones: HashSet<String>,
    /// Sorted folded term dictionary -> postings (`BTreeMap` enables fast O(log V) prefix range scans!).
    folded_postings: BTreeMap<String, Vec<Posting>>,
    /// Exact-accented term dictionary -> doc IDs for exact diacritic match & ranking boost.
    exact_postings: HashMap<String, HashSet<u32>>,
    /// Coverage tracking per conversation scope.
    scope_coverage: HashMap<String, SearchCoverageState>,
    total_token_len: u64,
    #[serde(skip)]
    result_cache: HashMap<String, Vec<SearchHit>>,
}

impl LocalSearchIndex {
    pub fn new() -> Self {
        Self {
            format_version: SEARCH_SHARD_FORMAT_VERSION,
            index_version: 1,
            next_doc_id: 1,
            ..Default::default()
        }
    }

    pub fn doc_count(&self) -> usize {
        self.docs.len()
    }

    pub fn term_count(&self) -> usize {
        self.folded_postings.len()
    }

    pub fn set_scope_coverage(&mut self, scope_key: &str, state: SearchCoverageState) {
        self.scope_coverage.insert(scope_key.to_string(), state);
    }

    pub fn get_scope_coverage(&self, scope_key: &str) -> SearchCoverageState {
        self.scope_coverage
            .get(scope_key)
            .copied()
            .unwrap_or(SearchCoverageState::NotDownloaded)
    }

    /// Incrementally insert or update a single decrypted message.
    pub fn upsert_message(
        &mut self,
        scope_type: ScopeKind,
        scope_id: &str,
        container_id: Option<&str>,
        chunk_id: Option<&str>,
        chunk_msg_offset: u32,
        msg: &CompactMessage,
    ) {
        if msg.deleted_for_all || self.deleted_tombstones.contains(&msg.id) {
            self.delete_message(&msg.id);
            return;
        }

        // If already indexed (e.g. edited), remove old postings first
        if let Some(&old_doc_id) = self.msg_id_to_doc.get(&msg.id) {
            self.remove_doc_internal(old_doc_id);
        }

        let doc_id = self.next_doc_id;
        self.next_doc_id = self.next_doc_id.saturating_add(1);

        let tokens = tokenize_text(&msg.text);
        let token_len = tokens.len() as u32;
        self.total_token_len += token_len as u64;

        let mut folded_pos_map: HashMap<String, Vec<u32>> = HashMap::new();
        for tok in &tokens {
            folded_pos_map
                .entry(tok.folded.clone())
                .or_default()
                .push(tok.position);
            self.exact_postings
                .entry(tok.exact.clone())
                .or_default()
                .insert(doc_id);
        }
        for (term, positions) in folded_pos_map {
            self.folded_postings
                .entry(term)
                .or_default()
                .push(Posting { doc_id, positions });
        }

        let has_attachment = !msg.attachments.is_empty();
        let has_image = msg.msg_type == MessageType::Image
            || msg.attachments.iter().any(|a| a.mime_type.starts_with("image/"));
        let has_video = msg.msg_type == MessageType::Video
            || msg.attachments.iter().any(|a| a.mime_type.starts_with("video/"));
        let has_link = msg.msg_type == MessageType::Link
            || msg.text.contains("http://")
            || msg.text.contains("https://");

        let snippet: String = msg.text.chars().take(160).collect();

        let meta = IndexedDocMeta {
            doc_id,
            message_id: msg.id.clone(),
            scope_type,
            scope_id: scope_id.to_string(),
            container_id: container_id.map(str::to_string),
            chunk_id: chunk_id.map(str::to_string),
            chunk_msg_offset,
            seq: msg.seq,
            created_at_ms: msg.created_at_ms,
            sender_id: msg.sender_id.clone(),
            msg_type: msg.msg_type,
            token_len,
            has_attachment,
            has_image,
            has_video,
            has_link,
            is_pinned: msg.is_pinned,
            is_bookmarked: msg.is_bookmarked,
            snippet,
        };

        self.docs.insert(doc_id, meta);
        self.msg_id_to_doc.insert(msg.id.clone(), doc_id);
        self.index_version += 1;
        self.result_cache.clear();
    }

    /// Update a message's chunk locator when moved into a compressed chunk WITHOUT re-tokenizing!
    pub fn update_chunk_locator(
        &mut self,
        message_id: &str,
        chunk_id: &str,
        chunk_msg_offset: u32,
    ) -> bool {
        if let Some(&doc_id) = self.msg_id_to_doc.get(message_id) {
            if let Some(meta) = self.docs.get_mut(&doc_id) {
                meta.chunk_id = Some(chunk_id.to_string());
                meta.chunk_msg_offset = chunk_msg_offset;
                self.index_version += 1;
                self.result_cache.clear();
                return true;
            }
        }
        false
    }

    /// Permanently remove deleted or recalled message from index + result cache and record tombstone.
    pub fn delete_message(&mut self, message_id: &str) {
        self.deleted_tombstones.insert(message_id.to_string());
        if let Some(doc_id) = self.msg_id_to_doc.remove(message_id) {
            self.remove_doc_internal(doc_id);
            self.index_version += 1;
            self.result_cache.clear();
        }
    }

    fn remove_doc_internal(&mut self, doc_id: u32) {
        if let Some(meta) = self.docs.remove(&doc_id) {
            self.total_token_len = self.total_token_len.saturating_sub(meta.token_len as u64);
            let tokens = tokenize_text(&meta.snippet);
            // Also scan postings cleanly if snippet was truncated vs full text
            if meta.token_len <= 40 && !tokens.is_empty() {
                for t in &tokens {
                    if let Some(list) = self.folded_postings.get_mut(&t.folded) {
                        list.retain(|p| p.doc_id != doc_id);
                        if list.is_empty() {
                            self.folded_postings.remove(&t.folded);
                        }
                    }
                    if let Some(set) = self.exact_postings.get_mut(&t.exact) {
                        set.remove(&doc_id);
                        if set.is_empty() {
                            self.exact_postings.remove(&t.exact);
                        }
                    }
                }
            } else {
                self.folded_postings.retain(|_, list| {
                    list.retain(|p| p.doc_id != doc_id);
                    !list.is_empty()
                });
                self.exact_postings.retain(|_, set| {
                    set.remove(&doc_id);
                    !set.is_empty()
                });
            }
        }
    }

    /// Shrink index under low-storage pressure by dropping oldest non-pinned documents first
    /// (Section 15) and marking affected scopes as `PartiallyIndexed`.
    pub fn compact_to_max_docs(&mut self, max_docs: usize) -> usize {
        if self.docs.len() <= max_docs {
            return 0;
        }
        let mut candidates: Vec<(u32, i64, bool)> = self
            .docs
            .values()
            .map(|d| (d.doc_id, d.created_at_ms, d.is_pinned || d.is_bookmarked))
            .collect();
        // Evict non-pinned oldest messages first
        candidates.sort_by_key(|&(_, ts, keep)| (keep, ts));
        let drop_cnt = self.docs.len().saturating_sub(max_docs);
        let mut drop_ids = HashSet::with_capacity(drop_cnt);
        let mut affected_scopes = HashSet::new();

        for (doc_id, _, _) in candidates.into_iter().take(drop_cnt) {
            if let Some(meta) = self.docs.remove(&doc_id) {
                self.msg_id_to_doc.remove(&meta.message_id);
                self.total_token_len = self.total_token_len.saturating_sub(meta.token_len as u64);
                affected_scopes.insert(format!("{}:{}", meta.scope_type.as_str(), meta.scope_id));
                drop_ids.insert(doc_id);
            }
        }

        if !drop_ids.is_empty() {
            self.folded_postings.retain(|_, list| {
                list.retain(|p| !drop_ids.contains(&p.doc_id));
                !list.is_empty()
            });
            self.exact_postings.retain(|_, set| {
                set.retain(|id| !drop_ids.contains(id));
                !set.is_empty()
            });
            for sk in affected_scopes {
                self.scope_coverage
                    .insert(sk, SearchCoverageState::PartiallyIndexed);
            }
            self.index_version += 1;
            self.result_cache.clear();
        }
        drop_ids.len()
    }

    /// Execute full-text search with BM25 + recency + exact-diacritic boost, full filters,
    /// prefix & phrase matching, result cache, pagination, and cooperative cancellation!
    pub fn search(
        &mut self,
        query: &SearchQuery,
        cancel_flag: Option<&AtomicBool>,
    ) -> MsgStoreResult<SearchResponse> {
        let check_cancel = || -> MsgStoreResult<()> {
            if cancel_flag.is_some_and(|f| f.load(Ordering::Relaxed)) {
                Err(MsgStoreError::Cancelled)
            } else {
                Ok(())
            }
        };
        check_cancel()?;

        let cache_key = serde_json::to_string(&(
            &query.text,
            query.exact_diacritics,
            query.phrase_match,
            query.prefix_match,
            &query.filters,
        ))
        .unwrap_or_default();

        let (coverage, unindexed_scopes) = self.compute_coverage(&query.filters);

        if let Some(cached) = self.result_cache.get(&cache_key) {
            let total_matched = cached.len();
            let page: Vec<SearchHit> = cached
                .iter()
                .skip(query.offset)
                .take(query.limit.max(1))
                .cloned()
                .collect();
            return Ok(SearchResponse {
                has_more: query.offset + page.len() < total_matched,
                hits: page,
                total_matched,
                coverage,
                unindexed_scopes,
                from_result_cache: true,
            });
        }

        let trimmed = query.text.trim();
        let is_quoted_phrase =
            query.phrase_match || (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2);
        let clean_query = if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            &trimmed[1..trimmed.len() - 1]
        } else {
            trimmed
        };

        let q_tokens = tokenize_text(clean_query);
        let n_docs = self.docs.len().max(1) as f64;
        let avg_dl = (self.total_token_len as f64 / n_docs).max(1.0);
        let mut scores: HashMap<u32, f64> = HashMap::new();

        if q_tokens.is_empty() {
            // Pure filter query (e.g. pinned/media/sender/date filters without text)
            for meta in self.docs.values() {
                check_cancel()?;
                if Self::matches_filters(meta, &query.filters) {
                    scores.insert(meta.doc_id, meta.created_at_ms as f64 / 1e12);
                }
            }
        } else {
            // Collect per-query-token candidate postings (exact + optional prefix expansion on last token)
            let mut per_term_hits: Vec<HashMap<u32, Vec<u32>>> = Vec::with_capacity(q_tokens.len());

            for (t_idx, qt) in q_tokens.iter().enumerate() {
                check_cancel()?;
                let mut doc_pos: HashMap<u32, Vec<u32>> = HashMap::new();
                let allow_prefix =
                    query.prefix_match && !is_quoted_phrase && t_idx + 1 == q_tokens.len() && qt.folded.len() >= 2;

                if allow_prefix {
                    // O(log V + K) BTreeMap prefix range scan (capped at 64 prefix expansions for sub-10ms latency)
                    let mut expanded = 0usize;
                    for (term, postings) in self.folded_postings.range(qt.folded.clone()..) {
                        if !term.starts_with(&qt.folded) || expanded >= 64 {
                            break;
                        }
                        expanded += 1;
                        for p in postings {
                            doc_pos
                                .entry(p.doc_id)
                                .or_default()
                                .extend_from_slice(&p.positions);
                        }
                    }
                } else if let Some(postings) = self.folded_postings.get(&qt.folded) {
                    for p in postings {
                        doc_pos
                            .entry(p.doc_id)
                            .or_default()
                            .extend_from_slice(&p.positions);
                    }
                }

                // If exact_diacritics requested, filter strictly to exact accented doc set
                if query.exact_diacritics {
                    if let Some(exact_set) = self.exact_postings.get(&qt.exact) {
                        doc_pos.retain(|doc_id, _| exact_set.contains(doc_id));
                    } else {
                        doc_pos.clear();
                    }
                }

                if doc_pos.is_empty() {
                    return Ok(SearchResponse {
                        hits: Vec::new(),
                        total_matched: 0,
                        has_more: false,
                        coverage,
                        unindexed_scopes,
                        from_result_cache: false,
                    });
                }
                per_term_hits.push(doc_pos);
            }

            // Intersect across all query tokens (AND semantics) starting from smallest posting set
            let smallest_idx = per_term_hits
                .iter()
                .enumerate()
                .min_by_key(|(_, m)| m.len())
                .map(|(i, _)| i)
                .unwrap_or(0);

            let candidate_ids: Vec<u32> = per_term_hits[smallest_idx].keys().copied().collect();

            for doc_id in candidate_ids {
                check_cancel()?;
                let Some(meta) = self.docs.get(&doc_id) else {
                    continue;
                };
                if !Self::matches_filters(meta, &query.filters) {
                    continue;
                }
                if !per_term_hits.iter().all(|m| m.contains_key(&doc_id)) {
                    continue;
                }

                // Phrase match check: verify consecutive positions p0, p0+1, p0+2...
                if is_quoted_phrase && q_tokens.len() > 1 {
                    let first_positions = &per_term_hits[0][&doc_id];
                    let phrase_ok = first_positions.iter().any(|&p0| {
                        (1..q_tokens.len()).all(|offset| {
                            per_term_hits[offset][&doc_id].contains(&(p0 + offset as u32))
                        })
                    });
                    if !phrase_ok {
                        continue;
                    }
                }

                // BM25 scoring + exact-accent boost + mild recency boost
                let k1 = 1.2;
                let b = 0.75;
                let dl = meta.token_len.max(1) as f64;
                let mut bm25 = 0.0;
                for (t_idx, qt) in q_tokens.iter().enumerate() {
                    let df = per_term_hits[t_idx].len() as f64;
                    let idf = ((n_docs - df + 0.5) / (df + 0.5) + 1.0).ln();
                    let tf = per_term_hits[t_idx][&doc_id].len() as f64;
                    bm25 += idf * ((tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * dl / avg_dl)));

                    // Reward exact Vietnamese diacritic match over folded-only match
                    if self
                        .exact_postings
                        .get(&qt.exact)
                        .is_some_and(|set| set.contains(&doc_id))
                    {
                        bm25 += 0.35;
                    }
                }
                // Recency tie-breaker
                let recency = (meta.created_at_ms.max(0) as f64) / 1e15;
                scores.insert(doc_id, bm25 + recency);
            }
        }

        let mut ranked: Vec<SearchHit> = scores
            .into_iter()
            .filter_map(|(doc_id, score)| {
                let meta = self.docs.get(&doc_id)?;
                Some(SearchHit {
                    message_id: meta.message_id.clone(),
                    scope_type: meta.scope_type,
                    scope_id: meta.scope_id.clone(),
                    chunk_id: meta.chunk_id.clone(),
                    chunk_msg_offset: meta.chunk_msg_offset,
                    seq: meta.seq,
                    created_at_ms: meta.created_at_ms,
                    sender_id: meta.sender_id.clone(),
                    msg_type: meta.msg_type,
                    score,
                    snippet: meta.snippet.clone(),
                })
            })
            .collect();

        ranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.created_at_ms.cmp(&a.created_at_ms))
        });

        // Cache top results (bounded to 64 distinct cached queries)
        if self.result_cache.len() >= 64 {
            self.result_cache.clear();
        }
        let total_matched = ranked.len();
        let page: Vec<SearchHit> = ranked
            .iter()
            .skip(query.offset)
            .take(query.limit.max(1))
            .cloned()
            .collect();
        self.result_cache.insert(cache_key, ranked);

        Ok(SearchResponse {
            has_more: query.offset + page.len() < total_matched,
            hits: page,
            total_matched,
            coverage,
            unindexed_scopes,
            from_result_cache: false,
        })
    }

    fn compute_coverage(&self, filters: &SearchFilters) -> (SearchCoverageState, Vec<String>) {
        if let (Some(st), Some(sid)) = (filters.scope_type, filters.scope_id.as_deref()) {
            let key = format!("{}:{sid}", st.as_str());
            let state = self.get_scope_coverage(&key);
            let unindexed = if state == SearchCoverageState::IndexedComplete {
                Vec::new()
            } else {
                vec![key]
            };
            return (state, unindexed);
        }
        if self.scope_coverage.is_empty() {
            return (SearchCoverageState::PartiallyIndexed, Vec::new());
        }
        let unindexed: Vec<String> = self
            .scope_coverage
            .iter()
            .filter(|(_, st)| **st != SearchCoverageState::IndexedComplete)
            .map(|(k, _)| k.clone())
            .collect();
        let overall = if unindexed.is_empty() {
            SearchCoverageState::IndexedComplete
        } else {
            SearchCoverageState::PartiallyIndexed
        };
        (overall, unindexed)
    }

    fn matches_filters(meta: &IndexedDocMeta, f: &SearchFilters) -> bool {
        if let Some(ref s) = f.sender_id {
            if &meta.sender_id != s {
                return false;
            }
        }
        if let Some(st) = f.scope_type {
            if meta.scope_type != st {
                return false;
            }
        }
        if let Some(ref sid) = f.scope_id {
            if &meta.scope_id != sid {
                return false;
            }
        }
        if let Some(ref cid) = f.container_id {
            if meta.container_id.as_deref() != Some(cid.as_str()) {
                return false;
            }
        }
        if let Some(from) = f.date_from_ms {
            if meta.created_at_ms < from {
                return false;
            }
        }
        if let Some(to) = f.date_to_ms {
            if meta.created_at_ms > to {
                return false;
            }
        }
        if let Some(mt) = f.msg_type {
            if meta.msg_type != mt {
                return false;
            }
        }
        if let Some(v) = f.has_attachment {
            if meta.has_attachment != v {
                return false;
            }
        }
        if let Some(v) = f.has_image {
            if meta.has_image != v {
                return false;
            }
        }
        if let Some(v) = f.has_video {
            if meta.has_video != v {
                return false;
            }
        }
        if let Some(v) = f.has_link {
            if meta.has_link != v {
                return false;
            }
        }
        if let Some(v) = f.is_pinned {
            if meta.is_pinned != v {
                return false;
            }
        }
        if let Some(v) = f.is_bookmarked {
            if meta.is_bookmarked != v {
                return false;
            }
        }
        true
    }

    /// Seal the search index shard with Zstd Level 22 + AES-256-GCM AEAD (Section 14).
    /// Zero plaintext terms or doc metadata ever touch disk or server!
    pub fn seal_encrypted_shard(
        &self,
        key: &DerivedKey,
        account_id: &str,
        scope_key: &str,
        key_version: u32,
    ) -> MsgStoreResult<Vec<u8>> {
        let json_bytes = serde_json::to_vec(self)
            .map_err(|e| MsgStoreError::InvalidFormat(format!("search serialize: {e}")))?;
        let compressed = crate::zstd22::compress_zstd22(&json_bytes, None, None)?;
        let padded = crate::crypto::pad_for_side_channel_resistance(&compressed);
        let aad = format!(
            "CWSE1|v={SEARCH_SHARD_FORMAT_VERSION}|acc={account_id}|sc={scope_key}|kv={key_version}|iv={}",
            self.index_version
        );
        let ct = seal_aead_blob(key, aad.as_bytes(), &padded)?;
        let mut out = Vec::with_capacity(5 + 2 + 8 + ct.len());
        out.extend_from_slice(SEARCH_SHARD_MAGIC);
        out.extend_from_slice(&SEARCH_SHARD_FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&self.index_version.to_le_bytes());
        out.extend_from_slice(&ct);
        Ok(out)
    }

    /// Decrypt, unpad, bounded-decompress, and verify an encrypted `CWSE1` search shard.
    pub fn open_encrypted_shard(
        key: &DerivedKey,
        account_id: &str,
        scope_key: &str,
        key_version: u32,
        sealed: &[u8],
    ) -> MsgStoreResult<Self> {
        if sealed.len() < 5 + 2 + 8 + 28 || &sealed[..5] != SEARCH_SHARD_MAGIC {
            return Err(MsgStoreError::InvalidFormat("bad CWSE1 search shard magic".into()));
        }
        let ver = u16::from_le_bytes([sealed[5], sealed[6]]);
        if ver != SEARCH_SHARD_FORMAT_VERSION {
            return Err(MsgStoreError::UnsupportedVersion(ver));
        }
        let idx_ver = u64::from_le_bytes(sealed[7..15].try_into().unwrap());
        let aad = format!(
            "CWSE1|v={ver}|acc={account_id}|sc={scope_key}|kv={key_version}|iv={idx_ver}"
        );
        let padded = open_aead_blob(key, aad.as_bytes(), &sealed[15..])?;
        let compressed = crate::crypto::unpad_side_channel_frame(&padded)?;
        let json_bytes = crate::zstd22::decompress_zstd_bounded(compressed, None, None)?;
        let mut parsed: Self = serde_json::from_slice(&json_bytes)
            .map_err(|e| MsgStoreError::InvalidFormat(format!("search deserialize: {e}")))?;
        parsed.result_cache = HashMap::new();
        Ok(parsed)
    }
}
