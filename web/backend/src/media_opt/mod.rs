use image::{
    DynamicImage, ImageFormat,
    codecs::{avif::AvifEncoder, jpeg::JpegEncoder, png::PngEncoder, webp::WebPEncoder},
    imageops::FilterType,
};
use std::io::Cursor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    AnimatedImage,
    Video,
    Audio,
    Document,
}

#[derive(Clone, Debug)]
pub struct SniffedFormat {
    pub ext: &'static str,
    pub mime: &'static str,
    pub kind: MediaKind,
}

#[derive(Clone, Debug)]
pub struct ProcessedAvatar {
    pub main_bytes: Vec<u8>,
    pub thumb_bytes: Vec<u8>,
    pub ext: &'static str,
    pub mime: &'static str,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct AdaptiveImageOutput {
    pub bytes: Vec<u8>,
    pub ext: &'static str,
    pub mime: &'static str,
    pub width: u32,
    pub height: u32,
    pub thumb_bytes: Option<Vec<u8>>,
    pub thumb_ext: &'static str,
    pub thumb_mime: &'static str,
    pub is_text_or_crisp_ui: bool,
}

/// Real magic-byte detection — never trusts client Content-Type or filename extension blindly.
pub fn sniff_media_bytes(bytes: &[u8]) -> SniffedFormat {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return SniffedFormat { ext: "png", mime: "image/png", kind: MediaKind::Image };
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return SniffedFormat { ext: "jpg", mime: "image/jpeg", kind: MediaKind::Image };
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return SniffedFormat { ext: "gif", mime: "image/gif", kind: MediaKind::AnimatedImage };
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        let is_anim = bytes.windows(4).any(|w| w == b"ANIM");
        return SniffedFormat {
            ext: "webp",
            mime: "image/webp",
            kind: if is_anim { MediaKind::AnimatedImage } else { MediaKind::Image },
        };
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"avif" || brand == b"avis" {
            return SniffedFormat {
                ext: "avif",
                mime: "image/avif",
                kind: if brand == b"avis" { MediaKind::AnimatedImage } else { MediaKind::Image },
            };
        }
        if matches!(brand, b"isom" | b"mp42" | b"M4V " | b"qt  " | b"avc1") {
            return SniffedFormat { ext: "mp4", mime: "video/mp4", kind: MediaKind::Video };
        }
        if brand == b"M4A " {
            return SniffedFormat { ext: "m4a", mime: "audio/mp4", kind: MediaKind::Audio };
        }
    }
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        let is_audio_only = bytes.windows(4).any(|w| w == b"opus") && !bytes.windows(4).any(|w| w == b"V_VP");
        return SniffedFormat {
            ext: "webm",
            mime: if is_audio_only { "audio/webm" } else { "video/webm" },
            kind: if is_audio_only { MediaKind::Audio } else { MediaKind::Video },
        };
    }
    if bytes.starts_with(b"OggS") {
        return SniffedFormat { ext: "ogg", mime: "audio/ogg", kind: MediaKind::Audio };
    }
    if bytes.starts_with(b"%PDF-") {
        return SniffedFormat { ext: "pdf", mime: "application/pdf", kind: MediaKind::Document };
    }
    if bytes.starts_with(b"ID3") || (bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0) {
        return SniffedFormat { ext: "mp3", mime: "audio/mpeg", kind: MediaKind::Audio };
    }
    SniffedFormat { ext: "bin", mime: "application/octet-stream", kind: MediaKind::Document }
}

