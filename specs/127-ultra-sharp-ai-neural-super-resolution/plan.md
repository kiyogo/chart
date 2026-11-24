# Plan 127: Ultra-Sharp AI Neural Super-Resolution Architecture

## 1. Technical Analysis & Root Causes
- Current `GL_FIDELITY_CAS_FS` in `app/src/storage/realesrgan_gpu.rs`:
  - Clamping `clamp(cas, mnRgb, mxRgb)` truncates the high-frequency overshoot/undershoot, eliminating edge contrast enhancement.
  - The gate `smoothstep(0.025, 0.12, edgeRange)` suppresses fine lines (< 0.12).
  - Taps are restricted to 4 cross neighbors, missing 45-degree diagonal lines.
  - Strength is limited by `color1[2] = 1.0 / 0.90` and peak `-0.125`.

## 2. Architecture & Shader Math
- **Neighborhood**: Sample 8 neighbors around center `e`:
  - Cross: `b (0, -1)`, `d (-1, 0)`, `f (1, 0)`, `h (0, 1)` with weight 1.0.
  - Diagonal: `a (-1, -1)`, `c (1, -1)`, `g (-1, 1)`, `i (1, 1)` with weight 0.5.
- **Scalar Luminance**: Convert all taps to luma (`dot(c, vec3(0.299, 0.587, 0.114))`).
- **High-Frequency Laplacian**:
  - `avgNeighborsL = ((bL + dL + fL + hL) + (aL + cL + gL + iL) * 0.5) / 6.0`
  - `highFreqL = eL - avgNeighborsL`
- **Gate & Strength**:
  - `edgeRange = mxAllL - mnAllL`
  - `gate = smoothstep(0.008, 0.045, edgeRange)`
  - `amp = sqrt(clamp(min(mnCrossL, 2.0 - mxCrossL) / max(mxCrossL, 1e-4), 0.0, 1.0))`
  - `boostStrength = (0.35 + 0.45 * amp) * clamp(uColor1.z, 0.0, 1.8) * gate`
  - `enhancedL = eL + highFreqL * boostStrength`
- **Headroom Envelope**:
  - `boundMinL = clamp(mnAllL - edgeRange * 0.35, 0.0, 1.0)`
  - `boundMaxL = clamp(mxAllL + edgeRange * 0.35, 0.0, 1.0)`
  - `targetL = clamp(enhancedL, boundMinL, boundMaxL)`
- **Color-Preserving Uniform Scalar Ratio**:
  - `scalarRatio = clamp((eL > 1e-4) ? (targetL / eL) : 1.0, 0.65, 1.45)`
  - `sharp = clamp(e * scalarRatio, 0.0, 1.0)`
- **Knob Value**:
  - `color1[2] = if kind == RealEsrganModelKind::AnimeVideoV3 { 1.25 } else { 1.15 }`

## 3. Files to Modify
- `app/src/storage/realesrgan_gpu.rs`: Update `GL_FIDELITY_CAS_FS` shader and `color1[2]` knob.
- `app/e2e/spec127-ultra-sharp-ai-enhancement.mjs`: E2E verification test.

## 4. Verification Strategy
- Rebuild web application with `./app/build-web.sh`.
- Run Playwright test `app/e2e/spec127-ultra-sharp-ai-enhancement.mjs`.
- Confirm 4x upscale (1600x1440), clean white spread (< 4.5), and save screenshot artifact.
- Run regression tests `spec123` and `spec126`.
