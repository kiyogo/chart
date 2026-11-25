# Technical Plan: Spec 157

1. Update `GL_FIDELITY_CAS_FS` in `app/src/storage/realesrgan_gpu.rs`:
   - Remove the `±2px/±3px` wide unsharp tightener that magnified skin/shadow grain.
   - Compute CAS strictly in Luminance (`eL, bL, dL, fL, hL, aL, cL, gL, iL`), suppress isolated spikes via RCAS noise detector `nz` and dark-shadow gate `smoothstep(0.08, 0.28, mxCrossL)`, apply the luminance delta equally across `rgb` (`e + vec3(sharpL - eL)`), and clamp to `[mnRgb, mxRgb]`.
   - Set balanced sharpness gains (`NomosUniSpanX2 => 0.50`, `is_span() => 0.60`, `AnimeVideoV3 => 0.90`).
2. Set `maxInEdge = 1024` in `app/web/ai_restorer.js` for hardware WebGL2 (`single_pass_ok <= 1_100_000` still runs `1024×768` in 1 pass).
3. Rebuild web bundle, run visual inspection on `media_1795549648014_4417d69b.jpg` (full + nose crop) and `crop_orig.png`, confirm zero red nostril speckles & clean LongTask profile, commit & push.
