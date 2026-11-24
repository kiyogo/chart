//! 100% Pure-Rust GPU Engine for Real-ESRGAN / Upscayl (`SRVGGNetCompact`) on Browser WASM (`web-sys` WebGL2 MRT).
//!
//! Runs the EXACT pretrained BSD-3-Clause `SRVGGNetCompact` neural architecture (`realesr-animevideov3` 16×64ch
//! & `realesr-general-x4v3` / `upscayl-lite-4x` 32×64ch) directly on the user's GPU via WebGL2 `TEXTURE_2D_ARRAY`
//! + 4-attachment Multiple Render Targets (works out-of-the-box on 100% of browsers: Linux, Windows, macOS, Android, iOS —
//! ZERO experimental flags needed!), with automatic fallback to Pure-Rust CPU `chatw_media_core::ai::realesrgan::run_realesrgan_tiled`.

#![cfg(target_arch = "wasm32")]

use chatw_media_core::ai::realesrgan::{
    OriginalQualityProfile, RealEsrganModel, RealEsrganModelKind, calibrate_color_preserve_sr,
    evaluate_sr_quality_gate, run_realesrgan_tiled,
};
use js_sys::{Array, Float32Array, Object, Reflect};
use std::{cell::RefCell, collections::HashMap};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as Gl, WebGlFramebuffer, WebGlProgram, WebGlTexture,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

const GL_VS: &str = r#"#version 300 es
precision highp float;
const vec2 POS[3] = vec2[3](vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
void main() {
  gl_Position = vec4(POS[gl_VertexID], 0.0, 1.0);
}"#;

const GL_CONV_IN_FS: &str = r#"#version 300 es
precision highp float;
precision highp sampler2D;
uniform sampler2D uSrc;
uniform sampler2D uWeights;
uniform vec4 uBias[4];
uniform vec4 uPrelu[4];
uniform int uGroupBase;
uniform ivec2 uDims;
layout(location = 0) out vec4 o0;
layout(location = 1) out vec4 o1;
layout(location = 2) out vec4 o2;
layout(location = 3) out vec4 o3;

void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  vec4 a0 = uBias[0], a1 = uBias[1], a2 = uBias[2], a3 = uBias[3];
  int oc0 = uGroupBase * 4;
  for (int dy = -1; dy <= 1; dy++) {
    int sy = p.y + dy;
    if (sy < 0 || sy >= uDims.y) continue;
    for (int dx = -1; dx <= 1; dx++) {
      int sx = p.x + dx;
      if (sx < 0 || sx >= uDims.x) continue;
      int k = (dy + 1) * 3 + (dx + 1);
      vec3 rgb = texelFetch(uSrc, ivec2(sx, sy), 0).rgb;
      a0.x += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 0), 0).rgb);
      a0.y += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 1), 0).rgb);
      a0.z += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 2), 0).rgb);
      a0.w += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 3), 0).rgb);
      a1.x += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 4), 0).rgb);
      a1.y += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 5), 0).rgb);
      a1.z += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 6), 0).rgb);
      a1.w += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 7), 0).rgb);
      a2.x += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 8), 0).rgb);
      a2.y += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 9), 0).rgb);
      a2.z += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 10), 0).rgb);
      a2.w += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 11), 0).rgb);
      a3.x += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 12), 0).rgb);
      a3.y += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 13), 0).rgb);
      a3.z += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 14), 0).rgb);
      a3.w += dot(rgb, texelFetch(uWeights, ivec2(k, oc0 + 15), 0).rgb);
    }
  }
  o0 = mix(a0 * uPrelu[0], a0, greaterThanEqual(a0, vec4(0.0)));
  o1 = mix(a1 * uPrelu[1], a1, greaterThanEqual(a1, vec4(0.0)));
  o2 = mix(a2 * uPrelu[2], a2, greaterThanEqual(a2, vec4(0.0)));
  o3 = mix(a3 * uPrelu[3], a3, greaterThanEqual(a3, vec4(0.0)));
}"#;

