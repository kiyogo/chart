//! Adaptive Storage Management, Dynamic Budget with Hysteresis, Multi-Tier Smart Cache (L1/L2/L3),
//! Cache Eviction Benchmark Suite (LRU, LFU, TinyLFU, W-TinyLFU, SLRU, Weighted),
//! and Intelligent Prefetch Planner (Sections 9, 10, 11, 12).

use crate::model::{
    CompactMessage, DeviceCapabilityProfile, PlatformKind, ResourceSignals, StorageMode,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * 1024 * 1024;

/// Detailed dynamic storage budget breakdown (Section 10.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicStorageBudget {
    pub mode: StorageMode,
    pub total_local_budget_bytes: u64,
    pub message_cache_budget_bytes: u64,
    pub search_index_budget_bytes: u64,
    pub dictionary_budget_bytes: u64,
    pub media_cache_budget_bytes: u64,
    pub temp_storage_budget_bytes: u64,
    pub offline_data_budget_bytes: u64,
    pub reserved_free_space_bytes: u64,
    pub l1_ram_budget_bytes: u64,
    pub effective_safe_ceiling_bytes: u64,
}

/// Compute platform-aware reserved headroom so ChatW NEVER exhausts OS/browser disk space.
fn compute_reserved_free_space(profile: &DeviceCapabilityProfile, free: u64) -> u64 {
    match profile.platform {
        PlatformKind::Web => {
            // Browser quota is virtualized; reserve 20% headroom (min 64 MiB when possible)
            if free < 256 * MIB {
                free / 3
            } else {
                (free / 5).clamp(64 * MIB, 1 * GIB)
            }
        }
        PlatformKind::Android | PlatformKind::Ios => {
            // Mobile OSes trigger aggressive app eviction below ~1-2 GB free
            if free < 1 * GIB {
                (free / 2).max(free.saturating_sub(64 * MIB))
            } else {
                (free / 6).clamp(512 * MIB, 4 * GIB)
            }
        }
        PlatformKind::Windows | PlatformKind::Linux | PlatformKind::MacOs | PlatformKind::Server => {
            if free < 2 * GIB {
                free / 2
            } else {
                (free / 10).clamp(1 * GIB, 8 * GIB)
            }
        }
    }
}

/// Tiered adaptive baseline budget curve (Section 10) calibrated per platform.
fn baseline_auto_budget_from_free(free_bytes: u64, platform: PlatformKind) -> u64 {
    let raw = if free_bytes < 1 * GIB {
        // < 1 GB -> 32..64 MB
        32 * MIB + ((free_bytes * 32 * MIB) / (1 * GIB))
    } else if free_bytes < 5 * GIB {
        // 1..5 GB -> 64..256 MB
        64 * MIB + (((free_bytes - 1 * GIB) * 192 * MIB) / (4 * GIB))
    } else if free_bytes < 10 * GIB {
        // 5..10 GB -> 256..512 MB
        256 * MIB + (((free_bytes - 5 * GIB) * 256 * MIB) / (5 * GIB))
    } else if free_bytes < 25 * GIB {
        // 10..25 GB -> 512 MB..1 GB
        512 * MIB + (((free_bytes - 10 * GIB) * 512 * MIB) / (15 * GIB))
    } else if free_bytes < 50 * GIB {
        // 25..50 GB -> 1..2 GB
        1 * GIB + (((free_bytes - 25 * GIB) * 1 * GIB) / (25 * GIB))
    } else if free_bytes < 100 * GIB {
        // 50..100 GB -> 2..5 GB
        2 * GIB + (((free_bytes - 50 * GIB) * 3 * GIB) / (50 * GIB))
    } else if free_bytes < 250 * GIB {
        // 100..250 GB -> 5..10 GB
        5 * GIB + (((free_bytes - 100 * GIB) * 5 * GIB) / (150 * GIB))
    } else {
        // > 250 GB -> adaptive up to 16 GB on desktop, 8 GB on mobile, 4 GB on Web
        10 * GIB + (((free_bytes - 250 * GIB) / 64).min(6 * GIB))
    };

    match platform {
        PlatformKind::Web => raw.min(4 * GIB),
        PlatformKind::Android | PlatformKind::Ios => raw.min(8 * GIB),
        _ => raw.min(16 * GIB),
    }
}

