# Spec 010: ChatW Studio — drawing and animation

Prompt section VIII (Drawing Studio, Animation Editor, outputs). Replaces the single-layer
drawing dialog from spec 006 with a full studio while keeping every entry point it had.

## Acceptance criteria
- AC1 Canvas editor: pencil, brush, eraser, fill (flood, tolerance), line, rectangle, ellipse, polygon
  (star/n-gon), text, eyedropper, move (layer offset); colour picker + palette + recent colours; size,
  opacity, outline/fill; zoom 25–800 %, pan, grid with snap; transparent background.
- AC2 Layers (max 12): add, duplicate, delete, reorder, rename, show/hide, opacity, blend mode.
- AC3 Undo / redo (≥ 40 steps) and a visible history count; autosave of a local draft.
- AC4 Animation: frames (max 60) add / duplicate / delete / move; onion skin; playback with FPS 1–30 and
  loop; motion tween (position, scale, rotation, opacity) over N frames with easing; motion presets.
- AC5 Export PNG (current frame), animated GIF (all frames, transparency kept), project file (.chatwdraw.json)
  that keeps every layer and frame; import an image as a new layer; import a project file.
- AC6 Projects saved on the server, private to their owner, listed with thumbnails, reopened for editing.
- AC7 Use the result as: avatar, cover (animated GIF kept), sticker (own pack "My Drawings"), custom emoji,
  "My GIFs", avatar decoration, nameplate, profile effect.
- AC8 Limits: canvas 16–1024 px, ≤ 12 layers, ≤ 60 frames, project ≤ 6 MB, ≤ 100 projects; server validates.

## Out of scope
Animated WebP and APNG export (no encoder available in the browser; GIF is exported instead),
collaborative drawing, pressure-sensitive brushes, free-form selection.
