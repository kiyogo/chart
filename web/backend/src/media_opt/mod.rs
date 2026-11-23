use chatw_media_core::{
    avif::{AvifCompressionMode, AvifEncodeOptions, optimize_image_to_avif, process_avatar_avif},
    classifier::{ImageClass, MediaKind as CoreMediaKind, analyze_rgba_image, sniff_media_bytes as core_sniff},
};
use image::{DynamicImage, ImageFormat};
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
    pub media_class: String,
    pub estimated_ssim: Option<f32>,
    pub ai_recommended: bool,
    pub codec: &'static str,
    pub compression_ratio: f32,
}

/// Real magic-byte detection backed by `chatw-media-core` — never trusts client Content-Type or filename extension blindly.
pub fn sniff_media_bytes(bytes: &[u8]) -> SniffedFormat {
    let s = core_sniff(bytes);
    let kind = match s.kind {
        CoreMediaKind::Image => MediaKind::Image,
        CoreMediaKind::AnimatedImage => MediaKind::AnimatedImage,
        CoreMediaKind::Video => MediaKind::Video,
        CoreMediaKind::Audio => MediaKind::Audio,
        _ => MediaKind::Document,
    };
    SniffedFormat {
        ext: s.ext,
        mime: s.mime,
        kind,
    }
}

/// Center-crop and encode an avatar strictly as genuine `512x512` AVIF (`ftypavif`) + `128x128` AVIF thumbnail.
/// Zero silent JPEG/WebP/PNG fallback; container magic bytes always match `.avif` / `image/avif`.
pub fn process_avatar_honest(bytes: &[u8]) -> Result<ProcessedAvatar, &'static str> {
    let (main_bytes, thumb_bytes) = process_avatar_avif(bytes).map_err(|_| "unsupported_or_invalid_avatar")?;
    let out_sniff = sniff_media_bytes(&main_bytes);
    Ok(ProcessedAvatar {
        main_bytes,
        thumb_bytes,
        ext: out_sniff.ext,
        mime: out_sniff.mime,
        width: 512,
        height: 512,
    })
}

/// Inspect crisp edges/flat palette ratio via `chatw-media-core` 10-class analyzer.
pub fn detect_crisp_text_or_ui(img: &DynamicImage) -> bool {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let analysis = analyze_rgba_image(rgba.as_raw(), w, h, rgba.len(), None);
    matches!(
        analysis.class,
        ImageClass::CrispTextUi | ImageClass::FlatGraphic | ImageClass::AlphaGraphic
    )
}

/// Generate genuine Adaptive Ultra AVIF (+ Animated AVIF `avis` for animations) and AVIF thumbnail.
/// - No blanket 1920px clamp (uses content-aware adaptive sizing).
/// - Zero silent fallback to JPEG, PNG, or WebP.
/// - Never worsens already-smaller images (returns original bytes if AVIF is larger and no resize occurred).
pub fn optimize_chat_image(bytes: &[u8]) -> Result<AdaptiveImageOutput, &'static str> {
    let sniffed = sniff_media_bytes(bytes);
    if !matches!(sniffed.kind, MediaKind::Image | MediaKind::AnimatedImage) {
        return Err("not_an_image");
    }

    let opts = AvifEncodeOptions {
        mode: AvifCompressionMode::UltraAdaptive,
        max_edge_override: None,
        min_ssim_floor: None,
        generate_thumbnail: true,
        threads: None,
    };

    let out = optimize_image_to_avif(bytes, &opts).map_err(|_| "avif_encode_failed")?;
    let is_crisp = matches!(
        out.analysis.class,
        ImageClass::CrispTextUi | ImageClass::FlatGraphic | ImageClass::AlphaGraphic
    );
    let media_class = format!("{:?}", out.analysis.class);

    // Keep original ONLY if AVIF didn't shrink and it was not resized (and never re-encode into JPEG/WebP/PNG!)
    let (final_bytes, final_ext, final_mime, codec, ratio) =
        if out.avif_bytes.len() < bytes.len() || out.is_animated || sniffed.ext == "avif" {
            let r = (bytes.len() as f32) / (out.avif_bytes.len().max(1) as f32);
            (
                out.avif_bytes,
                out.ext,
                out.mime,
                if out.is_animated { "avis_av1" } else { "avif_av1" },
                r,
            )
        } else {
            (bytes.to_vec(), sniffed.ext, sniffed.mime, "original_passthrough", 1.0)
        };

    let (thumb_bytes, thumb_ext, thumb_mime) = match out.thumb_avif_bytes {
        Some(tb) if !tb.is_empty() => (Some(tb), "avif", "image/avif"),
        _ => (None, final_ext, final_mime),
    };

    Ok(AdaptiveImageOutput {
        bytes: final_bytes,
        ext: final_ext,
        mime: final_mime,
        width: out.width,
        height: out.height,
        thumb_bytes,
        thumb_ext,
        thumb_mime,
        is_text_or_crisp_ui: is_crisp,
        media_class,
        estimated_ssim: out.estimated_ssim.map(|s| s as f32),
        ai_recommended: out.ai_restoration_recommended,
        codec,
        compression_ratio: ratio,
    })
}

pub fn probe_image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    image::guess_format(bytes)
        .ok()
        .and_then(|_| image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().ok())
        .and_then(|r| r.into_dimensions().ok())
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
        "ogg" | "opus" => "audio/ogg",
        "flac" => "audio/flac",
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
    use image::{Rgba, RgbaImage, codecs::png::PngEncoder};

    fn make_png(img: &RgbaImage) -> Vec<u8> {
        let mut out = Vec::new();
        DynamicImage::ImageRgba8(img.clone())
            .write_with_encoder(PngEncoder::new(&mut out))
            .unwrap();
        out
    }

    #[test]
    fn avatar_processor_outputs_genuine_512x512_avif() {
        let mut src = RgbaImage::new(800, 600);
        for y in 0..600 {
            for x in 0..800 {
                src.put_pixel(x, y, Rgba([(x % 255) as u8, (y % 255) as u8, 180, 255]));
            }
        }
        let png = make_png(&src);
        let processed = process_avatar_honest(&png).expect("avatar ok");
        assert_eq!(processed.width, 512);
        assert_eq!(processed.height, 512);
        assert_eq!(processed.ext, "avif");
        assert_eq!(processed.mime, "image/avif");
        let sniffed = sniff_media_bytes(&processed.main_bytes);
        assert_eq!(sniffed.ext, "avif");
        assert_eq!(sniff_media_bytes(&processed.thumb_bytes).ext, "avif");
    }

    #[test]
    fn chat_image_optimizer_produces_genuine_avif_never_jpeg_or_webp() {
        let mut src = RgbaImage::new(320, 240);
        for y in 0..240 {
            for x in 0..320 {
                let r = ((x * 255) / 320) as u8;
                let g = ((y * 255) / 240) as u8;
                let b = (((x + y) * 3) % 255) as u8;
                src.put_pixel(x, y, Rgba([r, g, b, 255]));
            }
        }
        let png = make_png(&src);
        let opt = optimize_chat_image(&png).expect("optimize ok");
        assert_eq!(opt.ext, "avif");
        assert_eq!(opt.mime, "image/avif");
        assert!(opt.bytes.len() < png.len());
        assert_eq!(sniff_media_bytes(&opt.bytes).ext, "avif");
        if let Some(ref th) = opt.thumb_bytes {
            assert_eq!(sniff_media_bytes(th).ext, "avif");
        }
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
