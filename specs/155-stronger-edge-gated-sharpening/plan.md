# Technical Plan: Spec 155

1. Update `GL_FIDELITY_CAS_FS` in `app/src/storage/realesrgan_gpu.rs`:
   - Fix `w` lower clamp from `-0.18` (which inverted `1.0 + 5.8 * w` near `-0.1724`) to `-0.155` while allowing `uColor1.z` up to `1.65`.
   - Add a multi-scale `±2px` / `±3px` wide-ramp contrast tightener gated by `smoothstep(0.022, 0.080, edgeRange2)` and clamped strictly within the local neighborhood `[min, max]` to collapse blurry 4× edge ramps into sharp edges without haloing or disturbing flat regions.
2. Raise `color1[2]` sharpness weights in `restore_image_realesrgan_rust` (`NomosUniSpanX2 => 1.15`, `is_span() => 1.35`, `AnimeVideoV3 => 1.50`).
3. Rebuild web bundle & server binary, test visually on both anime and real portrait photography in live Chrome WebGL2, restart daemon, commit & push.
