# Spec 133: Luminous AI Laser Scan Sweep Beam Across Images

## 1. Context & User Problem
- **User Request**:
  > "sao có một số chỗ không hiện hiệu ứng tạo ảnh nhỉ ,cái quét qua ảnh ý ,..."
- **User Attachments**:
  - `media_1795543691524_90fd00e5.jpg`: Lightbox photo of an anime character ("Conan") during AI enhancement (`• 3%`). The sweep wave ("cái quét qua ảnh") is extremely faint / washed out against the character's light clothing and skin, rendering it virtually invisible to the user.
  - `media_1795543694883_3e754f58.jpg`: Lightbox photo of a wide landscape banner ("CHRONIC...") during AI enhancement (`• 14%`). Due to wide aspect ratio, excessive horizontal offsets (`translate3d(-48%, 0, 0)` on `inset: -12% -55%`), and low opacity (18%-40%) with `mix-blend-mode: screen`, the sweep beam was completely offscreen or invisible.
- **Root Cause Analysis**:
  1. **Low contrast & washed-out sweep beam**:
     - `.ai-gen-dots-wave` in `app/assets/main.css:3918` used a soft gradient with low opacity (`rgba(96, 165, 250, 0.18)` to `0.40}`) and `mix-blend-mode: screen`. On bright or colorful images, `screen` blend mode produces near-zero luminance difference, making the sweep invisible.
     - Lacked a crisp, radiant laser scan line (core leading edge) that defines an authentic AI scanning / generative fill effect.
  2. **Animation traversal range mismatch**:
     - `inset: -12% -55%` gave the wave a width of 210%. Sweeping from `-48%` to `48%` meant the center of the beam was outside the visible image container for nearly 50% of each cycle, creating periods where users saw only static dots and no sweep.
  3. **Missing in media loading placeholder (`blur_parts`)**:
     - In `app/src/components/rich.rs:1042-1045`, the loading stage of chat image attachments rendered `ai-gen-frost`, `ai-gen-aurora`, and `ai-gen-dots-base`, but completely omitted `ai-gen-dots-wave`. Thus, images loading in chat messages had zero sweep wave.

---

## 2. Goals & Acceptance Criteria

### Goal 1: Radiant & High-Contrast Laser Scan Sweep Beam
- [ ] `.ai-gen-dots-wave` features a vibrant, high-energy composite gradient with cyan (`#38BDF8`), electric blue (`#60A5FA`), pure white core (`#FFFFFF`), and purple aura (`#C084FC`) with `drop-shadow` glow.
- [ ] Add an ultra-crisp laser scan line (`.ai-gen-dots-wave::after`) with bright white core (`#FFFFFF`) and neon cyan/violet aura, skewed at -18° to provide an unmistakable, futuristic scanning laser that is clearly visible across ANY image background (dark, bright, anime, photo, portrait, landscape).

### Goal 2: Smooth, Continuous, Visible Sweep Traversal
- [ ] Adjust traversal parameters so the laser beam traverses the image continuously within a smooth 1.8s cadence, spending the vast majority of each cycle actively scanning across the visible image area.

### Goal 3: Complete Coverage Across All Image Stages
- [ ] Add `div { class: "ai-gen-dots-wave", aria_hidden: "true" }` to `app/src/components/rich.rs:1045` so image attachments in chat during initial download/decryption show the scan sweep.
- [ ] Both in-chat `RichImage` and full-screen `Lightbox` display the radiant laser sweep during AI enhancement.

### Goal 4: Quality Verification & Zero Regressions
- [ ] Playwright E2E test captures and verifies the laser sweep beam across multiple frames and confirms visibility on dark, midtone, and bright image surfaces.
