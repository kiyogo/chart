use chatw_media_core::{
    ai::realesrgan::{
        RealEsrganModel, calibrate_color_preserve_sr, extract_original_quality_profile,
        run_realesrgan_tiled,
    },
    avif::{AvifCompressionMode, AvifEncodeOptions, optimize_image_to_avif},
    quality::evaluate_rgba_quality,
};
use image::{DynamicImage, RgbaImage, imageops::FilterType};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let img_path = args.get(1).expect("Usage: realesrgan_bench <input_png> <model_cwsr> <out_dir>");
    let model_path = args.get(2).expect("missing model_cwsr");
    let out_dir = args.get(3).expect("missing out_dir");
    std::fs::create_dir_all(out_dir).unwrap();

    let raw_bytes = std::fs::read(img_path).unwrap();
    let orig = image::load_from_memory(&raw_bytes).unwrap().to_rgba8();
    let (ow, oh) = orig.dimensions();
    println!("Loaded original: {}x{} ({:.1} KB)", ow, oh, raw_bytes.len() as f64 / 1024.0);

    let t0 = Instant::now();
    let avif = optimize_image_to_avif(
        &raw_bytes,
        &AvifEncodeOptions {
            mode: AvifCompressionMode::UltraAggressive,
            generate_thumbnail: false,
            threads: None,
            ..Default::default()
        },
    )
    .unwrap();
    println!(
        "10-bit UltraAggressive AVIF: {}x{} -> {} bytes ({:.1} KB, {:.1}x smaller) in {}ms",
        avif.width,
        avif.height,
        avif.avif_bytes.len(),
        avif.avif_bytes.len() as f64 / 1024.0,
        raw_bytes.len() as f64 / avif.avif_bytes.len().max(1) as f64,
        t0.elapsed().as_millis()
    );
    let avif_path = format!("{out_dir}/compressed.avif");
    std::fs::write(&avif_path, &avif.avif_bytes).unwrap();

    // Decode AVIF via ffmpeg/python or image crate (note: ravif encodes, image::avif may use dav1d if enabled, so test decoded pixels!)
    let dec = match image::load_from_memory_with_format(&avif.avif_bytes, image::ImageFormat::Avif) {
        Ok(im) => im.to_rgba8(),
        Err(_) => {
            let tmp_png = format!("{out_dir}/decoded_avif.png");
            if !std::path::Path::new(&tmp_png).exists() {
                let status = std::process::Command::new("ffmpeg")
                    .args(["-y", "-loglevel", "error", "-i", &avif_path, &tmp_png])
                    .status()
                    .expect("ffmpeg decode avif");
                assert!(status.success(), "ffmpeg failed decoding avif");
            }
            image::open(&tmp_png).unwrap().to_rgba8()
        }
    };
    let (dw, dh) = dec.dimensions();
    let dec_raw = dec.as_raw();
    println!("Decoded AVIF: {}x{}", dw, dh);

    let model_bytes = std::fs::read(model_path).unwrap();
    let model = RealEsrganModel::from_cwsr_bytes(&model_bytes).unwrap();
    println!(
        "Loaded Real-ESRGAN CWSR1: num_feat={}, num_conv={}, upscale={}x ({:.2} MB)",
        model.num_feat,
        model.num_conv,
        model.upscale,
        model_bytes.len() as f64 / 1024.0 / 1024.0
    );

    let t1 = Instant::now();
    let (sr4_rgba, sr4_w, sr4_h) = run_realesrgan_tiled(dec_raw, dw, dh, &model, 160, None).unwrap();
    println!(
        "Real-ESRGAN 4x completed: {}x{} -> {}x{} in {}ms",
        dw,
        dh,
        sr4_w,
        sr4_h,
        t1.elapsed().as_millis()
    );

    // Save raw 4x Real-ESRGAN (3344x1884!) AND 2x supersampled Lanczos3 -> 1672x941!
    let sr4_img = RgbaImage::from_raw(sr4_w, sr4_h, sr4_rgba).unwrap();
    sr4_img.save(format!("{out_dir}/realesrgan_4x.png")).unwrap();

    let sr2_img = DynamicImage::ImageRgba8(sr4_img)
        .resize_exact(ow, oh, FilterType::Lanczos3)
        .to_rgba8();
    sr2_img.save(format!("{out_dir}/realesrgan_supersampled_orig_res.png")).unwrap();

    // Also evaluate with subtle OriginalQualityProfile color/sharpness calibration
    let mut sr2_cal = sr2_img.as_raw().to_vec();
    let base2 = DynamicImage::ImageRgba8(dec.clone())
        .resize_exact(ow, oh, FilterType::CatmullRom)
        .to_rgba8()
        .into_raw();
    let match_pct = calibrate_color_preserve_sr(&mut sr2_cal, ow, oh, avif.orig_profile.as_ref());
    let sr2_cal_img = RgbaImage::from_raw(ow, oh, sr2_cal.clone()).unwrap();
    sr2_cal_img.save(format!("{out_dir}/realesrgan_calibrated.png")).unwrap();

    let orig_prof = extract_original_quality_profile(orig.as_raw(), ow, oh);
    let bic_q = evaluate_rgba_quality(orig.as_raw(), &base2, ow, oh);
    let sr2_q = evaluate_rgba_quality(orig.as_raw(), sr2_img.as_raw(), ow, oh);
    let cal_q = evaluate_rgba_quality(orig.as_raw(), &sr2_cal, ow, oh);
    let sr2_prof = extract_original_quality_profile(sr2_img.as_raw(), ow, oh);
    let cal_prof = extract_original_quality_profile(&sr2_cal, ow, oh);

    println!("=== TRUE PIXEL METRICS vs 1672x941 ORIGINAL PNG ===");
    println!("  Original:            sharp={:.2}, edge_p95={:.1}", orig_prof.sharp, orig_prof.edge_p95);
    println!("  Bicubic AVIF:        PSNR={:.2} dB, SSIM={:.4}, color_drift={:.2}", bic_q.psnr_db, bic_q.ssim, bic_q.color_drift_l1);
    println!("  Real-ESRGAN raw 2x:  PSNR={:.2} dB, SSIM={:.4}, sharp={:.2} ({:.1}%), color_drift={:.2}", sr2_q.psnr_db, sr2_q.ssim, sr2_prof.sharp, 100.0 * sr2_prof.sharp / orig_prof.sharp.max(0.1), sr2_q.color_drift_l1);
    println!("  Real-ESRGAN + QP:    PSNR={:.2} dB, SSIM={:.4}, sharp={:.2} ({:.1}%), match={:.1}%, color_drift={:.2}", cal_q.psnr_db, cal_q.ssim, cal_prof.sharp, 100.0 * cal_prof.sharp / orig_prof.sharp.max(0.1), match_pct, cal_q.color_drift_l1);
}
