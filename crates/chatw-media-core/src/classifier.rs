//! Real magic-byte media sniffer, Shannon entropy analyzer, 10-class image analyzer,
//! and adaptive resolution policy (no blanket 1920px clamp!).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    AnimatedImage,
    Video,
    Audio,
    TextStructured,
    ArchiveAlreadyCompressed,
    Document,
    Binary,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SniffedFormat {
    pub ext: &'static str,
    pub mime: &'static str,
    pub kind: MediaKind,
    pub already_compressed: bool,
}

/// Rich 10-class image content taxonomy driving Adaptive AVIF encoding & AI Model Routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageClass {
    Photo,
    Portrait,
    CrispTextUi,
    AnimeManga,
    Meme,
    AlphaGraphic,
    FlatGraphic,
    LowLightNoisy,
    HdrWideGamut,
    AlreadyHeavilyCompressed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageAnalysis {
    pub width: u32,
    pub height: u32,
    pub class: ImageClass,
    pub has_alpha: bool,
    pub is_hdr_candidate: bool,
    pub sharp_edge_ratio: f32,
    pub flat_pixel_ratio: f32,
    pub noise_estimate: f32,
    pub mean_luma: f32,
    pub skin_tone_ratio: f32,
    pub color_saturation_mean: f32,
    pub palette_sparsity: f32,
    pub source_bpp: f32,
    pub has_text_risk: bool,
    pub recommended_max_edge: Option<u32>,
}

