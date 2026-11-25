# Spec 157: Luma-Only Grain-Safe Shadow-Gated Sharpening (Natural Portrait & Clean Anime)

## Problem Statement
Inspecting user-submitted comparisons (`media_1795614320923_11a50040.png` nostril crop and `media_1795614328400_d50c77a5.jpg` portrait):
1. Spec 155's aggressive `±2px / ±3px` `wideGain` unsharp mask + per-channel RGB CAS at `1.15` amplified subtle sensor grain inside dark nostril shadows into visible reddish speckles and gave skin/hair an overly crunchy, artificial look.
2. Capping `maxInEdge` at `900` caused `1024×768` portraits to pre-downscale `1024 -> 900` before `2×` restoration, softening fine facial detail even though `1024×768` (`786k` px) fits well inside WebGL2 `single_pass_ok` (`<= 1.1M` px).

## Acceptance Criteria
1. Replace per-channel RGB + `±3px` `wideGain` in `GL_FIDELITY_CAS_FS` with **Luma-only 8-tap CAS** featuring:
   - RCAS isolated-grain suppression (`nz = |0.25*(bL+dL+fL+hL) - eL| / rangeL`) so isolated noise/pore specks are never sharpened (`0%` nostril speckles).
   - Dark-shadow gate (`smoothstep(0.08, 0.28, mxCrossL)`) + clean-edge gate (`smoothstep(0.025, 0.090, edgeRange)`) protecting skin shading and shadow troughs.
   - Strict local `[mnRgb, mxRgb]` clamp + balanced gains (`NomosUniSpanX2: 0.50`, `SPAN 4×: 0.60`, `AnimeVideoV3: 0.90`).
2. Restore `maxInEdge = 1024` on hardware WebGL2 so standard `1024×768` images process at 100% native resolution in 1 single pass.
3. Verify visually via nose crop (`orig_nose` vs fixed nose) + full portrait + anime output in Chrome WebGL2 that nostril red speckles are 100% gone while hair/eyes/outlines remain crisp and natural.