/// Compute Dynamic Storage Budget with **±12% hysteresis** to prevent oscillation when
/// device free space fluctuates slightly (Section 10.2).
pub fn compute_dynamic_storage_budget(
    profile: &DeviceCapabilityProfile,
    mode: StorageMode,
    custom_limit_mb: Option<u64>,
    previous_total_budget_bytes: Option<u64>,
    signals: &ResourceSignals,
) -> DynamicStorageBudget {
    let effective_free = match profile.os_quota_bytes {
        Some(q) if q > 0 => profile.free_disk_bytes.min(q),
        _ => profile.free_disk_bytes,
    };

    let reserved_free_space_bytes = compute_reserved_free_space(profile, effective_free);
    let safe_ceiling = effective_free
        .saturating_sub(reserved_free_space_bytes)
        .max(16 * MIB.min(effective_free));

    let base_auto = baseline_auto_budget_from_free(effective_free, profile.platform);

    let desired = match mode {
        StorageMode::Auto => base_auto,
        StorageMode::SpaceSaver => (base_auto * 45 / 100).max(24 * MIB),
        StorageMode::HighPerformance => (base_auto * 150 / 100).min(safe_ceiling),
        StorageMode::Custom => {
            let req = custom_limit_mb.unwrap_or(512).max(32) * MIB;
            req.min(safe_ceiling)
        }
    };

    // Never exceed safe ceiling; under extreme RAM/data-saver pressure keep conservative
    let mut target = desired.min(safe_ceiling);
    if signals.ram_pressure_pct >= 85 && mode == StorageMode::Auto {
        target = (target * 80 / 100).max(24 * MIB.min(safe_ceiling));
    }

    // Apply ±12% hysteresis deadband unless safety ceiling forces immediate shrink!
    let total_local_budget_bytes = match previous_total_budget_bytes {
        Some(prev) if prev > 0 && prev <= safe_ceiling => {
            let diff = target.abs_diff(prev);
            let deadband = (prev * 12) / 100;
            if diff <= deadband {
                prev
            } else {
                target
            }
        }
        _ => target,
    };

    // Sub-budget allocations summing <= total_local_budget_bytes
    let (msg_pct, idx_pct, dict_pct, media_pct, offline_pct, temp_pct) = match mode {
        StorageMode::SpaceSaver => (24, 14, 2, 36, 16, 8),
        StorageMode::HighPerformance => (28, 16, 2, 30, 18, 6),
        _ => (26, 15, 2, 34, 17, 6),
    };

    let message_cache_budget_bytes = total_local_budget_bytes * msg_pct / 100;
    let search_index_budget_bytes = total_local_budget_bytes * idx_pct / 100;
    let dictionary_budget_bytes = (total_local_budget_bytes * dict_pct / 100).clamp(2 * MIB, 64 * MIB);
    let media_cache_budget_bytes = total_local_budget_bytes * media_pct / 100;
    let offline_data_budget_bytes = total_local_budget_bytes * offline_pct / 100;
    let temp_storage_budget_bytes = total_local_budget_bytes * temp_pct / 100;

    // L1 RAM cache budget: strictly capped by device RAM (never keeps full history in RAM!)
    let ram_bytes = profile.available_ram_mb.max(256) * MIB;
    let l1_ram_budget_bytes = match (profile.platform, mode) {
        (PlatformKind::Web, StorageMode::SpaceSaver) => (ram_bytes / 40).clamp(8 * MIB, 32 * MIB),
        (PlatformKind::Web, _) => (ram_bytes / 24).clamp(16 * MIB, 64 * MIB),
        (PlatformKind::Android | PlatformKind::Ios, StorageMode::SpaceSaver) => {
            (ram_bytes / 40).clamp(12 * MIB, 48 * MIB)
        }
        (PlatformKind::Android | PlatformKind::Ios, _) => {
            (ram_bytes / 20).clamp(24 * MIB, 96 * MIB)
        }
        (_, StorageMode::SpaceSaver) => (ram_bytes / 32).clamp(16 * MIB, 64 * MIB),
        (_, StorageMode::HighPerformance) => (ram_bytes / 12).clamp(64 * MIB, 256 * MIB),
        _ => (ram_bytes / 18).clamp(32 * MIB, 128 * MIB),
    };

    DynamicStorageBudget {
        mode,
        total_local_budget_bytes,
        message_cache_budget_bytes,
        search_index_budget_bytes,
        dictionary_budget_bytes,
        media_cache_budget_bytes,
        temp_storage_budget_bytes,
        offline_data_budget_bytes,
        reserved_free_space_bytes,
        l1_ram_budget_bytes,
        effective_safe_ceiling_bytes: safe_ceiling,
    }
}

