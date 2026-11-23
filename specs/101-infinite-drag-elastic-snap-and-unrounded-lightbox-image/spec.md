# Spec 101: Infinite Drag with Elastic Snap-Back and True Unrounded Lightbox Image

## Problem Statement
1. In the global image viewer (`GlobalImageLightbox`):
   - At 1.0x (100% zoom), dragging was completely blocked (`if zoom() > 1.01`), preventing users from dragging the photo at all.
   - At higher zoom levels, dragging past corner limits was artificially capped to a tiny 40px resistance (`MAX_PULL: 40.0` in `rubber_band_axis`), restricting free fluid movement.
   - The user requires infinite drag in all directions ("kéo vô hạn được tất cả") at any zoom level, and when released, the image must smoothly glide/snap back to its proper position ("nó sẽ vẫn chảy lại chỗ cũ").
2. The lightbox image (`.lightbox-img`) had `border-radius: 6px;` which cropped corners off artwork and photos. The user requires true unrounded rectangular presentation ("ảnh không được bo góc nữa, hiện đúng như ảnh thôi").

## Acceptance Criteria
- **AC1**: Infinite drag in all directions at all zoom levels (100% to 400%). Users can drag the image freely across the screen without hard resistance caps.
- **AC2**: On release (`onmouseup` / `onmouseleave`), the image smoothly snaps/glides back via CSS transition (`transition: transform 300ms cubic-bezier(0.22, 1, 0.36, 1)`) to its valid clamped position (center `(0, 0)` at 1.0x, or corner-clamped within viewport bounds when zoomed).
- **AC3**: `.lightbox-img` has `border-radius: 0 !important;` (zero rounded corners), displaying the exact rectangular artwork.
- **AC4**: Double-click to toggle zoom (1.0x <-> 2.0x) and click backdrop to close remain 100% functional without conflict.
- **AC5**: Automated Playwright E2E verification confirms infinite drag, smooth spring snap-back, and 0px border-radius.
