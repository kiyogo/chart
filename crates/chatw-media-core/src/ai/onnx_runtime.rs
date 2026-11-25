//! Real On-Device ONNX Neural Network Inference Runtime powered by `tract-onnx` (Section X).
//!
//! - Verifies SPDX commercial license AND pinned SHA-256 digest BEFORE loading any `.onnx` model!
//! - Executes standard super-resolution / restoration ONNX graphs (`[1, 3, H, W]` FP32 NCHW -> `[1, 3, s*H, s*W]`)
//!   with tiled overlapping Hann-feathered blending and atomic cancellation!
//! - Pure Rust runtime — zero external C++ shared library deployment issues!

#[cfg(feature = "onnx-runtime")]
mod impl_onnx {
    use crate::{
        MediaError, MediaResult,
        ai::{
            registry::{ModelDescriptor, verify_model_checksum, verify_model_license},
            tiling::run_tiled_rgba,
        },
    };
    use std::{io::Cursor, sync::Arc, sync::atomic::AtomicBool};
    use tract_onnx::prelude::*;

    pub struct LoadedOnnxRestorationModel {
        pub descriptor: ModelDescriptor,
        plan: Arc<TypedRunnableModel>,
    }

    impl LoadedOnnxRestorationModel {
        /// Verify license + SHA-256 checksum and load an `.onnx` restoration model for tiled `[1, 3, tile_h, tile_w]` inference.
        pub fn from_verified_bytes(
            descriptor: ModelDescriptor,
            onnx_bytes: &[u8],
            tile_h: usize,
            tile_w: usize,
        ) -> MediaResult<Self> {
            verify_model_license(&descriptor.model_id, &descriptor.spdx_license)?;
            verify_model_checksum(onnx_bytes, &descriptor.sha256_hex)?;

            let mut cursor = Cursor::new(onnx_bytes);
            let model = tract_onnx::onnx()
                .model_for_read(&mut cursor)
                .map_err(|e| MediaError::DecodeFailed(format!("onnx parse: {e}")))?
                .with_input_fact(0, InferenceFact::dt_shape(f32::datum_type(), tvec!(1, 3, tile_h, tile_w)))
                .map_err(|e| MediaError::DecodeFailed(format!("onnx input fact: {e}")))?
                .into_optimized()
                .map_err(|e| MediaError::DecodeFailed(format!("onnx optimize: {e}")))?
                .into_runnable()
                .map_err(|e| MediaError::DecodeFailed(format!("onnx runnable: {e}")))?;

            Ok(Self { descriptor, plan: model })
        }

        /// Run real neural network inference on a single RGBA8 tile (`tw x th`) with optional padding to fixed graph dimensions.
        pub fn infer_tile_rgba(
            &self,
            tile_rgba: &[u8],
            tw: u32,
            th: u32,
            graph_w: usize,
            graph_h: usize,
        ) -> MediaResult<Vec<u8>> {
            let w = tw as usize;
            let h = th as usize;
            let scale = self.descriptor.scale_factor.max(1) as usize;
            let req_bytes = w.checked_mul(h).and_then(|p| p.checked_mul(4));
            if w == 0 || h == 0 || graph_w == 0 || graph_h == 0 || req_bytes.is_none_or(|n| tile_rgba.len() < n) {
                return Err(MediaError::EmptyInput);
            }

            // Pack RGB into `[1, 3, graph_h, graph_w]` FP32 NCHW tensor `[0.0, 1.0]` using edge replicate padding
            let mut input_data = vec![0.0f32; 3 * graph_h * graph_w];
            for y in 0..graph_h {
                let sy = y.min(h - 1);
                for x in 0..graph_w {
                    let sx = x.min(w - 1);
                    let src_i = (sy * w + sx) * 4;
                    let hw = graph_h * graph_w;
                    input_data[y * graph_w + x] = (tile_rgba[src_i] as f32) / 255.0;
                    input_data[hw + y * graph_w + x] = (tile_rgba[src_i + 1] as f32) / 255.0;
                    input_data[2 * hw + y * graph_w + x] = (tile_rgba[src_i + 2] as f32) / 255.0;
                }
            }

            let tensor = tract_ndarray::Array4::from_shape_vec((1, 3, graph_h, graph_w), input_data)
                .map_err(|e| MediaError::DecodeFailed(format!("tensor shape: {e}")))?
                .into_tensor();

            let outputs = self
                .plan
                .run(tvec!(tensor.into()))
                .map_err(|e| MediaError::DecodeFailed(format!("onnx run: {e}")))?;

            let out_view = outputs[0]
                .to_plain_array_view::<f32>()
                .map_err(|e| MediaError::DecodeFailed(format!("onnx output view: {e}")))?;
            let shape = out_view.shape();
            if shape.len() != 4 || shape[1] < 3 {
                return Err(MediaError::DecodeFailed(format!("unexpected onnx output shape: {shape:?}")));
            }
            let oh_full = shape[2];
            let ow_full = shape[3];
            let out_w = (w * scale).min(ow_full);
            let out_h = (h * scale).min(oh_full);

            let mut out_rgba = vec![255u8; out_w * out_h * 4];
            for y in 0..out_h {
                let src_y = (y / scale).min(h - 1);
                for x in 0..out_w {
                    let src_x = (x / scale).min(w - 1);
                    let dst_i = (y * out_w + x) * 4;
                    out_rgba[dst_i] = (out_view[[0, 0, y, x]].clamp(0.0, 1.0) * 255.0).round() as u8;
                    out_rgba[dst_i + 1] = (out_view[[0, 1, y, x]].clamp(0.0, 1.0) * 255.0).round() as u8;
                    out_rgba[dst_i + 2] = (out_view[[0, 2, y, x]].clamp(0.0, 1.0) * 255.0).round() as u8;
                    out_rgba[dst_i + 3] = tile_rgba[(src_y * w + src_x) * 4 + 3];
                }
            }
            Ok(out_rgba)
        }

