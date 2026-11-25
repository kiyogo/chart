//! Incremental Message Compression Planner, Adaptive Chunk Sizer (64KB..16MB),
//! Crash-Safe Checkpoints, Pause/Resume, Backpressure & Mutation Overlay Compaction (Sections 2, 3, 4, 22).

use crate::model::{
    ChunkMutation, CompactMessage, DeviceCapabilityProfile, LOW_VOLUME_MAX_WAIT_SECS,
    MIN_COMPRESS_AGE_SECS, MutationOp, PlatformKind, ReactionEntry, ResourceSignals, ScopeKind,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Benchmarked target pre-compression chunk sizes (Section 4.1).
pub const BENCHMARK_CHUNK_SIZES: &[usize] = &[
    64 * 1024,
    128 * 1024,
    256 * 1024,
    512 * 1024,
    1024 * 1024,
    2 * 1024 * 1024,
    4 * 1024 * 1024,
    8 * 1024 * 1024,
    16 * 1024 * 1024,
];

/// Strictly enforces Section 2.1 / 2.2: messages < 7 days old are NEVER eligible for compression.
#[inline]
pub fn is_message_eligible_for_compression(created_at_ms: i64, now_ms: i64) -> bool {
    let age_secs = (now_ms.saturating_sub(created_at_ms)) / 1000;
    age_secs >= MIN_COMPRESS_AGE_SECS
}

/// Automatically select optimal pre-compression chunk target size from 64KB..16MB based on
/// device RAM, CPU, disk speed, platform, and Zstd Level 22 window memory bounds (Section 4.1).
pub fn select_adaptive_chunk_target_bytes(
    profile: &DeviceCapabilityProfile,
    signals: &ResourceSignals,
) -> usize {
    if signals.ram_pressure_pct >= 75 || profile.available_ram_mb < 1024 {
        return 64 * 1024;
    }
    match profile.platform {
        PlatformKind::Web => {
            if profile.available_ram_mb >= 4096 && profile.cpu_cores >= 4 {
                512 * 1024
            } else if profile.available_ram_mb >= 2048 {
                256 * 1024
            } else {
                128 * 1024
            }
        }
        PlatformKind::Android | PlatformKind::Ios => {
            if profile.available_ram_mb >= 4096 && profile.cpu_cores >= 6 {
                512 * 1024
            } else if profile.available_ram_mb >= 2048 {
                256 * 1024
            } else {
                128 * 1024
            }
        }
        PlatformKind::Windows | PlatformKind::Linux | PlatformKind::MacOs => {
            if profile.available_ram_mb >= 8192 && profile.cpu_cores >= 8 && profile.is_ssd {
                2 * 1024 * 1024
            } else if profile.available_ram_mb >= 4096 && profile.cpu_cores >= 4 {
                1024 * 1024
            } else {
                512 * 1024
            }
        }
        PlatformKind::Server => {
            if profile.available_ram_mb >= 16384 && profile.cpu_cores >= 8 {
                4 * 1024 * 1024
            } else {
                1024 * 1024
            }
        }
    }
}

/// Per-tick resource & backlog bounds ensuring zero bulk spikes (Section 3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncrementalTickLimits {
    pub max_messages_per_tick: usize,
    pub max_bytes_per_tick: usize,
    pub max_cpu_ms_per_tick: u32,
    pub max_chunks_per_tick: usize,
    pub max_concurrent_tasks: usize,
    pub target_chunk_bytes: usize,
    pub min_messages_low_volume: usize,
}