/// Real magic-byte detection — NEVER trusts client Content-Type or filename extension blindly.
pub fn sniff_media_bytes(bytes: &[u8]) -> SniffedFormat {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        let is_apng = bytes.windows(4).take(4096).any(|w| w == b"acTL");
        return SniffedFormat {
            ext: "png",
            mime: if is_apng { "image/apng" } else { "image/png" },
            kind: if is_apng { MediaKind::AnimatedImage } else { MediaKind::Image },
            already_compressed: false,
        };
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return SniffedFormat {
            ext: "jpg",
            mime: "image/jpeg",
            kind: MediaKind::Image,
            already_compressed: true,
        };
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return SniffedFormat {
            ext: "gif",
            mime: "image/gif",
            kind: MediaKind::AnimatedImage,
            already_compressed: true,
        };
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        let is_anim = bytes.windows(4).take(4096).any(|w| w == b"ANIM");
        return SniffedFormat {
            ext: "webp",
            mime: "image/webp",
            kind: if is_anim { MediaKind::AnimatedImage } else { MediaKind::Image },
            already_compressed: true,
        };
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"avif" || brand == b"avis" {
            return SniffedFormat {
                ext: "avif",
                mime: "image/avif",
                kind: if brand == b"avis" { MediaKind::AnimatedImage } else { MediaKind::Image },
                already_compressed: true,
            };
        }
        if matches!(brand, b"heic" | b"heix" | b"mif1") {
            return SniffedFormat {
                ext: "heic",
                mime: "image/heic",
                kind: MediaKind::Image,
                already_compressed: true,
            };
        }
        if matches!(brand, b"isom" | b"iso2" | b"iso5" | b"iso6" | b"mp41" | b"mp42" | b"M4V " | b"qt  " | b"avc1" | b"av01" | b"cmfc" | b"dash") {
            return SniffedFormat {
                ext: "mp4",
                mime: "video/mp4",
                kind: MediaKind::Video,
                already_compressed: true,
            };
        }
        if brand == b"M4A " {
            return SniffedFormat {
                ext: "m4a",
                mime: "audio/mp4",
                kind: MediaKind::Audio,
                already_compressed: true,
            };
        }
    }
    if bytes.starts_with(b"DKIF") && bytes.len() >= 12 && &bytes[8..12] == b"AV01" {
        return SniffedFormat {
            ext: "ivf",
            mime: "video/x-ivf",
            kind: MediaKind::Video,
            already_compressed: true,
        };
    }
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        let head = &bytes[..bytes.len().min(8192)];
        let has_video = head.windows(5).any(|w| matches!(w, b"V_AV1" | b"V_VP9" | b"V_VP8" | b"V_MPE"));
        let is_audio_only = !has_video && (head.windows(4).any(|w| w == b"opus") || head.windows(6).any(|w| w == b"A_OPUS"));
        return SniffedFormat {
            ext: "webm",
            mime: if is_audio_only { "audio/webm" } else { "video/webm" },
            kind: if is_audio_only { MediaKind::Audio } else { MediaKind::Video },
            already_compressed: true,
        };
    }
    if bytes.starts_with(b"OggS") {
        return SniffedFormat {
            ext: "ogg",
            mime: "audio/ogg",
            kind: MediaKind::Audio,
            already_compressed: true,
        };
    }
    if bytes.starts_with(b"fLaC") {
        return SniffedFormat {
            ext: "flac",
            mime: "audio/flac",
            kind: MediaKind::Audio,
            already_compressed: true,
        };
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return SniffedFormat {
            ext: "wav",
            mime: "audio/wav",
            kind: MediaKind::Audio,
            already_compressed: false,
        };
    }
    if bytes.starts_with(b"ID3") || (bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0) {
        return SniffedFormat {
            ext: "mp3",
            mime: "audio/mpeg",
            kind: MediaKind::Audio,
            already_compressed: true,
        };
    }
    if bytes.starts_with(b"%PDF-") {
        return SniffedFormat {
            ext: "pdf",
            mime: "application/pdf",
            kind: MediaKind::Document,
            already_compressed: false, // PDFs can hold uncompressed streams; file compressor probes actual gain
        };
    }
    if bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]) || bytes.starts_with(&[0x50, 0x4B, 0x05, 0x06]) {
        // Distinguish DOCX/XLSX/PPTX vs plain ZIP
        let head = &bytes[..bytes.len().min(4096)];
        if head.windows(5).any(|w| w == b"word/") || head.windows(18).any(|w| w == b"[Content_Types].xm") {
            return SniffedFormat {
                ext: "docx",
                mime: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                kind: MediaKind::Document,
                already_compressed: true,
            };
        }
        return SniffedFormat {
            ext: "zip",
            mime: "application/zip",
            kind: MediaKind::ArchiveAlreadyCompressed,
            already_compressed: true,
        };
    }
    if bytes.starts_with(b"Rar!\x1A\x07") {
        return SniffedFormat {
            ext: "rar",
            mime: "application/vnd.rar",
            kind: MediaKind::ArchiveAlreadyCompressed,
            already_compressed: true,
        };
    }
    if bytes.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
        return SniffedFormat {
            ext: "7z",
            mime: "application/x-7z-compressed",
            kind: MediaKind::ArchiveAlreadyCompressed,
            already_compressed: true,
        };
    }
    if bytes.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        return SniffedFormat {
            ext: "zst",
            mime: "application/zstd",
            kind: MediaKind::ArchiveAlreadyCompressed,
            already_compressed: true,
        };
    }
    if bytes.starts_with(&[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00]) {
        return SniffedFormat {
            ext: "xz",
            mime: "application/x-xz",
            kind: MediaKind::ArchiveAlreadyCompressed,
            already_compressed: true,
        };
    }
    if bytes.starts_with(&[0x1F, 0x8B]) {
        return SniffedFormat {
            ext: "gz",
            mime: "application/gzip",
            kind: MediaKind::ArchiveAlreadyCompressed,
            already_compressed: true,
        };
    }
    if looks_like_text(bytes) {
        let trimmed = trim_ascii_start(bytes);
        if trimmed.starts_with(b"{") || trimmed.starts_with(b"[") {
            return SniffedFormat {
                ext: "json",
                mime: "application/json",
                kind: MediaKind::TextStructured,
                already_compressed: false,
            };
        }
        if trimmed.starts_with(b"<?xml") || trimmed.starts_with(b"<svg") || trimmed.starts_with(b"<html") || trimmed.starts_with(b"<!DOCTYPE") {
            return SniffedFormat {
                ext: "xml",
                mime: "application/xml",
                kind: MediaKind::TextStructured,
                already_compressed: false,
            };
        }
        return SniffedFormat {
            ext: "txt",
            mime: "text/plain",
            kind: MediaKind::TextStructured,
            already_compressed: false,
        };
    }

    SniffedFormat {
        ext: "bin",
        mime: "application/octet-stream",
        kind: MediaKind::Binary,
        already_compressed: false,
    }
}

fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    &bytes[i..]
}

fn looks_like_text(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let sample = &bytes[..bytes.len().min(4096)];
    if sample.contains(&0) {
        return false;
    }
    if std::str::from_utf8(sample).is_err() {
        // Might have cut a multi-byte UTF-8 sequence at 4096; trim last 3 bytes
        let safe_end = sample.len().saturating_sub(3);
        if std::str::from_utf8(&sample[..safe_end]).is_err() {
            return false;
        }
    }
    let printable = sample
        .iter()
        .filter(|&&b| b >= 0x20 || matches!(b, b'\n' | b'\r' | b'\t'))
        .count();
    (printable as f32) / (sample.len() as f32) > 0.92
}

