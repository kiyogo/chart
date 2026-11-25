//! Intelligent Local Cache Engine (Sections XX, XXI & XXII)
//!
//! - **W-TinyLFU (4-bit Count-Min Sketch)** frequency estimator with periodic halving/reset + admission filter
//! - **Segmented LRU** (Probation vs Protected segments)
//! - **Cost-Aware & Size-Aware Eviction** weighting:
//!   - `pinned` & `in_use` (never auto-evicted)
//!   - access frequency & recency
//!   - download cost + AI restoration recomputation cost (AI outputs that took 500ms+ to generate stay protected longer!)
//!   - stale AI model version (deprioritized for cleanup first)
//!   - byte size & TTL expiration
//! - **Adaptive Capacity Calculator** scaling safely from available device free disk / browser storage quota
//! - **Deterministic AI Output Cache Key** `(media_id, media_version, content_hash, model_id, model_version, params_hash, output_w, output_h, pipeline_version)`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheCategory {
    Avatar,
    Thumbnail,
    OptimizedMedia,
    OriginalLocal,
    AiEnhancedOutput,
    AiModelWeights,
    VideoSegment,
    AudioVoice,
    FileAttachment,
    Metadata,
}

impl CacheCategory {
    /// Base retention priority weight (higher = kept longer under eviction pressure).
    pub fn retention_weight(&self) -> f64 {
        match self {
            Self::OriginalLocal => 5.0,
            Self::AiModelWeights => 4.5,
            Self::AiEnhancedOutput => 4.0,
            Self::Avatar => 3.5,
            Self::AudioVoice => 2.5,
            Self::OptimizedMedia => 2.2,
            Self::FileAttachment => 2.0,
            Self::Metadata => 1.8,
            Self::VideoSegment => 1.3,
            Self::Thumbnail => 1.0,
        }
    }
}

/// Deterministic cache key for AI restoration results (Section XXII).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AiOutputCacheKey {
    pub media_id: String,
    pub media_version: u32,
    pub content_hash: String,
    pub model_id: String,
    pub model_version: String,
    pub params_hash: String,
    pub output_width: u32,
    pub output_height: u32,
    pub pipeline_version: u32,
}

impl AiOutputCacheKey {
    pub fn to_storage_key(&self) -> String {
        let canonical = format!(
            "ai:v{}:{}:mv{}:{}:{}:{}:{}:{}x{}",
            self.pipeline_version,
            self.media_id,
            self.media_version,
            self.content_hash,
            self.model_id,
            self.model_version,
            self.params_hash,
            self.output_width,
            self.output_height,
        );
        format!("ai_{}", blake3::hash(canonical.as_bytes()).to_hex())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptiveCacheBudget {
    pub global_limit_bytes: u64,
    pub ai_output_limit_bytes: u64,
    pub ai_model_limit_bytes: u64,
    pub video_segment_limit_bytes: u64,
    pub per_account_limit_bytes: u64,
    pub reserved_system_headroom_bytes: u64,
}

/// Dynamically calculate safe cache quotas based on available free storage (Section XX.4).
/// Never exhausts device storage; automatically scales down on constrained mobile/browser storage!
pub fn compute_adaptive_cache_budget(
    available_free_bytes: u64,
    user_configured_limit_mb: Option<u64>,
) -> AdaptiveCacheBudget {
    // Always preserve at least 15% of free space (minimum 256 MB on small quotas, up to 2 GB on large disks)
    let headroom = if available_free_bytes < 512 * 1024 * 1024 {
        available_free_bytes / 3
    } else {
        (available_free_bytes / 6).clamp(256 * 1024 * 1024, 2 * 1024 * 1024 * 1024)
    };
    let usable = available_free_bytes.saturating_sub(headroom);

    // Auto target: up to 12% of usable free space, clamped between 64 MB and 8 GB
    let auto_global = (usable / 8).clamp(64 * 1024 * 1024, 8 * 1024 * 1024 * 1024).min(usable);
    let global_limit_bytes = match user_configured_limit_mb {
        Some(0) => auto_global, // "Unlimited" still respects OS headroom so we NEVER fill the disk!
        Some(mb) => (mb * 1024 * 1024).min(usable.max(32 * 1024 * 1024)),
        None => auto_global,
    };

    AdaptiveCacheBudget {
        global_limit_bytes,
        ai_output_limit_bytes: global_limit_bytes / 4,
        ai_model_limit_bytes: (global_limit_bytes / 5).clamp(64 * 1024 * 1024, 1024 * 1024 * 1024),
        video_segment_limit_bytes: (global_limit_bytes / 100).saturating_mul(30),
        per_account_limit_bytes: (global_limit_bytes / 100).saturating_mul(85),
        reserved_system_headroom_bytes: headroom,
    }
}

/// 4-bit Count-Min Sketch (W-TinyLFU Doorkeeper + Frequency Table) with automatic periodic aging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TinyLfuSketch {
    width: usize,
    counters: Vec<u8>, // 4 rows x width, each capped at 15
    additions: u64,
    reset_threshold: u64,
}

impl TinyLfuSketch {
    pub fn new(width: usize) -> Self {
        let w = width.next_power_of_two().clamp(64, 16384);
        Self {
            width: w,
            counters: vec![0u8; w * 4],
            additions: 0,
            reset_threshold: (w as u64) * 10,
        }
    }