/// Compact 4-bit Count-Min Sketch (W-TinyLFU frequency estimator with periodic halving).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TinyLfuFrequencySketch {
    width: usize,
    counters: Vec<u8>,
    additions: u64,
    reset_threshold: u64,
}

impl TinyLfuFrequencySketch {
    pub fn new(width: usize) -> Self {
        let w = width.next_power_of_two().clamp(64, 16384);
        Self {
            width: w,
            counters: vec![0u8; w * 4],
            additions: 0,
            reset_threshold: (w as u64) * 10,
        }
    }

    fn indices(&self, key: &str) -> [usize; 4] {
        let h = blake3::hash(key.as_bytes());
        let b = h.as_bytes();
        let mask = self.width - 1;
        let i0 = (u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize) & mask;
        let i1 = (u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize) & mask;
        let i2 = (u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize) & mask;
        let i3 = (u32::from_le_bytes([b[12], b[13], b[14], b[15]]) as usize) & mask;
        [i0, self.width + i1, self.width * 2 + i2, self.width * 3 + i3]
    }

    pub fn record(&mut self, key: &str) {
        for idx in self.indices(key) {
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
        self.indices(key)
            .into_iter()
            .map(|idx| self.counters[idx])
            .min()
            .unwrap_or(0)
    }
}

/// Entry metadata for L2/L1 cost-aware W-TinyLFU + Segmented LRU cache eviction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedChunkEntry {
    pub key: String,
    pub account_id: String,
    pub scope_key: String,
    pub byte_size: u64,
    pub last_accessed_ms: i64,
    pub access_count: u32,
    pub download_cost_ms: u32,
    pub decompress_cost_ms: u32,
    pub importance_weight: f64,
    pub protected_segment: bool,
    pub pinned_offline: bool,
    pub in_use: bool,
    pub unsynced_or_unique: bool,
    pub verified_on_server: bool,
}

impl CachedChunkEntry {
    /// Safety invariant (Section 11.4): NEVER evict unsynced, unique, in-use, pinned, or
    /// unverified-on-server data!
    pub fn is_evictable(&self) -> bool {
        !self.pinned_offline
            && !self.in_use
            && !self.unsynced_or_unique
            && self.verified_on_server
    }
}

/// L1 Bounded RAM Cache for decompressed chunks & hot messages (Section 11.1).
/// Strictly enforces RAM cap and clears all plaintext immediately on `wipe_sensitive()`.
pub struct L1ChunkCache {
    max_bytes: u64,
    used_bytes: u64,
    sketch: TinyLfuFrequencySketch,
    entries: HashMap<String, L1Entry>,
    hits: u64,
    misses: u64,
    tick: u64,
}

struct L1Entry {
    messages: Vec<CompactMessage>,
    est_bytes: u64,
    last_tick: u64,
    decompress_cost_ms: u32,
    protected: bool,
    pinned: bool,
}

