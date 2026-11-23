//! Adaptive Bitrate (ABR) Streaming Selector, E2EE Segment Scheduler, and Smart Prefetch Planner
//! (Sections XIV & XXXI).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamVariantDescriptor {
    pub variant_id: String,
    pub object_key: String,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub bitrate_kbps: u32,
    pub total_bytes: u64,
    pub segment_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamSegmentDescriptor {
    pub index: u32,
    pub byte_offset: u64,
    pub cipher_len: u32,
    pub plain_len: u32,
    pub start_ms: u64,
    pub duration_ms: u64,
    pub is_keyframe: bool,
    pub blake3_hex: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaStreamManifest {
    pub version: u8,
    pub media_id: String,
    pub mime_type: String,
    pub duration_ms: u64,
    pub is_e2ee: bool,
    pub default_variant_id: String,
    pub variants: Vec<StreamVariantDescriptor>,
    pub segments: Vec<StreamSegmentDescriptor>,
}

/// Network & playback state fed into the EWMA Adaptive Bitrate selector.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbrPlaybackState {
    pub estimated_bandwidth_kbps: f64,
    pub buffer_seconds: f64,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub av1_decode_supported: bool,
    pub low_power_mode: bool,
    pub data_saver_mode: bool,
}

/// EWMA throughput estimator with hysteresis to prevent ABR oscillation.
#[derive(Clone, Debug)]
pub struct BandwidthEstimator {
    ewma_kbps: f64,
    alpha: f64,
    samples: u32,
}

impl BandwidthEstimator {
    pub fn new(initial_kbps: f64) -> Self {
        Self {
            ewma_kbps: initial_kbps.max(64.0),
            alpha: 0.30,
            samples: 0,
        }
    }

    pub fn record_download(&mut self, bytes: u64, elapsed_ms: u64) {
        if bytes == 0 || elapsed_ms == 0 {
            return;
        }
        let kbps = ((bytes as f64) * 8.0) / (elapsed_ms as f64);
        if self.samples == 0 {
            self.ewma_kbps = kbps;
        } else {
            self.ewma_kbps = self.alpha * kbps + (1.0 - self.alpha) * self.ewma_kbps;
        }
        self.samples = self.samples.saturating_add(1);
    }

    pub fn estimate_kbps(&self) -> f64 {
        self.ewma_kbps
    }
}

/// Pick the best variant given current bandwidth, buffer health, viewport, power & data-saver settings.
pub fn select_abr_variant<'a>(
    variants: &'a [StreamVariantDescriptor],
    state: &AbrPlaybackState,
) -> Option<&'a StreamVariantDescriptor> {
    if variants.is_empty() {
        return None;
    }
    let mut sorted: Vec<&StreamVariantDescriptor> = variants.iter().collect();
    sorted.sort_by_key(|v| v.bitrate_kbps);

    // Safety factor: require 25% headroom above variant bitrate, or 55% when buffer is low (< 4s)
    let safety = if state.buffer_seconds < 4.0 {
        0.55
    } else if state.data_saver_mode || state.low_power_mode {
        0.50
    } else {
        0.78
    };
    let effective_budget = state.estimated_bandwidth_kbps * safety;

    // Never pick a 4K rendition on a 720p viewport when a closer fit exists
    let max_useful_height = if state.data_saver_mode {
        480
    } else if state.low_power_mode {
        720
    } else {
        state.viewport_height.max(360).saturating_mul(2)
    };

    let mut chosen = sorted[0];
    for v in &sorted {
        let fits_bw = (v.bitrate_kbps as f64) <= effective_budget;
        let fits_screen = v.height <= max_useful_height || v.height == sorted[0].height;
        if fits_bw && fits_screen {
            chosen = v;
        }
    }
    Some(chosen)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrefetchContext {
    pub is_wifi: bool,
    pub data_saver_mode: bool,
    pub battery_percent: u8,
    pub cache_free_bytes: u64,
    pub auto_download_mode: String, // "always" | "wifi" | "manual"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrefetchDecision {
    PrefetchThumbnailOnly,
    PrefetchFullImage,
    PrefetchNextSegments { count: u32 },
    Skip,
}

