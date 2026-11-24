# Spec 125: Subtle Shadow & Zero Hover Zoom for Focus Button

## Problem Statement
In Spec 122, the transparent Focus button was given a double heavy drop shadow:
`filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.85)) drop-shadow(0 0 6px rgba(0, 0, 0, 0.75));`
and on hover:
`transform: scale(1.15); filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.95)) drop-shadow(0 0 8px rgba(0, 0, 0, 0.85));`

This caused two prominent visual defects reported by the user:
1. **"lỗi bị hiện mờ đen quá nhiều"**: A large, murky dark cloud/halo spreads around the icon over the image corner because of the high radius (6px-8px) and excessive opacity (0.75-0.95) blur filters.
2. **"khi di trỏ chuột vô không đưo hiện phóng to nữa nhé"**: Hovering over the icon triggers `transform: scale(1.15)`, zooming in the icon, which looks jarring and worsens the shadow halo.

## Goals & Acceptance Criteria
1. **Crisp, Subtle Shadow**:
   - Replace heavy double shadows with a tight, subtle outline: `filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.45));`
   - Ensure the white reticle icon remains distinct and legible over any background without muddying or darkening the image.
   - On hover: maintain a tight crisp shadow `filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.6)) !important;` without spreading out into a blurry dark patch.
2. **Zero Hover Zoom**:
   - On hover: `transform: none !important;`
   - On active: `transform: none !important;` (or subtle opacity change `opacity: 0.8`).
   - The icon stays perfectly stable at 1:1 scale (18px) when the cursor enters, hovers, or leaves.
3. **Transparent & Clean**:
   - Keep 100% transparent background, no grey box, no border.
4. **Verification**:
   - Automated E2E test verifying `transform: none` on hover and subtle drop-shadow filter.
   - Capture comparison screenshot demonstrating clean appearance without dark blur halo.
