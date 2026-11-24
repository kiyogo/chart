# Spec 127: Ultra-Sharp AI Neural Super-Resolution (Luma 8-Tap CAS & Micro-Contrast Boost)

## 1. Problem Statement
In previous updates (Spec 123), conservative anti-ringing measures were introduced to eliminate chromatic fringing:
1. Strict clamping `clamp(cas, mnRgb, mxRgb)` completely eliminated high-frequency overshoot/undershoot, stripping out edge contrast pop and muting perceived sharpness.
2. The noise gate `smoothstep(0.025, 0.12, edgeRange)` had an overly high threshold (>0.12 or >30/255) that suppressed sharpening on fine lines, anime hair contours, and subtle textures.
3. Sharpening was capped at a mild peak (`-0.125`), and only 4 cross-taps were sampled, missing diagonal line gradients common in anime line art.

The user explicitly requested:
> *"làm nét ảnh mạnh hơn nữa đi"* (Make image sharpening significantly stronger).

## 2. Requirements & Goals
1. **Dramatic Visual Sharpness ("Mạnh Hơn Nữa")**:
   - Enhance edges, line art contours, hair strands, eyes, and fabric textures with unmistakable punch and crispness.
   - Utilize full 8-tap spatial neighborhood (4 cross + 4 diagonal) in `GL_FIDELITY_CAS_FS` to capture diagonal line gradients.
   - Lower the noise gate threshold to `smoothstep(0.008, 0.045, edgeRange)` so fine lines and delicate textures receive full enhancement.
   - Increase sharpening strength knob to `1.25` for anime (`AnimeVideoV3`) and `1.15` for general photo models.
2. **Zero Chromatic Aberration (0.000 Rainbow Halos)**:
   - Compute all high-frequency edge response and CAS adaptation strictly in scalar **LUMINANCE** space (`eL`, `avgNeighborsL`, `edgeRange`).
   - Transfer luminance target to RGB via a single uniform scalar ratio: `scalarRatio = clamp(targetL / eL, 0.65, 1.45)`.
   - Because `R`, `G`, and `B` are scaled by the exact same factor, chromatic ratios (`R/G`, `G/B`) are preserved identically, guaranteeing zero chromatic divergence.
3. **Adaptive Contrast Overshoot Envelope (Zero Ringing / Halo Halos)**:
   - Replace strict `[min, max]` clamping with an adaptive 35% edge-delta headroom:
     `boundMinL = clamp(mnAllL - edgeRange * 0.35, 0.0, 1.0)`
     `boundMaxL = clamp(mxAllL + edgeRange * 0.35, 0.0, 1.0)`
   - Flat fills (sky, skin fills, clean white suit) where `edgeRange < 0.008` receive zero sharpening, keeping them pure and silky-smooth (white channel spread < 4.5).
4. **Verification**:
   - Playwright E2E verification test confirming 4x super-resolution, clean white spread (< 4.5), and capturing visual screenshot.
   - Automated commit and push to `origin main` as required by Rule 1.