impl L1ChunkCache {
    pub fn new(max_bytes: u64) -> Self {
        Self {
            max_bytes: max_bytes.max(1024 * 1024),
            used_bytes: 0,
            sketch: TinyLfuFrequencySketch::new(1024),
            entries: HashMap::new(),
            hits: 0,
            misses: 0,
            tick: 0,
        }
    }

    pub fn set_max_bytes(&mut self, max_bytes: u64) {
        self.max_bytes = max_bytes.max(1024 * 1024);
        self.evict_until_within_budget(0);
    }

    pub fn get(&mut self, chunk_key: &str) -> Option<&[CompactMessage]> {
        self.tick += 1;
        self.sketch.record(chunk_key);
        if let Some(e) = self.entries.get_mut(chunk_key) {
            e.last_tick = self.tick;
            e.protected = true;
            self.hits += 1;
            return Some(&e.messages);
        }
        self.misses += 1;
        None
    }

    pub fn put(
        &mut self,
        chunk_key: String,
        messages: Vec<CompactMessage>,
        decompress_cost_ms: u32,
        pinned: bool,
    ) {
        self.tick += 1;
        self.sketch.record(&chunk_key);
        let est_bytes = estimate_messages_ram_bytes(&messages);
        if let Some(old) = self.entries.remove(&chunk_key) {
            self.used_bytes = self.used_bytes.saturating_sub(old.est_bytes);
        }
        if est_bytes > self.max_bytes && !pinned {
            return;
        }
        self.evict_until_within_budget(est_bytes);
        self.used_bytes += est_bytes;
        self.entries.insert(
            chunk_key,
            L1Entry {
                messages,
                est_bytes,
                last_tick: self.tick,
                decompress_cost_ms,
                protected: false,
                pinned,
            },
        );
    }

    pub fn invalidate(&mut self, chunk_key: &str) {
        if let Some(old) = self.entries.remove(chunk_key) {
            self.used_bytes = self.used_bytes.saturating_sub(old.est_bytes);
        }
    }

    /// Immediately zero & drop all decrypted plaintext messages from RAM on lock/logout!
    pub fn wipe_sensitive(&mut self) {
        for (_, mut e) in self.entries.drain() {
            for m in &mut e.messages {
                // Overwrite sensitive UTF-8 message buffers in place before deallocation
                unsafe {
                    for b in m.text.as_bytes_mut() {
                        std::ptr::write_volatile(b, 0);
                    }
                }
                m.text.clear();
            }
        }
        self.used_bytes = 0;
    }

    pub fn stats(&self) -> (u64, u64, u64, usize) {
        (self.used_bytes, self.hits, self.misses, self.entries.len())
    }

    fn evict_until_within_budget(&mut self, incoming: u64) {
        while self.used_bytes.saturating_add(incoming) > self.max_bytes {
            let victim = self
                .entries
                .iter()
                .filter(|(_, e)| !e.pinned)
                .min_by(|(k1, e1), (k2, e2)| {
                    let s1 = self.score(k1, e1);
                    let s2 = self.score(k2, e2);
                    s1.partial_cmp(&s2).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(k, _)| k.clone());
            match victim {
                Some(k) => {
                    if let Some(old) = self.entries.remove(&k) {
                        self.used_bytes = self.used_bytes.saturating_sub(old.est_bytes);
                    }
                }
                None => break,
            }
        }
    }

    fn score(&self, key: &str, e: &L1Entry) -> f64 {
        let freq = self.sketch.estimate(key) as f64 + if e.protected { 3.0 } else { 0.0 };
        let age = self.tick.saturating_sub(e.last_tick) as f64 + 1.0;
        let recency = 1.0 / (1.0 + age.log2());
        let cost = 1.0 + (e.decompress_cost_ms as f64 / 40.0).min(6.0);
        let size_kb = (e.est_bytes as f64 / 1024.0).max(1.0);
        ((freq * 1.5 + recency * 3.0) * cost) / size_kb.powf(0.25)
    }
}

pub fn estimate_messages_ram_bytes(messages: &[CompactMessage]) -> u64 {
    let mut sum = 64u64;
    for m in messages {
        sum += 128
            + m.id.len() as u64
            + m.sender_id.len() as u64
            + m.text.len() as u64
            + m.extra_meta.as_ref().map_or(0, |s| s.len() as u64)
            + (m.reactions.len() as u64) * 64
            + (m.attachments.len() as u64) * 160;
    }
    sum
}

/// Eviction algorithm selector for real comparative benchmarking (Section 11.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvictionAlgorithm {
    Lru,
    Lfu,
    TinyLfu,
    WTinyLfu,
    SegmentedLru,
    WeightedCostAware,
}