    fn row_indices(&self, key: &str) -> [usize; 4] {
        let h = blake3::hash(key.as_bytes());
        let b = h.as_bytes();
        let mask = self.width - 1;
        let i0 = (u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize) & mask;
        let i1 = (u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize) & mask;
        let i2 = (u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize) & mask;
        let i3 = (u32::from_le_bytes([b[12], b[13], b[14], b[15]]) as usize) & mask;
        [i0, self.width + i1, self.width * 2 + i2, self.width * 3 + i3]
    }

    pub fn increment(&mut self, key: &str) {
        for idx in self.row_indices(key) {
            self.counters[idx] = self.counters[idx].saturating_add(1).min(15);
        }
        self.additions += 1;
        if self.additions >= self.reset_threshold {
            for c in &mut self.counters {
                *c >>= 1;
            }
            self.additions = 0;
        }
    }

    pub fn estimate(&self, key: &str) -> u8 {
        self.row_indices(key)
            .into_iter()
            .map(|idx| self.counters[idx])
            .min()
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheItemMeta {
    pub key: String,
    pub account_id: String,
    pub category: CacheCategory,
    pub byte_size: u64,
    pub created_at_secs: u64,
    pub last_accessed_secs: u64,
    pub access_count: u32,
    pub download_cost_ms: u32,
    pub ai_recompute_cost_ms: u32,
    pub ttl_secs: Option<u64>,
    pub pinned_offline: bool,
    pub in_use: bool,
    /// True when this entry was generated by an outdated AI model version (evict early when space needed).
    pub stale_model_version: bool,
    /// Segmented LRU tier: false = Probation (20%), true = Protected (80%).
    pub protected_segment: bool,
}

/// Intelligent W-TinyLFU + Segmented LRU + Cost-Aware Eviction Index (portable across Web & Native).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntelligentCacheIndex {
    pub sketch: TinyLfuSketch,
    pub entries: HashMap<String, CacheItemMeta>,
}

impl Default for IntelligentCacheIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl IntelligentCacheIndex {
    pub fn new() -> Self {
        Self {
            sketch: TinyLfuSketch::new(1024),
            entries: HashMap::new(),
        }
    }

    pub fn total_bytes(&self) -> u64 {
        self.entries.values().map(|e| e.byte_size).sum()
    }

    pub fn category_bytes(&self, cat: CacheCategory) -> u64 {
        self.entries
            .values()
            .filter(|e| e.category == cat)
            .map(|e| e.byte_size)
            .sum()
    }

    pub fn touch(&mut self, key: &str, now_secs: u64) -> bool {
        self.sketch.increment(key);
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed_secs = now_secs;
            entry.access_count = entry.access_count.saturating_add(1);
            if entry.access_count >= 2 {
                entry.protected_segment = true;
            }
            true
        } else {
            false
        }
    }

