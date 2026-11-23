//! Real perceptual image quality metrics (PSNR, SSIM, MS-SSIM, Laplacian edge sharpness,
//! Color Drift) and the Anti-Degradation Quality Gate for Adaptive AVIF & On-Device AI Restoration.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityReport {
    pub psnr_db: f64,
    pub ssim: f64,
    pub ms_ssim: f64,
    pub ref_sharpness: f64,
    pub test_sharpness: f64,
    pub color_drift_l1: f64,
    pub edge_preservation_ratio: f64,
}

/// Extract grayscale luma plane `[0.0, 255.0]` from RGBA8 buffer.
fn rgba_to_luma(rgba: &[u8], w: usize, h: usize) -> Vec<f64> {
    let n = w * h;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let base = i * 4;
        let r = rgba[base] as f64;
        let g = rgba[base + 1] as f64;
        let b = rgba[base + 2] as f64;
        out.push(0.299 * r + 0.587 * g + 0.114 * b);
    }
    out
}

/// Downsample luma plane by 2x using 2x2 box averaging for MS-SSIM pyramid.
fn downsample_2x(plane: &[f64], w: usize, h: usize) -> (Vec<f64>, usize, usize) {
    let nw = w / 2;
    let nh = h / 2;
    let mut out = Vec::with_capacity(nw * nh);
    for y in 0..nh {
        let sy = y * 2;
        for x in 0..nw {
            let sx = x * 2;
            let v = 0.25
                * (plane[sy * w + sx]
                    + plane[sy * w + sx + 1]
                    + plane[(sy + 1) * w + sx]
                    + plane[(sy + 1) * w + sx + 1]);
            out.push(v);
        }
    }
    (out, nw, nh)
}

/// Compute single-scale windowed SSIM (8x8 non-overlapping or strided blocks) returning `(ssim, cs_product)`.
fn ssim_plane(a: &[f64], b: &[f64], w: usize, h: usize) -> (f64, f64) {
    const C1: f64 = 6.5025; // (0.01 * 255)^2
    const C2: f64 = 58.5225; // (0.03 * 255)^2
    let win = 8usize.min(w).min(h).max(1);
    let step = if w > 512 || h > 512 { win * 2 } else { win };

    let mut ssim_sum = 0.0;
    let mut cs_sum = 0.0;
    let mut blocks = 0usize;

    let mut y = 0;
    while y + win <= h {
        let mut x = 0;
        while x + win <= w {
            let mut sum_a = 0.0;
            let mut sum_b = 0.0;
            let mut sum_a2 = 0.0;
            let mut sum_b2 = 0.0;
            let mut sum_ab = 0.0;
            let n = (win * win) as f64;
            for dy in 0..win {
                let row = (y + dy) * w + x;
                for dx in 0..win {
                    let va = a[row + dx];
                    let vb = b[row + dx];
                    sum_a += va;
                    sum_b += vb;
                    sum_a2 += va * va;
                    sum_b2 += vb * vb;
                    sum_ab += va * vb;
                }
            }
            let mu_a = sum_a / n;
            let mu_b = sum_b / n;
            let var_a = (sum_a2 / n - mu_a * mu_a).max(0.0);
            let var_b = (sum_b2 / n - mu_b * mu_b).max(0.0);
            let cov = sum_ab / n - mu_a * mu_b;

            let lum = (2.0 * mu_a * mu_b + C1) / (mu_a * mu_a + mu_b * mu_b + C1);
            let cs = (2.0 * cov + C2) / (var_a + var_b + C2);
            ssim_sum += (lum * cs).clamp(-1.0, 1.0);
            cs_sum += cs.clamp(0.0, 1.0);
            blocks += 1;
            x += step;
        }
        y += step;
    }
    if blocks == 0 {
        return (1.0, 1.0);
    }
    (ssim_sum / (blocks as f64), cs_sum / (blocks as f64))
}

/// Variance of the 3x3 discrete Laplacian kernel — measures crispness / micro-edge sharpness.
pub fn laplacian_sharpness(luma: &[f64], w: usize, h: usize) -> f64 {
    if w < 3 || h < 3 {
        return 0.0;
    }
    let step = if w > 512 || h > 512 { 2 } else { 1 };
    let mut sum = 0.0;
    let mut sum2 = 0.0;
    let mut n = 0usize;
    let mut y = 1;
    while y + 1 < h {
        let mut x = 1;
        while x + 1 < w {
            let lap = luma[(y - 1) * w + x]
                + luma[(y + 1) * w + x]
                + luma[y * w + (x - 1)]
                + luma[y * w + (x + 1)]
                - 4.0 * luma[y * w + x];
            sum += lap;
            sum2 += lap * lap;
            n += 1;
            x += step;
        }
        y += step;
    }
    if n == 0 {
        return 0.0;
    }
    let mean = sum / (n as f64);
    (sum2 / (n as f64) - mean * mean).max(0.0).sqrt()
}

