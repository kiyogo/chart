//! Audio & Voice Codec Pipeline (Section XVI):
//! - Adaptive Opus configuration planner (Voice vs Music, VBR, mono/stereo, frame duration) & Ogg/Opus header parser
//! - Genuine pure-Rust FLAC lossless encoder (`flacenc`) & decoder (`claxon`) with bit-exact PCM verification
//! - Lightweight on-device adaptive speech denoiser (spectral noise-floor gate + high-pass DC/hum filter; never fabricates fake speech!)

#[cfg(feature = "audio-codecs")]
use crate::{MediaError, MediaResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioUseCase {
    VoiceNote,
    MusicOrGeneral,
    VideoAudioTrack,
    ArchivalLossless,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpusEncodeProfile {
    pub bitrate_bps: u32,
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub frame_duration_ms: u8,
    pub vbr: bool,
    pub application: &'static str, // "voip" | "audio"
    pub dtx: bool,
    pub mime_type: &'static str,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpusHeadInfo {
    pub version: u8,
    pub channels: u8,
    pub pre_skip: u16,
    pub input_sample_rate: u32,
    pub output_gain_db: f32,
}

/// Choose optimal Opus encoding parameters (Section XVI.2) ensuring clear speech & high efficiency.
pub fn plan_opus_profile(use_case: AudioUseCase, input_channels: u8, data_saver: bool) -> OpusEncodeProfile {
    match use_case {
        AudioUseCase::VoiceNote => OpusEncodeProfile {
            bitrate_bps: if data_saver { 20_000 } else { 28_000 },
            sample_rate_hz: 48_000,
            channels: 1,
            frame_duration_ms: 20,
            vbr: true,
            application: "voip",
            dtx: true,
            mime_type: "audio/ogg;codecs=opus",
        },
        AudioUseCase::MusicOrGeneral => OpusEncodeProfile {
            bitrate_bps: if data_saver { 48_000 } else { 80_000 },
            sample_rate_hz: 48_000,
            channels: input_channels.clamp(1, 2),
            frame_duration_ms: 20,
            vbr: true,
            application: "audio",
            dtx: false,
            mime_type: "audio/ogg;codecs=opus",
        },
        AudioUseCase::VideoAudioTrack => OpusEncodeProfile {
            bitrate_bps: if data_saver { 48_000 } else { 64_000 },
            sample_rate_hz: 48_000,
            channels: input_channels.clamp(1, 2),
            frame_duration_ms: 20,
            vbr: true,
            application: "audio",
            dtx: false,
            mime_type: "audio/webm;codecs=opus",
        },
        AudioUseCase::ArchivalLossless => OpusEncodeProfile {
            bitrate_bps: 160_000,
            sample_rate_hz: 48_000,
            channels: input_channels.clamp(1, 2),
            frame_duration_ms: 20,
            vbr: true,
            application: "audio",
            dtx: false,
            mime_type: "audio/flac",
        },
    }
}

/// Inspect Ogg/Opus binary header (`OpusHead` RFC 7845) to verify real Opus streams.
pub fn parse_ogg_opus_head(bytes: &[u8]) -> Option<OpusHeadInfo> {
    let pos = bytes.windows(8).position(|w| w == b"OpusHead")?;
    let slice = bytes.get(pos..pos + 19)?;
    let version = slice[8];
    let channels = slice[9];
    let pre_skip = u16::from_le_bytes([slice[10], slice[11]]);
    let input_sample_rate = u32::from_le_bytes([slice[12], slice[13], slice[14], slice[15]]);
    let gain_raw = i16::from_le_bytes([slice[16], slice[17]]);
    Some(OpusHeadInfo {
        version,
        channels,
        pre_skip,
        input_sample_rate,
        output_gain_db: (gain_raw as f32) / 256.0,
    })
}

/// Lightweight on-device voice denoiser (Section XVI.3):
/// 1. 1st-order high-pass filter (~80 Hz) removes AC mains hum & mic rumble without distorting speech.
/// 2. Frame-wise adaptive noise-floor soft-thresholding (Wiener-like gain smoothing) suppresses steady background hiss.
/// Never synthesizes fake speech! Leaves high-SNR clean audio virtually untouched.
pub fn denoise_voice_pcm_i16(samples: &[i16], sample_rate_hz: u32) -> Vec<i16> {
    if samples.len() < 320 || sample_rate_hz == 0 {
        return samples.to_vec();
    }

    // Step 1: High-pass filter at ~80Hz
    let rc = 1.0 / (2.0 * std::f32::consts::PI * 80.0);
    let dt = 1.0 / (sample_rate_hz as f32);
    let alpha = rc / (rc + dt);
    let mut hp = vec![0.0f32; samples.len()];
    let mut prev_x = samples[0] as f32;
    let mut prev_y = 0.0f32;
    for (i, &s) in samples.iter().enumerate() {
        let x = s as f32;
        let y = alpha * (prev_y + x - prev_x);
        hp[i] = y;
        prev_x = x;
        prev_y = y;
    }

    // Step 2: Frame energy analysis (10ms frames) to estimate stationary noise floor (15th percentile RMS)
    let frame_len = ((sample_rate_hz / 100) as usize).clamp(64, 1024);
    let mut frame_rms: Vec<f32> = hp
        .chunks(frame_len)
        .map(|c| {
            let e: f32 = c.iter().map(|v| v * v).sum::<f32>() / (c.len().max(1) as f32);
            e.sqrt()
        })
        .collect();
    if frame_rms.len() < 4 {
        return samples.to_vec();
    }
    let mut sorted_rms = frame_rms.clone();
    sorted_rms.sort_by(|a, b| a.total_cmp(b));
    let noise_floor = sorted_rms[(sorted_rms.len() as f32 * 0.15) as usize];
    let peak_rms = sorted_rms[sorted_rms.len() - 1];

    // If audio is already very clean (SNR > 28dB) or nearly silent, return untouched to avoid any distortion!
    if noise_floor < 30.0 || peak_rms < noise_floor * 1.5 || peak_rms > noise_floor * 28.0 {
        return hp.into_iter().map(|v| v.clamp(-32768.0, 32767.0) as i16).collect();
    }

    // Smooth Wiener-style gain per frame (never hard-mutes to 0 so voice stays natural)
    let mut prev_gain = 1.0f32;
    let mut out = Vec::with_capacity(samples.len());
    for (idx, chunk) in hp.chunks(frame_len).enumerate() {
        let rms = frame_rms[idx].max(1e-3);
        let snr_sq = (rms * rms) / ((noise_floor * noise_floor).max(1.0));
        let target_gain = if snr_sq <= 1.2 {
            0.22 // gentle -13dB noise floor attenuation
        } else {
            ((snr_sq - 1.0) / snr_sq).clamp(0.22, 1.0)
        };
        for &v in chunk {
            prev_gain = 0.92 * prev_gain + 0.08 * target_gain;
            out.push((v * prev_gain).clamp(-32768.0, 32767.0) as i16);
        }
    }
    frame_rms.clear();
    out
}

/// Extract compact 5-bit amplitude waveform (`0..=31`, 48 bars) for ChatW voice message bubbles.
pub fn extract_waveform_5bit(samples: &[i16], bars: usize) -> Vec<u8> {
    if samples.is_empty() || bars == 0 {
        return Vec::new();
    }
    let step = (samples.len() / bars).max(1);
    let mut peaks = Vec::with_capacity(bars);
    let mut max_peak = 1i32;
    for i in 0..bars {
        let start = i * step;
        if start >= samples.len() {
            break;
        }
        let end = (start + step).min(samples.len());
        let p = samples[start..end]
            .iter()
            .map(|&s| (s as i32).abs())
            .max()
            .unwrap_or(0);
        if p > max_peak {
            max_peak = p;
        }
        peaks.push(p);
    }
    peaks
        .into_iter()
        .map(|p| (((p as f32) / (max_peak as f32)) * 31.0).round().clamp(1.0, 31.0) as u8)
        .collect()
}

#[cfg(feature = "audio-codecs")]
mod impl_flac {
    use super::*;
    use flacenc::bitsink::ByteSink;
    use flacenc::component::BitRepr;
    use flacenc::config::Encoder as FlacConfig;
    use flacenc::error::Verify;
    use flacenc::source::MemSource;
    use std::io::Cursor;

    #[derive(Clone, Debug)]
    pub struct DecodedFlacPcm {
        pub samples_i32: Vec<i32>,
        pub sample_rate_hz: u32,
        pub channels: u32,
        pub bits_per_sample: u32,
    }

    /// Encode interleaved signed 16-bit PCM into genuine lossless FLAC (`fLaC` magic bytes).
    pub fn encode_pcm_i16_to_flac(
        interleaved_pcm: &[i16],
        sample_rate_hz: usize,
        channels: usize,
    ) -> MediaResult<Vec<u8>> {
        if interleaved_pcm.is_empty() || channels == 0 || channels > 8 || sample_rate_hz == 0 {
            return Err(MediaError::EmptyInput);
        }
        let samples_i32: Vec<i32> = interleaved_pcm.iter().map(|&s| s as i32).collect();
        let cfg = FlacConfig::default()
            .into_verified()
            .map_err(|(_, e)| MediaError::AudioCodecFailed(format!("flac config: {e:?}")))?;
        let source = MemSource::from_samples(&samples_i32, channels, 16, sample_rate_hz);
        let flac_stream = flacenc::encode_with_fixed_block_size(&cfg, source, cfg.block_size)
            .map_err(|e| MediaError::AudioCodecFailed(format!("flac encode: {e:?}")))?;
        let mut sink = ByteSink::new();
        flac_stream
            .write(&mut sink)
            .map_err(|e| MediaError::AudioCodecFailed(format!("flac write: {e:?}")))?;
        let out = sink.into_inner();
        if !out.starts_with(b"fLaC") {
            return Err(MediaError::AudioCodecFailed("missing fLaC header".into()));
        }
        Ok(out)
    }

    /// Decode genuine FLAC bytes back to exact PCM samples using `claxon`.
    pub fn decode_flac_to_pcm(flac_bytes: &[u8]) -> MediaResult<DecodedFlacPcm> {
        let mut reader = claxon::FlacReader::new(Cursor::new(flac_bytes))
            .map_err(|e| MediaError::AudioCodecFailed(format!("flac open: {e}")))?;
        let info = reader.streaminfo();
        let mut samples = Vec::new();
        for s in reader.samples() {
            samples.push(s.map_err(|e| MediaError::AudioCodecFailed(format!("flac sample: {e}")))?);
        }
        Ok(DecodedFlacPcm {
            samples_i32: samples,
            sample_rate_hz: info.sample_rate,
            channels: info.channels,
            bits_per_sample: info.bits_per_sample,
        })
    }
}

#[cfg(feature = "audio-codecs")]
pub use impl_flac::{DecodedFlacPcm, decode_flac_to_pcm, encode_pcm_i16_to_flac};

#[cfg(all(test, feature = "audio-codecs"))]
mod tests {
    use super::*;

    #[test]
    fn flac_lossless_roundtrip_is_bit_exact_and_smaller_than_pcm() {
        let sr = 48_000usize;
        // 0.25s stereo 440Hz + 880Hz tone
        let mut pcm = Vec::with_capacity(sr / 2);
        for i in 0..(sr / 4) {
            let t = (i as f32) / (sr as f32);
            let l = ((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 14000.0) as i16;
            let r = ((2.0 * std::f32::consts::PI * 880.0 * t).sin() * 12000.0) as i16;
            pcm.push(l);
            pcm.push(r);
        }
        let raw_pcm_bytes = pcm.len() * 2;
        let flac = encode_pcm_i16_to_flac(&pcm, sr, 2).expect("flac encode");
        assert!(flac.starts_with(b"fLaC"));
        assert!(flac.len() < raw_pcm_bytes, "FLAC ({}) must compress raw PCM ({})", flac.len(), raw_pcm_bytes);

        let decoded = decode_flac_to_pcm(&flac).expect("flac decode");
        assert_eq!(decoded.sample_rate_hz, 48_000);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples_i32.len(), pcm.len());
        for (orig, dec) in pcm.iter().zip(decoded.samples_i32.iter()) {
            assert_eq!(*orig as i32, *dec, "FLAC lossless must match bit-for-bit!");
        }
    }

    #[test]
    fn voice_denoiser_attenuates_background_hiss_while_preserving_speech_peaks() {
        let sr = 16_000u32;
        let mut samples = Vec::with_capacity(sr as usize);
        // 0.5s stationary noise + 0.5s strong voice tone + noise
        for i in 0..sr {
            let noise = (((i.wrapping_mul(1103515245).wrapping_add(12345)) >> 16) as i16) % 400;
            let voice = if i >= sr / 2 {
                ((2.0 * std::f32::consts::PI * 300.0 * (i as f32) / (sr as f32)).sin() * 9000.0) as i16
            } else {
                0
            };
            samples.push(voice.saturating_add(noise));
        }
        let cleaned = denoise_voice_pcm_i16(&samples, sr);
        assert_eq!(cleaned.len(), samples.len());
        let noise_before: f64 = samples[1600..6400].iter().map(|&s| (s as f64).abs()).sum::<f64>() / 4800.0;
        let noise_after: f64 = cleaned[1600..6400].iter().map(|&s| (s as f64).abs()).sum::<f64>() / 4800.0;
        let speech_after: f64 = cleaned[9600..14400].iter().map(|&s| (s as f64).abs()).sum::<f64>() / 4800.0;
        assert!(noise_after < noise_before * 0.55, "Noise must drop noticeably ({noise_before} -> {noise_after})");
        assert!(speech_after > 3500.0, "Speech signal must remain clear ({speech_after})");
    }
}