const GL_CONV64_FS: &str = r#"#version 300 es
precision highp float;
precision highp sampler2D;
precision highp sampler2DArray;
uniform sampler2DArray uSrc;
uniform sampler2D uWeights;
uniform vec4 uBias[4];
uniform vec4 uPrelu[4];
uniform int uGroupBase;
uniform int uUsePrelu;
uniform ivec2 uDims;
layout(location = 0) out vec4 o0;
layout(location = 1) out vec4 o1;
layout(location = 2) out vec4 o2;
layout(location = 3) out vec4 o3;

void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  vec4 a0 = uBias[0], a1 = uBias[1], a2 = uBias[2], a3 = uBias[3];
  int oc0 = uGroupBase * 4;
  for (int dy = -1; dy <= 1; dy++) {
    int sy = p.y + dy;
    if (sy < 0 || sy >= uDims.y) continue;
    for (int dx = -1; dx <= 1; dx++) {
      int sx = p.x + dx;
      if (sx < 0 || sx >= uDims.x) continue;
      int kBase = ((dy + 1) * 3 + (dx + 1)) * 16;
      for (int ig = 0; ig < 16; ig++) {
        vec4 v = texelFetch(uSrc, ivec3(sx, sy, ig), 0);
        int wx = kBase + ig;
        a0.x += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 0), 0));
        a0.y += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 1), 0));
        a0.z += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 2), 0));
        a0.w += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 3), 0));
        a1.x += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 4), 0));
        a1.y += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 5), 0));
        a1.z += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 6), 0));
        a1.w += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 7), 0));
        a2.x += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 8), 0));
        a2.y += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 9), 0));
        a2.z += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 10), 0));
        a2.w += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 11), 0));
        a3.x += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 12), 0));
        a3.y += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 13), 0));
        a3.z += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 14), 0));
        a3.w += dot(v, texelFetch(uWeights, ivec2(wx, oc0 + 15), 0));
      }
    }
  }
  if (uUsePrelu != 0) {
    a0 = mix(a0 * uPrelu[0], a0, greaterThanEqual(a0, vec4(0.0)));
    a1 = mix(a1 * uPrelu[1], a1, greaterThanEqual(a1, vec4(0.0)));
    a2 = mix(a2 * uPrelu[2], a2, greaterThanEqual(a2, vec4(0.0)));
    a3 = mix(a3 * uPrelu[3], a3, greaterThanEqual(a3, vec4(0.0)));
  }
  o0 = a0; o1 = a1; o2 = a2; o3 = a3;
}"#;

const GL_SHUFFLE4X_FS: &str = r#"#version 300 es
precision highp float;
precision highp sampler2D;
precision highp sampler2DArray;
uniform sampler2D uOrig;
uniform sampler2DArray uHead;
out vec4 fragColor;

void main() {
  ivec2 op = ivec2(gl_FragCoord.xy);
  ivec2 sp = op >> 2;
  int sub = (op.y & 3) * 4 + (op.x & 3);
  int g = sub >> 2;
  int c = sub & 3;
  vec4 r4 = texelFetch(uHead, ivec3(sp, g), 0);
  vec4 g4 = texelFetch(uHead, ivec3(sp, 4 + g), 0);
  vec4 b4 = texelFetch(uHead, ivec3(sp, 8 + g), 0);
  float rr = (c == 0) ? r4.x : ((c == 1) ? r4.y : ((c == 2) ? r4.z : r4.w));
  float gg = (c == 0) ? g4.x : ((c == 1) ? g4.y : ((c == 2) ? g4.z : g4.w));
  float bb = (c == 0) ? b4.x : ((c == 1) ? b4.y : ((c == 2) ? b4.z : b4.w));
  vec4 orig = texelFetch(uOrig, sp, 0);
  fragColor = vec4(clamp(orig.rgb + vec3(rr, gg, bb), 0.0, 1.0), orig.a);
}"#;

struct ConvInProg {
    prog: WebGlProgram,
    u_src: Option<WebGlUniformLocation>,
    u_weights: Option<WebGlUniformLocation>,
    u_bias: Option<WebGlUniformLocation>,
    u_prelu: Option<WebGlUniformLocation>,
    u_group_base: Option<WebGlUniformLocation>,
    u_dims: Option<WebGlUniformLocation>,
}

