//! Genuine AV1 Video Encoder (`rav1e`), Motion/Scene Complexity Analyzer,
//! IVF & Fragmented MP4 (CMAF) Muxer, Animated AVIF (`avis`) Muxer, ABR Ladder Planner,
//! and Honest Hardware AV1 Capability Probe (Sections XIII & XIV).

#[cfg(feature = "av1-codec")]
use crate::{MediaError, MediaResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoContentClass {
    RealWorldStandard,
    HighMotionAction,
    AnimeAnimation,
    ScreenRecordingText,
    StaticTalkingHead,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoSceneAnalysis {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub frame_count: u32,
    pub duration_ms: u64,
    pub mean_motion_sad: f32,
    pub peak_motion_sad: f32,
    pub flat_ratio: f32,
    pub sharp_edge_ratio: f32,
    pub content_class: VideoContentClass,
    pub recommended_quantizer: usize,
    pub recommended_speed: u8,
    pub recommended_keyframe_interval: u64,
    pub recommended_bitrate_kbps: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Av1HardwareProbe {
    pub hardware_av1_encode_available: bool,
    pub hardware_av1_decode_available: bool,
    pub selected_encoder: String,
    pub detected_devices: Vec<String>,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbrVariantPlan {
    pub variant_id: String,
    pub width: u32,
    pub height: u32,
    pub target_bitrate_kbps: u32,
    pub quantizer: usize,
    pub codec_string: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedAv1Segment {
    pub segment_index: u32,
    pub start_ms: u64,
    pub duration_ms: u64,
    pub is_keyframe: bool,
    pub fmp4_chunk: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedAv1Video {
    pub init_segment: Vec<u8>,
    pub segments: Vec<EncodedAv1Segment>,
    pub complete_fmp4: Vec<u8>,
    pub ivf_bytes: Vec<u8>,
    pub codec_string: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub frame_count: u32,
    pub duration_ms: u64,
    pub analysis: VideoSceneAnalysis,
    pub abr_plan: Vec<AbrVariantPlan>,
}

/// Honestly probe hardware AV1 acceleration on the current host — NEVER assumes every GPU supports HW AV1!
pub fn probe_av1_hardware() -> Av1HardwareProbe {
    #[allow(unused_mut)]
    let mut devices = Vec::new();
    #[allow(unused_mut)]
    let mut hw_enc = false;
    #[allow(unused_mut)]
    let mut hw_dec = false;

    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/dev/dri") {
            for e in entries.flatten() {
                if let Some(name) = e.file_name().to_str() {
                    if name.starts_with("renderD") {
                        devices.push(format!("/dev/dri/{name}"));
                    }
                }
            }
        }
        // Check PCI vendor/device: AMD Polaris (RX 580) & older GPUs do NOT have hardware AV1 encode/decode.
        // Only Intel Arc (Alchemist+), NVIDIA Ada (RTX 40xx+), AMD RDNA3+ (RX 7xxx+) provide HW AV1 encode.
        if let Ok(uevents) = std::fs::read_dir("/sys/class/drm") {
            for entry in uevents.flatten() {
                let p = entry.path().join("device/uevent");
                if let Ok(txt) = std::fs::read_to_string(p) {
                    if let Some(line) = txt.lines().find(|l| l.starts_with("PCI_ID=")) {
                        let id = line.trim_start_matches("PCI_ID=").to_ascii_uppercase();
                        devices.push(format!("PCI:{id}"));
                        // Known Polaris 20 XL RX 580 (1002:67DF / 1002:6FDF) explicitly has NO HW AV1!
                        if id.starts_with("1002:74") || id.starts_with("10DE:26") || id.starts_with("10DE:27") || id.starts_with("8086:56") {
                            hw_enc = true;
                            hw_dec = true;
                        }
                    }
                }
            }
        }
    }

    let selected_encoder = if hw_enc {
        "hardware_av1_vaapi".to_string()
    } else {
        "rav1e_software_simd".to_string()
    };

    let note = if hw_enc {
        "Hardware AV1 encoder detected; software rav1e available as fallback.".to_string()
    } else {
        "No hardware AV1 encoder detected on this GPU; using pure-Rust rav1e software AV1 encoder with CPU SIMD.".to_string()
    };

    Av1HardwareProbe {
        hardware_av1_encode_available: hw_enc,
        hardware_av1_decode_available: hw_dec,
        selected_encoder,
        detected_devices: devices,
        note,
    }
}

/// Analyze motion & frame complexity across RGBA frames to select optimal AV1 quantizer, preset & ABR ladder.
pub fn analyze_video_frames(frames: &[&[u8]], width: u32, height: u32, fps: u32) -> VideoSceneAnalysis {
    let w = width as usize;
    let h = height as usize;
    let frame_count = frames.len() as u32;
    let fps_safe = fps.max(1);
    let duration_ms = ((frame_count as u64) * 1000) / (fps_safe as u64);

    if frames.is_empty() || w < 4 || h < 4 {
        return VideoSceneAnalysis {
            width,
            height,
            fps: fps_safe,
            frame_count,
            duration_ms,
            mean_motion_sad: 0.0,
            peak_motion_sad: 0.0,
            flat_ratio: 0.0,
            sharp_edge_ratio: 0.0,
            content_class: VideoContentClass::RealWorldStandard,
            recommended_quantizer: 105,
            recommended_speed: 8,
            recommended_keyframe_interval: (fps_safe * 2) as u64,
            recommended_bitrate_kbps: 1200,
        };
    }

    let step = (w.max(h) / 96).max(1);
    let mut motion_sum = 0.0f32;
    let mut peak_motion = 0.0f32;
    let mut motion_pairs = 0usize;

    let req_bytes = w.checked_mul(h).and_then(|p| p.checked_mul(4));
    for pair in frames.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if req_bytes.is_none_or(|n| a.len() < n || b.len() < n) {
            continue;
        }
        let mut sad = 0u64;
        let mut samples = 0u64;
        let mut y = 0;
        while y < h {
            let mut x = 0;
            while x < w {
                let i = (y * w + x) * 4;
                sad += (a[i] as i32 - b[i] as i32).unsigned_abs() as u64
                    + (a[i + 1] as i32 - b[i + 1] as i32).unsigned_abs() as u64
                    + (a[i + 2] as i32 - b[i + 2] as i32).unsigned_abs() as u64;
                samples += 3;
                x += step;
            }
            y += step;
        }
        let mean_sad = (sad as f32) / (samples.max(1) as f32);
        motion_sum += mean_sad;
        if mean_sad > peak_motion {
            peak_motion = mean_sad;
        }
        motion_pairs += 1;
    }

    let first_analysis = crate::classifier::analyze_rgba_image(frames[0], width, height, 0, None);
    let mean_motion_sad = if motion_pairs > 0 {
        motion_sum / (motion_pairs as f32)
    } else {
        0.0
    };

    let content_class = if first_analysis.has_text_risk && first_analysis.flat_pixel_ratio > 0.50 {
        VideoContentClass::ScreenRecordingText
    } else if first_analysis.class == crate::classifier::ImageClass::AnimeManga {
        VideoContentClass::AnimeAnimation
    } else if mean_motion_sad > 18.0 || peak_motion > 35.0 {
        VideoContentClass::HighMotionAction
    } else if mean_motion_sad < 3.5 {
        VideoContentClass::StaticTalkingHead
    } else {
        VideoContentClass::RealWorldStandard
    };

    // Quantizer 0..=255 (lower = higher quality). Leave enough structural fidelity for AI video restoration!
    let (recommended_quantizer, recommended_speed, bpp_factor) = match content_class {
        VideoContentClass::ScreenRecordingText => (72, 8, 0.045f32),
        VideoContentClass::AnimeAnimation => (88, 7, 0.055),
        VideoContentClass::HighMotionAction => (96, 8, 0.085),
        VideoContentClass::StaticTalkingHead => (110, 9, 0.035),
        VideoContentClass::RealWorldStandard => (102, 8, 0.060),
    };

    let pixels_per_sec = (width as f32) * (height as f32) * (fps_safe as f32);
    let recommended_bitrate_kbps = ((pixels_per_sec * bpp_factor) / 1000.0).clamp(180.0, 12_000.0) as u32;

    VideoSceneAnalysis {
        width,
        height,
        fps: fps_safe,
        frame_count,
        duration_ms,
        mean_motion_sad,
        peak_motion_sad: peak_motion,
        flat_ratio: first_analysis.flat_pixel_ratio,
        sharp_edge_ratio: first_analysis.sharp_edge_ratio,
        content_class,
        recommended_quantizer,
        recommended_speed,
        recommended_keyframe_interval: (fps_safe * 2).clamp(12, 120) as u64,
        recommended_bitrate_kbps,
    }
}

/// Plan cost-effective Adaptive Bitrate (ABR) variants (Section XIV.3):
/// Never generates wasteful variants for short clips or low-resolution videos!
pub fn plan_abr_variants(analysis: &VideoSceneAnalysis) -> Vec<AbrVariantPlan> {
    let mut ladder = Vec::new();
    let max_h = analysis.height;
    let w = analysis.width;

    // Short clips (< 15 seconds) or <= 480p only need 1 well-optimized AV1 variant to save storage & upload cost.
    if analysis.duration_ms < 15_000 || max_h <= 480 {
        ladder.push(AbrVariantPlan {
            variant_id: format!("av1_{}p", max_h),
            width: w,
            height: max_h,
            target_bitrate_kbps: analysis.recommended_bitrate_kbps,
            quantizer: analysis.recommended_quantizer,
            codec_string: "av01.0.04M.08".into(),
        });
        return ladder;
    }

    // Standard multi-tier ABR for longer HD / 4K videos (max 3 variants to control storage overhead)
    let tiers: &[(u32, f32, usize, &str)] = &[
        (360, 0.25, 130, "av01.0.01M.08"),
        (720, 0.55, 108, "av01.0.04M.08"),
        (1080, 1.0, 96, "av01.0.08M.08"),
    ];
    for &(th, br_scale, q, codec) in tiers {
        if th <= max_h {
            let tw = (((w as u64) * (th as u64)) / (max_h.max(1) as u64)) as u32;
            let even_w = (tw & !1).max(2);
            let even_h = (th & !1).max(2);
            ladder.push(AbrVariantPlan {
                variant_id: format!("av1_{}p", even_h),
                width: even_w,
                height: even_h,
                target_bitrate_kbps: ((analysis.recommended_bitrate_kbps as f32) * br_scale).max(160.0) as u32,
                quantizer: q,
                codec_string: codec.into(),
            });
        }
    }
    if max_h > 1080 {
        ladder.push(AbrVariantPlan {
            variant_id: format!("av1_{}p", max_h),
            width: w & !1,
            height: max_h & !1,
            target_bitrate_kbps: analysis.recommended_bitrate_kbps,
            quantizer: analysis.recommended_quantizer,
            codec_string: "av01.0.12M.08".into(),
        });
    }
    ladder
}

/// Convert RGBA8 frame (with explicit `src_w` pixel row stride) to YUV420p (BT.709) planar arrays `(y, u, v)` of size `w x h`.
/// Using `src_w` prevents diagonal skew corruption when cropping odd-width inputs to even `w`.
#[allow(dead_code)]
fn rgba_to_yuv420_strided(rgba: &[u8], src_w: usize, w: usize, h: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let cw = (w + 1) / 2;
    let ch = (h + 1) / 2;
    let mut y_plane = vec![0u8; w * h];
    let mut u_plane = vec![128u8; cw * ch];
    let mut v_plane = vec![128u8; cw * ch];

    for y in 0..h {
        for x in 0..w {
            let i = (y * src_w + x) * 4;
            let r = rgba[i] as f32;
            let g = rgba[i + 1] as f32;
            let b = rgba[i + 2] as f32;
            let luma = (0.2126 * r + 0.7152 * g + 0.0722 * b).clamp(0.0, 255.0);
            y_plane[y * w + x] = luma as u8;
        }
    }

    for cy in 0..ch {
        for cx in 0..cw {
            let mut r_sum = 0.0f32;
            let mut g_sum = 0.0f32;
            let mut b_sum = 0.0f32;
            let mut cnt = 0.0f32;
            for dy in 0..2 {
                let sy = (cy * 2 + dy).min(h - 1);
                for dx in 0..2 {
                    let sx = (cx * 2 + dx).min(w - 1);
                    let i = (sy * src_w + sx) * 4;
                    r_sum += rgba[i] as f32;
                    g_sum += rgba[i + 1] as f32;
                    b_sum += rgba[i + 2] as f32;
                    cnt += 1.0;
                }
            }
            let r = r_sum / cnt;
            let g = g_sum / cnt;
            let b = b_sum / cnt;
            let cb = (128.0 - 0.1146 * r - 0.3854 * g + 0.5 * b).clamp(0.0, 255.0);
            let cr = (128.0 + 0.5 * r - 0.4542 * g - 0.0458 * b).clamp(0.0, 255.0);
            u_plane[cy * cw + cx] = cb as u8;
            v_plane[cy * cw + cx] = cr as u8;
        }
    }
    (y_plane, u_plane, v_plane)
}

#[cfg(feature = "av1-codec")]
mod impl_av1 {
    use super::*;
    use rav1e::prelude::*;

    struct RawAv1Packet {
        data: Vec<u8>,
        is_keyframe: bool,
        pts: u64,
    }

    fn encode_packets_rav1e(
        frames: &[&[u8]],
        width: u32,
        height: u32,
        fps: u32,
        quantizer: usize,
        speed: u8,
        keyframe_interval: u64,
    ) -> MediaResult<Vec<RawAv1Packet>> {
        if frames.is_empty() || width < 2 || height < 2 {
            return Err(MediaError::EmptyInput);
        }
        let w = (width & !1).max(2) as usize;
        let h = (height & !1).max(2) as usize;
        let fps_safe = fps.max(1) as usize;

        let mut enc = EncoderConfig::with_speed_preset(speed.clamp(1, 10));
        enc.width = w;
        enc.height = h;
        enc.bit_depth = 8;
        enc.chroma_sampling = ChromaSampling::Cs420;
        enc.quantizer = quantizer.clamp(20, 220);
        enc.min_quantizer = (quantizer / 2).clamp(10, 180) as u8;
        enc.time_base = Rational::new(1, fps_safe as u64);
        enc.max_key_frame_interval = keyframe_interval.max(1);
        enc.min_key_frame_interval = (keyframe_interval / 4).max(1);
        enc.low_latency = true;

        let cfg = Config::new().with_encoder_config(enc);
        let mut ctx: Context<u8> = cfg
            .new_context()
            .map_err(|e| MediaError::Av1EncodeFailed(format!("rav1e context: {e}")))?;

        let src_w = width as usize;
        let src_h = height as usize;
        let min_src_bytes = src_w
            .checked_mul(src_h)
            .and_then(|p| p.checked_mul(4))
            .ok_or_else(|| MediaError::InvalidParameter("frame_dimensions_overflow".into()))?;
        for &rgba in frames {
            if rgba.len() < min_src_bytes {
                return Err(MediaError::EmptyInput);
            }
            let (y, u, v) = rgba_to_yuv420_strided(rgba, src_w, w, h);
            let cw = (w + 1) / 2;
            let mut frame = ctx.new_frame();
            frame.planes[0].copy_from_raw_u8(&y, w, 1);
            frame.planes[1].copy_from_raw_u8(&u, cw, 1);
            frame.planes[2].copy_from_raw_u8(&v, cw, 1);
            ctx.send_frame(frame)
                .map_err(|e| MediaError::Av1EncodeFailed(format!("rav1e send_frame: {e}")))?;
        }
        ctx.flush();

        let mut packets = Vec::with_capacity(frames.len());
        loop {
            match ctx.receive_packet() {
                Ok(pkt) => {
                    packets.push(RawAv1Packet {
                        data: pkt.data,
                        is_keyframe: pkt.frame_type == FrameType::KEY,
                        pts: pkt.input_frameno,
                    });
                }
                Err(EncoderStatus::LimitReached) => break,
                Err(EncoderStatus::Encoded) | Err(EncoderStatus::NeedMoreData) => continue,
                Err(e) => return Err(MediaError::Av1EncodeFailed(format!("rav1e receive_packet: {e:?}"))),
            }
        }
        if packets.is_empty() {
            return Err(MediaError::Av1EncodeFailed("rav1e emitted 0 packets".into()));
        }
        Ok(packets)
    }

    /// Wrap AV1 packets in standard IVF container (`DKIF` + `AV01`).
    fn mux_ivf(packets: &[RawAv1Packet], width: u16, height: u16, fps: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"DKIF");
        out.extend_from_slice(&0u16.to_le_bytes()); // version
        out.extend_from_slice(&32u16.to_le_bytes()); // header len
        out.extend_from_slice(b"AV01");
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.extend_from_slice(&fps.max(1).to_le_bytes()); // rate
        out.extend_from_slice(&1u32.to_le_bytes()); // scale
        out.extend_from_slice(&(packets.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // unused

        for p in packets {
            out.extend_from_slice(&(p.data.len() as u32).to_le_bytes());
            out.extend_from_slice(&p.pts.to_le_bytes());
            out.extend_from_slice(&p.data);
        }
        out
    }

    fn mp4_box(fourcc: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = (8 + payload.len()) as u32;
        let mut b = Vec::with_capacity(size as usize);
        b.extend_from_slice(&size.to_be_bytes());
        b.extend_from_slice(fourcc);
        b.extend_from_slice(payload);
        b
    }

    fn mp4_fullbox(fourcc: &[u8; 4], version: u8, flags: u32, payload: &[u8]) -> Vec<u8> {
        let mut body = Vec::with_capacity(4 + payload.len());
        let vf = ((version as u32) << 24) | (flags & 0x00FF_FFFF);
        body.extend_from_slice(&vf.to_be_bytes());
        body.extend_from_slice(payload);
        mp4_box(fourcc, &body)
    }

    /// Build a standards-compliant ISOBMFF Fragmented MP4 (CMAF) init segment (`ftyp` + `moov` with `av01`/`av1C` + `mvex`).
    fn build_fmp4_init_segment(major_brand: &[u8; 4], compat_brands: &[&[u8; 4]], width: u16, height: u16, timescale: u32) -> Vec<u8> {
        // ftyp
        let mut ftyp_body = Vec::new();
        ftyp_body.extend_from_slice(major_brand);
        ftyp_body.extend_from_slice(&0u32.to_be_bytes());
        for b in compat_brands {
            ftyp_body.extend_from_slice(*b);
        }
        let ftyp = mp4_box(b"ftyp", &ftyp_body);

        // mvhd (v0)
        let mut mvhd_body = vec![0u8; 96];
        mvhd_body[8..12].copy_from_slice(&timescale.to_be_bytes());
        mvhd_body[16..20].copy_from_slice(&0x0001_0000u32.to_be_bytes()); // rate 1.0
        mvhd_body[20..22].copy_from_slice(&0x0100u16.to_be_bytes()); // volume 1.0
        // Unity matrix
        mvhd_body[32..36].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        mvhd_body[48..52].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        mvhd_body[64..68].copy_from_slice(&0x4000_0000u32.to_be_bytes());
        mvhd_body[92..96].copy_from_slice(&2u32.to_be_bytes()); // next_track_ID = 2
        let mvhd = mp4_fullbox(b"mvhd", 0, 0, &mvhd_body);

        // tkhd (v0, flags = 3: enabled + in_movie)
        let mut tkhd_body = vec![0u8; 80];
        tkhd_body[8..12].copy_from_slice(&1u32.to_be_bytes()); // track_ID = 1
        tkhd_body[36..40].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        tkhd_body[52..56].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        tkhd_body[68..72].copy_from_slice(&0x4000_0000u32.to_be_bytes());
        tkhd_body[72..76].copy_from_slice(&((width as u32) << 16).to_be_bytes());
        tkhd_body[76..80].copy_from_slice(&((height as u32) << 16).to_be_bytes());
        let tkhd = mp4_fullbox(b"tkhd", 0, 3, &tkhd_body);

        // mdhd (v0)
        let mut mdhd_body = vec![0u8; 20];
        mdhd_body[8..12].copy_from_slice(&timescale.to_be_bytes());
        mdhd_body[16..18].copy_from_slice(&0x55C4u16.to_be_bytes()); // und
        let mdhd = mp4_fullbox(b"mdhd", 0, 0, &mdhd_body);

        // hdlr (vide)
        let mut hdlr_body = Vec::new();
        hdlr_body.extend_from_slice(&0u32.to_be_bytes());
        hdlr_body.extend_from_slice(b"vide");
        hdlr_body.extend_from_slice(&[0u8; 12]);
        hdlr_body.extend_from_slice(b"ChatW AV1 Video\0");
        let hdlr = mp4_fullbox(b"hdlr", 0, 0, &hdlr_body);

        // vmhd, dinf/dref
        let vmhd = mp4_fullbox(b"vmhd", 0, 1, &[0u8; 8]);
        let url_box = mp4_fullbox(b"url ", 0, 1, &[]);
        let mut dref_body = Vec::new();
        dref_body.extend_from_slice(&1u32.to_be_bytes());
        dref_body.extend_from_slice(&url_box);
        let dinf = mp4_box(b"dinf", &mp4_fullbox(b"dref", 0, 0, &dref_body));

        // av1C (4-byte AV1CodecConfigurationRecord: marker=1, version=1, seq_profile=0, seq_level_idx_0=4, 8-bit 4:2:0)
        let av1c = mp4_box(b"av1C", &[0x81, 0x04, 0x0C, 0x00]);

        // av01 VisualSampleEntry
        let mut av01_body = vec![0u8; 78];
        av01_body[6..8].copy_from_slice(&1u16.to_be_bytes()); // data_reference_index
        av01_body[24..26].copy_from_slice(&width.to_be_bytes());
        av01_body[26..28].copy_from_slice(&height.to_be_bytes());
        av01_body[28..32].copy_from_slice(&0x0048_0000u32.to_be_bytes()); // horiz 72dpi
        av01_body[32..36].copy_from_slice(&0x0048_0000u32.to_be_bytes()); // vert 72dpi
        av01_body[40..42].copy_from_slice(&1u16.to_be_bytes()); // frame_count
        av01_body[74..76].copy_from_slice(&0x0018u16.to_be_bytes()); // depth 24
        av01_body[76..78].copy_from_slice(&0xFFFFu16.to_be_bytes());
        av01_body.extend_from_slice(&av1c);
        let av01 = mp4_box(b"av01", &av01_body);

        let mut stsd_body = Vec::new();
        stsd_body.extend_from_slice(&1u32.to_be_bytes());
        stsd_body.extend_from_slice(&av01);
        let stsd = mp4_fullbox(b"stsd", 0, 0, &stsd_body);

        let stts = mp4_fullbox(b"stts", 0, 0, &0u32.to_be_bytes());
        let stsc = mp4_fullbox(b"stsc", 0, 0, &0u32.to_be_bytes());
        let stsz = mp4_fullbox(b"stsz", 0, 0, &[0u8; 8]);
        let stco = mp4_fullbox(b"stco", 0, 0, &0u32.to_be_bytes());

        let mut stbl_body = Vec::new();
        stbl_body.extend_from_slice(&stsd);
        stbl_body.extend_from_slice(&stts);
        stbl_body.extend_from_slice(&stsc);
        stbl_body.extend_from_slice(&stsz);
        stbl_body.extend_from_slice(&stco);
        let stbl = mp4_box(b"stbl", &stbl_body);

        let mut minf_body = Vec::new();
        minf_body.extend_from_slice(&vmhd);
        minf_body.extend_from_slice(&dinf);
        minf_body.extend_from_slice(&stbl);
        let minf = mp4_box(b"minf", &minf_body);

        let mut mdia_body = Vec::new();
        mdia_body.extend_from_slice(&mdhd);
        mdia_body.extend_from_slice(&hdlr);
        mdia_body.extend_from_slice(&minf);
        let mdia = mp4_box(b"mdia", &mdia_body);

        let mut trak_body = Vec::new();
        trak_body.extend_from_slice(&tkhd);
        trak_body.extend_from_slice(&mdia);
        let trak = mp4_box(b"trak", &trak_body);

        // mvex / trex (required for Fragmented MP4 / MSE / CMAF streaming!)
        let mut trex_body = Vec::new();
        trex_body.extend_from_slice(&1u32.to_be_bytes()); // track_ID
        trex_body.extend_from_slice(&1u32.to_be_bytes()); // default_sample_description_index
        trex_body.extend_from_slice(&1u32.to_be_bytes()); // default_sample_duration
        trex_body.extend_from_slice(&0u32.to_be_bytes()); // default_sample_size
        trex_body.extend_from_slice(&0u32.to_be_bytes()); // default_sample_flags
        let mvex = mp4_box(b"mvex", &mp4_fullbox(b"trex", 0, 0, &trex_body));

        let mut moov_body = Vec::new();
        moov_body.extend_from_slice(&mvhd);
        moov_body.extend_from_slice(&trak);
        moov_body.extend_from_slice(&mvex);
        let moov = mp4_box(b"moov", &moov_body);

        let mut init = Vec::with_capacity(ftyp.len() + moov.len());
        init.extend_from_slice(&ftyp);
        init.extend_from_slice(&moov);
        init
    }

    /// Build a single Fragmented MP4 media segment (`moof` + `mdat`) for progressive streaming & seeking!
    fn build_fmp4_media_segment(seq_num: u32, base_decode_time: u64, samples: &[&RawAv1Packet]) -> Vec<u8> {
        let mfhd = mp4_fullbox(b"mfhd", 0, 0, &seq_num.to_be_bytes());
        // tfhd: flags = 0x020000 (default-base-is-moof)
        let tfhd = mp4_fullbox(b"tfhd", 0, 0x020000, &1u32.to_be_bytes());
        // tfdt (v1, 64-bit baseMediaDecodeTime)
        let tfdt = mp4_fullbox(b"tfdt", 1, 0, &base_decode_time.to_be_bytes());

        // trun: flags = 0x000001 (data-offset) | 0x000100 (sample-duration) | 0x000200 (sample-size) | 0x000400 (sample-flags)
        let trun_flags: u32 = 0x000701;
        let mut trun_body = Vec::new();
        trun_body.extend_from_slice(&(samples.len() as u32).to_be_bytes());
        let data_offset_pos = trun_body.len();
        trun_body.extend_from_slice(&0i32.to_be_bytes()); // placeholder patched below
        for s in samples {
            trun_body.extend_from_slice(&1u32.to_be_bytes()); // sample_duration = 1 tick (timescale = fps)
            trun_body.extend_from_slice(&(s.data.len() as u32).to_be_bytes());
            let sample_flags: u32 = if s.is_keyframe { 0x0200_0000 } else { 0x0101_0000 };
            trun_body.extend_from_slice(&sample_flags.to_be_bytes());
        }

        // Compute exact data_offset = size(moof) + 8 (mdat header)
        let trun_placeholder = mp4_fullbox(b"trun", 0, trun_flags, &trun_body);
        let traf_len = 8 + tfhd.len() + tfdt.len() + trun_placeholder.len();
        let moof_len = 8 + mfhd.len() + traf_len;
        let data_offset = (moof_len + 8) as i32;
        trun_body[data_offset_pos..data_offset_pos + 4].copy_from_slice(&data_offset.to_be_bytes());
        let trun = mp4_fullbox(b"trun", 0, trun_flags, &trun_body);

        let mut traf_body = Vec::new();
        traf_body.extend_from_slice(&tfhd);
        traf_body.extend_from_slice(&tfdt);
        traf_body.extend_from_slice(&trun);
        let traf = mp4_box(b"traf", &traf_body);

        let mut moof_body = Vec::new();
        moof_body.extend_from_slice(&mfhd);
        moof_body.extend_from_slice(&traf);
        let moof = mp4_box(b"moof", &moof_body);

        let mut mdat_payload = Vec::new();
        for s in samples {
            mdat_payload.extend_from_slice(&s.data);
        }
        let mdat = mp4_box(b"mdat", &mdat_payload);

        let mut seg = Vec::with_capacity(moof.len() + mdat.len());
        seg.extend_from_slice(&moof);
        seg.extend_from_slice(&mdat);
        seg
    }

    /// Encode RGBA8 frames into a complete Fragmented MP4 (CMAF) AV1 stream + IVF bitstream + ABR plan.
    pub fn encode_rgba_frames_to_av1_cmaf(
        frames: &[&[u8]],
        width: u32,
        height: u32,
        fps: u32,
        segment_frames: usize,
    ) -> MediaResult<EncodedAv1Video> {
        let analysis = analyze_video_frames(frames, width, height, fps);
        let abr_plan = plan_abr_variants(&analysis);
        let seg_len = segment_frames.max(1);
        let key_int = (seg_len as u64).min(analysis.recommended_keyframe_interval.max(1));

        let packets = encode_packets_rav1e(
            frames,
            width,
            height,
            fps,
            analysis.recommended_quantizer,
            analysis.recommended_speed,
            key_int,
        )?;

        let even_w = (width & !1).max(2) as u16;
        let even_h = (height & !1).max(2) as u16;
        let fps_safe = fps.max(1);

        let ivf_bytes = mux_ivf(&packets, even_w, even_h, fps_safe);
        let init_segment = build_fmp4_init_segment(
            b"av01",
            &[b"isom", b"iso6", b"cmfc", b"av01"],
            even_w,
            even_h,
            fps_safe,
        );

        let mut segments = Vec::new();
        let mut complete_fmp4 = init_segment.clone();
        for (seg_idx, chunk) in packets.chunks(seg_len).enumerate() {
            let refs: Vec<&RawAv1Packet> = chunk.iter().collect();
            let base_dts = (seg_idx * seg_len) as u64;
            let fmp4_chunk = build_fmp4_media_segment((seg_idx + 1) as u32, base_dts, &refs);
            complete_fmp4.extend_from_slice(&fmp4_chunk);
            segments.push(EncodedAv1Segment {
                segment_index: seg_idx as u32,
                start_ms: (base_dts * 1000) / (fps_safe as u64),
                duration_ms: ((chunk.len() as u64) * 1000) / (fps_safe as u64),
                is_keyframe: chunk[0].is_keyframe,
                fmp4_chunk,
            });
        }

        Ok(EncodedAv1Video {
            init_segment,
            segments,
            complete_fmp4,
            ivf_bytes,
            codec_string: "av01.0.04M.08".into(),
            width: even_w as u32,
            height: even_h as u32,
            fps: fps_safe,
            frame_count: packets.len() as u32,
            duration_ms: analysis.duration_ms,
            analysis,
            abr_plan,
        })
    }

    /// Encode RGBA8 frames into an Animated AVIF (`avis` major brand + AV1 fragmented sequence).
    pub fn encode_rgba_frames_to_animated_avif(
        frames: &[&[u8]],
        width: u32,
        height: u32,
        fps: u32,
        quantizer: usize,
    ) -> MediaResult<Vec<u8>> {
        let packets = encode_packets_rav1e(frames, width, height, fps, quantizer, 8, (fps.max(1) * 2) as u64)?;
        let even_w = (width & !1).max(2) as u16;
        let even_h = (height & !1).max(2) as u16;
        let mut out = build_fmp4_init_segment(
            b"avis",
            &[b"avis", b"avif", b"msf1", b"iso8", b"miaf", b"MA1B"],
            even_w,
            even_h,
            fps.max(1),
        );
        let refs: Vec<&RawAv1Packet> = packets.iter().collect();
        let seg = build_fmp4_media_segment(1, 0, &refs);
        out.extend_from_slice(&seg);
        Ok(out)
    }
}

#[cfg(feature = "av1-codec")]
pub use impl_av1::{encode_rgba_frames_to_animated_avif, encode_rgba_frames_to_av1_cmaf};

#[cfg(all(test, feature = "av1-codec"))]
mod tests {
    use super::*;
    use crate::classifier::{MediaKind, sniff_media_bytes};

    #[test]
    fn encodes_real_av1_cmaf_fmp4_and_ivf_segments() {
        let (w, h) = (128u32, 96u32);
        let mut raw_frames = Vec::new();
        for f in 0..6u32 {
            let mut buf = vec![0u8; (w * h * 4) as usize];
            for y in 0..h {
                for x in 0..w {
                    let i = ((y * w + x) * 4) as usize;
                    buf[i] = ((x + f * 12) % 255) as u8;
                    buf[i + 1] = ((y + f * 6) % 255) as u8;
                    buf[i + 2] = 160;
                    buf[i + 3] = 255;
                }
            }
            raw_frames.push(buf);
        }
        let refs: Vec<&[u8]> = raw_frames.iter().map(|b| b.as_slice()).collect();
        let res = encode_rgba_frames_to_av1_cmaf(&refs, w, h, 24, 3).expect("av1 encode");
        assert_eq!(res.frame_count, 6);
        assert_eq!(res.segments.len(), 2);
        assert!(res.segments[0].is_keyframe);
        assert!(res.ivf_bytes.starts_with(b"DKIF"));
        assert_eq!(&res.ivf_bytes[8..12], b"AV01");
        assert_eq!(sniff_media_bytes(&res.complete_fmp4).kind, MediaKind::Video);
        assert!(res.complete_fmp4.windows(4).any(|w| w == b"av01"));
        assert!(res.complete_fmp4.windows(4).any(|w| w == b"av1C"));
        assert!(res.complete_fmp4.windows(4).any(|w| w == b"moof"));
        assert!(res.complete_fmp4.windows(4).any(|w| w == b"mdat"));
    }

    #[test]
    fn hardware_probe_is_honest_and_never_panics() {
        let probe = probe_av1_hardware();
        assert!(!probe.selected_encoder.is_empty());
        assert!(!probe.note.is_empty());
    }

    #[test]
    fn odd_width_rgba_to_yuv420_preserves_row_stride_without_skew() {
        // Vertical stripe: column 0 is bright white, all other columns are black, odd width = 17.
        let (src_w, src_h) = (17usize, 11usize);
        let mut rgba = vec![0u8; src_w * src_h * 4];
        for y in 0..src_h {
            let i = (y * src_w) * 4;
            rgba[i] = 255;
            rgba[i + 1] = 255;
            rgba[i + 2] = 255;
            rgba[i + 3] = 255;
        }
        let (y_plane, _, _) = rgba_to_yuv420_strided(&rgba, src_w, 16, 10);
        for row in 0..10 {
            assert!(y_plane[row * 16] > 240, "col 0 row {row} must remain bright");
            assert_eq!(y_plane[row * 16 + 1], 0, "col 1 row {row} must remain black without skew");
        }
    }
}
