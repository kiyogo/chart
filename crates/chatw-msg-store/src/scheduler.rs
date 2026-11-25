//! Unified Device Resource Scheduler & Privacy-Safe Telemetry Collector (Sections 19 & 32).
//!
//! Strictly prioritizes interactive messaging, voice/video calls, and UI over background
//! indexing, Zstd-22 compression, prefetching, and cache maintenance.

use crate::model::{DeviceCapabilityProfile, ResourceSignals, TelemetrySnapshot};
use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;
use std::cmp::Ordering;

/// Strict 10-level priority hierarchy matching Section 19 (lower numeric rank = higher priority).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TaskPriority {
    MessageRealtime = 1,
    VoiceVideoCall = 2,
    UiRender = 3,
    ActiveReadDecompress = 4,
    SearchQuery = 5,
    EssentialSync = 6,
    Prefetch = 7,
    BackgroundIndex = 8,
    BackgroundCompress = 9,
    CacheCleanupMaintenance = 10,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub task_id: String,
    pub priority: TaskPriority,
    pub enqueued_at_ms: i64,
    pub estimated_cpu_ms: u32,
    pub estimated_ram_bytes: u64,
    pub scope_key: Option<String>,
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order because Rust's BinaryHeap is a max-heap
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| other.enqueued_at_ms.cmp(&self.enqueued_at_ms))
            .then_with(|| other.task_id.cmp(&self.task_id))
    }
}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Resource budget computed for a single background execution window.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeResourceBudget {
    pub cpu_budget_ms_per_tick: u32,
    pub ram_budget_bytes: u64,
    pub max_concurrent_background: usize,
    pub network_budget_kbps: u32,
    pub allow_compression: bool,
    pub allow_prefetch: bool,
    pub allow_indexing: bool,
    pub pause_reason: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ResourceScheduler {
    queue: BinaryHeap<ScheduledTask>,
    cancelled_scopes: std::collections::HashSet<String>,
    pub telemetry: TelemetrySnapshot,
}

impl ResourceScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute safe CPU, RAM, Thread, and Network budgets given device hardware & live signals.
    pub fn compute_budget(
        profile: &DeviceCapabilityProfile,
        signals: &ResourceSignals,
    ) -> RuntimeResourceBudget {
        // Hard suspension of heavy background tasks during calls, thermal throttling, or extreme RAM pressure
        if signals.active_voice_or_video_call {
            return RuntimeResourceBudget {
                cpu_budget_ms_per_tick: 10,
                ram_budget_bytes: 32 * 1024 * 1024,
                max_concurrent_background: 0,
                network_budget_kbps: 0,
                allow_compression: false,
                allow_prefetch: false,
                allow_indexing: false,
                pause_reason: Some("active_voice_or_video_call".into()),
            };
        }
        if signals.thermal_throttled {
            return RuntimeResourceBudget {
                cpu_budget_ms_per_tick: 10,
                ram_budget_bytes: 32 * 1024 * 1024,
                max_concurrent_background: 0,
                network_budget_kbps: 256,
                allow_compression: false,
                allow_prefetch: false,
                allow_indexing: false,
                pause_reason: Some("thermal_throttled".into()),
            };
        }
        let low_battery = signals.on_battery_saver
            || signals.battery_level_pct.is_some_and(|pct| pct < 20);
        if signals.ram_pressure_pct >= 88 {
            return RuntimeResourceBudget {
                cpu_budget_ms_per_tick: 15,
                ram_budget_bytes: 24 * 1024 * 1024,
                max_concurrent_background: 0,
                network_budget_kbps: 512,
                allow_compression: false,
                allow_prefetch: false,
                allow_indexing: false,
                pause_reason: Some("high_ram_pressure".into()),
            };
        }

        let base_cpu_ms: u32 = if profile.is_mobile {
            if signals.user_actively_scrolling_or_typing {
                12
            } else {
                45
            }
        } else if signals.user_actively_scrolling_or_typing {
            20
        } else {
            110
        };

        let ram_budget_bytes = ((profile.available_ram_mb as u64) * 1024 * 1024 / 12)
            .clamp(24 * 1024 * 1024, 256 * 1024 * 1024);

        let max_concurrent_background = if low_battery || profile.is_mobile || profile.cpu_cores <= 2
        {
            1
        } else {
            2
        };

        let allow_compression = !low_battery && !signals.user_actively_scrolling_or_typing;
        let allow_prefetch = !signals.is_offline
            && !signals.data_saver_enabled
            && !low_battery
            && !(signals.on_metered_or_cellular && signals.estimated_bandwidth_kbps < 1500);

        let network_budget_kbps = if signals.is_offline || signals.data_saver_enabled {
            0
        } else if signals.on_metered_or_cellular {
            512
        } else if signals.estimated_bandwidth_kbps > 0 {
            (signals.estimated_bandwidth_kbps / 3).clamp(512, 20_000)
        } else {
            4_000
        };

        RuntimeResourceBudget {
            cpu_budget_ms_per_tick: if low_battery {
                base_cpu_ms / 2
            } else {
                base_cpu_ms
            },
            ram_budget_bytes,
            max_concurrent_background,
            network_budget_kbps,
            allow_compression,
            allow_prefetch,
            allow_indexing: !signals.user_actively_scrolling_or_typing,
            pause_reason: if low_battery {
                Some("low_battery_or_saver".into())
            } else {
                None
            },
        }
    }

    pub fn enqueue(&mut self, task: ScheduledTask) {
        if let Some(ref sc) = task.scope_key {
            self.cancelled_scopes.remove(sc);
        }
        self.queue.push(task);
        self.telemetry.compression_queue_size = self
            .queue
            .iter()
            .filter(|t| t.priority == TaskPriority::BackgroundCompress)
            .count() as u64;
    }

    /// Cancel all pending low-priority prefetch/search tasks belonging to a left conversation.
    pub fn cancel_scope(&mut self, scope_key: &str) {
        self.cancelled_scopes.insert(scope_key.to_string());
    }

    /// Pop next runnable task strictly honoring priority and live resource budget.
    pub fn pop_next_allowed(&mut self, budget: &RuntimeResourceBudget) -> Option<ScheduledTask> {
        let mut deferred = Vec::new();
        let mut chosen = None;

        while let Some(task) = self.queue.pop() {
            if let Some(ref sc) = task.scope_key {
                if self.cancelled_scopes.contains(sc) && task.priority >= TaskPriority::Prefetch {
                    continue;
                }
            }
            let allowed = match task.priority {
                TaskPriority::BackgroundCompress => {
                    budget.allow_compression && task.estimated_ram_bytes <= budget.ram_budget_bytes
                }
                TaskPriority::Prefetch => budget.allow_prefetch,
                TaskPriority::BackgroundIndex => budget.allow_indexing,
                TaskPriority::CacheCleanupMaintenance => budget.max_concurrent_background > 0,
                _ => true,
            };
            if allowed {
                chosen = Some(task);
                break;
            } else {
                deferred.push(task);
            }
        }
        for t in deferred {
            self.queue.push(t);
        }
        self.telemetry.compression_queue_size = self
            .queue
            .iter()
            .filter(|t| t.priority == TaskPriority::BackgroundCompress)
            .count() as u64;
        chosen
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
