# Spec 087 — ChatW LGBT Font v2.0 Overhaul: Crisp Upright Sculpted Typography & Unified Liquid-Prism 7-Color Luxury Theme

## Problem Statement
User feedback on Spec 086 (`media_1795431029549_521e62fd.jpg`): the LGBT font still looked ugly, washed-out, and amateurish due to four root visual causes:
1. **Wobbly warped letterforms**: `pride_x_shift` applied horizontal `sin(y)` warping and faux-slant to `Baloo 2`, making upright stems look crooked/blurry at 13–15px UI sizes, while thin weight made gradient text faint.
2. **Washed-out pale yellow on white & monochrome short labels**: `%`-based `background-size: 200%` over `flex: 1` labels stretched one cycle to ~500px so short labels sat entirely in faint pale yellow/orange (`1.6:1` contrast) instead of rich multi-color rainbow.
3. **Harsh/muddy icons & broken top-left logo**: 7 stops including faint yellow on 18px icons washed out icon centers, and `.rail-brand` logo got rainbow bleed over its blue tile.
4. **Inconsistent details**: `.range-value` ("Tự động") stayed grey, `<select>` showed a harsh purple selection box, and white text over bright neon yellow inside `.message-row.sent` bubbles had poor readability.

## Redesign Goals (Apple Pride / Figma / Gilbert Quality)
1. **ChatW LGBT v2.0 Typeface (`scripts/build_chatw_lgbt_font.py`)**:
   - Crisp, 100% upright geometric-rounded SemiBold (`600`) & ExtraBold (`800`) architecture (`Nunito` base for Latin/Vietnamese/Cyrillic + upright rounded multi-script bases for Arabic/Devanagari/Thai/Hebrew/Indic — ZERO `sin()` warp, ZERO crooked slant).
   - Perfectly upright, symmetric, crisp Bézier **Pride Heart tittles (`♥`)** on `i`, `j`, `ị`, Cyrillic `і`/`ј`/`ї`, `!`, `?`, `¡`, `¿`.
2. **Unified Pixel-Locked Liquid-Prism 7-Color Spectrum**:
   - OKLCH-balanced jewel-vivid 7 colors (`#F41F5E → #FF5E1A → #E69500 → #00B359 → #0095F6 → #4F46E5 → #B026FF` in Light Mode; bright luminous neon-pastel stops in Dark Mode) — EVERY hue is fresh, vibrant, AND high-contrast (`>= 3.8:1` on white).
   - Fixed-pixel `background-size: 280px 100%` flowing `0px → 280px` so ALL text across the entire screen glides at one unified liquid speed AND even short labels display lush multi-hue Pride colors.
   - Velvety diagonal Pride prism pills on `.message-row.sent .message-bubble`, `.btn-primary`, `.switch:checked`, and `.rail-brand` with crisp `#FFFFFF` icons/text.
3. **Jewel-Crisp Rainbow Icons**: High-contrast `#cw-lgbt-icon-grad` (no invisible yellow center) + crisp white `W` logo inside `.rail-brand`.

## Acceptance Criteria
1. Visual specimen PNG of `ChatW LGBT v2.0` shows razor-clean upright stems, crisp symmetric ♥ hearts, and flawless Vietnamese/Arabic/Devanagari/Thai/Cyrillic/CJK rendering.
2. Live Playwright E2E + full-screen visual inspection in Light Mode, Dark Mode, Settings, and Chat confirm a sleek, legible, modern luxury 7-color Pride UI with zero washed-out labels, zero broken logos, and clean static freeze when `Reduce motion` is ON.
3. Rust unit tests + E2E suites pass and changes are committed & pushed to `origin main`.
