//! Pure-Rust SPAN (Swift Parameter-free Attention Network, CVPR NTIRE 2024) 4× Super-Resolution Engine.
//!
//! Reference open-source architectures & weights:
//! - Architecture: `hongyuanyu/SPAN` & `chaiNNer-org/spandrel` (`SPAN` 48ch, 6 SPAB blocks, 4× PixelShuffle)
//! - Pretrained photorealistic weights: `Kim2091/ClearRealityV1` (`4x-ClearRealityV1`, Apache-2.0)
//!
//! Inference graph (all 20 `Conv3XC` re-parameterized training blocks pre-fused into single 3×3 convs):
//! 1. Input normalization (when `use_norm == true`): `x = (rgb - (0.4488, 0.4371, 0.4040)) * 255.0`
//! 2. `feat0 = conv_1(x)` (`3 -> 48`, 3×3, pad=1)
//! 3. 6 `SPAB` blocks (`b = 0..6`):
//!    - `t1 = SiLU(c1_r(cur))`  (save `t1` on `b == 5` as `b6_c1_act`, matching PyTorch in-place `SiLU`)
//!    - `t2 = SiLU(c2_r(t1))`
//!    - `t3 = c3_r(t2)`
//!    - `cur = (t3 + cur) * (sigmoid(t3) - 0.5)`  (save `cur` on `b == 0` as `b1_out`)
//! 4. `b6_conv2 = conv_2(b6_out)` (`48 -> 48`, 3×3, pad=1)
//! 5. `cat = conv_cat(concat([feat0, b6_conv2, b1_out, b6_c1_act]))` (`192 -> 48`, 1×1)
//! 6. `up = upsampler(cat)` (`48 -> 48`, 3×3, pad=1) -> `PixelShuffle(4×)` -> `clamp(rgb, 0.0, 1.0)`

use crate::{MediaError, MediaResult, ai::realesrgan::f16_to_f32};
use std::sync::atomic::{AtomicBool, Ordering};

pub const CWSP_MAGIC: &[u8; 4] = b"CWSP";
pub const SPAN_RGB_MEAN: [f32; 3] = [0.4488, 0.4371, 0.4040];
pub const SPAN_IMG_RANGE: f32 = 255.0;
pub const SPAN_FEAT_CH: usize = 48;
pub const SPAN_NUM_BLOCKS: usize = 6;
pub const SPAN_SCALE: usize = 4;

#[derive(Clone, Debug)]
pub struct SpanGpuF16Layer {
    /// Cache-line-contiguous RGBA16F texture data as raw IEEE-754 half-float `u16`s.
    pub weights_u16: Vec<u16>,
    /// FP32 bias vector (`len == 48`).
    pub bias: Vec<f32>,
}

/// GPU-ready FP16 SPAN 4× model parsed directly from `CWSP1` bytes (852,596 bytes = 832.6 KB).
#[derive(Clone, Debug)]
pub struct SpanGpuF16Model {
    pub scale: u32,
    pub feat_ch: usize,
    pub use_norm: bool,
    /// Packed `48 × 9 × 4` RGBA16F (`conv_1`: 3 -> 48, 3×3).
    pub conv_1: SpanGpuF16Layer,
    /// 6 SPAB blocks × 3 convs (`c1_r`, `c2_r`, `c3_r`), each packed `48 × 108 × 4` RGBA16F.
    pub blocks: Vec<[SpanGpuF16Layer; 3]>,
    /// Packed `48 × 108 × 4` RGBA16F (`conv_2`: 48 -> 48, 3×3).
    pub conv_2: SpanGpuF16Layer,
    /// Packed `48 × 48 × 4` RGBA16F (`conv_cat`: 192 -> 48, 1×1).
    pub conv_cat: SpanGpuF16Layer,
    /// Packed `48 × 108 × 4` RGBA16F (`upsampler.0`: 48 -> 48, 3×3).
    pub upsampler: SpanGpuF16Layer,
}