/// Shannon entropy in bits per byte `[0.0, 8.0]`. Values `> 7.85` typically indicate encrypted or already compressed data.
pub fn shannon_entropy_bits(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let sample = if bytes.len() > 262_144 {
        // Sample start + middle + end to avoid being fooled by an uncompressed header
        let mut s = Vec::with_capacity(98_304);
        s.extend_from_slice(&bytes[..32_768]);
        let mid = bytes.len() / 2;
        s.extend_from_slice(&bytes[mid..mid + 32_768]);
        s.extend_from_slice(&bytes[bytes.len() - 32_768..]);
        s
    } else {
        bytes.to_vec()
    };
    let mut counts = [0u64; 256];
    for &b in &sample {
        counts[b as usize] += 1;
    }
    let len = sample.len() as f64;
    let mut ent = 0.0;
    for &c in &counts {
        if c > 0 {
            let p = (c as f64) / len;
            ent -= p * p.log2();
        }
    }
    ent
}

/// Analyze RGBA8 image pixels (sampled cleanly) to classify into one of 10 content classes
/// and derive an adaptive resolution & quality policy (NO blanket 1920px clamp!).
pub fn analyze_rgba_image(
    rgba: &[u8],
    width: u32,
    height: u32,
    source_encoded_bytes: usize,
    raw_file_bytes: Option<&[u8]>,
) -> ImageAnalysis {
    let px_count = (width as usize).saturating_mul(height as usize).max(1);
    let source_bpp = ((source_encoded_bytes * 8) as f32) / (px_count as f32);

    // Check raw file for HDR / wide-gamut markers (cICP / iCCP PQ / HLG / Display-P3)
    let is_hdr_candidate = raw_file_bytes
        .map(|raw| {
            let head = &raw[..raw.len().min(65536)];
            head.windows(4).any(|w| w == b"cICP" || w == b"colr")
                && (head.windows(10).any(|w| w == b"Display P3")
                    || head.windows(7).any(|w| w == b"BT.2020")
                    || head.windows(6).any(|w| w == b"Rec202")
                    || head.windows(4).any(|w| w == b"PQ10"))
        })
        .unwrap_or(false);

    if width < 4 || height < 4 || rgba.len() < px_count * 4 {
        return ImageAnalysis {
            width,
            height,
            class: ImageClass::FlatGraphic,
            has_alpha: false,
            is_hdr_candidate,
            sharp_edge_ratio: 0.0,
            flat_pixel_ratio: 1.0,
            noise_estimate: 0.0,
            mean_luma: 128.0,
            skin_tone_ratio: 0.0,
            color_saturation_mean: 0.0,
            palette_sparsity: 0.0,
            source_bpp,
            has_text_risk: false,
            recommended_max_edge: None,
        };
    }

    // Stride sampling up to ~256x256 grid so 8K images analyze in < 2ms
    let step_x = (width / 256).max(1) as usize;
    let step_y = (height / 256).max(1) as usize;
    let w = width as usize;
    let h = height as usize;

    let mut has_alpha = false;
    let mut sharp_edges = 0usize;
    let mut flat_pixels = 0usize;
    let mut strong_outline_edges = 0usize;
    let mut skin_pixels = 0usize;
    let mut center_skin_pixels = 0usize;
    let mut center_total = 0usize;
    let mut luma_sum = 0.0f64;
    let mut sat_sum = 0.0f64;
    let mut noise_accum = 0.0f64;
    let mut sampled = 0usize;
    let mut quantized_bins = [false; 4096]; // 4-bit per RGB channel (16x16x16)

    // Top & bottom band high-contrast edge density helps detect captioned Memes
    let mut band_sharp = 0usize;
    let mut band_total = 0usize;

    let mut y = 0usize;
    while y + step_y < h {
        let mut x = 0usize;
        while x + step_x < w {
            let idx = (y * w + x) * 4;
            let r = rgba[idx] as i32;
            let g = rgba[idx + 1] as i32;
            let b = rgba[idx + 2] as i32;
            let a = rgba[idx + 3];
            if a < 250 {
                has_alpha = true;
            }

            let idx_r = (y * w + (x + step_x)) * 4;
            let idx_d = ((y + step_y) * w + x) * 4;
            let dh = (r - rgba[idx_r] as i32).abs()
                + (g - rgba[idx_r + 1] as i32).abs()
                + (b - rgba[idx_r + 2] as i32).abs();
            let dv = (r - rgba[idx_d] as i32).abs()
                + (g - rgba[idx_d + 1] as i32).abs()
                + (b - rgba[idx_d + 2] as i32).abs();

            if dh > 120 || dv > 120 {
                sharp_edges += 1;
                if dh > 220 || dv > 220 {
                    strong_outline_edges += 1;
                }
            } else if dh <= 5 && dv <= 5 {
                flat_pixels += 1;
            } else if dh <= 28 && dv <= 28 {
                // Fine-grain local variance captures sensor noise / film grain
                noise_accum += ((dh + dv) as f64) * 0.5;
            }

            let in_caption_band = y < h / 5 || y > (h * 4) / 5;
            if in_caption_band {
                band_total += 1;
                if dh > 160 || dv > 160 {
                    band_sharp += 1;
                }
            }

            let luma = 0.299 * (r as f64) + 0.587 * (g as f64) + 0.114 * (b as f64);
            luma_sum += luma;

            let maxc = r.max(g).max(b) as f64;
            let minc = r.min(g).min(b) as f64;
            let sat = if maxc > 0.0 { (maxc - minc) / maxc } else { 0.0 };
            sat_sum += sat;

            // YCbCr skin-tone heuristic for portrait detection
            let cb = 128.0 - 0.168736 * (r as f64) - 0.331264 * (g as f64) + 0.5 * (b as f64);
            let cr = 128.0 + 0.5 * (r as f64) - 0.418688 * (g as f64) - 0.081312 * (b as f64);
            let is_skin = (77.0..=127.0).contains(&cb) && (133.0..=173.0).contains(&cr) && luma > 45.0;
            if is_skin {
                skin_pixels += 1;
            }
            if x > w / 4 && x < (w * 3) / 4 && y > h / 6 && y < (h * 3) / 4 {
                center_total += 1;
                if is_skin {
                    center_skin_pixels += 1;
                }
            }

            let qbin = (((r >> 4) as usize) << 8) | (((g >> 4) as usize) << 4) | ((b >> 4) as usize);
            quantized_bins[qbin] = true;
            sampled += 1;
            x += step_x;
        }
        y += step_y;
    }

    let s = sampled.max(1) as f32;
    let sharp_edge_ratio = (sharp_edges as f32) / s;
    let flat_pixel_ratio = (flat_pixels as f32) / s;
    let strong_outline_ratio = (strong_outline_edges as f32) / s;
    let noise_estimate = ((noise_accum as f32) / s).min(40.0) / 40.0;
    let mean_luma = (luma_sum as f32) / s;
    let color_saturation_mean = (sat_sum as f32) / s;
    let skin_tone_ratio = (skin_pixels as f32) / s;
    let center_skin_ratio = (center_skin_pixels as f32) / (center_total.max(1) as f32);
    let unique_bins = quantized_bins.iter().filter(|&&v| v).count() as f32;
    let palette_sparsity = 1.0 - (unique_bins / 1024.0).min(1.0);
    let band_sharp_ratio = (band_sharp as f32) / (band_total.max(1) as f32);

    let class = if is_hdr_candidate {
        ImageClass::HdrWideGamut
    } else if has_alpha {
        ImageClass::AlphaGraphic
    } else if flat_pixel_ratio > 0.72 && sharp_edge_ratio < 0.015 && palette_sparsity > 0.80 {
        ImageClass::FlatGraphic
    } else if flat_pixel_ratio > 0.48 && sharp_edge_ratio > 0.012 && palette_sparsity > 0.65 && color_saturation_mean < 0.35 {
        ImageClass::CrispTextUi
    } else if band_sharp_ratio > sharp_edge_ratio * 1.45 && band_sharp_ratio > 0.025 && width <= 1600 && height <= 1600 {
        ImageClass::Meme
    } else if flat_pixel_ratio > 0.35 && (color_saturation_mean > 0.30 || strong_outline_ratio > 0.015) && palette_sparsity > 0.55 {
        ImageClass::AnimeManga
    } else if source_bpp < 0.45 && source_encoded_bytes > 0 && flat_pixel_ratio < 0.6 {
        ImageClass::AlreadyHeavilyCompressed
    } else if mean_luma < 62.0 && noise_estimate > 0.12 {
        ImageClass::LowLightNoisy
    } else if center_skin_ratio > 0.16 && skin_tone_ratio > 0.08 {
        ImageClass::Portrait
    } else {
        ImageClass::Photo
    };

    let has_text_risk = matches!(class, ImageClass::CrispTextUi | ImageClass::Meme | ImageClass::FlatGraphic | ImageClass::AlphaGraphic)
        || (flat_pixel_ratio > 0.45 && sharp_edge_ratio > 0.012);

    // Smart adaptive resolution cap — NEVER blindly clamps 4K crisp screenshots/documents to 1920px!
    let max_dim = width.max(height);
    let recommended_max_edge = match class {
        // Keep full crisp resolution for screenshots/text/UI/alpha up to 4096px
        ImageClass::CrispTextUi | ImageClass::FlatGraphic | ImageClass::AlphaGraphic => {
            if max_dim > 4096 { Some(4096) } else { None }
        }
        // Keep anime/manga/HDR sharp up to 3840px (4K)
        ImageClass::AnimeManga | ImageClass::HdrWideGamut => {
            if max_dim > 3840 { Some(3840) } else { None }
        }
        // Photos/portraits: keep up to 2560px in balanced ultra mode, or 3840px for ultra-high-res 8K downscale
        ImageClass::Photo | ImageClass::Portrait | ImageClass::LowLightNoisy => {
            if max_dim > 3840 { Some(3840) } else { None }
        }
        ImageClass::Meme | ImageClass::AlreadyHeavilyCompressed => None,
    };

    ImageAnalysis {
        width,
        height,
        class,
        has_alpha,
        is_hdr_candidate,
        sharp_edge_ratio,
        flat_pixel_ratio,
        noise_estimate,
        mean_luma,
        skin_tone_ratio,
        color_saturation_mean,
        palette_sparsity,
        source_bpp,
        has_text_risk,
        recommended_max_edge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_all_major_containers_truthfully() {
        assert_eq!(sniff_media_bytes(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]).ext, "png");
        assert_eq!(sniff_media_bytes(&[0xFF, 0xD8, 0xFF, 0xE0]).ext, "jpg");
        assert_eq!(sniff_media_bytes(b"GIF89a......").kind, MediaKind::AnimatedImage);
        assert_eq!(sniff_media_bytes(b"\0\0\0\x1cftypavif\0\0\0\0").ext, "avif");
        assert_eq!(sniff_media_bytes(b"\0\0\0\x1cftypavis\0\0\0\0").kind, MediaKind::AnimatedImage);
        assert_eq!(sniff_media_bytes(b"fLaC\x00\x00\x00\x22").ext, "flac");
        assert_eq!(sniff_media_bytes(b"OggSOpusHead").ext, "ogg");
        assert_eq!(sniff_media_bytes(b"{\"hello\":true}").kind, MediaKind::TextStructured);
        assert_eq!(sniff_media_bytes(&[0x28, 0xB5, 0x2F, 0xFD, 0, 0]).kind, MediaKind::ArchiveAlreadyCompressed);
    }

    #[test]
    fn entropy_distinguishes_text_vs_random() {
        let text = b"Hello ChatW end-to-end encrypted messaging system! ".repeat(200);
        let pseudo_rand: Vec<u8> = (0u32..10000)
            .map(|i| ((i.wrapping_mul(1103515245).wrapping_add(12345)) >> 16) as u8)
            .collect();
        assert!(shannon_entropy_bits(&text) < 5.0);
        assert!(shannon_entropy_bits(&pseudo_rand) > 7.9);
    }

    #[test]
    fn classifies_text_ui_and_preserves_4k_without_1920_clamp() {
        let (w, h) = (2560u32, 1440u32);
        let mut rgba = vec![255u8; (w * h * 4) as usize];
        // Draw high-contrast black text bars on white canvas
        for row in (100..1300).step_by(40) {
            for y in row..row + 12 {
                for x in 120..2200 {
                    let idx = ((y * w + x) * 4) as usize;
                    rgba[idx] = 15;
                    rgba[idx + 1] = 15;
                    rgba[idx + 2] = 15;
                }
            }
        }
        let analysis = analyze_rgba_image(&rgba, w, h, 350_000, None);
        assert_eq!(analysis.class, ImageClass::CrispTextUi);
        assert!(analysis.has_text_risk);
        assert_eq!(analysis.recommended_max_edge, None, "Must NOT blindly clamp 2560px crisp UI to 1920px");
    }
}
