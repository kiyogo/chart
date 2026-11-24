# Implementation Plan - Spec 123: Pure Real-ESRGAN AI Super-Resolution with Zero Chromatic Aberration

## 1. Architectural Strategy
The browser-side Real-ESRGAN engine runs true `SRVGGNetCompact` 4x weights in WebGL2 (`app/src/storage/realesrgan_gpu.rs`).
The final output stage (`GL_FIDELITY_CAS_FS`) currently applies a destructive multi-scale RGB unsharp filter on top of the neural network's output, causing severe color fringing and jagged lines.

We will:
1. **Refactor `GL_FIDELITY_CAS_FS`**:
   - Compute edge contrast and AMD CAS strictly using scalar luminance: `float eL = dot(e, vec3(0.299, 0.587, 0.114))`.
   - Calculate a single scalar weight `w = max(amp * peak, -0.125)`.
   - Apply scalar `w` uniformly to all color channels: `cas = ((b + d + f + h) * w + e) / (1.0 + 4.0 * w)`.
   - Clamp within local 3x3 cross bounds `[min(cross), max(cross)]` to guarantee zero ringing and zero overshoot.
   - Remove RGB deconvolution `(cas1 - avg2) * 1.28` and `darkLine` snapping to `mn5`.
   - Apply sender OriginalQualityProfile color adjustment via clean global tone mapping only when profile delta exists, avoiding low-res spatial injection.
2. **Normalize Intensity Multiplier in `realesrgan_gpu.rs`**:
   - Set `color1[2] = 1.0` for AnimeVideoV3 (standard balanced neural intensity).
3. **Rebuild Web Client Bundle**:
   - Compile via `./app/build-web.sh`.
4. **Validate with Playwright E2E**:
   - Create `app/e2e/spec123-pure-ai-super-res.mjs` comparing high-contrast anime edges (orange hair on green background) to ensure:
     - Color fringing (cross-channel gradient difference) is < 0.05 (near zero).
     - SSIM against ground-truth neural output is > 0.82.
     - Zero NaN/infinity/blown pixels.
5. **Auto-Push to GitHub** according to Rule 1.

## 2. File Change Plan
| File Path | Action | Description |
|---|---|---|
| `app/src/storage/realesrgan_gpu.rs` | Edit | Rewrite `GL_FIDELITY_CAS_FS` with pure luma-based AMD CAS and zero RGB deconvolution. |
| `specs/123-pure-ai-neural-super-res-zero-artifacts/tasks.md` | Create | SDD task checklist. |
| `app/e2e/spec123-pure-ai-super-res.mjs` | Create | E2E validation script. |
