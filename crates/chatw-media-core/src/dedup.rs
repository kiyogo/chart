//! Smart Privacy-Safe Deduplication Engine (Section XIX)
//!
//! - Exact cryptographic hashing: BLAKE3 (primary fast hash) + SHA-256 (S3/legacy compatibility).
//! - Perceptual image hashing: 256-bit 2D-DCT PDQ-style fingerprint (inspired by Meta ThreatExchange PDQ, BSD-3-Clause)
//!   with Hamming distance + quality confidence + meme-caption difference guard (NEVER merges memes with different text!).
//! - Video fingerprinting: vPDQ-style multi-keyframe perceptual hash sequence + duration & frame-count guard
//!   (NEVER merges trimmed, cut, or altered videos!).
//! - **E2EE Privacy Guard (`DedupScopePolicy`)**: STRICTLY FORBIDS global plaintext hash lookup or cross-user
//!   deduplication on private E2EE content!

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Cryptographic exact content hashes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentHashes {
    pub blake3_hex: String,
    pub sha256_hex: String,
    pub byte_size: u64,
}

/// 256-bit (32-byte) 2D-DCT PDQ-style perceptual image hash (BSD-3-Clause algorithmic design).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PdqHash256 {
    pub bytes: [u8; 32],
    /// Quality / texture confidence `[0..=100]`. Flat solid images score near 0 and must NOT be perceptually merged.
    pub quality: u8,
    /// Separate high-contrast caption-band fingerprint (`top 20%` + `bottom 20%`) to prevent merging two memes with different text!
    pub caption_band_hash: u64,
}

impl PdqHash256 {
    pub fn to_hex(&self) -> String {
        hex::encode(self.bytes)
    }

    pub fn from_hex(s: &str, quality: u8, caption_band_hash: u64) -> Option<Self> {
        let v = hex::decode(s.trim()).ok()?;
        if v.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&v);
        Some(Self {
            bytes,
            quality,
            caption_band_hash,
        })
    }

    /// Hamming distance in `[0..=256]` across all 256 perceptual DCT bits.
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        self.bytes
            .iter()
            .zip(other.bytes.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum()
    }

    pub fn caption_band_distance(&self, other: &Self) -> u32 {
        (self.caption_band_hash ^ other.caption_band_hash).count_ones()
    }
}

/// Result of comparing two perceptual image candidates under ChatW's safety policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerceptualMatchVerdict {
    /// Identical or visually equivalent re-encode / metadata-stripped / resized copy (safe candidate).
    EquivalentCandidate { hamming: u32 },
    /// Visually similar template (e.g., same meme background) but caption/text differs — MUST NOT MERGE!
    MemeOrTextDiffers { hamming: u32, caption_hamming: u32 },
    /// Low-texture / blank image where perceptual hash has insufficient entropy — MUST NOT MERGE!
    InsufficientTexture,
    /// Distinct images.
    Distinct { hamming: u32 },
}

/// Multi-keyframe vPDQ-style video fingerprint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoFingerprint {
    pub duration_ms: u64,
    pub frame_count: u32,
    pub width: u32,
    pub height: u32,
    pub keyframe_hashes: Vec<PdqHash256>,
}

/// Privacy-enforced deduplication scope policy (Section XIX.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DedupScopePolicy {
    /// Public/shared non-E2EE scope (e.g. group/channel/sticker/library where server already stores plaintext).
    SharedNonE2ee,
    /// Forwarded authorized ciphertext object (same encrypted object & wrapped key shared explicitly by sender).
    ForwardedCiphertext,
    /// Private E2EE content — STRICTLY ISOLATED! Global plaintext hash lookup is FORBIDDEN.
    PrivateE2eeIsolated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DedupLookupDecision {
    pub allow_exact_global_lookup: bool,
    pub allow_perceptual_lookup: bool,
    pub cas_scope_key: String,
    pub reason: &'static str,
}