/// Pack `conv_1` `[48, 3, 9]` FP16 weights into `[width=48, height=9]` RGBA16F (`texel(oc, k) = (w[oc,0,k], w[oc,1,k], w[oc,2,k], 0)`).
pub fn pack_span_conv_in_weights(w_u16: &[u16]) -> Vec<u16> {
    let mut out = vec![0u16; SPAN_FEAT_CH * 9 * 4];
    for oc in 0..SPAN_FEAT_CH {
        for k in 0..9 {
            let dst = (k * SPAN_FEAT_CH + oc) * 4;
            for ic in 0..3 {
                out[dst + ic] = w_u16[(oc * 3 + ic) * 9 + k];
            }
        }
    }
    out
}

/// Pack `48 -> 48` 3×3 FP16 weights into `[width=48, height=108]` RGBA16F (`row = k * 12 + ig`, `texel(oc, row) = 4 in channels`).
pub fn pack_span_conv48_weights(w_u16: &[u16]) -> Vec<u16> {
    let mut out = vec![0u16; SPAN_FEAT_CH * 108 * 4];
    for oc in 0..SPAN_FEAT_CH {
        for k in 0..9 {
            for ig in 0..12 {
                let row = k * 12 + ig;
                let dst = (row * SPAN_FEAT_CH + oc) * 4;
                for c in 0..4 {
                    out[dst + c] = w_u16[(oc * SPAN_FEAT_CH + ig * 4 + c) * 9 + k];
                }
            }
        }
    }
    out
}

/// Pack `conv_cat` `192 -> 48` 1×1 FP16 weights into `[width=48, height=48]` RGBA16F (`row = src_idx * 12 + ig`).
pub fn pack_span_conv_cat_weights(w_u16: &[u16]) -> Vec<u16> {
    let mut out = vec![0u16; SPAN_FEAT_CH * 48 * 4];
    for oc in 0..SPAN_FEAT_CH {
        for row in 0..48 {
            let dst = (row * SPAN_FEAT_CH + oc) * 4;
            for c in 0..4 {
                out[dst + c] = w_u16[oc * 192 + row * 4 + c];
            }
        }
    }
    out
}