impl IncrementalTickLimits {
    pub fn for_device(profile: &DeviceCapabilityProfile, signals: &ResourceSignals) -> Self {
        let target_chunk_bytes = select_adaptive_chunk_target_bytes(profile, signals);
        if profile.is_mobile || profile.platform == PlatformKind::Web {
            Self {
                max_messages_per_tick: 1_500,
                max_bytes_per_tick: target_chunk_bytes.max(256 * 1024),
                max_cpu_ms_per_tick: 60,
                max_chunks_per_tick: 1,
                max_concurrent_tasks: 1,
                target_chunk_bytes,
                min_messages_low_volume: 16,
            }
        } else {
            Self {
                max_messages_per_tick: 4_000,
                max_bytes_per_tick: (target_chunk_bytes * 2).min(4 * 1024 * 1024),
                max_cpu_ms_per_tick: 140,
                max_chunks_per_tick: 2,
                max_concurrent_tasks: 1,
                target_chunk_bytes,
                min_messages_low_volume: 16,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointState {
    Idle,
    Running,
    Paused,
    BackoffWait,
    Completed,
}

/// Persistent crash-recovery & deduplication checkpoint (Section 3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressionCheckpoint {
    pub scope_type: ScopeKind,
    pub scope_id: String,
    pub state: CheckpointState,
    pub last_sealed_seq: i64,
    pub pending_lease_token: Option<i64>,
    pub retry_count: u32,
    pub next_retry_after_ms: i64,
    pub completed_ranges: HashSet<(i64, i64)>,
    pub updated_at_ms: i64,
}

impl CompressionCheckpoint {
    pub fn new(scope_type: ScopeKind, scope_id: impl Into<String>, now_ms: i64) -> Self {
        Self {
            scope_type,
            scope_id: scope_id.into(),
            state: CheckpointState::Idle,
            last_sealed_seq: 0,
            pending_lease_token: None,
            retry_count: 0,
            next_retry_after_ms: 0,
            completed_ranges: HashSet::new(),
            updated_at_ms: now_ms,
        }
    }

    pub fn pause(&mut self, now_ms: i64) {
        self.state = CheckpointState::Paused;
        self.updated_at_ms = now_ms;
    }

    pub fn resume(&mut self, now_ms: i64) {
        if self.state == CheckpointState::Paused {
            self.state = CheckpointState::Idle;
            self.updated_at_ms = now_ms;
        }
    }

    pub fn record_failure_with_backoff(&mut self, now_ms: i64) {
        self.retry_count = self.retry_count.saturating_add(1);
        // Exponential backoff: 15s, 30s, 60s, 120s, capped at 30m
        let delay_secs = (15i64 << self.retry_count.min(7)).min(1800);
        self.next_retry_after_ms = now_ms + delay_secs * 1000;
        self.state = CheckpointState::BackoffWait;
        self.updated_at_ms = now_ms;
    }

    pub fn record_sealed(&mut self, seq_start: i64, seq_end: i64, now_ms: i64) {
        self.completed_ranges.insert((seq_start, seq_end));
        if seq_end > self.last_sealed_seq {
            self.last_sealed_seq = seq_end;
        }
        self.retry_count = 0;
        self.next_retry_after_ms = 0;
        self.pending_lease_token = None;
        self.state = CheckpointState::Idle;
        self.updated_at_ms = now_ms;
    }

    pub fn is_range_already_sealed(&self, seq_start: i64, seq_end: i64) -> bool {
        self.completed_ranges.contains(&(seq_start, seq_end))
    }

    pub fn can_run(&self, now_ms: i64) -> bool {
        match self.state {
            CheckpointState::Paused => false,
            CheckpointState::BackoffWait => now_ms >= self.next_retry_after_ms,
            _ => true,
        }
    }
}

/// Planned single incremental compression batch (never mixes scopes or epochs).
#[derive(Clone, Debug)]
pub struct PlannedChunkBatch {
    pub scope_type: ScopeKind,
    pub scope_id: String,
    pub seq_start: i64,
    pub seq_end: i64,
    pub estimated_raw_bytes: usize,
    pub messages: Vec<CompactMessage>,
}

/// Plan at most `limits.max_chunks_per_tick` small incremental batches from eligible (>= 7d)
/// messages of a single conversation, honoring deduplication checkpoints and low-volume wait.
pub fn plan_incremental_batches(
    scope_type: ScopeKind,
    scope_id: &str,
    ordered_messages: &[CompactMessage],
    now_ms: i64,
    limits: &IncrementalTickLimits,
    checkpoint: &CompressionCheckpoint,
) -> Vec<PlannedChunkBatch> {
    if !checkpoint.can_run(now_ms) {
        return Vec::new();
    }

    // Filter strictly to messages >= 7 days old and strictly after `last_sealed_seq`
    let eligible: Vec<&CompactMessage> = ordered_messages
        .iter()
        .filter(|m| {
            m.seq > checkpoint.last_sealed_seq
                && is_message_eligible_for_compression(m.created_at_ms, now_ms)
        })
        .collect();

    if eligible.is_empty() {
        return Vec::new();
    }

    let mut batches = Vec::new();
    let mut idx = 0usize;
    let mut total_msgs_planned = 0usize;
    let mut total_bytes_planned = 0usize;

    while idx < eligible.len()
        && batches.len() < limits.max_chunks_per_tick
        && total_msgs_planned < limits.max_messages_per_tick
        && total_bytes_planned < limits.max_bytes_per_tick
    {
        let mut chunk_msgs = Vec::new();
        let mut chunk_raw_bytes = 0usize;

        while idx < eligible.len()
            && chunk_raw_bytes < limits.target_chunk_bytes
            && total_msgs_planned + chunk_msgs.len() < limits.max_messages_per_tick
            && total_bytes_planned + chunk_raw_bytes < limits.max_bytes_per_tick
        {
            let m = eligible[idx];
            let est = 64 + m.text.len() + m.extra_meta.as_ref().map_or(0, String::len);
            chunk_msgs.push(m.clone());
            chunk_raw_bytes += est;
            idx += 1;
        }

        if chunk_msgs.is_empty() {
            break;
        }

        // If below target chunk size, only seal when conversation has waited >= LOW_VOLUME_MAX_WAIT_SECS
        // past the 7-day mark (i.e. oldest message >= 9 days) and has enough messages to benefit!
        let reached_target = chunk_raw_bytes >= limits.target_chunk_bytes;
        let oldest_age_secs = (now_ms.saturating_sub(chunk_msgs[0].created_at_ms)) / 1000;
        let waited_long_enough =
            oldest_age_secs >= (MIN_COMPRESS_AGE_SECS + LOW_VOLUME_MAX_WAIT_SECS);
        let enough_low_vol_msgs = chunk_msgs.len() >= limits.min_messages_low_volume
            || oldest_age_secs >= (MIN_COMPRESS_AGE_SECS * 2);

        if !reached_target && !(waited_long_enough && enough_low_vol_msgs) {
            break;
        }

        let seq_start = chunk_msgs.first().map(|m| m.seq).unwrap_or(0);
        let seq_end = chunk_msgs.last().map(|m| m.seq).unwrap_or(0);
        if checkpoint.is_range_already_sealed(seq_start, seq_end) {
            continue;
        }

        total_msgs_planned += chunk_msgs.len();
        total_bytes_planned += chunk_raw_bytes;
        batches.push(PlannedChunkBatch {
            scope_type,
            scope_id: scope_id.to_string(),
            seq_start,
            seq_end,
            estimated_raw_bytes: chunk_raw_bytes,
            messages: chunk_msgs,
        });
    }

    batches
}

/// Apply append-only `ChunkMutation` records (edit, recall, delete, pin, reaction, bookmark)
/// over an immutable decompressed chunk in exact `mutation_seq` order (Section 22).
pub fn apply_mutations_to_chunk(
    messages: &mut Vec<CompactMessage>,
    mutations: &[ChunkMutation],
) -> usize {
    if mutations.is_empty() || messages.is_empty() {
        return 0;
    }
    let mut index_by_id: HashMap<String, usize> = HashMap::with_capacity(messages.len());
    for (i, m) in messages.iter().enumerate() {
        index_by_id.insert(m.id.clone(), i);
    }

    let mut sorted = mutations.to_vec();
    sorted.sort_by_key(|m| m.mutation_seq);

    let mut applied = 0usize;
    for mut_rec in sorted {
        let Some(&pos) = index_by_id.get(&mut_rec.message_id) else {
            continue;
        };
        let msg = &mut messages[pos];
        match mut_rec.op {
            MutationOp::Edit => {
                if let Some(txt) = mut_rec.new_text {
                    if !msg.deleted_for_all {
                        msg.text = txt;
                        msg.is_edited = true;
                        applied += 1;
                    }
                }
            }
            MutationOp::Delete | MutationOp::Recall => {
                msg.deleted_for_all = true;
                msg.text.clear();
                msg.attachments.clear();
                msg.reactions.clear();
                msg.is_pinned = false;
                applied += 1;
            }
            MutationOp::Pin => {
                msg.is_pinned = true;
                applied += 1;
            }
            MutationOp::Unpin => {
                msg.is_pinned = false;
                applied += 1;
            }
            MutationOp::Bookmark => {
                msg.is_bookmarked = true;
                applied += 1;
            }
            MutationOp::Unbookmark => {
                msg.is_bookmarked = false;
                applied += 1;
            }
            MutationOp::ReactionAdd => {
                if let Some(emoji) = mut_rec.emoji {
                    let exists = msg
                        .reactions
                        .iter()
                        .any(|r| r.emoji == emoji && r.user_id == mut_rec.actor_id);
                    if !exists {
                        msg.reactions.push(ReactionEntry {
                            emoji,
                            user_id: mut_rec.actor_id,
                            created_at_ms: mut_rec.created_at_ms,
                        });
                        applied += 1;
                    }
                }
            }
            MutationOp::ReactionRemove => {
                if let Some(emoji) = mut_rec.emoji {
                    let before = msg.reactions.len();
                    msg.reactions
                        .retain(|r| !(r.emoji == emoji && r.user_id == mut_rec.actor_id));
                    if msg.reactions.len() != before {
                        applied += 1;
                    }
                }
            }
        }
    }
    applied
}

/// Determine whether a chunk's mutation/tombstone log warrants bounded compaction
/// (never compacts on every single edit/delete — Section 22).
pub fn should_compact_chunk_mutations(chunk_msg_count: usize, pending_mutations: usize) -> bool {
    if chunk_msg_count == 0 || pending_mutations == 0 {
        return false;
    }
    pending_mutations >= 128 || (pending_mutations * 4 >= chunk_msg_count && pending_mutations >= 16)
}