struct Conv64Prog {
    prog: WebGlProgram,
    u_src: Option<WebGlUniformLocation>,
    u_weights: Option<WebGlUniformLocation>,
    u_bias: Option<WebGlUniformLocation>,
    u_prelu: Option<WebGlUniformLocation>,
    u_group_base: Option<WebGlUniformLocation>,
    u_use_prelu: Option<WebGlUniformLocation>,
    u_dims: Option<WebGlUniformLocation>,
}

struct Shuffle4xProg {
    prog: WebGlProgram,
    u_orig: Option<WebGlUniformLocation>,
    u_head: Option<WebGlUniformLocation>,
}

struct GlBodyLayer {
    tex: WebGlTexture,
    bias: Vec<f32>,
    prelu: Vec<f32>,
}

struct GlUploadedModel {
    first_tex: WebGlTexture,
    first_b: Vec<f32>,
    first_p: Vec<f32>,
    body: Vec<GlBodyLayer>,
    last_tex: WebGlTexture,
    last_b: Vec<f32>,
}

pub struct GpuProbeInfo {
    pub webgl2_available: bool,
    pub is_software: bool,
    pub renderer: String,
    pub backend: &'static str,
}

struct RealEsrganGlEngine {
    _canvas: HtmlCanvasElement,
    gl: Gl,
    feat_internal_fmt: u32,
    feat_tex_type: u32,
    is_software: bool,
    renderer: String,
    conv_in: ConvInProg,
    conv64: Conv64Prog,
    shuffle4x: Shuffle4xProg,
    vao: WebGlVertexArrayObject,
    fbo: WebGlFramebuffer,
    models: HashMap<String, GlUploadedModel>,
}

thread_local! {
    static GL_ENGINE: RefCell<Option<Result<RealEsrganGlEngine, String>>> = const { RefCell::new(None) };
}

fn compile_shader(gl: &Gl, ty: u32, src: &str) -> Result<web_sys::WebGlShader, String> {
    let sh = gl.create_shader(ty).ok_or("create_shader failed")?;
    gl.shader_source(&sh, src);
    gl.compile_shader(&sh);
    if gl.get_shader_parameter(&sh, Gl::COMPILE_STATUS).as_bool() != Some(true) {
        let log = gl.get_shader_info_log(&sh).unwrap_or_default();
        gl.delete_shader(Some(&sh));
        return Err(format!("shader compile error: {log}"));
    }
    Ok(sh)
}

fn compile_program(gl: &Gl, vs_src: &str, fs_src: &str) -> Result<WebGlProgram, String> {
    let vs = compile_shader(gl, Gl::VERTEX_SHADER, vs_src)?;
    let fs = compile_shader(gl, Gl::FRAGMENT_SHADER, fs_src)?;
    let prog = gl.create_program().ok_or("create_program failed")?;
    gl.attach_shader(&prog, &vs);
    gl.attach_shader(&prog, &fs);
    gl.link_program(&prog);
    gl.delete_shader(Some(&vs));
    gl.delete_shader(Some(&fs));
    if gl.get_program_parameter(&prog, Gl::LINK_STATUS).as_bool() != Some(true) {
        let log = gl.get_program_info_log(&prog).unwrap_or_default();
        gl.delete_program(Some(&prog));
        return Err(format!("program link error: {log}"));
    }
    Ok(prog)
}

