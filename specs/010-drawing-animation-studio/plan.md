# Plan 010
- Migration 0017 `drawing_projects` (owner, name, kind, width, height, frame_count, layer_count, data BYTEA,
  thumb BYTEA, timestamps; size checks). Backend module `drawings`: list / create / get / update / delete /
  thumbnail, owner-only, structure + size validation, PNG-only cells and thumbnails.
- `app/src/components/studio_engine.js` (included with `include_str!`): document model (layers × frames of
  canvases), tools, history, onion skin, playback, tween, export helpers. Runs the same in web and desktop.
- `app/src/services/gif_export.rs`: animated GIF encoder with the `gif` crate (NeuQuant palette, transparency).
- `app/src/services/drawings.rs`: server API. `app/src/components/studio.rs`: full-screen Studio UI.
- `draw_studio.rs` keeps its `DrawStudio` API (profile editor) and now opens the Studio with a preset target.
- Entry points: Settings › Studio sáng tạo, profile editor › Tự tạo, Library.