        /// Run full-image overlapping tiled ONNX inference with seamless Hann feathering and cancellation.
        pub fn infer_image_tiled(
            &self,
            rgba: &[u8],
            width: u32,
            height: u32,
            tile_size: u32,
            overlap: u32,
            cancel: Option<&AtomicBool>,
        ) -> MediaResult<(Vec<u8>, u32, u32)> {
            let g = tile_size as usize;
            let scale = self.descriptor.scale_factor.max(1) as u32;
            run_tiled_rgba(rgba, width, height, tile_size, overlap, scale, cancel, |tile, tw, th| {
                self.infer_tile_rgba(tile, tw, th, g, g)
            })
        }
    }
}

#[cfg(feature = "onnx-runtime")]
pub use impl_onnx::LoadedOnnxRestorationModel;

#[cfg(all(test, feature = "onnx-runtime"))]
mod tests {
    use super::*;
    use crate::MediaError;
    use crate::ai::hardware::{AccelerationBackend, DeviceTier};
    use crate::ai::registry::{ModelDescriptor, ModelDomain, ModelQuantization};
    use sha2::{Digest, Sha256};

    /// Build a minimal, 100% standards-compliant ONNX protobuf binary representing a real
    /// `Conv(3->3, 3x3, pad=1)` + `Add` residual neural restoration graph (opset 13) so our unit test
    /// verifies genuine `tract-onnx` graph parsing, SHA-256 verification, tensor execution & tiled blending!
    fn build_real_residual_conv_onnx_bytes() -> Vec<u8> {
        use prost::Message;
        use tract_onnx::pb::{
            AttributeProto, GraphProto, ModelProto, NodeProto, OperatorSetIdProto, TensorProto,
            TensorShapeProto, TypeProto, ValueInfoProto, attribute_proto::AttributeType,
            tensor_proto::DataType, tensor_shape_proto::Dimension,
            tensor_shape_proto::dimension::Value as DimValue, type_proto::Tensor as TypeTensor,
            type_proto::Value as TypeValue,
        };

        let make_vi = |name: &str| ValueInfoProto {
            name: name.to_string(),
            r#type: Some(TypeProto {
                value: Some(TypeValue::TensorType(TypeTensor {
                    elem_type: DataType::Float as i32,
                    shape: Some(TensorShapeProto {
                        dim: [1, 3, 32, 32]
                            .into_iter()
                            .map(|d| Dimension {
                                value: Some(DimValue::DimValue(d)),
                                denotation: String::new(),
                            })
                            .collect(),
                    }),
                })),
                denotation: String::new(),
            }),
            doc_string: String::new(),
        };

        let mut weights = vec![0.0f32; 3 * 3 * 3 * 3];
        for c in 0..3 {
            let base = (c * 3 + c) * 9;
            weights[base + 1] = -0.04;
            weights[base + 3] = -0.04;
            weights[base + 4] = 0.16;
            weights[base + 5] = -0.04;
            weights[base + 7] = -0.04;
        }
        let mut raw_w = Vec::with_capacity(weights.len() * 4);
        for w in &weights {
            raw_w.extend_from_slice(&w.to_le_bytes());
        }

        let tensor_w = TensorProto {
            dims: vec![3, 3, 3, 3],
            data_type: DataType::Float as i32,
            name: "W".to_string(),
            raw_data: raw_w,
            ..Default::default()
        };

        let node_conv = NodeProto {
            input: vec!["X".into(), "W".into()],
            output: vec!["R".into()],
            name: "conv_residual".into(),
            op_type: "Conv".into(),
            attribute: vec![AttributeProto {
                name: "pads".into(),
                ints: vec![1, 1, 1, 1],
                r#type: AttributeType::Ints as i32,
                ..Default::default()
            }],
            ..Default::default()
        };

        let node_add = NodeProto {
            input: vec!["X".into(), "R".into()],
            output: vec!["Y".into()],
            name: "residual_skip".into(),
            op_type: "Add".into(),
            ..Default::default()
        };

        let model = ModelProto {
            ir_version: 7,
            opset_import: vec![OperatorSetIdProto {
                domain: String::new(),
                version: 13,
            }],
            graph: Some(GraphProto {
                node: vec![node_conv, node_add],
                name: "chatw_residual_conv_test".into(),
                initializer: vec![tensor_w],
                input: vec![make_vi("X")],
                output: vec![make_vi("Y")],
                ..Default::default()
            }),
            ..Default::default()
        };

        model.encode_to_vec()
    }

    #[test]
    fn tract_onnx_loads_verifies_sha256_and_executes_real_tiled_residual_conv_graph() {
        let onnx_bytes = build_real_residual_conv_onnx_bytes();
        let sha_hex = hex::encode(Sha256::digest(&onnx_bytes));

        let desc = ModelDescriptor {
            model_id: "test-residual-conv-onnx".into(),
            version: "1.0.0".into(),
            sha256_hex: sha_hex,
            spdx_license: "Apache-2.0".into(),
            commercial_allowed: true,
            source_repo: "https://github.com/kiyogo/chart".into(),
            domain: ModelDomain::ArtifactDeblock,
            scale_factor: 1,
            model_size_bytes: onnx_bytes.len() as u64,
            min_ram_mb: 128,
            recommended_ram_mb: 256,
            min_device_tier: DeviceTier::Constrained,
            supported_backends: vec![AccelerationBackend::OnnxCpuSimd],
            quantization: ModelQuantization::Fp32,
            built_in: false,
            notes: "Real ONNX residual Conv2D test model".into(),
        };

        // 1. Checksum tampering MUST be rejected!
        let mut bad_desc = desc.clone();
        bad_desc.sha256_hex = "0".repeat(64);
        assert!(matches!(
            LoadedOnnxRestorationModel::from_verified_bytes(bad_desc, &onnx_bytes, 32, 32),
            Err(MediaError::IntegrityMismatch { .. })
        ));

        // 2. Non-commercial license MUST be rejected!
        let mut nc_desc = desc.clone();
        nc_desc.spdx_license = "NTU-SLab-1.0-NonCommercial".into();
        assert!(matches!(
            LoadedOnnxRestorationModel::from_verified_bytes(nc_desc, &onnx_bytes, 32, 32),
            Err(MediaError::LicenseViolation { .. })
        ));

        // 3. Valid verified ONNX graph executes tiled neural inference end-to-end!
        let loaded = LoadedOnnxRestorationModel::from_verified_bytes(desc, &onnx_bytes, 32, 32)
            .expect("tract-onnx load");
        let (w, h) = (48u32, 48u32);
        let mut img = vec![120u8; (w * h * 4) as usize];
        for y in 16..32 {
            for x in 16..32 {
                let i = ((y * w + x) * 4) as usize;
                img[i] = 220;
                img[i + 1] = 180;
                img[i + 2] = 90;
                img[i + 3] = 255;
            }
        }
        let (out, ow, oh) = loaded.infer_image_tiled(&img, w, h, 32, 8, None).expect("onnx tiled infer");
        assert_eq!((ow, oh), (w, h));
        assert_eq!(out.len(), img.len());
        assert_ne!(out, img, "Residual Conv2D ONNX graph must actively enhance edge transitions");
    }
}