struct CwspCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> CwspCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn read_u32(&mut self) -> MediaResult<u32> {
        if self.pos + 4 > self.bytes.len() {
            return Err(MediaError::InvalidParameter("CWSP1 truncated header".into()));
        }
        let v = u32::from_le_bytes(self.bytes[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(v)
    }

    fn read_u16_vec(&mut self, n: usize) -> MediaResult<Vec<u16>> {
        let byte_len = n * 2;
        if self.pos + byte_len > self.bytes.len() {
            return Err(MediaError::InvalidParameter(format!(
                "CWSP1 truncated weights at byte {} (need {})",
                self.pos, byte_len
            )));
        }
        let mut out = Vec::with_capacity(n);
        for chunk in self.bytes[self.pos..self.pos + byte_len].chunks_exact(2) {
            out.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        self.pos += byte_len;
        Ok(out)
    }

    fn read_f16_as_f32_vec(&mut self, n: usize) -> MediaResult<Vec<f32>> {
        let u16s = self.read_u16_vec(n)?;
        Ok(u16s.into_iter().map(f16_to_f32).collect())
    }
}

impl SpanGpuF16Model {
    pub fn from_cwsp_bytes(bytes: &[u8]) -> MediaResult<Self> {
        if bytes.len() < 20 || &bytes[0..4] != CWSP_MAGIC {
            return Err(MediaError::InvalidParameter(
                "Invalid CWSP1 SPAN model magic header".into(),
            ));
        }
        let mut cur = CwspCursor::new(bytes);
        cur.pos = 4;
        let version = cur.read_u32()?;
        let scale = cur.read_u32()?;
        let feat_ch = cur.read_u32()? as usize;
        let flags = cur.read_u32()?;
        if version != 1 || scale != SPAN_SCALE as u32 || feat_ch != SPAN_FEAT_CH {
            return Err(MediaError::InvalidParameter(format!(
                "Unsupported CWSP header: version={version}, scale={scale}, feat_ch={feat_ch}"
            )));
        }
        let use_norm = (flags & 1) != 0;

        let conv1_w = cur.read_u16_vec(SPAN_FEAT_CH * 3 * 9)?;
        let conv1_b = cur.read_f16_as_f32_vec(SPAN_FEAT_CH)?;
        let conv_1 = SpanGpuF16Layer {
            weights_u16: pack_span_conv_in_weights(&conv1_w),
            bias: conv1_b,
        };

        let mut blocks = Vec::with_capacity(SPAN_NUM_BLOCKS);
        for _ in 0..SPAN_NUM_BLOCKS {
            let mut blk = Vec::with_capacity(3);
            for _ in 0..3 {
                let w = cur.read_u16_vec(SPAN_FEAT_CH * SPAN_FEAT_CH * 9)?;
                let b = cur.read_f16_as_f32_vec(SPAN_FEAT_CH)?;
                blk.push(SpanGpuF16Layer {
                    weights_u16: pack_span_conv48_weights(&w),
                    bias: b,
                });
            }
            let [c1, c2, c3]: [SpanGpuF16Layer; 3] = blk.try_into().unwrap();
            blocks.push([c1, c2, c3]);
        }

        let conv2_w = cur.read_u16_vec(SPAN_FEAT_CH * SPAN_FEAT_CH * 9)?;
        let conv2_b = cur.read_f16_as_f32_vec(SPAN_FEAT_CH)?;
        let conv_2 = SpanGpuF16Layer {
            weights_u16: pack_span_conv48_weights(&conv2_w),
            bias: conv2_b,
        };

        let cat_w = cur.read_u16_vec(SPAN_FEAT_CH * (SPAN_FEAT_CH * 4))?;
        let cat_b = cur.read_f16_as_f32_vec(SPAN_FEAT_CH)?;
        let conv_cat = SpanGpuF16Layer {
            weights_u16: pack_span_conv_cat_weights(&cat_w),
            bias: cat_b,
        };

        let up_w = cur.read_u16_vec(SPAN_FEAT_CH * SPAN_FEAT_CH * 9)?;
        let up_b = cur.read_f16_as_f32_vec(SPAN_FEAT_CH)?;
        let upsampler = SpanGpuF16Layer {
            weights_u16: pack_span_conv48_weights(&up_w),
            bias: up_b,
        };

        Ok(Self {
            scale,
            feat_ch,
            use_norm,
            conv_1,
            blocks,
            conv_2,
            conv_cat,
            upsampler,
        })
    }
}

/// CPU FP32 SPAN 4× model (fallback when WebGL2 `EXT_color_buffer_float` is unavailable).
#[derive(Clone, Debug)]
pub struct SpanCpuLayer {
    pub weights: Vec<f32>,
    pub bias: Vec<f32>,
}

#[derive(Clone, Debug)]
pub struct SpanModel {
    pub scale: usize,
    pub feat_ch: usize,
    pub use_norm: bool,
    pub conv_1: SpanCpuLayer,
    pub blocks: Vec<[SpanCpuLayer; 3]>,
    pub conv_2: SpanCpuLayer,
    pub conv_cat: SpanCpuLayer,
    pub upsampler: SpanCpuLayer,
}

impl SpanModel {
    pub fn from_cwsp_bytes(bytes: &[u8]) -> MediaResult<Self> {
        if bytes.len() < 20 || &bytes[0..4] != CWSP_MAGIC {
            return Err(MediaError::InvalidParameter("Invalid CWSP1 magic".into()));
        }
        let mut cur = CwspCursor::new(bytes);
        cur.pos = 4;
        let version = cur.read_u32()?;
        let scale = cur.read_u32()? as usize;
        let feat_ch = cur.read_u32()? as usize;
        let flags = cur.read_u32()?;
        if version != 1 || scale != SPAN_SCALE || feat_ch != SPAN_FEAT_CH {
            return Err(MediaError::InvalidParameter("Unsupported CWSP1 config".into()));
        }
        let use_norm = (flags & 1) != 0;
        let read_layer = |c: &mut CwspCursor<'_>, w_len: usize, b_len: usize| -> MediaResult<SpanCpuLayer> {
            Ok(SpanCpuLayer {
                weights: c.read_f16_as_f32_vec(w_len)?,
                bias: c.read_f16_as_f32_vec(b_len)?,
            })
        };
        let conv_1 = read_layer(&mut cur, SPAN_FEAT_CH * 3 * 9, SPAN_FEAT_CH)?;
        let mut blocks = Vec::with_capacity(SPAN_NUM_BLOCKS);
        for _ in 0..SPAN_NUM_BLOCKS {
            let c1 = read_layer(&mut cur, SPAN_FEAT_CH * SPAN_FEAT_CH * 9, SPAN_FEAT_CH)?;
            let c2 = read_layer(&mut cur, SPAN_FEAT_CH * SPAN_FEAT_CH * 9, SPAN_FEAT_CH)?;
            let c3 = read_layer(&mut cur, SPAN_FEAT_CH * SPAN_FEAT_CH * 9, SPAN_FEAT_CH)?;
            blocks.push([c1, c2, c3]);
        }
        let conv_2 = read_layer(&mut cur, SPAN_FEAT_CH * SPAN_FEAT_CH * 9, SPAN_FEAT_CH)?;
        let conv_cat = read_layer(&mut cur, SPAN_FEAT_CH * (SPAN_FEAT_CH * 4), SPAN_FEAT_CH)?;
        let upsampler = read_layer(&mut cur, SPAN_FEAT_CH * SPAN_FEAT_CH * 9, SPAN_FEAT_CH)?;
        Ok(Self {
            scale,
            feat_ch,
            use_norm,
            conv_1,
            blocks,
            conv_2,
            conv_cat,
            upsampler,
        })
    }
}

#[inline]
fn silu(x: f32) -> f32 {
    x / (1.0 + (-x.clamp(-20.0, 20.0)).exp())
}

#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x.clamp(-20.0, 20.0)).exp())
}

