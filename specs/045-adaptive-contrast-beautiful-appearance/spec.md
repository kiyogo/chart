# Spec 045: Adaptive Contrast & Foolproof Aesthetic Compensation Engine

## Problem Statement
Users experimenting with Appearance Settings (`Giao diện`) can easily produce unreadable or visually degraded interfaces under valid setting combinations:
1. **Darkened/Dark Backdrops in Light Theme**: Setting a dark wallpaper (`Đêm sao`, dark color, dark photo) or turning up `Độ tối phủ lên ảnh` (e.g., 40%–80%) while making panels transparent (`Độ trong suốt khung trắng` -> 70%–100%) turns the effective background dark charcoal/black, while text, icons, tile captions, nav labels, and buttons remain in Light Theme colors (black text on dark background + blinding solid white buttons).
2. **Bright Backdrops in Dark Theme**: Selecting a bright wallpaper or bright custom background color in Dark Theme with high panel transparency leaves white text floating over bright surfaces with near-zero contrast.
3. **Busy / Multi-Tone Photo Wallpapers (e.g., Anime / High-Contrast Art) with High Transparency**: Even with average mid-tone brightness, parts of a photo are pure white while other parts are dark navy/black. At 100% transparency with glass blur OFF, bare floating text (nav items, headings, tile captions, sliders) collides directly with detailed dark artwork and becomes illegible.
4. **Rail Icon Invisibility**: `.app-rail` sits directly over the app wallpaper; selecting a dark wallpaper (`Đêm sao` or dark image) in Light Theme leaves rail icons dark grey on dark blue.
5. **Custom Accent & Message Bubble Contrast Traps**:
   - Choosing a very bright `custom_accent` (e.g., `#FFFF00`, `#FFFFFF`) in Light Mode makes primary buttons, active switches, slider tracks, and `--accent-text` invisible or unreadable; choosing a near-black accent in Dark Mode makes controls vanish.
   - Choosing a bright `bubble_self` or dark `bubble_other` left `--msg-sent-meta` / `--msg-recv-meta` (timestamps) unreadable, or allowed bubbles to blend invisibly into the chat surface.
   - Choosing Chat Wallpaper `"Đêm sao"` (`wp-night`) in Light Mode darkened the chat background while leaving chat header text, icons, date dividers, and composer input dark.
6. **Layout Micro-Polish**: `.settings-group-title` icons stacked vertically above heading text instead of inline; `.settings-preview-chat` did not preview the chosen default chat wallpaper.

## Goals & Acceptance Criteria
1. **Real-Time Effective Luminance & Wallpaper Pixel Sampling**:
   - Compute effective composite brightness for the left rail (`railBackdropB`), navigation sidebar (`navEffectiveB`), and main panels (`panelEffectiveB`) combining wallpaper luminance (preset table, hex color, or 32×32 offscreen canvas pixel sampling for uploaded images), dim overlay (`app_wallpaper_dim`), and panel opacity (`panel_opacity`).
   - Compute image luminance standard deviation (`stdB`) to detect busy multi-tone wallpapers (`data-wp-busy="true"`).
   - Recompute immediately on ANY change to theme, system color scheme, wallpaper, dim, panel opacity, panel blur, custom accent, or custom bubble colors.
2. **Automatic Surface & Rail Ink Compensation (`data-surface-tone`, `data-rail-tone`)**:
   - When `panelEffectiveB < 0.48`, automatically apply `data-surface-tone="dark"` (crisp white/light foreground typography `#F4F6FA`, cool silver secondary `#C7D0DF`, harmonious translucent dark glass surfaces/cards/buttons/pills, bright readable accent text).
   - When `panelEffectiveB >= 0.48` (even in Dark Theme with bright wallpaper + high transparency), automatically apply `data-surface-tone="light"` (deep charcoal ink `#111827`, crisp luminous snow-white translucent cards/pills, dark readable accent text).
   - Independently compute `data-rail-tone="dark" | "light"` from the leftmost strip of the backdrop behind `.app-rail` so rail icons NEVER lose contrast against dark or light wallpapers.
3. **Busy & High-Transparency Protective Scrim (Without Forcing Blur)**:
   - Strictly honor the user's `panel_glass` toggle (NEVER force `backdrop-filter: blur(...)` when glass blur is OFF).
   - When `data-panel-scrim="true"` (translucent panels over wallpaper, especially busy photos `data-wp-busy="true"`), provide crisp, clean, borderless, shadowless translucent protective cards/pills on interactive items (`.settings-row`, `.settings-list`, `.wallpaper-tile`, `.theme-card`, `.settings-nav-item`, `.settings-group-title`, `.settings-page-title`, `.btn-secondary`, `.chat-item`, `.chat-header` info/action clusters) so every word achieves WCAG AA/AAA readability ($\ge 7:1$) over BOTH bright white and dark navy parts of a photo while letting the sharp wallpaper shine cleanly behind the workspace.
4. **Foolproof Custom Accent & Bubble Color Compensation**:
   - Automatically compensate extreme `custom_accent` choices (pure white on light surface, pure black on dark surface, neon yellow, etc.) so `--accent` maintains $\ge 3.5:1$ UI contrast, `--on-accent` achieves $\ge 4.5:1$ text contrast, and `--accent-text` achieves $\ge 5.0:1$ text contrast on the active surface tone.
   - Automatically compute `--msg-sent-text`, `--msg-sent-meta`, `--msg-recv-text`, `--msg-recv-meta` using exact WCAG 2.1 contrast ratio against custom bubble colors, and gently separate bubbles that would otherwise blend invisibly into the chat background.
   - Apply dark-surface legibility tokens to `.chat-column.wp-night` and `.settings-preview-chat.wp-night` so `"Đêm sao"` chat wallpaper always looks sharp in Light Mode.
5. **Visual Polish & Zero Regressions**:
   - Align `.settings-group-title` icon and label cleanly inline (`display: flex; align-items: center; gap: 8px;`).
   - Show the chosen default chat wallpaper (`wp-{pr.wallpaper}`) live inside `.settings-preview-chat`.
   - Maintain 100% Flat Minimalist borderless/shadowless standards and pass ALL existing regression suites (`Spec 028`, `Spec 040`, `Spec 041`, `Spec 042`, `Spec 043`, `Spec 044`).