fn init_gl_engine() -> Result<RealEsrganGlEngine, String> {
    let win = web_sys::window().ok_or("no window")?;
    let doc = win.document().ok_or("no document")?;
    let canvas: HtmlCanvasElement = doc
        .create_element("canvas")
        .map_err(|_| "create canvas failed")?
        .dyn_into()
        .map_err(|_| "not HtmlCanvasElement")?;
    canvas.set_width(4);
    canvas.set_height(4);

    let opts = Object::new();
    let _ = Reflect::set(&opts, &"alpha".into(), &JsValue::TRUE);
    let _ = Reflect::set(&opts, &"antialias".into(), &JsValue::FALSE);
    let _ = Reflect::set(&opts, &"depth".into(), &JsValue::FALSE);
    let _ = Reflect::set(&opts, &"stencil".into(), &JsValue::FALSE);
    let _ = Reflect::set(&opts, &"premultipliedAlpha".into(), &JsValue::FALSE);
    let _ = Reflect::set(&opts, &"powerPreference".into(), &"high-performance".into());

    let ctx_val = canvas
        .get_context_with_context_options("webgl2", &opts)
        .map_err(|_| "webgl2 get_context failed")?
        .ok_or("webgl2 unavailable")?;
    let gl: Gl = ctx_val.dyn_into().map_err(|_| "not WebGl2RenderingContext")?;

    let has_f32 = gl.get_extension("EXT_color_buffer_float").ok().flatten().is_some();
    let has_f16 = gl.get_extension("EXT_color_buffer_half_float").ok().flatten().is_some();
    if !has_f32 && !has_f16 {
        return Err("WebGL2 float render targets unavailable".into());
    }
    let max_draw = gl
        .get_parameter(Gl::MAX_DRAW_BUFFERS)
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as i32;
    if max_draw < 4 {
        return Err("WebGL2 MAX_DRAW_BUFFERS < 4".into());
    }

    let mut renderer = gl
        .get_parameter(Gl::RENDERER)
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "WebGL2 GPU".into());
    if let Ok(Some(_)) = gl.get_extension("WEBGL_debug_renderer_info") {
        // UNMASKED_RENDERER_WEBGL = 0x9246
        if let Ok(v) = gl.get_parameter(0x9246) {
            if let Some(s) = v.as_string() {
                if !s.is_empty() {
                    renderer = s;
                }
            }
        }
    }
    let ren_low = renderer.to_ascii_lowercase();
    let is_software = ren_low.contains("swiftshader")
        || ren_low.contains("llvmpipe")
        || ren_low.contains("softpipe")
        || ren_low.contains("software");

    let (feat_internal_fmt, feat_tex_type) = if has_f32 {
        (Gl::RGBA32F, Gl::FLOAT)
    } else {
        (Gl::RGBA16F, Gl::HALF_FLOAT)
    };

    let p_in = compile_program(&gl, GL_VS, GL_CONV_IN_FS)?;
    let conv_in = ConvInProg {
        u_src: gl.get_uniform_location(&p_in, "uSrc"),
        u_weights: gl.get_uniform_location(&p_in, "uWeights"),
        u_bias: gl.get_uniform_location(&p_in, "uBias"),
        u_prelu: gl.get_uniform_location(&p_in, "uPrelu"),
        u_group_base: gl.get_uniform_location(&p_in, "uGroupBase"),
        u_dims: gl.get_uniform_location(&p_in, "uDims"),
        prog: p_in,
    };

    let p_64 = compile_program(&gl, GL_VS, GL_CONV64_FS)?;
    let conv64 = Conv64Prog {
        u_src: gl.get_uniform_location(&p_64, "uSrc"),
        u_weights: gl.get_uniform_location(&p_64, "uWeights"),
        u_bias: gl.get_uniform_location(&p_64, "uBias"),
        u_prelu: gl.get_uniform_location(&p_64, "uPrelu"),
        u_group_base: gl.get_uniform_location(&p_64, "uGroupBase"),
        u_use_prelu: gl.get_uniform_location(&p_64, "uUsePrelu"),
        u_dims: gl.get_uniform_location(&p_64, "uDims"),
        prog: p_64,
    };

    let p_sh = compile_program(&gl, GL_VS, GL_SHUFFLE4X_FS)?;
    let shuffle4x = Shuffle4xProg {
        u_orig: gl.get_uniform_location(&p_sh, "uOrig"),
        u_head: gl.get_uniform_location(&p_sh, "uHead"),
        prog: p_sh,
    };

    let vao = gl.create_vertex_array().ok_or("create_vertex_array failed")?;
    let fbo = gl.create_framebuffer().ok_or("create_framebuffer failed")?;

    Ok(RealEsrganGlEngine {
        _canvas: canvas,
        gl,
        feat_internal_fmt,
        feat_tex_type,
        is_software,
        renderer,
        conv_in,
        conv64,
        shuffle4x,
        vao,
        fbo,
        models: HashMap::new(),
    })
}

fn with_gl_engine<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&mut RealEsrganGlEngine) -> Result<R, String>,
{
    GL_ENGINE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(init_gl_engine());
        }
        match slot.as_mut().unwrap() {
            Ok(eng) => f(eng),
            Err(e) => Err(e.clone()),
        }
    })
}