fn conv3x3_chw(
    src: &[f32],
    in_ch: usize,
    out_ch: usize,
    w: usize,
    h: usize,
    layer: &SpanCpuLayer,
    apply_silu: bool,
) -> Vec<f32> {
    let hw = w * h;
    let mut dst = vec![0.0f32; out_ch * hw];
    for oc in 0..out_ch {
        let bias = layer.bias[oc];
        let dst_plane = &mut dst[oc * hw..(oc + 1) * hw];
        dst_plane.fill(bias);
        for ic in 0..in_ch {
            let src_plane = &src[ic * hw..(ic + 1) * hw];
            let kbase = (oc * in_ch + ic) * 9;
            for ky in 0..3usize {
                for kx in 0..3usize {
                    let wt = layer.weights[kbase + ky * 3 + kx];
                    if wt == 0.0 {
                        continue;
                    }
                    let y_start = if ky == 0 { 1 } else { 0 };
                    let y_end = if ky == 2 { h.saturating_sub(1) } else { h };
                    let x_start = if kx == 0 { 1 } else { 0 };
                    let x_end = if kx == 2 { w.saturating_sub(1) } else { w };
                    for y in y_start..y_end {
                        let sy = y + ky - 1;
                        let dst_row = &mut dst_plane[y * w..y * w + w];
                        let src_row = &src_plane[sy * w..sy * w + w];
                        for x in x_start..x_end {
                            dst_row[x] += src_row[x + kx - 1] * wt;
                        }
                    }
                }
            }
        }
        if apply_silu {
            for v in dst_plane.iter_mut() {
                *v = silu(*v);
            }
        }
    }
    dst
}

