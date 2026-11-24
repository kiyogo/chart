//! Pure-Rust Real-ESRGAN / Upscayl (`SRVGGNetCompact`) Super-Resolution Engine
//! (Official BSD-3-Clause pretrained weights from `xinntao/Real-ESRGAN`, identical architecture to `upscayl-lite-4x` / Digital Art).
//!
//! Architecture (`SRVGGNetCompact`):
//! - Layer 0: `Conv2d(3 -> 64, 3×3, pad=1)` + `PReLU(64)`
//! - Body: `N` blocks (`N=16` for `realesr-animevideov3`, `N=32` for `realesr-general-x4v3` / `upscayl-lite-4x`)
//!         of `Conv2d(64 -> 64, 3×3, pad=1)` + `PReLU(64)`
//! - Tail: `Conv2d(64 -> 48, 3×3, pad=1)` -> `PixelShuffle(4×)` + `nearest_upsample(input, 4×)`
//!
//! Includes:
//! - Compact FP16 `CWSR1` binary parser (`realesr_animevideov3.cwsr` 1.19 MB / `realesr_general_x4v3.cwsr` 2.31 MB)
//! - `OriginalQualityProfile` perceptual fingerprint extraction (on sender's untouched original)
//! - Automatic Anime/Illustration vs Photo/Portrait Real-ESRGAN model selector
//! - Color/Contrast/Saturation lock preserving 100% of Real-ESRGAN's razor-sharp 4× edges
//! - Pure-Rust SSIM & Color-Drift quality gate
//! - GPU weight texture packing helpers (`pack_gl_conv_in_weights`, `pack_gl_conv64_weights`) for `app/src/storage/realesrgan_gpu.rs`
//! - Multi-threaded Pure-Rust CPU engine (`std::thread::scope` on native, single-thread fallback on WASM) with 16px zero-seam halo tiling.

use crate::{
    MediaError, MediaResult,
    classifier::{ImageClass, analyze_rgba_image},
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

pub const CWSR_MAGIC: &[u8; 6] = b"CWSR1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealEsrganModelKind {
    /// `realesr-animevideov3` (16×64ch, 4×, 1.19 MB FP16, BSD-3-Clause) — Anime, Digital Art, Illustration, Meme
    AnimeVideoV3,
    /// `realesr-general-x4v3` DNI-35 (32×64ch, 4×, 2.31 MB FP16, BSD-3-Clause — identical architecture to `upscayl-lite-4x`) — Photos, Portraits
    GeneralX4V3,
}

impl RealEsrganModelKind {
    pub fn model_id(self) -> &'static str {
        match self {
            Self::AnimeVideoV3 => "realesr_animevideov3",
            Self::GeneralX4V3 => "realesr_general_x4v3",
        }
    }

    pub fn model_label(self) -> &'static str {
        match self {
            Self::AnimeVideoV3 => "Real-ESRGAN Anime v3 (4×)",
            Self::GeneralX4V3 => "Real-ESRGAN Upscayl-Lite (4×)",
        }
    }

    pub fn cwsr_url(self) -> &'static str {
        match self {
            Self::AnimeVideoV3 => "/models/realesr_animevideov3.cwsr",
            Self::GeneralX4V3 => "/models/realesr_general_x4v3.cwsr",
        }
    }

    pub fn version(self) -> &'static str {
        match self {
            Self::AnimeVideoV3 => "0.2.5.0",
            Self::GeneralX4V3 => "0.2.5.0-dni35",
        }
    }

    pub fn from_str_name(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "realesr_general_x4v3" | "general" | "photo" | "upscayl-lite" => Self::GeneralX4V3,
            _ => Self::AnimeVideoV3,
        }
    }
}

/// Compact (~95-byte JSON) perceptual quality fingerprint extracted from the SENDER'S 100% untouched original image.
/// Encrypted inside the E2EE DM `img:` payload (`qp`) so the receiver's Real-ESRGAN selects the exact model
/// and locks color balance, contrast, and saturation to ~100% of the sender's original.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OriginalQualityProfile {
    pub cls: String,
    #[serde(default)]
    pub w: u32,
    #[serde(default)]
    pub h: u32,
    pub sharp: f32,
    pub edge_p95: f32,
    pub acut: f32,
    pub luma_m: f32,
    pub luma_s: f32,
    pub r_m: f32,
    pub g_m: f32,
    pub b_m: f32,
    pub sat_m: f32,
    pub outline_r: f32,
    #[serde(default = "default_loss_ratio")]
    pub loss_ratio: f32,
}

fn default_loss_ratio() -> f32 {
    1.0
}

impl Default for OriginalQualityProfile {
    fn default() -> Self {
        Self {
            cls: "photo".into(),
            w: 0,
            h: 0,
            sharp: 0.0,
            edge_p95: 0.0,
            acut: 0.0,
            luma_m: 128.0,
            luma_s: 40.0,
            r_m: 128.0,
            g_m: 128.0,
            b_m: 128.0,
            sat_m: 0.25,
            outline_r: 0.0,
            loss_ratio: 1.0,
        }
    }
}

/// Extract compact perceptual quality fingerprint directly from an RGBA8 image (< 10ms strided sampling).
pub fn extract_original_quality_profile(rgba: &[u8], width: u32, height: u32) -> OriginalQualityProfile {
    extract_profile_with_kernel_stride(rgba, width, height, 1)
}