pub fn probe_gpu() -> GpuProbeInfo {
    match with_gl_engine(|eng| {
        Ok(GpuProbeInfo {
            webgl2_available: true,
            is_software: eng.is_software,
            renderer: eng.renderer.clone(),
            backend: if eng.is_software {
                "rust_webgl2_sw_realesrgan_srvgg"
            } else {
                "rust_webgl2_gpu_realesrgan_srvgg"
            },
        })
    }) {
        Ok(info) => info,
        Err(_) => GpuProbeInfo {
            webgl2_available: false,
            is_software: true,
            renderer: "Rust WASM CPU".into(),
            backend: "rust_wasm_cpu_realesrgan_srvgg",
        },
    }
}

fn upload_rgba32f_tex2d(gl: &Gl, w: i32, h: i32, data: &[f32]) -> Result<WebGlTexture, String> {
    let tex = gl.create_texture().ok_or("create_texture failed")?;
    gl.bind_texture(Gl::TEXTURE_2D, Some(&tex));
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::NEAREST as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::NEAREST as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
    // Safe: `view` is consumed synchronously by `tex_image_2d` with zero Rust heap allocations during the call.
    let view = unsafe { Float32Array::view(data) };
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
        Gl::TEXTURE_2D,
        0,
        Gl::RGBA32F as i32,
        w,
        h,
        0,
        Gl::RGBA,
        Gl::FLOAT,
        Some(&view),
    )
    .map_err(|e| format!("tex_image_2d float failed: {e:?}"))?;
    Ok(tex)
}

fn upload_rgba8_tex2d(gl: &Gl, w: i32, h: i32, data: Option<&[u8]>) -> Result<WebGlTexture, String> {
    let tex = gl.create_texture().ok_or("create_texture failed")?;
    gl.bind_texture(Gl::TEXTURE_2D, Some(&tex));
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::NEAREST as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::NEAREST as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        Gl::TEXTURE_2D,
        0,
        Gl::RGBA8 as i32,
        w,
        h,
        0,
        Gl::RGBA,
        Gl::UNSIGNED_BYTE,
        data,
    )
    .map_err(|e| format!("tex_image_2d rgba8 failed: {e:?}"))?;
    Ok(tex)
}

fn create_feat_tex_array(gl: &Gl, w: i32, h: i32, depth: i32, internal_fmt: u32, tex_type: u32) -> Result<WebGlTexture, String> {
    let tex = gl.create_texture().ok_or("create_texture array failed")?;
    gl.bind_texture(Gl::TEXTURE_2D_ARRAY, Some(&tex));
    gl.tex_parameteri(Gl::TEXTURE_2D_ARRAY, Gl::TEXTURE_MIN_FILTER, Gl::NEAREST as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D_ARRAY, Gl::TEXTURE_MAG_FILTER, Gl::NEAREST as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D_ARRAY, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D_ARRAY, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
    gl.tex_image_3d_with_opt_array_buffer_view(
        Gl::TEXTURE_2D_ARRAY,
        0,
        internal_fmt as i32,
        w,
        h,
        depth,
        0,
        Gl::RGBA,
        tex_type,
        None,
    )
    .map_err(|e| format!("tex_image_3d failed: {e:?}"))?;
    Ok(tex)
}

fn ensure_model_uploaded<'a>(
    eng: &'a mut RealEsrganGlEngine,
    model_id: &str,
    model_bytes: &[u8],
) -> Result<&'a GlUploadedModel, String> {
    if !eng.models.contains_key(model_id) {
        let parsed = RealEsrganModel::from_cwsr_bytes(model_bytes).map_err(|e| e.to_string())?;
        let first_tex = upload_rgba32f_tex2d(&eng.gl, 9, 64, &parsed.first_conv.pack_gl_conv_in_rgba32f())?;
        let mut body = Vec::with_capacity(parsed.body_convs.len());
        for layer in &parsed.body_convs {
            let tex = upload_rgba32f_tex2d(&eng.gl, 144, 64, &layer.pack_gl_conv64_rgba32f())?;
            body.push(GlBodyLayer {
                tex,
                bias: layer.bias.clone(),
                prelu: layer.prelu.clone().unwrap_or_else(|| vec![0.0; 64]),
            });
        }
        let last_tex = upload_rgba32f_tex2d(&eng.gl, 144, 48, &parsed.last_conv.pack_gl_conv64_rgba32f())?;
        eng.models.insert(
            model_id.to_string(),
            GlUploadedModel {
                first_tex,
                first_b: parsed.first_conv.bias,
                first_p: parsed.first_conv.prelu.unwrap_or_else(|| vec![0.0; 64]),
                body,
                last_tex,
                last_b: parsed.last_conv.bias,
            },
        );
    }
    Ok(eng.models.get(model_id).unwrap())
}