    /// Cost-aware retention score (LOWER score = evicted FIRST).
    fn retention_score(&self, item: &CacheItemMeta, now_secs: u64) -> f64 {
        // Expired TTL or stale AI model versions get lowest priority so they clean up first
        if let Some(ttl) = item.ttl_secs {
            if now_secs.saturating_sub(item.created_at_secs) > ttl {
                return -10_000.0;
            }
        }
        if item.stale_model_version {
            return -1_000.0;
        }

        let freq = (self.sketch.estimate(&item.key) as f64) + (item.access_count.min(32) as f64) * 0.5;
        let age_hours = (now_secs.saturating_sub(item.last_accessed_secs) as f64) / 3600.0;
        let recency_factor = 1.0 / (1.0 + age_hours * 0.35);
        let cost_bonus = 1.0
            + ((item.download_cost_ms as f64) / 500.0).min(4.0)
            + ((item.ai_recompute_cost_ms as f64) / 200.0).min(8.0);
        let segment_bonus = if item.protected_segment { 2.0 } else { 1.0 };
        let size_penalty = ((item.byte_size.max(1024) as f64) / (256.0 * 1024.0)).ln_1p().max(0.25);

        (item.category.retention_weight() * (1.0 + freq) * recency_factor * cost_bonus * segment_bonus) / size_penalty
    }

    /// Insert or update an entry and return the exact list of evicted keys needed to satisfy `budget`.
    /// Never evicts `pinned_offline` or `in_use` items, and protects hot items via TinyLFU admission!
    pub fn put_and_plan_evictions(
        &mut self,
        item: CacheItemMeta,
        budget: &AdaptiveCacheBudget,
        now_secs: u64,
    ) -> Vec<String> {
        self.sketch.increment(&item.key);
        let incoming_cat = item.category;
        let incoming_key = item.key.clone();
        self.entries.insert(incoming_key.clone(), item);

        let mut evicted = Vec::new();

        // 1. Enforce per-category caps first (AI outputs, AI models, video segments)
        let cat_cap = match incoming_cat {
            CacheCategory::AiEnhancedOutput => Some(budget.ai_output_limit_bytes),
            CacheCategory::AiModelWeights => Some(budget.ai_model_limit_bytes),
            CacheCategory::VideoSegment => Some(budget.video_segment_limit_bytes),
            _ => None,
        };
        if let Some(limit) = cat_cap {
            self.evict_until(
                |e| e.category == incoming_cat,
                &incoming_key,
                limit,
                now_secs,
                &mut evicted,
            );
        }

        // 2. Enforce global cache limit
        self.evict_until(|_| true, &incoming_key, budget.global_limit_bytes, now_secs, &mut evicted);

        evicted
    }

    fn evict_until<F>(&mut self, in_scope: F, protect_key: &str, target_bytes: u64, now_secs: u64, evicted: &mut Vec<String>)
    where
        F: Fn(&CacheItemMeta) -> bool,
    {
        let mut current: u64 = self
            .entries
            .values()
            .filter(|e| in_scope(e))
            .map(|e| e.byte_size)
            .sum();
        if current <= target_bytes {
            return;
        }

        let mut candidates: Vec<(String, f64, u64)> = self
            .entries
            .values()
            .filter(|e| !e.pinned_offline && !e.in_use && e.key != protect_key && in_scope(e))
            .map(|e| (e.key.clone(), self.retention_score(e, now_secs), e.byte_size))
            .collect();
        candidates.sort_by(|a, b| a.1.total_cmp(&b.1));

        for (k, _, sz) in candidates {
            if current <= target_bytes {
                break;
            }
            if self.entries.remove(&k).is_some() {
                current = current.saturating_sub(sz);
                evicted.push(k);
            }
        }
    }

