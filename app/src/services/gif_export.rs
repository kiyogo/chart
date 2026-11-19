//! Animated GIF export for ChatW Studio, using the `gif` crate (image-rs).
//! Every frame gets its own NeuQuant palette; fully transparent pixels stay
//! transparent so stickers keep their shape.

use gif::{DisposalMethod, Encoder, Frame, Repeat};

/// `frames`: RGBA bytes of each frame (`width * height * 4` each).
/// `fps`: 1 – 30; `repeat`: 0 = forever.
pub fn encode(width: u16, height: u16, frames: &mut [Vec<u8>], fps: u32, repeat: u16) -> Result<Vec<u8>, String> {
    if frames.is_empty() {
        return Err("no_frames".into());
    }
    let expected = width as usize * height as usize * 4;
    let delay = (100 / fps.clamp(1, 30)).max(2) as u16;
    let mut out = Vec::new();
    {
        let mut enc = Encoder::new(&mut out, width, height, &[]).map_err(|e| e.to_string())?;
        enc.set_repeat(if repeat == 0 { Repeat::Infinite } else { Repeat::Finite(repeat) }).map_err(|e| e.to_string())?;
        for rgba in frames.iter_mut() {
            if rgba.len() != expected {
                return Err("bad_frame_size".into());
            }
            // Snap half-transparent pixels to opaque / clear (GIF has 1-bit alpha).
            for px in rgba.chunks_exact_mut(4) {
                px[3] = if px[3] >= 128 { 255 } else { 0 };
            }
            let mut frame = Frame::from_rgba_speed(width, height, rgba, 10);
            frame.delay = delay;
            frame.dispose = DisposalMethod::Background;
            enc.write_frame(&frame).map_err(|e| e.to_string())?;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_transparent_two_frame_gif() {
        let (w, h) = (4u16, 4u16);
        let mut a = vec![0u8; 64];
        let mut b = vec![0u8; 64];
        for px in a.chunks_exact_mut(4).take(8) {
            px.copy_from_slice(&[255, 0, 0, 255]);
        }
        for px in b.chunks_exact_mut(4).skip(8) {
            px.copy_from_slice(&[0, 0, 255, 200]);
        }
        let gif = encode(w, h, &mut [a, b], 10, 0).unwrap();
        assert!(gif.starts_with(b"GIF89a"));
        // Decode it back: two frames, 4x4, with a transparent index.
        let mut opts = gif::DecodeOptions::new();
        opts.set_color_output(gif::ColorOutput::RGBA);
        let mut dec = opts.read_info(&gif[..]).unwrap();
        let mut n = 0;
        while let Some(f) = dec.read_next_frame().unwrap() {
            assert_eq!((f.width, f.height), (4, 4));
            assert_eq!(f.delay, 10);
            n += 1;
        }
        assert_eq!(n, 2);
        assert!(encode(w, h, &mut [vec![0u8; 10]], 10, 0).is_err());
    }
}