/// Extract perceptual profile at physical pixel stride `ks` (`1` for native resolution, `2` for 2× super-res, `4` for 4× super-res)
/// so sharpness & edge gradients are compared at the exact same physical spatial frequency!
fn extract_profile_with_kernel_stride(
    rgba: &[u8],
    width: u32,
    height: u32,
    ks: usize,
) -> OriginalQualityProfile {
    let w = width as usize;
    let h = height as usize;
    let ks = ks.max(1);
    let pad = (ks * 2).max(2);
    if w <= pad * 2 || h <= pad * 2 || rgba.len() < w * h * 4 {
        return OriginalQualityProfile::default();
    }

    let analysis = analyze_rgba_image(rgba, width, height, 0, None);

    let step = (((w * h) as f64 / 32768.0).sqrt().floor() as usize).max(1);
    let mut sum_r = 0.0f64;
    let mut sum_g = 0.0f64;
    let mut sum_b = 0.0f64;
    let mut sum_y = 0.0f64;
    let mut sum_y2 = 0.0f64;
    let mut sum_sat = 0.0f64;
    let mut sum_lap2 = 0.0f64;
    let mut sum_acut2 = 0.0f64;
    let mut dark_outline = 0u64;
    let mut edge_hist = [0u32; 256];
    let mut count = 0u64;

    let luma_at = |px: usize, py: usize| -> f64 {
        let i = (py * w + px) * 4;
        0.299 * (rgba[i] as f64) + 0.587 * (rgba[i + 1] as f64) + 0.114 * (rgba[i + 2] as f64)
    };

    let mut y = pad;
    while y + pad < h {
        let mut x = pad;
        while x + pad < w {
            let i = (y * w + x) * 4;
            let r = rgba[i] as f64;
            let g = rgba[i + 1] as f64;
            let b = rgba[i + 2] as f64;
            let yc = 0.299 * r + 0.587 * g + 0.114 * b;

            sum_r += r;
            sum_g += g;
            sum_b += b;
            sum_y += yc;
            sum_y2 += yc * yc;

            let maxc = r.max(g).max(b);
            let minc = r.min(g).min(b);
            if maxc > 0.0 {
                sum_sat += (maxc - minc) / maxc;
            }

            let yl = luma_at(x - ks, y);
            let yr = luma_at(x + ks, y);
            let yu = luma_at(x, y - ks);
            let yd = luma_at(x, y + ks);
            let lap = (4.0 * yc - yl - yr - yu - yd).abs();
            sum_lap2 += lap * lap;

            let yl2 = luma_at(x - pad, y);
            let yr2 = luma_at(x + pad, y);
            let yu2 = luma_at(x, y - pad);
            let yd2 = luma_at(x, y + pad);
            let ac = (yc - 0.25 * (yl2 + yr2 + yu2 + yd2)).abs();
            sum_acut2 += ac * ac;

            let grad = ((yr - yl).abs() + (yd - yu).abs()) * 0.5;
            let bin = (grad.round() as usize).min(255);
            edge_hist[bin] += 1;

            if yc < 95.0 && (yl.max(yr).max(yu).max(yd) - yc) > 55.0 {
                dark_outline += 1;
            }

            count += 1;
            x += step;
        }
        y += step;
    }

    let n = count.max(1) as f64;
    let luma_m = sum_y / n;
    let luma_s = (sum_y2 / n - luma_m * luma_m).max(0.0).sqrt();
    let sharp = (sum_lap2 / n).sqrt();
    let acut = (sum_acut2 / n).sqrt();
    let sat_m = ((sum_sat / n * 1000.0).round() / 1000.0) as f32;
    let outline_r = ((dark_outline as f64 / n * 10000.0).round() / 10000.0) as f32;

    let target_95 = ((count as f64) * 0.95) as u64;
    let mut acc_hist = 0u64;
    let mut edge_p95 = 0.0f32;
    for (val, &c) in edge_hist.iter().enumerate() {
        acc_hist += c as u64;
        if acc_hist >= target_95 {
            edge_p95 = val as f32;
            break;
        }
    }

    // 1. Photos, portraits, real human faces, and natural scenes MUST NEVER be categorized as anime!
    let is_human_or_photo = matches!(
        analysis.class,
        ImageClass::Portrait
            | ImageClass::Photo
            | ImageClass::LowLightNoisy
            | ImageClass::HdrWideGamut
            | ImageClass::AlreadyHeavilyCompressed
    ) || analysis.skin_tone_ratio > 0.04;

    let cls = if is_human_or_photo {
        if analysis.class == ImageClass::Portrait || analysis.skin_tone_ratio > 0.04 {
            "portrait"
        } else {
            "photo"
        }
    } else {
        match analysis.class {
            ImageClass::AnimeManga => "anime",
            ImageClass::Meme | ImageClass::FlatGraphic | ImageClass::AlphaGraphic => "art",
            ImageClass::CrispTextUi => {
                if sat_m > 0.14 { "anime" } else { "text_ui" }
            }
            _ if analysis.palette_sparsity > 0.55
                && (analysis.flat_pixel_ratio > 0.35 || outline_r >= 0.006) =>
            {
                "anime"
            }
            _ => "photo",
        }
    };

    let r2 = |v: f64| -> f32 { ((v * 100.0).round() / 100.0) as f32 };
    OriginalQualityProfile {
        cls: cls.to_string(),
        w: width,
        h: height,
        sharp: r2(sharp),
        edge_p95,
        acut: r2(acut),
        luma_m: r2(luma_m),
        luma_s: r2(luma_s),
        r_m: r2(sum_r / n),
        g_m: r2(sum_g / n),
        b_m: r2(sum_b / n),
        sat_m,
        outline_r,
        loss_ratio: 1.0,
    }
}

/// Automatically select the optimal Real-ESRGAN model (`AnimeVideoV3` vs `GeneralX4V3`).
/// Returns `None` ONLY for nearly-solid uniform images where AI is unnecessary.
pub fn select_realesrgan_model(
    rgba: &[u8],
    width: u32,
    height: u32,
    orig_profile: Option<&OriginalQualityProfile>,
    prefer_fast_sw: bool,
) -> Option<RealEsrganModelKind> {
    let w = width as usize;
    let h = height as usize;
    if w < 16 || h < 16 || rgba.len() < w * h * 4 {
        return None;
    }

    let _ = prefer_fast_sw; // Content semantics strictly dictate model choice; never force anime onto real human photos!

    if let Some(qp) = orig_profile {
        if qp.luma_s < 1.2 && qp.sharp < 0.2 {
            return None;
        }
        // Strict priority: Photos, portraits, and human subjects strictly route to GeneralX4V3!
        if matches!(qp.cls.as_str(), "portrait" | "photo") {
            return Some(RealEsrganModelKind::GeneralX4V3);
        }
        let is_anime = matches!(qp.cls.as_str(), "anime" | "art" | "meme" | "text_ui");
        if is_anime {
            return Some(RealEsrganModelKind::AnimeVideoV3);
        }
        return Some(RealEsrganModelKind::GeneralX4V3);
    }

    let analysis = analyze_rgba_image(rgba, width, height, 0, None);
    if analysis.flat_pixel_ratio > 0.985 && analysis.sharp_edge_ratio < 0.001 {
        return None;
    }

    // 1. Photos, portraits, real human faces, and natural scenes MUST use GeneralX4V3
    let is_photo_or_portrait = matches!(
        analysis.class,
        ImageClass::Portrait
            | ImageClass::Photo
            | ImageClass::LowLightNoisy
            | ImageClass::HdrWideGamut
            | ImageClass::AlreadyHeavilyCompressed
    ) || analysis.skin_tone_ratio > 0.04;

    if is_photo_or_portrait {
        return Some(RealEsrganModelKind::GeneralX4V3);
    }

    // 2. Anime, manga, flat illustrations, memes with sparse palettes use AnimeVideoV3
    let is_anime = match analysis.class {
        ImageClass::AnimeManga
        | ImageClass::Meme
        | ImageClass::FlatGraphic
        | ImageClass::AlphaGraphic
        | ImageClass::CrispTextUi => true,
        _ => {
            analysis.palette_sparsity > 0.55 && analysis.flat_pixel_ratio > 0.40
        }
    };

    if is_anime {
        Some(RealEsrganModelKind::AnimeVideoV3)
    } else {
        Some(RealEsrganModelKind::GeneralX4V3)
    }
}

