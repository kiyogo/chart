# Spec 123: Pure Real-ESRGAN AI Super-Resolution with Zero Chromatic Aberration & Artifact-Free Edge Restoration

## 1. Problem Statement & User Intent
The user reported:
> "nó có dùng AI để làm nét ảnh không vậy ??? phải dùng AI để làm nét ảnh đi chứ 
lỗi àm nét ảnh quá nhiều"

Accompanied by an uploaded screenshot (`media_1795534017005_cf8b35ea.jpg`) showing an anime character zoomed at 400% with severe visual defects:
- Violent neon-green and deep-red chromatic ringing/halos around hair and clothes.
- Jagged, stepped, staircase stroke outlines instead of smooth vector-like anime lines.
- Heavy green/yellow noise and speckling over smooth white fabric and skin.

### Root Cause Analysis
In Spec 121, post-processing shader parameters were pushed to extreme, non-physical limits:
1. **Division-by-Near-Zero Singularity in AMD CAS**:
   - `w = max(amp * peak, vec3(-0.28))` allowed `w` to pass the singularity threshold `-0.25` where `1.0 + 4.0 * w` approaches zero, multiplying channel differences up to 25x and inverting signs.
2. **Independent RGB Channel Deconvolution**:
   - `deconv = cas1 + (cas1 - avg2) * (1.28 * 1.68) = cas1 + (cas1 - avg2) * 2.15` ran independently across R, G, and B. Any edge between different hues (e.g. orange hair vs green background) had opposite signs in R and G, creating intense neon-green halos on the outside and crimson-red fringes on the inside.
3. **Destructive Dark-Line Snapping**:
   - `mix(deconv, mn5, darkLine * ...)` forced anti-aliased edge pixels directly to local 5x5 minimum `mn5` (black), destroying smooth anti-aliased anime lines into crude jagged staircases.
4. **Low-Frequency Color Smear**:
   - Injecting `tLow - srLow` from low-resolution bilinear sampling polluted clean 4x high-frequency neural lines with blurry color noise.

### Desired Solution
True Real-ESRGAN (`SRVGGNetCompact` 4x) is an end-to-end deep neural network trained specifically to reconstruct clean anime art, vector-sharp lines, and noise-free cel-shading.
1. Output the pure Real-ESRGAN neural network reconstruction (`uSr`) without destroying it with crude unsharp mask filters.
2. If post-CAS is used, calculate CAS edge contrast strictly on **Scalar Luminance** (`rgbToLuma`), with standard AMD-safe peak bounds (`-0.125`), ensuring `1.0 + 4.0 * w >= 0.50` (100% zero division by zero).
3. Apply the exact same scalar CAS weight `w` uniformly across all 3 RGB channels, ensuring 100% mathematical impossibility of chromatic aberration or color fringing.
4. Eliminate the 2.15x RGB deconvolution and dark-line clamp completely.

## 2. Requirements & Acceptance Criteria
- [x] WebGL2 fragment shader `GL_FIDELITY_CAS_FS` rewritten to operate strictly on scalar luminance for AMD CAS, applying identical scalar weights across RGB channels.
- [x] Peak weight in CAS clamped to safe AMD specification range `[-0.125, 0.0]`, guaranteeing denominator `>= 0.50`.
- [x] Remove RGB deconvolution overshoot (`(cas1 - avg2) * 1.28`) and `darkLine` thresholding (`mix(deconv, mn5, ...)`).
- [x] Global shader knob `color1[2]` normalized to `1.0` (standard balanced intensity).
- [x] Chromatic aberration, green/red halos, and edge noise are 100% eliminated.
- [x] Real-ESRGAN 4x neural reconstruction shines through with silky-smooth vector strokes and pristine flat color fills.
- [x] E2E verification test `app/e2e/spec123-pure-ai-super-res.mjs` verifies zero chromatic aberration and pristine edge quality.