/// Smart Prefetch Planner (Section XXXI):
/// Never auto-downloads large videos on scroll; respects Wi-Fi/data-saver/battery/cache headroom.
pub fn decide_prefetch(
    kind: crate::classifier::MediaKind,
    byte_size: u64,
    is_visible_in_viewport: bool,
    is_actively_playing: bool,
    ctx: &PrefetchContext,
) -> PrefetchDecision {
    if ctx.auto_download_mode == "manual" && !is_actively_playing {
        return PrefetchDecision::Skip;
    }
    if ctx.data_saver_mode || ctx.battery_percent < 15 || ctx.cache_free_bytes < 16 * 1024 * 1024 {
        return if is_actively_playing {
            PrefetchDecision::PrefetchNextSegments { count: 1 }
        } else if is_visible_in_viewport && matches!(kind, crate::classifier::MediaKind::Image) {
            PrefetchDecision::PrefetchThumbnailOnly
        } else {
            PrefetchDecision::Skip
        };
    }

    match kind {
        crate::classifier::MediaKind::Video => {
            if is_actively_playing {
                PrefetchDecision::PrefetchNextSegments {
                    count: if ctx.is_wifi { 2 } else { 1 },
                }
            } else if is_visible_in_viewport {
                PrefetchDecision::PrefetchThumbnailOnly
            } else {
                PrefetchDecision::Skip
            }
        }
        crate::classifier::MediaKind::Image | crate::classifier::MediaKind::AnimatedImage => {
            if !is_visible_in_viewport {
                return PrefetchDecision::Skip;
            }
            let max_auto = if ctx.is_wifi { 4 * 1024 * 1024 } else { 750 * 1024 };
            if byte_size <= max_auto && (ctx.auto_download_mode == "always" || ctx.is_wifi) {
                PrefetchDecision::PrefetchFullImage
            } else {
                PrefetchDecision::PrefetchThumbnailOnly
            }
        }
        crate::classifier::MediaKind::Audio => {
            // Short voice notes prefetch rapidly when visible; long audio streams on play
            if is_actively_playing {
                PrefetchDecision::PrefetchNextSegments { count: 2 }
            } else if is_visible_in_viewport && byte_size <= 512 * 1024 {
                PrefetchDecision::PrefetchFullImage
            } else {
                PrefetchDecision::Skip
            }
        }
        _ => PrefetchDecision::Skip, // Never auto-download arbitrary unopened files!
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abr_selects_higher_variant_on_fast_network_and_drops_on_low_buffer() {
        let vars = vec![
            StreamVariantDescriptor {
                variant_id: "av1_360p".into(),
                object_key: "k1".into(),
                codec: "av01.0.01M.08".into(),
                width: 640,
                height: 360,
                bitrate_kbps: 350,
                total_bytes: 1_000_000,
                segment_count: 10,
            },
            StreamVariantDescriptor {
                variant_id: "av1_720p".into(),
                object_key: "k2".into(),
                codec: "av01.0.04M.08".into(),
                width: 1280,
                height: 720,
                bitrate_kbps: 1100,
                total_bytes: 3_000_000,
                segment_count: 10,
            },
            StreamVariantDescriptor {
                variant_id: "av1_1080p".into(),
                object_key: "k3".into(),
                codec: "av01.0.08M.08".into(),
                width: 1920,
                height: 1080,
                bitrate_kbps: 2600,
                total_bytes: 7_000_000,
                segment_count: 10,
            },
        ];
        let fast = AbrPlaybackState {
            estimated_bandwidth_kbps: 5000.0,
            buffer_seconds: 12.0,
            viewport_width: 1920,
            viewport_height: 1080,
            av1_decode_supported: true,
            low_power_mode: false,
            data_saver_mode: false,
        };
        assert_eq!(select_abr_variant(&vars, &fast).unwrap().variant_id, "av1_1080p");

        let slow = AbrPlaybackState {
            estimated_bandwidth_kbps: 900.0,
            buffer_seconds: 2.0,
            ..fast
        };
        assert_eq!(select_abr_variant(&vars, &slow).unwrap().variant_id, "av1_360p");
    }

    #[test]
    fn prefetch_never_auto_downloads_large_video_just_for_scrolling() {
        let ctx = PrefetchContext {
            is_wifi: true,
            data_saver_mode: false,
            battery_percent: 85,
            cache_free_bytes: 512 * 1024 * 1024,
            auto_download_mode: "wifi".into(),
        };
        assert_eq!(
            decide_prefetch(crate::classifier::MediaKind::Video, 40 * 1024 * 1024, true, false, &ctx),
            PrefetchDecision::PrefetchThumbnailOnly
        );
        assert_eq!(
            decide_prefetch(crate::classifier::MediaKind::Video, 40 * 1024 * 1024, true, true, &ctx),
            PrefetchDecision::PrefetchNextSegments { count: 2 }
        );
    }
}