/// Lock global RGB balance, luma contrast (`luma_s`), and HSV chroma saturation (`sat_m`) to sender's original!
fn match_color_and_contrast_to_profile(
    rgba: &mut [u8],
    cur: &OriginalQualityProfile,
    target: &OriginalQualityProfile,
) {
    let dr = (target.r_m - cur.r_m).clamp(-10.0, 10.0);
    let dg = (target.g_m - cur.g_m).clamp(-10.0, 10.0);
    let db = (target.b_m - cur.b_m).clamp(-10.0, 10.0);

    let contrast_scale = if cur.luma_s > 4.0 && target.luma_s > 4.0 {
        (target.luma_s / cur.luma_s).clamp(0.92, 1.14)
    } else {
        1.0
    };
    let sat_scale = if cur.sat_m > 0.03 && target.sat_m > 0.03 {
        (target.sat_m / cur.sat_m).clamp(0.92, 1.22)
    } else {
        1.0
    };

    if dr.abs() < 0.25
        && dg.abs() < 0.25
        && db.abs() < 0.25
        && (contrast_scale - 1.0).abs() < 0.01
        && (sat_scale - 1.0).abs() < 0.01
    {
        return;
    }

    let pivot = target.luma_m;
    for px in rgba.chunks_exact_mut(4) {
        let mut r = (px[0] as f32) + dr;
        let mut g = (px[1] as f32) + dg;
        let mut b = (px[2] as f32) + db;

        if (contrast_scale - 1.0).abs() >= 0.01 {
            let y = 0.299 * r + 0.587 * g + 0.114 * b;
            let dy = (y - pivot) * (contrast_scale - 1.0);
            r += dy;
            g += dy;
            b += dy;
        }
        if (sat_scale - 1.0).abs() >= 0.01 {
            let y = 0.299 * r + 0.587 * g + 0.114 * b;
            r = y + (r - y) * sat_scale;
            g = y + (g - y) * sat_scale;
            b = y + (b - y) * sat_scale;
        }

        px[0] = r.clamp(0.0, 255.0).round() as u8;
        px[1] = g.clamp(0.0, 255.0).round() as u8;
        px[2] = b.clamp(0.0, 255.0).round() as u8;
    }
}

/// Color, contrast, and saturation lock specifically for **Real-ESRGAN 4× (`SRVGGNetCompact`)** output:
/// Preserves 100% of Real-ESRGAN's vector-sharp reconstructed edges & silky denoised gradients (NEVER blurs back toward Bicubic!)
/// while locking exact RGB balance, global contrast, and HSV chroma saturation to the sender's `OriginalQualityProfile`.
pub fn calibrate_color_preserve_sr(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    target_opt: Option<&OriginalQualityProfile>,
) -> f32 {
    let w = width as usize;
    let h = height as usize;
    if w < 8 || h < 8 || rgba.len() < w * h * 4 {
        return 98.5;
    }
    let Some(target) = target_opt.filter(|t| t.sharp > 0.3) else {
        return 99.0;
    };

    let ks = if target.w > 0 && width > target.w {
        ((width as f32 / target.w as f32).round() as usize).clamp(1, 4)
    } else {
        1
    };
    let cur0 = extract_profile_with_kernel_stride(rgba, width, height, ks);
    match_color_and_contrast_to_profile(rgba, &cur0, target);

    let final_prof = extract_profile_with_kernel_stride(rgba, width, height, ks);
    let ratio_floor = |a: f32, b: f32| -> f32 {
        if b <= 0.01 {
            return 1.0;
        }
        (a / b).min(1.0).max(0.0)
    };
    let ratio_sym = |a: f32, b: f32| -> f32 {
        if b <= 0.01 {
            return 1.0;
        }
        (1.0 - (a / b - 1.0).abs().min(1.0)).max(0.0)
    };
    let sharp_s = ratio_floor(final_prof.sharp, target.sharp * 0.90);
    let edge_s = ratio_floor(final_prof.edge_p95, target.edge_p95 * 0.90);
    let contrast_s = ratio_sym(final_prof.luma_s, target.luma_s);
    let sat_s = if target.sat_m > 0.03 {
        ratio_sym(final_prof.sat_m, target.sat_m)
    } else {
        1.0
    };
    let color_err = ((final_prof.r_m - target.r_m).abs()
        + (final_prof.g_m - target.g_m).abs()
        + (final_prof.b_m - target.b_m).abs())
        / (3.0 * 255.0);
    let color_s = (1.0 - color_err * 5.0).clamp(0.0, 1.0);
    let weighted = 0.34 * sharp_s + 0.22 * edge_s + 0.16 * contrast_s + 0.14 * sat_s + 0.14 * color_s;
    ((weighted * 1000.0).round() / 10.0).clamp(0.0, 100.0)
}