/// Center-crop and resize an image to `512x512` AVIF (+ `128x128` thumbnail), or keep genuine fallback format
/// with matching `.avif`/`.webp`/`.png`/`.jpg` extension so file extension NEVER mismatches container magic bytes.
pub fn process_avatar_honest(bytes: &[u8]) -> Result<ProcessedAvatar, &'static str> {
    let sniffed = sniff_media_bytes(bytes);
    if !matches!(sniffed.kind, MediaKind::Image) {
        return Err("unsupported_avatar_format");
    }

    let img = image::load_from_memory(bytes).map_err(|_| "invalid_image_data")?;
    let (w, h) = (img.width(), img.height());
    let side = w.min(h);
    let x = (w - side) / 2;
    let y = (h - side) / 2;
    let cropped = img.crop_imm(x, y, side, side);
    let main_512 = if side == 512 { cropped.clone() } else { cropped.resize_exact(512, 512, FilterType::Lanczos3) };
    let thumb_128 = cropped.resize_exact(128, 128, FilterType::Lanczos3);

    // If input was already 512x512 AVIF and reasonably sized, keep exact crisp bytes; otherwise encode genuine AVIF quickly.
    let main_bytes = if sniffed.ext == "avif" && w == 512 && h == 512 && bytes.len() <= 256 * 1024 {
        bytes.to_vec()
    } else {
        encode_avif(&main_512, 74, 10).or_else(|_| encode_webp(&main_512))?
    };
    let out_sniff = sniff_media_bytes(&main_bytes);
    let thumb_bytes = if out_sniff.ext == "avif" {
        encode_avif(&thumb_128, 66, 10).unwrap_or_else(|_| encode_webp(&thumb_128).unwrap_or_default())
    } else {
        encode_webp(&thumb_128).unwrap_or_default()
    };

    Ok(ProcessedAvatar {
        main_bytes,
        thumb_bytes,
        ext: out_sniff.ext,
        mime: out_sniff.mime,
        width: 512,
        height: 512,
    })
}

/// Inspect crisp edges/flat palette ratio to avoid blurring screenshots/text during adaptive compression.
pub fn detect_crisp_text_or_ui(img: &DynamicImage) -> bool {
    let small = img.thumbnail(256, 256).to_rgba8();
    let (w, h) = small.dimensions();
    if w < 4 || h < 4 {
        return false;
    }
    let mut sharp_edges = 0usize;
    let mut flat_pixels = 0usize;
    let total = ((w - 1) * (h - 1)) as usize;
    for y in 0..(h - 1) {
        for x in 0..(w - 1) {
            let p = small.get_pixel(x, y);
            if p[3] < 250 {
                return true;
            }
            let r = small.get_pixel(x + 1, y);
            let d = small.get_pixel(x, y + 1);
            let diff_h = (p[0] as i32 - r[0] as i32).abs()
                + (p[1] as i32 - r[1] as i32).abs()
                + (p[2] as i32 - r[2] as i32).abs();
            let diff_v = (p[0] as i32 - d[0] as i32).abs()
                + (p[1] as i32 - d[1] as i32).abs()
                + (p[2] as i32 - d[2] as i32).abs();
            if diff_h > 120 || diff_v > 120 {
                sharp_edges += 1;
            } else if diff_h <= 4 && diff_v <= 4 {
                flat_pixels += 1;
            }
        }
    }
    let edge_ratio = (sharp_edges as f32) / (total as f32);
    let flat_ratio = (flat_pixels as f32) / (total as f32);
    flat_ratio > 0.45 && edge_ratio > 0.008
}

/// Generate adaptive optimized variant + tiny thumbnail for chat media (never worsens already smaller files, preserves animations).
pub fn optimize_chat_image(bytes: &[u8]) -> Result<AdaptiveImageOutput, &'static str> {
    let sniffed = sniff_media_bytes(bytes);
    if sniffed.kind == MediaKind::AnimatedImage {
        return Ok(AdaptiveImageOutput {
            bytes: bytes.to_vec(),
            ext: sniffed.ext,
            mime: sniffed.mime,
            width: 0,
            height: 0,
            thumb_bytes: None,
            thumb_ext: sniffed.ext,
            thumb_mime: sniffed.mime,
            is_text_or_crisp_ui: false,
        });
    }
    if sniffed.kind != MediaKind::Image {
        return Err("not_an_image");
    }

    let img = image::load_from_memory(bytes).map_err(|_| "decode_failed")?;
    let (w, h) = (img.width(), img.height());
    let is_crisp = detect_crisp_text_or_ui(&img);
    let resized = if w > 1920 || h > 1920 {
        img.resize(1920, 1920, FilterType::Lanczos3)
    } else {
        img.clone()
    };

    // Text/screenshot/alpha preserves crisp edges via PNG/high-q AVIF; photos use adaptive AVIF.
    let candidate = if is_crisp {
        let avif_hq = encode_avif(&resized, 84, 7).ok();
        let png_opt = encode_png(&resized).ok();
        match (avif_hq, png_opt) {
            (Some(a), Some(p)) if a.len() <= p.len() => a,
            (_, Some(p)) => p,
            (Some(a), None) => a,
            (None, None) => encode_webp(&resized)?,
        }
    } else {
        encode_avif(&resized, 68, 7).or_else(|_| encode_webp(&resized)).or_else(|_| encode_jpeg(&resized, 82))?
    };

    let (final_bytes, final_sniff) = if candidate.len() < bytes.len() || w > 1920 || h > 1920 {
        let s = sniff_media_bytes(&candidate);
        (candidate, s)
    } else {
        (bytes.to_vec(), sniffed.clone())
    };

    let thumb_img = resized.thumbnail(320, 320);
    let thumb_bytes = encode_webp(&thumb_img).or_else(|_| encode_jpeg(&thumb_img, 75)).ok();
    let thumb_sniff = thumb_bytes.as_deref().map(sniff_media_bytes).unwrap_or(final_sniff.clone());

    Ok(AdaptiveImageOutput {
        bytes: final_bytes,
        ext: final_sniff.ext,
        mime: final_sniff.mime,
        width: resized.width(),
        height: resized.height(),
        thumb_bytes,
        thumb_ext: thumb_sniff.ext,
        thumb_mime: thumb_sniff.mime,
        is_text_or_crisp_ui: is_crisp,
    })
}

