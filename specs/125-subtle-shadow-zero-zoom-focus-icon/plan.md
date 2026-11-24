# Technical Plan: Spec 125 - Subtle Shadow & Zero Hover Zoom for Focus Button

## Architecture & Stylesheet Adjustments
1. **Target**: `app/assets/main.css` lines 3992-4034 (`.rich-image-enhance-corner`, `.rich-image-enhance-corner:hover`, `.rich-image-enhance-corner:active`).
2. **Implementation**:
   - Base `.rich-image-enhance-corner`:
     ```css
     .rich-image-enhance-corner {
       position: absolute;
       bottom: 8px;
       right: 8px;
       z-index: 3;
       width: 28px;
       height: 28px;
       padding: 0;
       border-radius: 4px;
       border: none !important;
       outline: none !important;
       box-shadow: none !important;
       background: transparent !important;
       backdrop-filter: none !important;
       -webkit-backdrop-filter: none !important;
       color: #ffffff !important;
       display: inline-flex;
       align-items: center;
       justify-content: center;
       cursor: pointer;
       opacity: 0.9;
       filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.45));
       transition: opacity 140ms ease, filter 140ms ease;
     }
     ```
   - Hover `.rich-image-enhance-corner:hover`:
     ```css
     .rich-image-enhance-corner:hover {
       background: transparent !important;
       backdrop-filter: none !important;
       -webkit-backdrop-filter: none !important;
       box-shadow: none !important;
       border: none !important;
       color: #ffffff !important;
       transform: none !important;
       opacity: 1 !important;
       filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.6)) !important;
     }
     ```
   - Active `.rich-image-enhance-corner:active`:
     ```css
     .rich-image-enhance-corner:active {
       transform: none !important;
       opacity: 0.8 !important;
       background: transparent !important;
     }
     ```
3. **Build & Verification**:
   - Rebuild web bundle via `./app/build-web.sh`.
   - Write and run Playwright test `app/e2e/spec125-subtle-shadow-zero-zoom.mjs` asserting:
     - Hover transform is `none` (matrix `1, 0, 0, 1, 0, 0` or `none`, scale = 1.0).
     - Hover filter is tight drop-shadow without heavy blur.
     - Capture screenshot in artifact directory `verify_spec125/01_clean_subtle_focus_icon.png`.
   - Run regression test `app/e2e/spec122-corner-focus-icon.mjs` and `app/e2e/spec124-settings-scale.mjs`.