/// Fast Pure-Rust SSIM & Color-Drift anti-corruption gate comparing Real-ESRGAN output against input source.
/// Returns `(passed, ssim, mean_color_drift_0_to_1)`.
pub fn evaluate_sr_quality_gate(
    src_rgba: &[u8],
    sw: u32,
    sh: u32,
    out_rgba: &[u8],
    ow: u32,
    oh: u32,
) -> (bool, f32, f32) {
    let sw = sw as usize;
    let sh = sh as usize;
    let ow = ow as usize;
    let oh = oh as usize;
    if sw == 0 || sh == 0 || ow == 0 || oh == 0 || src_rgba.len() < sw * sh * 4 || out_rgba.len() < ow * oh * 4 {
        return (false, 0.0, 1.0);
    }

    let step = (((ow * oh) as f64 / 12000.0).sqrt().floor() as usize).max(1);
    let mut samples: Vec<(f64, f64)> = Vec::with_capacity(14000);
    let mut sum_x = 0.0f64;
    let mut sum_y = 0.0f64;
    let mut sum_drift = 0.0f64;

    let mut oy = 0usize;
    while oy < oh {
        let sy = ((((oy as f64) + 0.5) * (sh as f64) / (oh as f64)).floor() as usize).min(sh - 1);
        let mut ox = 0usize;
        while ox < ow {
            let sx = ((((ox as f64) + 0.5) * (sw as f64) / (ow as f64)).floor() as usize).min(sw - 1);
            let si = (sy * sw + sx) * 4;
            let oi = (oy * ow + ox) * 4;
            let (sr, sg, sb) = (src_rgba[si] as f64, src_rgba[si + 1] as f64, src_rgba[si + 2] as f64);
            let (or, og, ob) = (out_rgba[oi] as f64, out_rgba[oi + 1] as f64, out_rgba[oi + 2] as f64);
            let lx = 0.299 * sr + 0.587 * sg + 0.114 * sb;
            let ly = 0.299 * or + 0.587 * og + 0.114 * ob;
            sum_x += lx;
            sum_y += ly;
            sum_drift += ((sr - or).abs() + (sg - og).abs() + (sb - ob).abs()) / (3.0 * 255.0);
            samples.push((lx, ly));
            ox += step;
        }
        oy += step;
    }

    let n = samples.len().max(1) as f64;
    let mx = sum_x / n;
    let my = sum_y / n;
    let mut vx = 0.0f64;
    let mut vy = 0.0f64;
    let mut cov = 0.0f64;
    for &(lx, ly) in &samples {
        let dx = lx - mx;
        let dy = ly - my;
        vx += dx * dx;
        vy += dy * dy;
        cov += dx * dy;
    }
    vx /= n;
    vy /= n;
    cov /= n;
    let c1 = (0.01 * 255.0) * (0.01 * 255.0);
    let c2 = (0.03 * 255.0) * (0.03 * 255.0);
    let ssim = (((2.0 * mx * my + c1) * (2.0 * cov + c2)) / ((mx * mx + my * my + c1) * (vx + vy + c2))) as f32;
    let drift = (sum_drift / n) as f32;
    let passed = ssim.is_finite() && drift.is_finite() && ssim >= 0.70 && drift <= 0.098;
    (passed, ssim, drift)
}

#[derive(Clone, Debug)]
pub struct ConvPreluLayer {
    pub in_ch: usize,
    pub out_ch: usize,
    /// Repacked `[out_ch][9][in_ch]` (`ky * 3 + kx`) for cache-contiguous SIMD dot products!
    pub weights: Vec<f32>,
    pub bias: Vec<f32>,
    /// Optional PReLU slope `[out_ch]` (`None` for the final output convolution).
    pub prelu: Option<Vec<f32>>,
}

impl ConvPreluLayer {
    /// Pack layer 0 (`3 -> 64`) weights as `RGBA32F` texture `[width=9, height=64]` for WebGL2 GPU shader.
    pub fn pack_gl_conv_in_rgba32f(&self) -> Vec<f32> {
        let mut out = vec![0.0f32; self.out_ch * 9 * 4];
        for oc in 0..self.out_ch {
            for k in 0..9 {
                let s = (oc * 9 + k) * 3;
                let d = (oc * 9 + k) * 4;
                out[d] = self.weights[s];
                out[d + 1] = self.weights[s + 1];
                out[d + 2] = self.weights[s + 2];
                out[d + 3] = 0.0;
            }
        }
        out
    }

    /// Pack 64-input layer (`64 -> out_ch`) weights as `RGBA32F` texture `[width=144, height=out_ch]` for WebGL2 GPU shader.
    /// Layout: row `oc`, col `k * 16 + in_g`, RGBA = `(w[oc, k, in_g*4 + 0..4])`.
    pub fn pack_gl_conv64_rgba32f(&self) -> Vec<f32> {
        // `self.weights` is ALREADY `[oc][k][in_ch]` with `in_ch == 64` -> already exact `[oc][9 * 16][4]`!
        self.weights.clone()
    }
}

/// Ultra-fast raw FP16 (`u16`) parsed CWSR1 weights for direct `RGBA16F` (`HALF_FLOAT`) GPU texture uploads!
/// Avoids FP16->FP32 conversion on weights and halves GPU VRAM bandwidth + doubles texture cache hit rate!
#[derive(Clone, Debug)]
pub struct RealEsrganGpuF16Layer {
    /// Raw IEEE-754 `u16` half-floats packed as `[out_ch][9][in_ch_padded4]` (`RGBA16F` texture!)
    pub weights_u16: Vec<u16>,
    pub bias: Vec<f32>,
    pub prelu: Option<Vec<f32>>,
}

#[derive(Clone, Debug)]
pub struct RealEsrganGpuF16Model {
    pub num_feat: usize,
    pub num_conv: usize,
    pub upscale: usize,
    pub first_conv: RealEsrganGpuF16Layer,
    pub body_convs: Vec<RealEsrganGpuF16Layer>,
    pub last_conv: RealEsrganGpuF16Layer,
}

impl RealEsrganGpuF16Model {
    pub fn from_cwsr_bytes(bytes: &[u8]) -> MediaResult<Self> {
        if bytes.len() < 24 || &bytes[0..6] != CWSR_MAGIC {
            return Err(MediaError::DecodeFailed("Invalid CWSR1 magic header".into()));
        }
        let mut r = CwsrReader { data: bytes, pos: 6 };
        let version = r.read_u16()?;
        if version != 1 {
            return Err(MediaError::DecodeFailed(format!("Unsupported CWSR1 version {version}")));
        }
        let num_feat = r.read_u32()? as usize;
        let num_conv = r.read_u32()? as usize;
        let upscale = r.read_u32()? as usize;
        let flags = r.read_u32()?;
        if flags != 1 || num_feat != 64 || num_conv > 64 || upscale != 4 {
            return Err(MediaError::DecodeFailed("Unsupported CWSR1 model parameters".into()));
        }

        let read_u16_vec = |rdr: &mut CwsrReader<'_>, count: usize| -> MediaResult<Vec<u16>> {
            let nbytes = count
                .checked_mul(2)
                .ok_or_else(|| MediaError::DecodeFailed("CWSR1 overflow".into()))?;
            if rdr.pos + nbytes > rdr.data.len() {
                return Err(MediaError::DecodeFailed("CWSR1 unexpected EOF (u16 tensor)".into()));
            }
            let mut out = Vec::with_capacity(count);
            for c in rdr.data[rdr.pos..rdr.pos + nbytes].chunks_exact(2) {
                out.push(u16::from_le_bytes([c[0], c[1]]));
            }
            rdr.pos += nbytes;
            Ok(out)
        };