pub fn probe_image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    image::guess_format(bytes)
        .ok()
        .and_then(|_| image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().ok())
        .and_then(|r| r.into_dimensions().ok())
}

fn encode_avif(img: &DynamicImage, quality: u8, speed: u8) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    let enc = AvifEncoder::new_with_speed_quality(&mut out, speed, quality);
    img.write_with_encoder(enc).map_err(|_| "avif_encode_failed")?;
    Ok(out)
}

fn encode_webp(img: &DynamicImage) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    let enc = WebPEncoder::new_lossless(&mut out);
    img.write_with_encoder(enc).map_err(|_| "webp_encode_failed")?;
    Ok(out)
}

fn encode_png(img: &DynamicImage) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    let enc = PngEncoder::new(&mut out);
    img.write_with_encoder(enc).map_err(|_| "png_encode_failed")?;
    Ok(out)
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    let mut enc = JpegEncoder::new_with_quality(&mut out, quality);
    enc.encode_image(&DynamicImage::ImageRgb8(img.to_rgb8())).map_err(|_| "jpeg_encode_failed")?;
    Ok(out)
}

pub fn content_type_for_format(ext: &str) -> &'static str {
    match ext {
        "avif" => "image/avif",
        "webp" => "image/webp",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogg" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub fn is_format_compatible(ext: &str, format: ImageFormat) -> bool {
    matches!(
        (ext, format),
        ("png", ImageFormat::Png)
            | ("jpg" | "jpeg", ImageFormat::Jpeg)
            | ("gif", ImageFormat::Gif)
            | ("webp", ImageFormat::WebP)
            | ("avif", ImageFormat::Avif)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn avatar_processor_outputs_honest_matching_extension_and_512x512() {
        let mut src = RgbaImage::new(800, 600);
        for y in 0..600 {
            for x in 0..800 {
                src.put_pixel(x, y, Rgba([(x % 255) as u8, (y % 255) as u8, 180, 255]));
            }
        }
        let png = encode_png(&DynamicImage::ImageRgba8(src)).unwrap();
        let processed = process_avatar_honest(&png).expect("avatar ok");
        assert_eq!(processed.width, 512);
        assert_eq!(processed.height, 512);
        let sniffed = sniff_media_bytes(&processed.main_bytes);
        assert_eq!(sniffed.ext, processed.ext);
        assert_eq!(sniffed.mime, processed.mime);
        assert!(!processed.thumb_bytes.is_empty());
    }

    #[test]
    fn crisp_text_ui_detector_identifies_high_contrast_blocks() {
        let mut img = RgbaImage::from_pixel(120, 80, Rgba([255, 255, 255, 255]));
        for y in 20..50 {
            for x in 20..100 {
                img.put_pixel(x, y, Rgba([10, 10, 10, 255]));
            }
        }
        assert!(detect_crisp_text_or_ui(&DynamicImage::ImageRgba8(img)));
    }
}