pub fn hash_bytes_exact(bytes: &[u8]) -> ContentHashes {
    let b3 = blake3::hash(bytes).to_hex().to_string();
    let mut sha = Sha256::new();
    sha.update(bytes);
    let sha_hex = hex::encode(sha.finalize());
    ContentHashes {
        blake3_hex: b3,
        sha256_hex: sha_hex,
        byte_size: bytes.len() as u64,
    }
}

/// Enforce Section XIX.6 E2EE deduplication safety rules unconditionally!
pub fn evaluate_dedup_policy(
    scope_type: &str,
    owner_id: &str,
    scope_id: Option<&str>,
    is_e2ee: bool,
    is_authorized_forward: bool,
    cipher_blake3_hex: &str,
) -> DedupLookupDecision {
    if is_e2ee {
        if is_authorized_forward {
            return DedupLookupDecision {
                allow_exact_global_lookup: false,
                allow_perceptual_lookup: false,
                cas_scope_key: format!("fwd_e2ee:{owner_id}:{cipher_blake3_hex}"),
                reason: "authorized_e2ee_forward_reference_reuse_only",
            };
        }
        // NEVER use global plaintext hash or cross-user lookup for private E2EE!
        let sid = scope_id.unwrap_or(owner_id);
        return DedupLookupDecision {
            allow_exact_global_lookup: false,
            allow_perceptual_lookup: false,
            cas_scope_key: format!("priv_e2ee:{owner_id}:{sid}:{cipher_blake3_hex}"),
            reason: "e2ee_strict_privacy_isolation_no_global_dedup",
        };
    }

    if matches!(scope_type, "group" | "channel" | "library" | "sticker") {
        return DedupLookupDecision {
            allow_exact_global_lookup: true,
            allow_perceptual_lookup: true,
            cas_scope_key: format!("shared:{cipher_blake3_hex}"),
            reason: "shared_non_e2ee_cas_and_perceptual_dedup_allowed",
        };
    }

    DedupLookupDecision {
        allow_exact_global_lookup: false,
        allow_perceptual_lookup: false,
        cas_scope_key: format!("scoped:{scope_type}:{owner_id}:{cipher_blake3_hex}"),
        reason: "private_scope_isolated",
    }
}