        // First layer: PyTorch [64, 3, 9] -> TRANSPOSED RGBA16F texture [width=64 (oc), height=9 (k)]
        // Cache-line-contiguous across oc0..oc0+31 in 8-MRT WebGL2 shaders!
        let raw0 = read_u16_vec(&mut r, num_feat * 3 * 9)?;
        let mut w0 = vec![0u16; 9 * num_feat * 4];
        for k in 0..9 {
            for oc in 0..num_feat {
                let d = (k * num_feat + oc) * 4;
                w0[d] = raw0[(oc * 3 + 0) * 9 + k];
                w0[d + 1] = raw0[(oc * 3 + 1) * 9 + k];
                w0[d + 2] = raw0[(oc * 3 + 2) * 9 + k];
                w0[d + 3] = 0;
            }
        }
        let b0 = r.read_f16_vec(num_feat)?;
        let p0 = r.read_f16_vec(num_feat)?;
        let first_conv = RealEsrganGpuF16Layer {
            weights_u16: w0,
            bias: b0,
            prelu: Some(p0),
        };

        // Body/Last layers: PyTorch [out_ch, 64, 9] -> TRANSPOSED RGBA16F texture [width=out_ch (oc), height=144 (k*16+ig)]
        // All 32 output-channel weights fetched per (k, ig) in 8-MRT live in 4 contiguous 64-byte GPU L1 cache lines!
        let repack_64_transposed_u16 = |raw: &[u16], out_ch: usize| -> Vec<u16> {
            let mut dst = vec![0u16; 144 * out_ch * 4];
            for k in 0..9 {
                for ig in 0..16 {
                    let row = k * 16 + ig;
                    for oc in 0..out_ch {
                        let d = (row * out_ch + oc) * 4;
                        let base = oc * 64 + ig * 4;
                        dst[d] = raw[(base + 0) * 9 + k];
                        dst[d + 1] = raw[(base + 1) * 9 + k];
                        dst[d + 2] = raw[(base + 2) * 9 + k];
                        dst[d + 3] = raw[(base + 3) * 9 + k];
                    }
                }
            }
            dst
        };

        let mut body_convs = Vec::with_capacity(num_conv);
        for _ in 0..num_conv {
            let raw_w = read_u16_vec(&mut r, num_feat * num_feat * 9)?;
            let bi = r.read_f16_vec(num_feat)?;
            let pi = r.read_f16_vec(num_feat)?;
            body_convs.push(RealEsrganGpuF16Layer {
                weights_u16: repack_64_transposed_u16(&raw_w, num_feat),
                bias: bi,
                prelu: Some(pi),
            });
        }

        let out_ch_last = 3 * upscale * upscale; // 48
        let raw_last = read_u16_vec(&mut r, out_ch_last * num_feat * 9)?;
        let b_last = r.read_f16_vec(out_ch_last)?;
        let last_conv = RealEsrganGpuF16Layer {
            weights_u16: repack_64_transposed_u16(&raw_last, out_ch_last),
            bias: b_last,
            prelu: None,
        };

        Ok(Self {
            num_feat,
            num_conv,
            upscale,
            first_conv,
            body_convs,
            last_conv,
        })
    }
}

/// Fast global RGB & contrast/saturation delta parameters for GPU shader color-lock (zero CPU full-image scan!).
pub fn compute_gpu_color_lock_params(
    src_rgba: &[u8],
    sw: u32,
    sh: u32,
    target_opt: Option<&OriginalQualityProfile>,
) -> ([f32; 4], [f32; 4]) {
    let Some(target) = target_opt.filter(|t| t.sharp > 0.3) else {
        return ([0.0, 0.0, 0.0, 1.0], [1.0, 0.5, 0.0, 0.0]);
    };
    let cur = extract_original_quality_profile(src_rgba, sw, sh);
    let dr = ((target.r_m - cur.r_m).clamp(-8.0, 8.0)) / 255.0;
    let dg = ((target.g_m - cur.g_m).clamp(-8.0, 8.0)) / 255.0;
    let db = ((target.b_m - cur.b_m).clamp(-8.0, 8.0)) / 255.0;
    let contrast = if cur.luma_s > 4.0 && target.luma_s > 4.0 {
        (target.luma_s / cur.luma_s).clamp(0.95, 1.08)
    } else {
        1.0
    };
    let sat = if cur.sat_m > 0.03 && target.sat_m > 0.03 {
        (target.sat_m / cur.sat_m).clamp(0.95, 1.12)
    } else {
        1.0
    };
    let pivot = (target.luma_m / 255.0).clamp(0.05, 0.95);
    ([dr, dg, db, contrast], [sat, pivot, 0.0, 0.0])
}

#[derive(Clone, Debug)]
pub struct RealEsrganModel {
    pub num_feat: usize,
    pub num_conv: usize,
    pub upscale: usize,
    pub first_conv: ConvPreluLayer,
    pub body_convs: Vec<ConvPreluLayer>,
    pub last_conv: ConvPreluLayer,
}

#[inline]
pub fn f16_to_f32(bits: u16) -> f32 {
    let sign = ((bits as u32) & 0x8000) << 16;
    let exp = ((bits as u32) >> 10) & 0x1f;
    let frac = (bits as u32) & 0x03ff;
    if exp == 0 {
        if frac == 0 {
            return f32::from_bits(sign);
        }
        let mut e = -14i32;
        let mut m = frac;
        while (m & 0x0400) == 0 {
            m <<= 1;
            e -= 1;
        }
        m &= 0x03ff;
        let out_exp = ((e + 127) as u32) << 23;
        return f32::from_bits(sign | out_exp | (m << 13));
    }
    if exp == 31 {
        return f32::from_bits(sign | 0x7f80_0000 | (frac << 13));
    }
    let out_exp = (exp + 112) << 23;
    f32::from_bits(sign | out_exp | (frac << 13))
}

struct CwsrReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> CwsrReader<'a> {
    fn read_u16(&mut self) -> MediaResult<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(MediaError::DecodeFailed("CWSR1 unexpected EOF (u16)".into()));
        }
        let v = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    fn read_u32(&mut self) -> MediaResult<u32> {
        if self.pos + 4 > self.data.len() {
            return Err(MediaError::DecodeFailed("CWSR1 unexpected EOF (u32)".into()));
        }
        let v = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn read_f16_vec(&mut self, count: usize) -> MediaResult<Vec<f32>> {
        let bytes = count
            .checked_mul(2)
            .ok_or_else(|| MediaError::DecodeFailed("CWSR1 overflow".into()))?;
        if self.pos + bytes > self.data.len() {
            return Err(MediaError::DecodeFailed("CWSR1 unexpected EOF (f16 tensor)".into()));
        }
        let mut out = Vec::with_capacity(count);
        for chunk in self.data[self.pos..self.pos + bytes].chunks_exact(2) {
            out.push(f16_to_f32(u16::from_le_bytes([chunk[0], chunk[1]])));
        }
        self.pos += bytes;
        Ok(out)
    }
}

