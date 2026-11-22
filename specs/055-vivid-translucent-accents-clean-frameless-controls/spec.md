# Feature Specification: Vivid Unmistakable Translucent Accents, Clean Frameless Smooth-Wallpaper Controls & Compact Color Swatches (Spec 055)

**Feature Branch**: `main`
**Created**: 2026-11-22
**Status**: Completed

## 1. Problem Statement (from user screenshot on `Cực quang` @ 100% transparency)

1. **Washed-Out / Invisible Selected Items (`Giao diện`, `Vừa`, `Tròn`, `Hệ thống`, Gear Icon, `Theo hệ thống`, `Không`)**:
   - Mixing `93–98%` white into `--accent-soft` on translucent wallpapers turned selected items into near-pure white (`rgba(250, 252, 255, 0.72)`), making them invisible white-on-white over bright wallpapers (`Cực quang`, `Bạc hà`, `Đại dương`, `Hoàng hôn`, `Giấy`, `Chấm bi`, light photos) and ignoring the user's chosen `Màu nhấn`.
2. **Cluttered Grey Wireframe Boxes Around All 18 Unselected Segmented Buttons (`Gọn`, `Rộng`, `Mềm`, `Vuông`, `Chiếc lá`, ...)**:
   - Enabling `data-framed-controls="true"` on smooth readable wallpapers (`Cực quang`, `Hoàng hôn`, `Đại dương`, `Bạc hà`, `Đêm sao`) wrapped every idle segmented button in dull pale grey bordered boxes, cluttering the interface and hiding which button was selected.
3. **Dirty Grey Borders on Secondary Buttons & Washed-Out Switch Track**:
   - `.btn-secondary` (`Tải ảnh của bạn`, `Khôi phục giao diện mặc định`) and `.btn-ghost` showed pale white boxes with faint grey borders over bright `Cực quang`, and OFF toggle switches showed muddy lavender tint with a double white rim.
4. **Stretched 165px Dark Navy Wallpaper Color Swatch**:
   - `.settings-col .inline-form > input` forced `min-width: 11rem` on the custom wallpaper `<input type="color">`, stretching it into an awkward dark `#1F2A44` bar next to `Tải ảnh của bạn`.

## 2. Acceptance Criteria

1. **Always-Vivid, Unmistakable Translucent Accent on Active/Selected Items**:
   - Across ALL themes (`Sáng`, `Xám tro`, `Tối`, `Đen huyền`, `Theo hệ thống`) and ALL smooth wallpapers (`Không`, `Chấm bi`, `Bong bóng`, `Cực quang`, `Giấy`, `Hoàng hôn`, `Đại dương`, `Bạc hà`, `Đêm sao`, solid colors) from `0%` to `100%` transparency, `--accent-soft` ALWAYS uses crisp, vibrant translucent user accent color (`rgba(vr, vg, vb, 0.14..0.18)` in Light tone, `rgba(vr, vg, vb, 0.24..0.30)` in Dark tone, **ZERO boxy border**, **ZERO white washout**).
   - Only on genuinely busy, high-variance uploaded photos at extreme transparency (`wpBusy && alpha < 0.36`) does `--accent-soft` blend frosted backing (`tintW: 0.24`, `alpha: 0.72` light / `rgba(vr, vg, vb, 0.36)` dark) so selected pills remain vibrantly colorful AND readable over photo detail.
2. **Clean, Frameless Minimalist Controls on All Smooth Wallpapers**:
   - `data-framed-controls="true"` activates **ONLY** on genuinely busy uploaded photos (`wpBusy && alpha < 0.38`) — NEVER cluttering smooth presets (`Cực quang`, `Hoàng hôn`, `Đại dương`, `Bạc hà`, `Đêm sao`, etc.) with 18 idle grey boxes!
   - Zero size shift (`0.00px`) is still preserved whenever `data-framed-controls` transitions `false` -> `true` on busy photos.
3. **Clean Borderless Flat Secondary Buttons & Crisp Neutral Hover/Switch**:
   - `--bg-hover`: clean translucent neutral `rgba(15, 23, 42, 0.055)` (light) / `rgba(255, 255, 255, 0.09)` (dark).
   - `.btn-secondary` / `.btn-ghost` / `.btn-avatar-upload`: clean borderless flat neutral fill (`rgba(15, 23, 42, 0.06)` -> hover `0.10` -> active `0.14` on light; `rgba(255, 255, 255, 0.10)` -> `0.16` -> `0.22` on dark; `box-shadow: none`), using subtle frosted glass only over busy photos (`wpBusy`).
   - OFF toggle switch track: clean neutral `rgba(15, 23, 42, 0.16)` (light) / `rgba(255, 255, 255, 0.22)` (dark) with zero double white border.
4. **Compact Uniform Color Swatch Next to `Tải ảnh của bạn`**:
   - `.settings-col .inline-form > input:not([type="color"])` prevents stretching `<input type="color">`, keeping it a crisp borderless `2.75rem × 2.25rem` swatch with fresh default `#60A5FA`.
5. **Zero Regressions**:
   - All 4 E2E suites (`spec048`, `spec050`, `spec052`, `spec053`) pass 100% with 0 page errors; live screenshots on `Cực quang` (100%), `Hoàng hôn` (72% & 100%), `Đại dương` (100%), `Đêm sao` (100%), and `Sáng` / `Xám tro` / `Tối` / `Đen huyền` visually confirmed crisp and cohesive.