fn infer_span_tile_cpu(model: &SpanModel, tile_rgba: &[u8], w: usize, h: usize) -> Vec<u8> {
    let hw = w * h;
    let mut x = vec![0.0f32; 3 * hw];
    for i in 0..hw {
        let r = tile_rgba[i * 4] as f32 / 255.0;
        let g = tile_rgba[i * 4 + 1] as f32 / 255.0;
        let b = tile_rgba[i * 4 + 2] as f32 / 255.0;
        if model.use_norm {
            x[i] = (r - SPAN_RGB_MEAN[0]) * SPAN_IMG_RANGE;
            x[hw + i] = (g - SPAN_RGB_MEAN[1]) * SPAN_IMG_RANGE;
            x[2 * hw + i] = (b - SPAN_RGB_MEAN[2]) * SPAN_IMG_RANGE;
        } else {
            x[i] = r;
            x[hw + i] = g;
            x[2 * hw + i] = b;
        }
    }

    let feat0 = conv3x3_chw(&x, 3, SPAN_FEAT_CH, w, h, &model.conv_1, false);
    let mut cur = feat0.clone();
    let mut b1_out = Vec::new();
    let mut b6_c1_act = Vec::new();

    for (bi, blk) in model.blocks.iter().enumerate() {
        let t1 = conv3x3_chw(&cur, SPAN_FEAT_CH, SPAN_FEAT_CH, w, h, &blk[0], true);
        if bi == SPAN_NUM_BLOCKS - 1 {
            b6_c1_act = t1.clone();
        }
        let t2 = conv3x3_chw(&t1, SPAN_FEAT_CH, SPAN_FEAT_CH, w, h, &blk[1], true);
        let mut t3 = conv3x3_chw(&t2, SPAN_FEAT_CH, SPAN_FEAT_CH, w, h, &blk[2], false);
        for (dst, &sk) in t3.iter_mut().zip(cur.iter()) {
            let att = sigmoid(*dst) - 0.5;
            *dst = (*dst + sk) * att;
        }
        if bi == 0 {
            b1_out = t3.clone();
        }
        cur = t3;
    }

    let b6_conv2 = conv3x3_chw(&cur, SPAN_FEAT_CH, SPAN_FEAT_CH, w, h, &model.conv_2, false);

    // conv_cat([feat0, b6_conv2, b1_out, b6_c1_act]) -> 48ch
    let srcs: [&[f32]; 4] = [&feat0, &b6_conv2, &b1_out, &b6_c1_act];
    let mut cat = vec![0.0f32; SPAN_FEAT_CH * hw];
    for oc in 0..SPAN_FEAT_CH {
        let dst_plane = &mut cat[oc * hw..(oc + 1) * hw];
        dst_plane.fill(model.conv_cat.bias[oc]);
        for (si, src) in srcs.iter().enumerate() {
            for ic in 0..SPAN_FEAT_CH {
                let wt = model.conv_cat.weights[oc * 192 + si * SPAN_FEAT_CH + ic];
                if wt == 0.0 {
                    continue;
                }
                let src_plane = &src[ic * hw..(ic + 1) * hw];
                for i in 0..hw {
                    dst_plane[i] += src_plane[i] * wt;
                }
            }
        }
    }

    let up = conv3x3_chw(&cat, SPAN_FEAT_CH, SPAN_FEAT_CH, w, h, &model.upsampler, false);

    // PixelShuffle(4) -> RGBA8
    let ow = w * SPAN_SCALE;
    let oh = h * SPAN_SCALE;
    let mut out = vec![255u8; ow * oh * 4];
    for sy in 0..h {
        for sx in 0..w {
            let si = sy * w + sx;
            let alpha = tile_rgba[si * 4 + 3];
            for dy in 0..SPAN_SCALE {
                let oy = sy * SPAN_SCALE + dy;
                for dx in 0..SPAN_SCALE {
                    let ox = sx * SPAN_SCALE + dx;
                    let sub = dy * SPAN_SCALE + dx;
                    let r = (up[sub * hw + si].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    let g = (up[(16 + sub) * hw + si].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    let b = (up[(32 + sub) * hw + si].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                    let oi = (oy * ow + ox) * 4;
                    out[oi] = r;
                    out[oi + 1] = g;
                    out[oi + 2] = b;
                    out[oi + 3] = alpha;
                }
            }
        }
    }
    out
}

/// Seamless halo-tiled Pure-Rust CPU SPAN 4× execution (fallback when WebGL2 FP16 is unavailable).
pub fn run_span_tiled<F>(
    model: &SpanModel,
    rgba: &[u8],
    width: u32,
    height: u32,
    tile_size: usize,
    cancel: Option<&AtomicBool>,
    mut on_progress: F,
) -> MediaResult<(Vec<u8>, u32, u32)>
where
    F: FnMut(u32),
{
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return Err(MediaError::InvalidParameter("Invalid RGBA dimensions for SPAN".into()));
    }
    let scale = model.scale;
    let ow = w * scale;
    let oh = h * scale;
    let mut out = vec![0u8; ow * oh * 4];
    let tile = tile_size.clamp(64, 384);
    let pad = 12usize;
    let cols = w.div_ceil(tile);
    let rows = h.div_ceil(tile);
    let total = (cols * rows).max(1);
    let mut done = 0usize;

    for ty in 0..rows {
        let y0 = ty * tile;
        let y1 = (y0 + tile).min(h);
        let py0 = y0.saturating_sub(pad);
        let py1 = (y1 + pad).min(h);
        let th = py1 - py0;
        for tx in 0..cols {
            if let Some(flag) = cancel {
                if flag.load(Ordering::Relaxed) {
                    return Err(MediaError::Cancelled);
                }
            }
            let x0 = tx * tile;
            let x1 = (x0 + tile).min(w);
            let px0 = x0.saturating_sub(pad);
            let px1 = (x1 + pad).min(w);
            let tw = px1 - px0;

            let mut tile_in = vec![0u8; tw * th * 4];
            for row in 0..th {
                let s0 = ((py0 + row) * w + px0) * 4;
                let d0 = row * tw * 4;
                tile_in[d0..d0 + tw * 4].copy_from_slice(&rgba[s0..s0 + tw * 4]);
            }

            let tile_out = infer_span_tile_cpu(model, &tile_in, tw, th);
            let tow = tw * scale;
            let crop_x0 = (x0 - px0) * scale;
            let crop_y0 = (y0 - py0) * scale;
            let copy_w = (x1 - x0) * scale;
            let copy_h = (y1 - y0) * scale;

            for row in 0..copy_h {
                let src_off = ((crop_y0 + row) * tow + crop_x0) * 4;
                let dst_off = ((y0 * scale + row) * ow + x0 * scale) * 4;
                out[dst_off..dst_off + copy_w * 4]
                    .copy_from_slice(&tile_out[src_off..src_off + copy_w * 4]);
            }

            done += 1;
            on_progress(((done * 95) / total).min(95) as u32);
        }
    }
    on_progress(100);
    Ok((out, ow as u32, oh as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_synthetic_cwsp() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(CWSP_MAGIC);
        buf.extend_from_slice(&1u32.to_le_bytes()); // version
        buf.extend_from_slice(&4u32.to_le_bytes()); // scale
        buf.extend_from_slice(&48u32.to_le_bytes()); // feat_ch
        buf.extend_from_slice(&1u32.to_le_bytes()); // norm=true
        // Total params = 426,288 f16 values -> 852,576 bytes
        buf.resize(20 + 426_288 * 2, 0u8);
        buf
    }

    #[test]
    fn test_cwsp1_parse_and_pack_invariants() {
        let bytes = make_synthetic_cwsp();
        assert_eq!(bytes.len(), 852_596);
        let gpu = SpanGpuF16Model::from_cwsp_bytes(&bytes).expect("parse GPU CWSP1");
        assert_eq!(gpu.scale, 4);
        assert_eq!(gpu.feat_ch, 48);
        assert!(gpu.use_norm);
        assert_eq!(gpu.conv_1.weights_u16.len(), 48 * 9 * 4);
        assert_eq!(gpu.blocks.len(), 6);
        assert_eq!(gpu.blocks[0][0].weights_u16.len(), 48 * 108 * 4);
        assert_eq!(gpu.conv_cat.weights_u16.len(), 48 * 48 * 4);
        assert_eq!(gpu.upsampler.weights_u16.len(), 48 * 108 * 4);
    }

    #[test]
    fn test_span_cpu_forward_4x_shapes_and_finite() {
        let bytes = make_synthetic_cwsp();
        let model = SpanModel::from_cwsp_bytes(&bytes).expect("parse CPU CWSP1");
        let rgba = vec![128u8; 8 * 6 * 4];
        let (out, ow, oh) = run_span_tiled(&model, &rgba, 8, 6, 64, None, |_| {}).expect("run SPAN CPU");
        assert_eq!(ow, 32);
        assert_eq!(oh, 24);
        assert_eq!(out.len(), 32 * 24 * 4);
    }

    #[test]
    fn test_bundled_clearreality_v1_cwsp_valid() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../app/public/models/clearreality_v1_x4.cwsp");
        if path.exists() {
            let bytes = std::fs::read(&path).expect("read clearreality_v1_x4.cwsp");
            assert_eq!(bytes.len(), 852_596);
            let gpu = SpanGpuF16Model::from_cwsp_bytes(&bytes).expect("valid bundled CWSP1");
            assert!(gpu.use_norm);
            // Verify upsampler bias is centered near RGB mean (~0.40..0.47)
            let mean_b: f32 = gpu.upsampler.bias.iter().sum::<f32>() / 48.0;
            assert!(mean_b > 0.35 && mean_b < 0.55, "unexpected upsampler bias mean: {mean_b}");
        }
    }
}