fn run_tile_gl(eng: &mut RealEsrganGlEngine, tile_rgba: &[u8], tw: usize, th: usize, model_id: &str, model_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let _ = ensure_model_uploaded(eng, model_id, model_bytes)?;
    let gl = &eng.gl;
    let mb = eng.models.get(model_id).unwrap();

    let tw_i = tw as i32;
    let th_i = th as i32;
    let ow = tw * 4;
    let oh = th * 4;
    let ow_i = ow as i32;
    let oh_i = oh as i32;

    let src_tex = upload_rgba8_tex2d(gl, tw_i, th_i, Some(tile_rgba))?;
    let mut ping = create_feat_tex_array(gl, tw_i, th_i, 16, eng.feat_internal_fmt, eng.feat_tex_type)?;
    let mut pong = create_feat_tex_array(gl, tw_i, th_i, 16, eng.feat_internal_fmt, eng.feat_tex_type)?;
    let head_tex = create_feat_tex_array(gl, tw_i, th_i, 12, eng.feat_internal_fmt, eng.feat_tex_type)?;
    let out_tex = upload_rgba8_tex2d(gl, ow_i, oh_i, None)?;

    let mrt4 = Array::new();
    mrt4.push(&JsValue::from(Gl::COLOR_ATTACHMENT0));
    mrt4.push(&JsValue::from(Gl::COLOR_ATTACHMENT1));
    mrt4.push(&JsValue::from(Gl::COLOR_ATTACHMENT2));
    mrt4.push(&JsValue::from(Gl::COLOR_ATTACHMENT3));
    let mrt1 = Array::new();
    mrt1.push(&JsValue::from(Gl::COLOR_ATTACHMENT0));
    let zero16 = [0.0f32; 16];

    gl.bind_vertex_array(Some(&eng.vao));
    gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&eng.fbo));
    gl.viewport(0, 0, tw_i, th_i);

    // Layer 0: Conv2d(3 -> 64, 3×3) + PReLU
    gl.use_program(Some(&eng.conv_in.prog));
    gl.active_texture(Gl::TEXTURE0);
    gl.bind_texture(Gl::TEXTURE_2D, Some(&src_tex));
    gl.uniform1i(eng.conv_in.u_src.as_ref(), 0);
    gl.active_texture(Gl::TEXTURE1);
    gl.bind_texture(Gl::TEXTURE_2D, Some(&mb.first_tex));
    gl.uniform1i(eng.conv_in.u_weights.as_ref(), 1);
    gl.uniform2i(eng.conv_in.u_dims.as_ref(), tw_i, th_i);
    gl.draw_buffers(&mrt4);

    for gb in (0..16).step_by(4) {
        for j in 0..4 {
            gl.framebuffer_texture_layer(Gl::FRAMEBUFFER, Gl::COLOR_ATTACHMENT0 + j as u32, Some(&ping), 0, (gb + j) as i32);
        }
        gl.uniform1i(eng.conv_in.u_group_base.as_ref(), gb as i32);
        gl.uniform4fv_with_f32_array(eng.conv_in.u_bias.as_ref(), &mb.first_b[gb * 4..gb * 4 + 16]);
        gl.uniform4fv_with_f32_array(eng.conv_in.u_prelu.as_ref(), &mb.first_p[gb * 4..gb * 4 + 16]);
        gl.draw_arrays(Gl::TRIANGLES, 0, 3);
    }

    // Body layers: 16 or 32 × Conv2d(64 -> 64, 3×3) + PReLU
    gl.use_program(Some(&eng.conv64.prog));
    gl.uniform1i(eng.conv64.u_src.as_ref(), 0);
    gl.uniform1i(eng.conv64.u_weights.as_ref(), 1);
    gl.uniform2i(eng.conv64.u_dims.as_ref(), tw_i, th_i);
    gl.uniform1i(eng.conv64.u_use_prelu.as_ref(), 1);

    for layer in &mb.body {
        gl.active_texture(Gl::TEXTURE0);
        gl.bind_texture(Gl::TEXTURE_2D_ARRAY, Some(&ping));
        gl.active_texture(Gl::TEXTURE1);
        gl.bind_texture(Gl::TEXTURE_2D, Some(&layer.tex));
        for gb in (0..16).step_by(4) {
            for j in 0..4 {
                gl.framebuffer_texture_layer(Gl::FRAMEBUFFER, Gl::COLOR_ATTACHMENT0 + j as u32, Some(&pong), 0, (gb + j) as i32);
            }
            gl.uniform1i(eng.conv64.u_group_base.as_ref(), gb as i32);
            gl.uniform4fv_with_f32_array(eng.conv64.u_bias.as_ref(), &layer.bias[gb * 4..gb * 4 + 16]);
            gl.uniform4fv_with_f32_array(eng.conv64.u_prelu.as_ref(), &layer.prelu[gb * 4..gb * 4 + 16]);
            gl.draw_arrays(Gl::TRIANGLES, 0, 3);
        }
        std::mem::swap(&mut ping, &mut pong);
    }

    // Tail Conv2d(64 -> 48, 3×3)
    gl.active_texture(Gl::TEXTURE0);
    gl.bind_texture(Gl::TEXTURE_2D_ARRAY, Some(&ping));
    gl.active_texture(Gl::TEXTURE1);
    gl.bind_texture(Gl::TEXTURE_2D, Some(&mb.last_tex));
    gl.uniform1i(eng.conv64.u_use_prelu.as_ref(), 0);
    gl.uniform4fv_with_f32_array(eng.conv64.u_prelu.as_ref(), &zero16);
    for gb in (0..12).step_by(4) {
        for j in 0..4 {
            gl.framebuffer_texture_layer(Gl::FRAMEBUFFER, Gl::COLOR_ATTACHMENT0 + j as u32, Some(&head_tex), 0, (gb + j) as i32);
        }
        gl.uniform1i(eng.conv64.u_group_base.as_ref(), gb as i32);
        gl.uniform4fv_with_f32_array(eng.conv64.u_bias.as_ref(), &mb.last_b[gb * 4..gb * 4 + 16]);
        gl.draw_arrays(Gl::TRIANGLES, 0, 3);
    }

    // PixelShuffle(4×) + nearest_4x(orig) -> RGBA8 output!
    for j in 1..4u32 {
        gl.framebuffer_texture_2d(Gl::FRAMEBUFFER, Gl::COLOR_ATTACHMENT0 + j, Gl::TEXTURE_2D, None, 0);
    }
    gl.framebuffer_texture_2d(Gl::FRAMEBUFFER, Gl::COLOR_ATTACHMENT0, Gl::TEXTURE_2D, Some(&out_tex), 0);
    gl.draw_buffers(&mrt1);
    gl.viewport(0, 0, ow_i, oh_i);
    gl.use_program(Some(&eng.shuffle4x.prog));
    gl.active_texture(Gl::TEXTURE0);
    gl.bind_texture(Gl::TEXTURE_2D, Some(&src_tex));
    gl.uniform1i(eng.shuffle4x.u_orig.as_ref(), 0);
    gl.active_texture(Gl::TEXTURE1);
    gl.bind_texture(Gl::TEXTURE_2D_ARRAY, Some(&head_tex));
    gl.uniform1i(eng.shuffle4x.u_head.as_ref(), 1);
    gl.draw_arrays(Gl::TRIANGLES, 0, 3);

    let mut out = vec![0u8; ow * oh * 4];
    let read_res = gl.read_pixels_with_opt_u8_array(0, 0, ow_i, oh_i, Gl::RGBA, Gl::UNSIGNED_BYTE, Some(&mut out));

    gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
    gl.delete_texture(Some(&src_tex));
    gl.delete_texture(Some(&ping));
    gl.delete_texture(Some(&pong));
    gl.delete_texture(Some(&head_tex));
    gl.delete_texture(Some(&out_tex));

    read_res.map_err(|e| format!("read_pixels failed: {e:?}"))?;
    Ok(out)
}