/// Compute 256-bit 2D-DCT PDQ-style perceptual hash + 64-bit caption-band dHash from RGBA8 pixels.
pub fn compute_pdq_hash_rgba(rgba: &[u8], width: u32, height: u32) -> PdqHash256 {
    let w = width as usize;
    let h = height as usize;
    if w < 8 || h < 8 || rgba.len() < w * h * 4 {
        return PdqHash256 {
            bytes: [0u8; 32],
            quality: 0,
            caption_band_hash: 0,
        };
    }

    // 1. Area-average downsample to 64x64 luma grid
    const N: usize = 64;
    let mut grid = [0.0f64; N * N];
    for gy in 0..N {
        let y0 = (gy * h) / N;
        let y1 = (((gy + 1) * h) / N).max(y0 + 1).min(h);
        for gx in 0..N {
            let x0 = (gx * w) / N;
            let x1 = (((gx + 1) * w) / N).max(x0 + 1).min(w);
            let mut sum = 0.0f64;
            let mut cnt = 0.0f64;
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y * w + x) * 4;
                    sum += 0.299 * (rgba[i] as f64) + 0.587 * (rgba[i + 1] as f64) + 0.114 * (rgba[i + 2] as f64);
                    cnt += 1.0;
                }
            }
            grid[gy * N + gx] = sum / cnt.max(1.0);
        }
    }

    // 2. Separable 2D-DCT keeping low-frequency 16x16 AC coefficients (u = 1..=16, v = 1..=16 -> 256 bits)
    const K: usize = 16;
    let mut cos_table = [[0.0f64; N]; K];
    for u in 0..K {
        let freq = (u + 1) as f64;
        for x in 0..N {
            cos_table[u][x] = (std::f64::consts::PI * freq * ((x as f64) + 0.5) / (N as f64)).cos();
        }
    }

    // Horizontal pass: 64 rows -> 16 columns
    let mut temp = [0.0f64; N * K];
    for y in 0..N {
        for u in 0..K {
            let mut s = 0.0;
            for x in 0..N {
                s += grid[y * N + x] * cos_table[u][x];
            }
            temp[y * K + u] = s;
        }
    }

    // Vertical pass: 16x16 DCT matrix
    let mut dct = [0.0f64; K * K];
    for v in 0..K {
        for u in 0..K {
            let mut s = 0.0;
            for y in 0..N {
                s += temp[y * K + u] * cos_table[v][y];
            }
            dct[v * K + u] = s;
        }
    }

    // 3. Median thresholding -> 256-bit hash
    let mut sorted = dct;
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = 0.5 * (sorted[127] + sorted[128]);
    let mut bytes = [0u8; 32];
    for (idx, &val) in dct.iter().enumerate() {
        if val > median {
            bytes[idx / 8] |= 1u8 << (idx % 8);
        }
    }

    // 4. Gradient-energy quality metric [0..=100]
    let mut grad_sum = 0.0;
    for y in 0..(N - 1) {
        for x in 0..(N - 1) {
            grad_sum += (grid[y * N + x] - grid[y * N + x + 1]).abs()
                + (grid[y * N + x] - grid[(y + 1) * N + x]).abs();
        }
    }
    let avg_grad = grad_sum / (((N - 1) * (N - 1) * 2) as f64);
    let quality = (avg_grad * 4.5).clamp(0.0, 100.0) as u8;

    // 5. High-contrast caption-band 64-bit dHash (top 20% + bottom 20% of 64x64 grid: 4 rows top + 4 rows bottom x 8 cols)
    let mut caption_hash = 0u64;
    let mut bit = 0u32;
    let band_rows = [2usize, 5, 8, 11, 52, 55, 58, 61];
    for &ry in &band_rows {
        for cx in 0..8 {
            let x0 = cx * 8;
            let left: f64 = (0..4).map(|d| grid[ry * N + x0 + d]).sum();
            let right: f64 = (4..8).map(|d| grid[ry * N + x0 + d]).sum();
            if left > right {
                caption_hash |= 1u64 << bit;
            }
            bit += 1;
        }
    }

    PdqHash256 {
        bytes,
        quality,
        caption_band_hash: caption_hash,
    }
}

/// Compare two perceptual image hashes safely (Section XIX.4):
/// - Rejects low-texture solid frames
/// - Rejects meme templates whose top/bottom captions changed!
/// - Refuses replacing a higher-resolution / higher-quality source with a lower-quality candidate!
pub fn compare_perceptual_images(
    incoming: &PdqHash256,
    existing: &PdqHash256,
    incoming_pixels: u64,
    existing_pixels: u64,
) -> PerceptualMatchVerdict {
    if incoming.quality < 18 || existing.quality < 18 {
        return PerceptualMatchVerdict::InsufficientTexture;
    }
    let hamming = incoming.hamming_distance(existing);
    if hamming > 31 {
        return PerceptualMatchVerdict::Distinct { hamming };
    }
    let cap_dist = incoming.caption_band_distance(existing);
    // Strict caption-band check prevents false-positive merging of two memes with different captions!
    if cap_dist > 10 {
        return PerceptualMatchVerdict::MemeOrTextDiffers {
            hamming,
            caption_hamming: cap_dist,
        };
    }
    // Never downgrade quality: only reuse existing object if existing resolution is >= 90% of incoming
    if existing_pixels * 10 < incoming_pixels * 9 {
        return PerceptualMatchVerdict::Distinct { hamming };
    }
    PerceptualMatchVerdict::EquivalentCandidate { hamming }
}

/// Build vPDQ-style multi-keyframe video fingerprint.
pub fn compute_video_fingerprint(
    keyframes_rgba: &[&[u8]],
    width: u32,
    height: u32,
    duration_ms: u64,
    frame_count: u32,
) -> VideoFingerprint {
    let keyframe_hashes = keyframes_rgba
        .iter()
        .take(16)
        .map(|f| compute_pdq_hash_rgba(f, width, height))
        .collect();
    VideoFingerprint {
        duration_ms,
        frame_count,
        width,
        height,
        keyframe_hashes,
    }
}