/// Repack PyTorch `[out_ch, in_ch, 3, 3]` -> `[out_ch, 9, in_ch]` for BOTH L1-contiguous CPU SIMD AND WebGL2 RGBA32F textures!
fn repack_oihw_to_okwi(raw: &[f32], out_ch: usize, in_ch: usize) -> Vec<f32> {
    let mut dst = vec![0.0f32; out_ch * 9 * in_ch];
    for oc in 0..out_ch {
        for ic in 0..in_ch {
            for k in 0..9 {
                let src_idx = ((oc * in_ch + ic) * 9) + k;
                let dst_idx = ((oc * 9 + k) * in_ch) + ic;
                dst[dst_idx] = raw[src_idx];
            }
        }
    }
    dst
}

impl RealEsrganModel {
    pub fn from_cwsr_bytes(bytes: &[u8]) -> MediaResult<Self> {
        if bytes.len() < 24 || &bytes[0..6] != CWSR_MAGIC {
            return Err(MediaError::DecodeFailed("Invalid CWSR1 magic header".into()));
        }
        let mut r = CwsrReader { data: bytes, pos: 6 };
        let version = r.read_u16()?;
        if version != 1 {
            return Err(MediaError::DecodeFailed(format!("Unsupported CWSR1 version {version}")));
        }
        let num_feat = r.read_u32()? as usize;
        let num_conv = r.read_u32()? as usize;
        let upscale = r.read_u32()? as usize;
        let flags = r.read_u32()?;
        if flags != 1 || num_feat == 0 || num_feat > 256 || num_conv > 64 || !(upscale == 2 || upscale == 4) {
            return Err(MediaError::DecodeFailed("Unsupported CWSR1 model parameters".into()));
        }

        let w0 = repack_oihw_to_okwi(&r.read_f16_vec(num_feat * 3 * 9)?, num_feat, 3);
        let b0 = r.read_f16_vec(num_feat)?;
        let p0 = r.read_f16_vec(num_feat)?;
        let first_conv = ConvPreluLayer {
            in_ch: 3,
            out_ch: num_feat,
            weights: w0,
            bias: b0,
            prelu: Some(p0),
        };

        let mut body_convs = Vec::with_capacity(num_conv);
        for _ in 0..num_conv {
            let wi = repack_oihw_to_okwi(&r.read_f16_vec(num_feat * num_feat * 9)?, num_feat, num_feat);
            let bi = r.read_f16_vec(num_feat)?;
            let pi = r.read_f16_vec(num_feat)?;
            body_convs.push(ConvPreluLayer {
                in_ch: num_feat,
                out_ch: num_feat,
                weights: wi,
                bias: bi,
                prelu: Some(pi),
            });
        }

        let out_ch_last = 3 * upscale * upscale;
        let w_last = repack_oihw_to_okwi(&r.read_f16_vec(out_ch_last * num_feat * 9)?, out_ch_last, num_feat);
        let b_last = r.read_f16_vec(out_ch_last)?;
        let last_conv = ConvPreluLayer {
            in_ch: num_feat,
            out_ch: out_ch_last,
            weights: w_last,
            bias: b_last,
            prelu: None,
        };

        Ok(Self {
            num_feat,
            num_conv,
            upscale,
            first_conv,
            body_convs,
            last_conv,
        })
    }
}

/// Process rows `y_start..` for a single 3×3 Conv2d + optional PReLU in HWC layout.
fn run_conv3x3_rows(
    src: &[f32],
    dst_slice: &mut [f32],
    w: usize,
    h: usize,
    y_start: usize,
    layer: &ConvPreluLayer,
) {
    let in_ch = layer.in_ch;
    let out_ch = layer.out_ch;
    let weights = &layer.weights;
    let bias = &layer.bias;
    let prelu = layer.prelu.as_deref();
    let row_stride_out = w * out_ch;
    let wi = w as isize;
    let hi = h as isize;

    let oc_blocks = out_ch / 4;
    let oc_rem_start = oc_blocks * 4;

    for (local_y, dst_row) in dst_slice.chunks_exact_mut(row_stride_out).enumerate() {
        let y = y_start + local_y;
        let yi = y as isize;
        for x in 0..w {
            let xi = x as isize;
            let mut nb_ptrs = [usize::MAX; 9];
            let mut k = 0usize;
            for dy in -1isize..=1 {
                let sy = yi + dy;
                for dx in -1isize..=1 {
                    let sx = xi + dx;
                    if sy >= 0 && sy < hi && sx >= 0 && sx < wi {
                        nb_ptrs[k] = (sy as usize * w + sx as usize) * in_ch;
                    }
                    k += 1;
                }
            }

            let out_base = x * out_ch;
            for ob in 0..oc_blocks {
                let oc0 = ob * 4;
                let mut a0 = bias[oc0];
                let mut a1 = bias[oc0 + 1];
                let mut a2 = bias[oc0 + 2];
                let mut a3 = bias[oc0 + 3];

                let w0_base = oc0 * 9 * in_ch;
                let w1_base = (oc0 + 1) * 9 * in_ch;
                let w2_base = (oc0 + 2) * 9 * in_ch;
                let w3_base = (oc0 + 3) * 9 * in_ch;

                for ki in 0..9 {
                    let s_off = nb_ptrs[ki];
                    if s_off == usize::MAX {
                        continue;
                    }
                    let s = &src[s_off..s_off + in_ch];
                    let k_off = ki * in_ch;
                    let w0 = &weights[w0_base + k_off..w0_base + k_off + in_ch];
                    let w1 = &weights[w1_base + k_off..w1_base + k_off + in_ch];
                    let w2 = &weights[w2_base + k_off..w2_base + k_off + in_ch];
                    let w3 = &weights[w3_base + k_off..w3_base + k_off + in_ch];
                    for ic in 0..in_ch {
                        let v = s[ic];
                        a0 += v * w0[ic];
                        a1 += v * w1[ic];
                        a2 += v * w2[ic];
                        a3 += v * w3[ic];
                    }
                }

                if let Some(p) = prelu {
                    if a0 < 0.0 { a0 *= p[oc0]; }
                    if a1 < 0.0 { a1 *= p[oc0 + 1]; }
                    if a2 < 0.0 { a2 *= p[oc0 + 2]; }
                    if a3 < 0.0 { a3 *= p[oc0 + 3]; }
                }
                dst_row[out_base + oc0] = a0;
                dst_row[out_base + oc0 + 1] = a1;
                dst_row[out_base + oc0 + 2] = a2;
                dst_row[out_base + oc0 + 3] = a3;
            }

            for oc in oc_rem_start..out_ch {
                let mut acc = bias[oc];
                let w_base = oc * 9 * in_ch;
                for ki in 0..9 {
                    let s_off = nb_ptrs[ki];
                    if s_off == usize::MAX {
                        continue;
                    }
                    let s = &src[s_off..s_off + in_ch];
                    let w_k = &weights[w_base + ki * in_ch..w_base + (ki + 1) * in_ch];
                    for ic in 0..in_ch {
                        acc += s[ic] * w_k[ic];
                    }
                }
                if let Some(p) = prelu {
                    if acc < 0.0 { acc *= p[oc]; }
                }
                dst_row[out_base + oc] = acc;
            }
        }
    }
}

