# Spec 130: Universal AI Image Enhancement Effects & Sender Enablement

## Problem Statement
The user asked: *"sao có một số chỗ không hiện hiệu ứng ảnh nhỉ"* (Why don't some places show the image effect?).
Testing and analysis revealed several key scenarios where the AI enhancement shimmer effect (frosted crystal aurora + dot grid wave) and AI enhancement capabilities were unintentionally absent:
1. **Self-sent images & Note to Self (`is_sender_original = true`)**:
   - In `app/src/components/rich.rs`, `let is_orig = is_sender_original();` guarded `can_ai = !is_orig && data.m != "image/gif";` and `show_corner_icon = needs_on_demand_ai() && !is_ai && !is_orig && !is_loading && !is_revealing;`.
   - In the Lightbox, lines 2409, 2539, and 2589 checked `!m.is_sender_original`, completely blocking auto-enhancement, suppressing the frosted shimmer effect, and hiding the wand toggle button on all images uploaded or sent by the user (notably in Note to Self "Chính mình").
2. **Details Panel / Shared Media gallery**:
   - In `app/src/components/details_panel.rs`, line 604 hardcoded `auto_enhance_on_open: false`, preventing any image opened from the shared media sidebar from triggering the AI auto-enhancement shimmer effect.
3. **Initial Media Loading State**:
   - During initial image download/decryption before the blob URL is created, only a 16x16 canvas blur with a plain grey spinner was displayed instead of the frosted aurora shimmer.

## Goals & Acceptance Criteria
1. **Enable AI Enhancement on Self-Sent Images**:
   - Self-sent images in any chat (including Note to Self "Chính mình") must support AI enhancement.
   - Display the sleek Focus icon on hover when not yet enhanced.
   - When clicked or opened in Lightbox, auto-enhance and render the frosted-crystal aurora shimmer with live progress percentage.
   - Provide the top-toolbar wand toggle button in Lightbox so users can switch between original and enhanced versions.
2. **Enable Auto-Enhance in Details Panel / Shared Media Gallery**:
   - Images opened from the Details panel must set `auto_enhance_on_open: media_can_ai`, triggering the AI shimmer effect on open.
3. **Initial Media Loading Shimmer**:
   - Enhance the initial placeholder loading state (`blur_parts`) with frosted aurora shimmer styles so downloading images also display the consistent crystal shimmer.
4. **Transparent Communication**:
   - Clarify for the user why cached images (already enhanced once, instant load from IndexedDB) and non-static raster formats (GIFs and video streams) behave differently.
5. **Quality Verification**:
   - Playwright E2E test verifying corner focus icon on self-sent images, Lightbox auto-enhance with aurora shimmer overlay, and wand toggle button.