/// Evaluate perceptual quality between two equal-sized RGBA8 images.
pub fn evaluate_rgba_quality(ref_rgba: &[u8], test_rgba: &[u8], width: u32, height: u32) -> QualityReport {
    let w = width as usize;
    let h = height as usize;
    let n = w * h;
    if n == 0 || ref_rgba.len() < n * 4 || test_rgba.len() < n * 4 {
        return QualityReport {
            psnr_db: 0.0,
            ssim: 0.0,
            ms_ssim: 0.0,
            ref_sharpness: 0.0,
            test_sharpness: 0.0,
            color_drift_l1: 255.0,
            edge_preservation_ratio: 0.0,
        };
    }

    let mut mse_sum = 0.0f64;
    let mut dr_sum = 0.0f64;
    let mut dg_sum = 0.0f64;
    let mut db_sum = 0.0f64;
    for i in 0..n {
        let base = i * 4;
        let dr = (ref_rgba[base] as f64) - (test_rgba[base] as f64);
        let dg = (ref_rgba[base + 1] as f64) - (test_rgba[base + 1] as f64);
        let db = (ref_rgba[base + 2] as f64) - (test_rgba[base + 2] as f64);
        mse_sum += (dr * dr + dg * dg + db * db) / 3.0;
        dr_sum += dr;
        dg_sum += dg;
        db_sum += db;
    }
    let mse = mse_sum / (n as f64);
    let psnr_db = if mse <= 1e-10 {
        99.9
    } else {
        10.0 * ((255.0 * 255.0) / mse).log10()
    };
    let color_drift_l1 = (dr_sum.abs() + dg_sum.abs() + db_sum.abs()) / (3.0 * (n as f64));

    let la = rgba_to_luma(ref_rgba, w, h);
    let lb = rgba_to_luma(test_rgba, w, h);
    let (ssim_s1, cs_s1) = ssim_plane(&la, &lb, w, h);

    // 3-scale MS-SSIM (weights normalized from Wang et al.: 0.2856, 0.4408, 0.2736)
    let ms_ssim = if w >= 32 && h >= 32 {
        let (la2, w2, h2) = downsample_2x(&la, w, h);
        let (lb2, _, _) = downsample_2x(&lb, w, h);
        let (_, cs_s2) = ssim_plane(&la2, &lb2, w2, h2);
        let (la3, w3, h3) = downsample_2x(&la2, w2, h2);
        let (lb3, _, _) = downsample_2x(&lb2, w2, h2);
        let (ssim_s3, _) = ssim_plane(&la3, &lb3, w3, h3);
        (cs_s1.max(1e-6).powf(0.2856) * cs_s2.max(1e-6).powf(0.4408) * ssim_s3.max(1e-6).powf(0.2736)).clamp(0.0, 1.0)
    } else {
        ssim_s1.clamp(0.0, 1.0)
    };

    let ref_sharpness = laplacian_sharpness(&la, w, h);
    let test_sharpness = laplacian_sharpness(&lb, w, h);
    let edge_preservation_ratio = if ref_sharpness > 1e-3 {
        test_sharpness / ref_sharpness
    } else {
        1.0
    };

    QualityReport {
        psnr_db,
        ssim: ssim_s1.clamp(0.0, 1.0),
        ms_ssim,
        ref_sharpness,
        test_sharpness,
        color_drift_l1,
        edge_preservation_ratio,
    }
}

/// Anti-Degradation Gate for On-Device AI Restoration (Section XII):
/// Ensures AI output did not shift colors, introduce ringing/over-sharpening, or hallucinate edges
/// worse than the decoded AVIF image. Returns `true` ONLY when AI output is safe & beneficial.
pub fn ai_output_passes_quality_gate(
    decoded_rgba: &[u8],
    enhanced_rgba: &[u8],
    width: u32,
    height: u32,
    is_text_sensitive: bool,
) -> (bool, QualityReport) {
    let report = evaluate_rgba_quality(decoded_rgba, enhanced_rgba, width, height);

    // Hard guards against color shift, hallucination/over-sharpening, or structural divergence:
    let max_color_drift = if is_text_sensitive { 2.5 } else { 6.0 };
    let min_ssim = if is_text_sensitive { 0.94 } else { 0.82 };
    let max_sharpness_boost = if is_text_sensitive { 1.45 } else { 2.35 };
    // Genuine denoising lowers raw Laplacian variance on noisy pixels while raising SSIM/MS-SSIM;
    // keep tighter floor on crisp text so glyphs never blur!
    let min_sharpness_ratio = if is_text_sensitive { 0.85 } else { 0.58 };

    let ok = report.color_drift_l1 <= max_color_drift
        && report.ssim >= min_ssim
        && report.edge_preservation_ratio >= min_sharpness_ratio
        && report.edge_preservation_ratio <= max_sharpness_boost;

    (ok, report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_images_score_perfect_ssim_and_psnr() {
        let mut rgba = vec![0u8; 64 * 64 * 4];
        for i in 0..(64 * 64) {
            rgba[i * 4] = (i % 251) as u8;
            rgba[i * 4 + 1] = ((i * 3) % 251) as u8;
            rgba[i * 4 + 2] = ((i * 7) % 251) as u8;
            rgba[i * 4 + 3] = 255;
        }
        let q = evaluate_rgba_quality(&rgba, &rgba, 64, 64);
        assert!(q.ssim > 0.9999);
        assert!(q.ms_ssim > 0.9999);
        assert!(q.psnr_db > 99.0);
        assert!(q.color_drift_l1 < 0.001);
    }

    #[test]
    fn quality_gate_rejects_color_shifted_ai_hallucination() {
        let src = vec![120u8; 64 * 64 * 4];
        let mut shifted = src.clone();
        for p in shifted.chunks_exact_mut(4) {
            p[0] = 200; // huge red color shift
            p[3] = 255;
        }
        let (ok, rep) = ai_output_passes_quality_gate(&src, &shifted, 64, 64, false);
        assert!(!ok, "Quality gate must reject severe color shift");
        assert!(rep.color_drift_l1 > 20.0);
    }
}