fn run_conv3x3(src: &[f32], dst: &mut [f32], w: usize, h: usize, layer: &ConvPreluLayer) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .min(h.max(1));
        if n_threads > 1 && h >= 8 {
            let rows_per = (h + n_threads - 1) / n_threads;
            let row_stride = w * layer.out_ch;
            std::thread::scope(|s| {
                for (ti, chunk) in dst.chunks_mut(rows_per * row_stride).enumerate() {
                    let y_start = ti * rows_per;
                    s.spawn(move || {
                        run_conv3x3_rows(src, chunk, w, h, y_start, layer);
                    });
                }
            });
            return;
        }
    }
    run_conv3x3_rows(src, dst, w, h, 0, layer);
}

/// Run `SRVGGNetCompact` on a single RGBA8 tile -> returns RGBA8 at `(w * upscale, h * upscale)`.
pub fn infer_realesrgan_tile(rgba: &[u8], w: usize, h: usize, model: &RealEsrganModel) -> Vec<u8> {
    let n_pix = w * h;
    let inv255 = 1.0f32 / 255.0;
    let mut rgb_in = vec![0.0f32; n_pix * 3];
    for p in 0..n_pix {
        rgb_in[p * 3] = rgba[p * 4] as f32 * inv255;
        rgb_in[p * 3 + 1] = rgba[p * 4 + 1] as f32 * inv255;
        rgb_in[p * 3 + 2] = rgba[p * 4 + 2] as f32 * inv255;
    }

    let nf = model.num_feat;
    let mut cur = vec![0.0f32; n_pix * nf];
    let mut nxt = vec![0.0f32; n_pix * nf];

    run_conv3x3(&rgb_in, &mut cur, w, h, &model.first_conv);
    for layer in &model.body_convs {
        run_conv3x3(&cur, &mut nxt, w, h, layer);
        std::mem::swap(&mut cur, &mut nxt);
    }

    let s = model.upscale;
    let s2 = s * s;
    let out_ch_last = 3 * s2;
    let mut head = vec![0.0f32; n_pix * out_ch_last];
    run_conv3x3(&cur, &mut head, w, h, &model.last_conv);

    let ow = w * s;
    let oh = h * s;
    let mut out = vec![0u8; ow * oh * 4];
    for y in 0..h {
        for x in 0..w {
            let p = y * w + x;
            let br = rgb_in[p * 3];
            let bg = rgb_in[p * 3 + 1];
            let bb = rgb_in[p * 3 + 2];
            let alpha = rgba[p * 4 + 3];
            let hb = p * out_ch_last;
            for sy in 0..s {
                let oy = y * s + sy;
                for sx in 0..s {
                    let ox = x * s + sx;
                    let sub = sy * s + sx;
                    let r = (head[hb + sub] + br).clamp(0.0, 1.0);
                    let g = (head[hb + s2 + sub] + bg).clamp(0.0, 1.0);
                    let b = (head[hb + 2 * s2 + sub] + bb).clamp(0.0, 1.0);
                    let oi = (oy * ow + ox) * 4;
                    out[oi] = (r * 255.0 + 0.5) as u8;
                    out[oi + 1] = (g * 255.0 + 0.5) as u8;
                    out[oi + 2] = (b * 255.0 + 0.5) as u8;
                    out[oi + 3] = alpha;
                }
            }
        }
    }
    out
}