pub struct RealEsrganFullOutput {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub scale: u32,
    pub model_id: &'static str,
    pub model_label: &'static str,
    pub backend: &'static str,
    pub ssim: f32,
    pub mean_color_drift: f32,
    pub match_pct: f32,
}

/// Yield one browser task slice between GPU tiles so scrolling/typing stays 60fps responsive!
async fn yield_browser_task() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(win) = web_sys::window() {
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0);
        } else {
            let _ = resolve.call0(&JsValue::UNDEFINED);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Complete 100% Pure-Rust Real-ESRGAN 4× restoration pipeline (GPU WebGL2 MRT -> Pure-Rust CPU fallback):
/// Zero-seam 16px halo tiling (async yielding per tile) -> Real-ESRGAN 4× -> OriginalQualityProfile Color Lock -> Pure-Rust SSIM Gate!
pub async fn restore_image_realesrgan_rust(
    rgba: &[u8],
    width: u32,
    height: u32,
    kind: RealEsrganModelKind,
    model_bytes: &[u8],
    orig_profile: Option<&OriginalQualityProfile>,
) -> Result<RealEsrganFullOutput, String> {
    let w = width as usize;
    let h = height as usize;
    if w < 16 || h < 16 || rgba.len() < w * h * 4 {
        return Err("Invalid input dimensions".into());
    }

    let model_id = kind.model_id();
    let model_label = kind.model_label();
    let scale = 4usize;
    let ow = w * scale;
    let oh = h * scale;

    let probe = probe_gpu();
    let (mut out_rgba, backend) = if probe.webgl2_available {
        let tile = if probe.is_software { 144usize } else { 160usize };
        let gpu_res: Result<Vec<u8>, String> = if w <= tile && h <= tile {
            with_gl_engine(|eng| run_tile_gl(eng, rgba, w, h, model_id, model_bytes))
        } else {
            let halo = 16usize;
            let mut out = vec![0u8; ow * oh * 4];
            let mut err_opt: Option<String> = None;
            let mut ty = 0usize;
            'outer: while ty < h {
                let valid_y1 = (ty + tile).min(h);
                let pad_y0 = ty.saturating_sub(halo);
                let pad_y1 = (valid_y1 + halo).min(h);
                let th = pad_y1 - pad_y0;
                let mut tx = 0usize;
                while tx < w {
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

                    let tile_out = match with_gl_engine(|eng| run_tile_gl(eng, &tile_in, tw, th, model_id, model_bytes)) {
                        Ok(buf) => buf,
                        Err(e) => {
                            err_opt = Some(e);
                            break 'outer;
                        }
                    };
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
                    yield_browser_task().await;
                    tx = valid_x1;
                }
                ty = valid_y1;
            }
            match err_opt {
                Some(e) => Err(e),
                None => Ok(out),
            }
        };

        match gpu_res {
            Ok(buf) => (buf, probe.backend),
            Err(_) => {
                let model = RealEsrganModel::from_cwsr_bytes(model_bytes).map_err(|e| e.to_string())?;
                let (buf, _, _) = run_realesrgan_tiled(rgba, width, height, &model, 128, None)
                    .map_err(|e| e.to_string())?;
                (buf, "rust_wasm_cpu_realesrgan_srvgg")
            }
        }
    } else {
        let model = RealEsrganModel::from_cwsr_bytes(model_bytes).map_err(|e| e.to_string())?;
        let (buf, _, _) = run_realesrgan_tiled(rgba, width, height, &model, 128, None)
            .map_err(|e| e.to_string())?;
        (buf, "rust_wasm_cpu_realesrgan_srvgg")
    };

    // 3. OriginalQualityProfile Color/Contrast/Saturation Lock (preserves 100% of Real-ESRGAN vector-sharp 4× edges!)
    let match_pct = calibrate_color_preserve_sr(&mut out_rgba, ow as u32, oh as u32, orig_profile);

    // 4. Pure-Rust Anti-Corruption Quality Gate (SSIM + Color Drift)
    let (passed, ssim, drift) = evaluate_sr_quality_gate(rgba, width, height, &out_rgba, ow as u32, oh as u32);
    if !passed {
        return Err(format!("quality_gate_rejected (ssim={ssim:.4}, drift={drift:.4})"));
    }

    Ok(RealEsrganFullOutput {
        rgba: out_rgba,
        width: ow as u32,
        height: oh as u32,
        scale: 4,
        model_id,
        model_label,
        backend,
        ssim,
        mean_color_drift: drift,
        match_pct,
    })
}