/// Benchmarkable L2 Cache Policy Simulator comparing all 6 algorithms on identical traces.
pub struct EvictionSimulator {
    algo: EvictionAlgorithm,
    capacity_bytes: u64,
    used_bytes: u64,
    sketch: TinyLfuFrequencySketch,
    items: HashMap<String, SimItem>,
    tick: u64,
    pub hits: u64,
    pub misses: u64,
    pub total_reload_cost_ms: u64,
}

#[derive(Clone)]
struct SimItem {
    size: u64,
    last_tick: u64,
    freq: u64,
    protected: bool,
    reload_cost_ms: u32,
}

impl EvictionSimulator {
    pub fn new(algo: EvictionAlgorithm, capacity_bytes: u64) -> Self {
        Self {
            algo,
            capacity_bytes,
            used_bytes: 0,
            sketch: TinyLfuFrequencySketch::new(1024),
            items: HashMap::new(),
            tick: 0,
            hits: 0,
            misses: 0,
            total_reload_cost_ms: 0,
        }
    }

    pub fn access(&mut self, key: &str, size: u64, reload_cost_ms: u32) -> bool {
        self.tick += 1;
        self.sketch.record(key);
        if let Some(item) = self.items.get_mut(key) {
            item.last_tick = self.tick;
            item.freq += 1;
            item.protected = true;
            self.hits += 1;
            return true;
        }
        self.misses += 1;
        self.total_reload_cost_ms += reload_cost_ms as u64;

        if size > self.capacity_bytes {
            return false;
        }

        // W-TinyLFU / TinyLFU admission filter when full: reject cold 1-hit scan items!
        if matches!(
            self.algo,
            EvictionAlgorithm::TinyLfu | EvictionAlgorithm::WTinyLfu
        ) && self.used_bytes + size > self.capacity_bytes
        {
            if let Some((vk, _)) = self.find_victim() {
                let cand_freq = self.sketch.estimate(key);
                let victim_freq = self.sketch.estimate(&vk);
                if cand_freq < victim_freq && self.algo == EvictionAlgorithm::TinyLfu {
                    return false;
                }
            }
        }

        while self.used_bytes + size > self.capacity_bytes {
            let Some((vk, vsize)) = self.find_victim() else {
                break;
            };
            self.items.remove(&vk);
            self.used_bytes = self.used_bytes.saturating_sub(vsize);
        }

        self.used_bytes += size;
        self.items.insert(
            key.to_string(),
            SimItem {
                size,
                last_tick: self.tick,
                freq: 1,
                protected: false,
                reload_cost_ms,
            },
        );
        false
    }