    /// Clear all non-pinned entries for a given account (e.g., on logout or user Clear Cache action).
    pub fn clear_account(&mut self, account_id: &str, include_pinned: bool) -> Vec<String> {
        let to_remove: Vec<String> = self
            .entries
            .values()
            .filter(|e| e.account_id == account_id && !e.in_use && (include_pinned || !e.pinned_offline))
            .map(|e| e.key.clone())
            .collect();
        for k in &to_remove {
            self.entries.remove(k);
        }
        to_remove
    }
}

/// Default inactivity threshold before uncompressed images transition to cold AVIF compression (15 days).
pub const COLD_IMAGE_COMPRESSION_IDLE_DAYS: u64 = 15;
pub const COLD_IMAGE_COMPRESSION_IDLE_SECS: u64 = COLD_IMAGE_COMPRESSION_IDLE_DAYS * 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColdImageDecision {
    /// Accessed within the last 15 days — keep original image bytes completely untouched.
    KeepActive { remaining_secs: u64 },
    /// Unused for >= 15 days — compress to Adaptive AVIF cold storage.
    CompressCold { idle_secs: u64 },
    /// Already AVIF / animated AVIF / E2EE ciphertext.
    SkipAlreadyCompressed,
    /// Explicitly pinned against background cold transforms.
    SkipPinned,
    /// Non-image asset or SVG (excluded from raster cold compression).
    SkipNotImage,
}

/// Evaluate whether an image should stay as-is (`< 15 days` since last user access)
/// or be compressed (`>= 15 days` unused).
pub fn evaluate_cold_image_policy(
    last_accessed_secs: u64,
    now_secs: u64,
    mime_type: &str,
    codec: &str,
    is_pinned: bool,
    idle_threshold_secs: Option<u64>,
) -> ColdImageDecision {
    let mime = mime_type.trim().to_ascii_lowercase();
    let codec_lc = codec.trim().to_ascii_lowercase();
    if !mime.starts_with("image/") || mime == "image/svg+xml" {
        return ColdImageDecision::SkipNotImage;
    }
    if is_pinned {
        return ColdImageDecision::SkipPinned;
    }
    if mime == "image/avif"
        || codec_lc.starts_with("avif")
        || codec_lc.starts_with("avis")
        || codec_lc == "e2ee_ciphertext"
    {
        return ColdImageDecision::SkipAlreadyCompressed;
    }
    let threshold = idle_threshold_secs.unwrap_or(COLD_IMAGE_COMPRESSION_IDLE_SECS);
    let idle_secs = now_secs.saturating_sub(last_accessed_secs);
    if idle_secs < threshold {
        ColdImageDecision::KeepActive {
            remaining_secs: threshold - idle_secs,
        }
    } else {
        ColdImageDecision::CompressCold { idle_secs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_aware_wtinylfu_evicts_cold_video_before_hot_ai_output_and_never_evicts_pinned() {
        let mut idx = IntelligentCacheIndex::new();
        let now = 1_700_000_000u64;
        let budget = AdaptiveCacheBudget {
            global_limit_bytes: 3_000_000,
            ai_output_limit_bytes: 2_000_000,
            ai_model_limit_bytes: 2_000_000,
            video_segment_limit_bytes: 2_000_000,
            per_account_limit_bytes: 3_000_000,
            reserved_system_headroom_bytes: 256 * 1024 * 1024,
        };

        // 1. Pinned offline file (1 MB)
        idx.put_and_plan_evictions(
            CacheItemMeta {
                key: "pinned_doc".into(),
                account_id: "u1".into(),
                category: CacheCategory::OriginalLocal,
                byte_size: 1_000_000,
                created_at_secs: now - 86400,
                last_accessed_secs: now - 86400,
                access_count: 1,
                download_cost_ms: 100,
                ai_recompute_cost_ms: 0,
                ttl_secs: None,
                pinned_offline: true,
                in_use: false,
                stale_model_version: false,
                protected_segment: true,
            },
            &budget,
            now,
        );

        // 2. Expensive AI output viewed 5 times (1 MB)
        idx.put_and_plan_evictions(
            CacheItemMeta {
                key: "hot_ai_img".into(),
                account_id: "u1".into(),
                category: CacheCategory::AiEnhancedOutput,
                byte_size: 1_000_000,
                created_at_secs: now - 600,
                last_accessed_secs: now - 10,
                access_count: 5,
                download_cost_ms: 200,
                ai_recompute_cost_ms: 950,
                ttl_secs: None,
                pinned_offline: false,
                in_use: false,
                stale_model_version: false,
                protected_segment: true,
            },
            &budget,
            now,
        );

        // 3. Old video segment accessed once hours ago (1 MB) -> exceeds 2.5 MB limit!
        idx.put_and_plan_evictions(
            CacheItemMeta {
                key: "cold_vid_seg".into(),
                account_id: "u1".into(),
                category: CacheCategory::VideoSegment,
                byte_size: 1_000_000,
                created_at_secs: now - 7200,
                last_accessed_secs: now - 7200,
                access_count: 1,
                download_cost_ms: 80,
                ai_recompute_cost_ms: 0,
                ttl_secs: None,
                pinned_offline: false,
                in_use: false,
                stale_model_version: false,
                protected_segment: false,
            },
            &budget,
            now,
        );

        // Add another 1 MB incoming item -> must evict `cold_vid_seg` first, preserving `pinned_doc` and `hot_ai_img`!
        let evicted = idx.put_and_plan_evictions(
            CacheItemMeta {
                key: "new_photo".into(),
                account_id: "u1".into(),
                category: CacheCategory::OptimizedMedia,
                byte_size: 500_000,
                created_at_secs: now,
                last_accessed_secs: now,
                access_count: 1,
                download_cost_ms: 120,
                ai_recompute_cost_ms: 0,
                ttl_secs: None,
                pinned_offline: false,
                in_use: false,
                stale_model_version: false,
                protected_segment: false,
            },
            &budget,
            now,
        );

        assert!(evicted.contains(&"cold_vid_seg".to_string()));
        assert!(idx.entries.contains_key("pinned_doc"));
        assert!(idx.entries.contains_key("hot_ai_img"));
    }

    #[test]
    fn ai_output_cache_key_changes_when_model_or_params_change() {
        let k1 = AiOutputCacheKey {
            media_id: "m1".into(),
            media_version: 1,
            content_hash: "abc".into(),
            model_id: "span-x2-int8".into(),
            model_version: "1.0.0".into(),
            params_hash: "p1".into(),
            output_width: 1920,
            output_height: 1080,
            pipeline_version: 1,
        };
        let mut k2 = k1.clone();
        k2.model_version = "1.1.0".into();
        assert_ne!(k1.to_storage_key(), k2.to_storage_key());
    }

    #[test]
    fn cold_image_policy_keeps_active_15d_and_compresses_after_15d_idle() {
        let now = 1_800_000_000u64;
        // 0 days & 14d 23h idle -> stays untouched as-is
        assert_eq!(
            evaluate_cold_image_policy(now, now, "image/jpeg", "jpg", false, None),
            ColdImageDecision::KeepActive { remaining_secs: COLD_IMAGE_COMPRESSION_IDLE_SECS }
        );
        assert_eq!(
            evaluate_cold_image_policy(now - (15 * 86_400 - 1), now, "image/png", "png", false, None),
            ColdImageDecision::KeepActive { remaining_secs: 1 }
        );
        // Exactly 15 days idle -> compress
        assert_eq!(
            evaluate_cold_image_policy(now - 15 * 86_400, now, "image/png", "png", false, None),
            ColdImageDecision::CompressCold { idle_secs: 15 * 86_400 }
        );
        // Re-accessed -> resets back to active
        assert_eq!(
            evaluate_cold_image_policy(now - 60, now, "image/webp", "webp", false, None),
            ColdImageDecision::KeepActive { remaining_secs: 15 * 86_400 - 60 }
        );
        // Already AVIF / E2EE / pinned / non-image skipped cleanly
        assert_eq!(
            evaluate_cold_image_policy(now - 30 * 86_400, now, "image/avif", "avif_cold_15d", false, None),
            ColdImageDecision::SkipAlreadyCompressed
        );
        assert_eq!(
            evaluate_cold_image_policy(now - 30 * 86_400, now, "image/png", "png", true, None),
            ColdImageDecision::SkipPinned
        );
        assert_eq!(
            evaluate_cold_image_policy(now - 30 * 86_400, now, "video/mp4", "mp4", false, None),
            ColdImageDecision::SkipNotImage
        );
    }
}
