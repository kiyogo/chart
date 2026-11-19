# Tasks 010
- [x] T1 Migration 0017 `drawing_projects`; backend `drawings` (owner-only CRUD, thumbnail, PNG-only cells, limits)
- [x] T2 `studio_engine.js`: 11 tools (brush, pencil, eraser, flood fill with tolerance, line, rectangle, ellipse,
      star / polygon, text, eyedropper, move), recent colours, zoom, grid + snap, transparent background
- [x] T3 Layers (≤ 12): add, duplicate, delete, reorder, rename, hide, opacity, blend mode, merge down; 40-step undo / redo
- [x] T4 Animation: frames (≤ 60) add / duplicate / delete / move, onion skin, playback FPS 1–30 + loop,
      10 motion presets and keyframe tween (x, y, scale, rotation, opacity) with 6 easing curves
- [x] T5 Export PNG, animated GIF (Rust `gif` crate, transparency kept), project file; import image or project; local draft autosave
- [x] T6 Server projects with thumbnails; reopen keeps layers and frames
- [x] T7 Use as avatar, cover (GIF kept), sticker (pack "My Drawings", animated), custom emoji, My GIFs, decoration, nameplate, profile effect
- [x] T8 Entry points: Settings › ChatW Studio, profile editor › Tự tạo (old quick drawing dialog kept)
- [x] T9 Tests: backend document validation, GIF encoder round trip, `app/e2e/studio-check.mjs` 30/30, UI audit 313/313

## Found and fixed while testing
- Reopening a saved project replaced it with an empty 0×0 canvas (re-binding the canvases reset the document).