    fn find_victim(&self) -> Option<(String, u64)> {
        self.items
            .iter()
            .min_by(|(k1, a), (k2, b)| {
                let s1 = self.score(k1, a);
                let s2 = self.score(k2, b);
                s1.partial_cmp(&s2).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(k, v)| (k.clone(), v.size))
    }

    fn score(&self, key: &str, it: &SimItem) -> f64 {
        match self.algo {
            EvictionAlgorithm::Lru => it.last_tick as f64,
            EvictionAlgorithm::Lfu => (it.freq as f64) * 1e6 + it.last_tick as f64,
            EvictionAlgorithm::TinyLfu => {
                (self.sketch.estimate(key) as f64) * 1e6 + it.last_tick as f64
            }
            EvictionAlgorithm::SegmentedLru => {
                (if it.protected { 1e9 } else { 0.0 }) + it.last_tick as f64
            }
            EvictionAlgorithm::WTinyLfu => {
                let seg = if it.protected { 5.0 } else { 0.0 };
                let f = self.sketch.estimate(key) as f64 + seg;
                let age = self.tick.saturating_sub(it.last_tick) as f64 + 1.0;
                f * 2.0 + 4.0 / (1.0 + age.log2())
            }
            EvictionAlgorithm::WeightedCostAware => {
                let seg = if it.protected { 4.0 } else { 0.0 };
                let f = self.sketch.estimate(key) as f64 + seg;
                let age = self.tick.saturating_sub(it.last_tick) as f64 + 1.0;
                let recency = 1.0 / (1.0 + age.log2());
                let cost = 1.0 + (it.reload_cost_ms as f64 / 50.0).min(8.0);
                let size_kb = (it.size as f64 / 1024.0).max(1.0);
                ((f * 1.6 + recency * 3.2) * cost) / size_kb.powf(0.30)
            }
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Intelligent Scroll-Direction, Affinity & Search-Aware Prefetch Planner (Section 12).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrefetchRequest {
    pub chunk_id: String,
    pub scope_key: String,
    pub priority_score: u32,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollDirection {
    OlderHistory,
    NewerHistory,
    Idle,
}

pub struct PrefetchPlanner;

impl PrefetchPlanner {
    /// Compute bounded, cancellable prefetch queue given scroll direction, storage budget,
    /// and live device/network pressure signals.
    pub fn plan_for_conversation(
        scope_key: &str,
        ordered_chunk_ids: &[String],
        current_chunk_idx: usize,
        direction: ScrollDirection,
        already_local: &std::collections::HashSet<String>,
        budget: &DynamicStorageBudget,
        signals: &ResourceSignals,
    ) -> VecDeque<PrefetchRequest> {
        let mut out = VecDeque::new();
        if signals.is_offline
            || signals.active_voice_or_video_call
            || signals.data_saver_enabled
            || signals.thermal_throttled
            || signals.on_battery_saver
        {
            return out;
        }
        if ordered_chunk_ids.is_empty() || current_chunk_idx >= ordered_chunk_ids.len() {
            return out;
        }

        // Adaptive prefetch depth based on storage mode and network type
        let max_depth: usize = match (budget.mode, signals.on_metered_or_cellular) {
            (StorageMode::SpaceSaver, _) => 1,
            (_, true) => 1,
            (StorageMode::HighPerformance, false) => 4,
            _ => 2,
        };

        let mut push_if_missing = |idx: usize, prio: u32, reason: &str| {
            if let Some(cid) = ordered_chunk_ids.get(idx) {
                if !already_local.contains(cid) && out.len() < max_depth {
                    out.push_back(PrefetchRequest {
                        chunk_id: cid.clone(),
                        scope_key: scope_key.to_string(),
                        priority_score: prio,
                        reason: reason.to_string(),
                    });
                }
            }
        };

        match direction {
            ScrollDirection::OlderHistory => {
                for step in 1..=max_depth {
                    if let Some(prev) = current_chunk_idx.checked_sub(step) {
                        push_if_missing(prev, 100 - step as u32 * 10, "scroll_older");
                    }
                }
            }
            ScrollDirection::NewerHistory => {
                for step in 1..=max_depth {
                    push_if_missing(current_chunk_idx + step, 100 - step as u32 * 10, "scroll_newer");
                }
            }
            ScrollDirection::Idle => {
                if let Some(prev) = current_chunk_idx.checked_sub(1) {
                    push_if_missing(prev, 80, "adjacent_older");
                }
                push_if_missing(current_chunk_idx + 1, 70, "adjacent_newer");
            }
        }
        out
    }
}