/// Compare two video fingerprints conservatively (Section XIX.5):
/// Never merges trimmed/cut/edited videos (duration must match within 3% and >= 90% of keyframes must match!).
pub fn videos_are_safe_dedup_equivalent(a: &VideoFingerprint, b: &VideoFingerprint) -> bool {
    if a.keyframe_hashes.is_empty() || a.keyframe_hashes.len() != b.keyframe_hashes.len() {
        return false;
    }
    let dur_diff = a.duration_ms.abs_diff(b.duration_ms);
    let max_dur = a.duration_ms.max(b.duration_ms).max(1);
    if (dur_diff * 100) / max_dur > 3 {
        return false; // Trimmed or lengthened video!
    }
    // Never replace higher resolution with lower resolution
    if (b.width as u64) * (b.height as u64) * 10 < (a.width as u64) * (a.height as u64) * 9 {
        return false;
    }
    let matched = a
        .keyframe_hashes
        .iter()
        .zip(b.keyframe_hashes.iter())
        .filter(|(ha, hb)| ha.quality >= 15 && hb.quality >= 15 && ha.hamming_distance(hb) <= 28)
        .count();
    matched * 10 >= a.keyframe_hashes.len() * 9
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_textured_scene(w: u32, h: u32, caption_variant: u8) -> Vec<u8> {
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                let wave = (((x as f32 * 0.08).sin() + (y as f32 * 0.07).cos()) * 60.0 + 128.0) as u8;
                rgba[i] = wave;
                rgba[i + 1] = (((x + y) / 2) % 255) as u8;
                rgba[i + 2] = (255u8).saturating_sub(wave / 2);
                rgba[i + 3] = 255;
            }
        }
        // Caption bar at top
        for y in 4..(h / 6) {
            for x in 8..(w - 8) {
                let i = ((y * w + x) * 4) as usize;
                let stripe = if caption_variant == 1 { (x / 6) % 2 == 0 } else { ((x + 15) / 11) % 3 == 0 };
                if stripe {
                    rgba[i] = 255;
                    rgba[i + 1] = 255;
                    rgba[i + 2] = 255;
                } else {
                    rgba[i] = 10;
                    rgba[i + 1] = 10;
                    rgba[i + 2] = 10;
                }
            }
        }
        rgba
    }

    #[test]
    fn pdq_matches_slightly_reencoded_image_and_protects_different_meme_captions() {
        let img1 = make_textured_scene(128, 128, 1);
        // Slightly perturbed pixels (simulating AVIF/JPEG re-encode)
        let img1_reenc: Vec<u8> = img1
            .iter()
            .enumerate()
            .map(|(i, &b)| if i % 4 == 3 { 255 } else { b.saturating_add(3) })
            .collect();
        let h1 = compute_pdq_hash_rgba(&img1, 128, 128);
        let h1_reenc = compute_pdq_hash_rgba(&img1_reenc, 128, 128);
        assert!(matches!(
            compare_perceptual_images(&h1, &h1_reenc, 128 * 128, 128 * 128),
            PerceptualMatchVerdict::EquivalentCandidate { .. }
        ));

        // Different caption band must NOT merge!
        let img2 = make_textured_scene(128, 128, 2);
        let h2 = compute_pdq_hash_rgba(&img2, 128, 128);
        assert!(!matches!(
            compare_perceptual_images(&h1, &h2, 128 * 128, 128 * 128),
            PerceptualMatchVerdict::EquivalentCandidate { .. }
        ));
    }

    #[test]
    fn e2ee_dedup_policy_forbids_global_plaintext_lookup_strictly() {
        let dec = evaluate_dedup_policy("dm", "user-a", Some("user-b"), true, false, "deadbeef");
        assert!(!dec.allow_exact_global_lookup);
        assert!(!dec.allow_perceptual_lookup);
        assert!(dec.cas_scope_key.starts_with("priv_e2ee:user-a:user-b:"));

        let pub_dec = evaluate_dedup_policy("channel", "user-a", Some("ch-1"), false, false, "deadbeef");
        assert!(pub_dec.allow_exact_global_lookup);
        assert!(pub_dec.allow_perceptual_lookup);
    }
}