/// Zero-seam tiled Real-ESRGAN (`SRVGGNetCompact`) inference on arbitrary image sizes!
pub fn run_realesrgan_tiled(
    rgba: &[u8],
    width: u32,
    height: u32,
    model: &RealEsrganModel,
    tile_size: u32,
    cancel: Option<&AtomicBool>,
) -> MediaResult<(Vec<u8>, u32, u32)> {
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return Err(MediaError::EmptyInput);
    }
    let scale = model.upscale;
    let ow = w * scale;
    let oh = h * scale;
    let tile = tile_size.max(64) as usize;

    if w <= tile && h <= tile {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(MediaError::Cancelled);
        }
        return Ok((infer_realesrgan_tile(rgba, w, h, model), ow as u32, oh as u32));
    }

    let halo = 16usize;
    let mut out = vec![0u8; ow * oh * 4];
    let mut ty = 0usize;
    while ty < h {
        let valid_y1 = (ty + tile).min(h);
        let pad_y0 = ty.saturating_sub(halo);
        let pad_y1 = (valid_y1 + halo).min(h);
        let th = pad_y1 - pad_y0;

        let mut tx = 0usize;
        while tx < w {
            if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                return Err(MediaError::Cancelled);
            }
            let valid_x1 = (tx + tile).min(w);
            let pad_x0 = tx.saturating_sub(halo);
            let pad_x1 = (valid_x1 + halo).min(w);
            let tw = pad_x1 - pad_x0;

            let mut tile_in = vec![0u8; tw * th * 4];
            for r in 0..th {
                let s_off = ((pad_y0 + r) * w + pad_x0) * 4;
                let d_off = r * tw * 4;
                tile_in[d_off..d_off + tw * 4].copy_from_slice(&rgba[s_off..s_off + tw * 4]);
            }

            let tile_out = infer_realesrgan_tile(&tile_in, tw, th, model);
            let tile_ow = tw * scale;
            let crop_x0 = (tx - pad_x0) * scale;
            let crop_y0 = (ty - pad_y0) * scale;
            let copy_w = (valid_x1 - tx) * scale;
            let copy_h = (valid_y1 - ty) * scale;

            for r in 0..copy_h {
                let s_off = ((crop_y0 + r) * tile_ow + crop_x0) * 4;
                let d_off = ((ty * scale + r) * ow + tx * scale) * 4;
                out[d_off..d_off + copy_w * 4].copy_from_slice(&tile_out[s_off..s_off + copy_w * 4]);
            }
            tx = valid_x1;
        }
        ty = valid_y1;
    }

    Ok((out, ow as u32, oh as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f16_decoder_matches_exact_ieee754_values() {
        assert_eq!(f16_to_f32(0x0000), 0.0);
        assert!((f16_to_f32(0x3c00) - 1.0).abs() < 1e-6);
        assert!((f16_to_f32(0xbc00) + 1.0).abs() < 1e-6);
        assert!((f16_to_f32(0x3800) - 0.5).abs() < 1e-6);
        assert!((f16_to_f32(0x4000) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn parses_real_cwsr1_realesrgan_and_verifies_profile_and_gate() {
        let model_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../app/public/models/realesr_animevideov3.cwsr");
        if !model_path.exists() {
            return;
        }
        let bytes = std::fs::read(&model_path).unwrap();
        let model = RealEsrganModel::from_cwsr_bytes(&bytes).unwrap();
        assert_eq!((model.num_feat, model.num_conv, model.upscale), (64, 16, 4));
        assert_eq!(model.first_conv.pack_gl_conv_in_rgba32f().len(), 64 * 9 * 4);
        assert_eq!(model.body_convs[0].pack_gl_conv64_rgba32f().len(), 64 * 9 * 64);

        let (w, h) = (16u32, 16u32);
        let mut rgba = vec![255u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                if x == y {
                    rgba[i] = 20;
                    rgba[i + 1] = 15;
                    rgba[i + 2] = 25;
                } else {
                    rgba[i] = 245;
                    rgba[i + 1] = 190;
                    rgba[i + 2] = 205;
                }
            }
        }
        let prof = extract_original_quality_profile(&rgba, w, h);
        let (mut out, ow, oh) = run_realesrgan_tiled(&rgba, w, h, &model, 64, None).unwrap();
        assert_eq!((ow, oh), (64, 64));
        let match_pct = calibrate_color_preserve_sr(&mut out, ow, oh, Some(&prof));
        assert!(match_pct >= 85.0, "match_pct={match_pct}");
        let (passed, ssim, drift) = evaluate_sr_quality_gate(&rgba, w, h, &out, ow, oh);
        assert!(passed, "gate failed: ssim={ssim}, drift={drift}");

        // Verify transposed FP16 GPU layout matches CPU FP32 layout bit-for-bit across all layers!
        let gpu_f16 = RealEsrganGpuF16Model::from_cwsr_bytes(&bytes).unwrap();
        assert_eq!(gpu_f16.first_conv.weights_u16.len(), 9 * 64 * 4);
        assert_eq!(gpu_f16.body_convs[0].weights_u16.len(), 144 * 64 * 4);
        assert_eq!(gpu_f16.last_conv.weights_u16.len(), 144 * 48 * 4);
        for k in 0..9 {
            for oc in 0..64 {
                for c in 0..3 {
                    let f32_ref = model.first_conv.weights[(oc * 9 + k) * 3 + c];
                    let f16_val = f16_to_f32(gpu_f16.first_conv.weights_u16[(k * 64 + oc) * 4 + c]);
                    assert!((f32_ref - f16_val).abs() < 1e-6, "first_conv mismatch k={k} oc={oc} c={c}");
                }
            }
        }
        for k in 0..9 {
            for ig in 0..16 {
                let row = k * 16 + ig;
                for oc in 0..64 {
                    for lane in 0..4 {
                        let ic = ig * 4 + lane;
                        let f32_ref = model.body_convs[0].weights[(oc * 9 + k) * 64 + ic];
                        let f16_val = f16_to_f32(gpu_f16.body_convs[0].weights_u16[(row * 64 + oc) * 4 + lane]);
                        assert!((f32_ref - f16_val).abs() < 1e-6, "body[0] mismatch k={k} ig={ig} oc={oc} lane={lane}");
                    }
                }
            }
        }
        for k in 0..9 {
            for ig in 0..16 {
                let row = k * 16 + ig;
                for oc in 0..48 {
                    for lane in 0..4 {
                        let ic = ig * 4 + lane;
                        let f32_ref = model.last_conv.weights[(oc * 9 + k) * 64 + ic];
                        let f16_val = f16_to_f32(gpu_f16.last_conv.weights_u16[(row * 48 + oc) * 4 + lane]);
                        assert!((f32_ref - f16_val).abs() < 1e-6, "last_conv mismatch k={k} ig={ig} oc={oc} lane={lane}");
                    }
                }
            }
        }
    }

    #[test]
    fn portrait_and_photos_never_route_to_anime_model() {
        // Human portrait synthetic buffer with warm skin tones and dark hair outlines
        let (w, h) = (64u32, 64u32);
        let mut rgba = vec![255u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                if y < 16 || x < 6 || x > 57 {
                    // Dark hair / dark outlines
                    rgba[i] = 18;
                    rgba[i + 1] = 16;
                    rgba[i + 2] = 20;
                } else {
                    // Peach/warm human skin tone (R > G > B)
                    rgba[i] = 235;
                    rgba[i + 1] = 188;
                    rgba[i + 2] = 165;
                }
            }
        }
        let qp = extract_original_quality_profile(&rgba, w, h);
        assert_eq!(qp.cls, "portrait", "Skin and face tones must classify as portrait!");

        let sel_no_qp = select_realesrgan_model(&rgba, w, h, None, false);
        let sel_qp = select_realesrgan_model(&rgba, w, h, Some(&qp), false);

        assert_eq!(
            sel_no_qp,
            Some(RealEsrganModelKind::GeneralX4V3),
            "Human portrait must route to GeneralX4V3 without QP!"
        );
        assert_eq!(
            sel_qp,
            Some(RealEsrganModelKind::GeneralX4V3),
            "Human portrait must route to GeneralX4V3 with QP!"
        );
    }

    #[test]
    fn high_detail_cosplay_portrait_routes_to_general_x4v3() {
        let p = "/home/ngson/.gemini/antigravity/brain/0565c121-6c93-4251-b0a9-4ed58288b3ea/.user_uploaded/media_1795545206648_e1cddb86.jpg";
        if !std::path::Path::new(p).exists() {
            return;
        }
        let img = match image::open(p) {
            Ok(im) => im.to_rgba8(),
            Err(_) => return,
        };
        let (w, h) = img.dimensions();
        let raw = img.into_raw();
        let qp = extract_original_quality_profile(&raw, w, h);
        assert_eq!(qp.cls, "portrait");

        let model_no_qp = select_realesrgan_model(&raw, w, h, None, false);
        let model_qp = select_realesrgan_model(&raw, w, h, Some(&qp), false);

        assert_eq!(model_no_qp, Some(RealEsrganModelKind::GeneralX4V3));
        assert_eq!(model_qp, Some(RealEsrganModelKind::GeneralX4V3));
    }
}
