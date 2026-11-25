# Spec 155: Multi-Scale Edge-Gated Zero-Halo Sharpening

## Problem Statement
After fixing Richardson-Lucy flat-region ripples and CAS `0.10` attenuation in Spec 154, soft 4× upscaled gradients (spanning 2–4 output pixels) appear too mild because 1px CAS (`±1px`) only inspects adjacent sub-pixels (`0.25` source pixels apart) and `AnimeVideoV3` / `SPAN` CAS gains were capped at `0.10–0.35`. Additionally, `1.0 + 5.8 * w` in `GL_FIDELITY_CAS_FS` needed strict `w >= -0.155` stability bounds.

## Acceptance Criteria
1. Upgrade `GL_FIDELITY_CAS_FS` with stable 8-tap 1px CAS (`w >= -0.155`) combined with a noise-gated 2px/3px wide-ramp unsharp tightener clamped strictly inside the local neighborhood `[min, max]` envelope (`0%` overshoot/undershoot halos, `gate2 == 0` in flat `< 0.022` regions).
2. Raise `uColor1.z` sharpness multipliers across `AnimeVideoV3` (`1.50`), `NomosUniSpanX4` / `ClearRealitySpanX4` (`1.35`), and `NomosUniSpanX2` (`1.15`).
3. Visually verify significantly crisper edges and fine details on both cel-shaded anime (`crop_orig.png`) and real portrait photography without wavy ripples or halo artifacts.
