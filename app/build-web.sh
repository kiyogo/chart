#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${1:-debug}"

cd "$ROOT_DIR"
if [ "$PROFILE" = "release" ]; then
  cargo build -p chatw-app --target wasm32-unknown-unknown --features web --release
  WASM_PATH="target/wasm32-unknown-unknown/release/chatw-app.wasm"
else
  cargo build -p chatw-app --target wasm32-unknown-unknown --features web
  WASM_PATH="target/wasm32-unknown-unknown/debug/chatw-app.wasm"
fi

rm -rf app/dist
mkdir -p app/dist/pkg
wasm-bindgen --target web --out-dir app/dist/pkg --out-name chatw_app "$WASM_PATH"
TS="$(date +%s)"
sed "s|/boot.js|/boot.js?v=${TS}|g" app/web/index.html > app/dist/index.html
sed -i "s|/i18n_runtime.js|/i18n_runtime.js?v=${TS}|g" app/dist/index.html
sed -i "s|/canvas_scrollbars.js|/canvas_scrollbars.js?v=${TS}|g" app/dist/index.html
sed -i "s|/panel_resizer.js|/panel_resizer.js?v=${TS}|g" app/dist/index.html
sed -i "s|/custom_pickers.js|/custom_pickers.js?v=${TS}|g" app/dist/index.html
sed -i "s|/living_motion.js|/living_motion.js?v=${TS}|g" app/dist/index.html
sed -i "s|/adaptive_surface.js|/adaptive_surface.js?v=${TS}|g" app/dist/index.html
sed -i "s|/storage_engine.js|/storage_engine.js?v=${TS}|g" app/dist/index.html
sed -i "s|/core_bridge.js|/core_bridge.js?v=${TS}|g" app/dist/index.html
sed -i "s|/media_stream_bridge.js|/media_stream_bridge.js?v=${TS}|g" app/dist/index.html
sed -i "s|/media_optimizer.js|/media_optimizer.js?v=${TS}|g" app/dist/index.html
sed -i "s|/plugin_runtime.js|/plugin_runtime.js?v=${TS}|g" app/dist/index.html
cp app/web/i18n_runtime.js app/dist/i18n_runtime.js
mkdir -p app/dist/locales
cp -r app/web/locales/. app/dist/locales/
cp app/web/canvas_scrollbars.js app/dist/canvas_scrollbars.js
cp app/web/panel_resizer.js app/dist/panel_resizer.js
cp app/web/custom_pickers.js app/dist/custom_pickers.js
cp app/web/living_motion.js app/dist/living_motion.js
cp app/web/adaptive_surface.js app/dist/adaptive_surface.js
cp app/web/storage_engine.js app/dist/storage_engine.js
cp app/web/core_worker.js app/dist/core_worker.js
sed "s|/core_worker.js|/core_worker.js?v=${TS}|g" app/web/core_bridge.js > app/dist/core_bridge.js
cp app/web/media_worker.js app/dist/media_worker.js
cp app/web/network_worker.js app/dist/network_worker.js
cp app/web/network_bridge.js app/dist/network_bridge.js
cp app/web/sw.js app/dist/sw.js
cp app/web/media_stream_bridge.js app/dist/media_stream_bridge.js
cp app/web/media_optimizer.js app/dist/media_optimizer.js
cp app/web/plugin_runtime.js app/dist/plugin_runtime.js
sed "s|/pkg/chatw_app.js|/pkg/chatw_app.js?v=${TS}|g" app/web/boot.js > app/dist/boot.js
sed -i "s|chatw_app_bg.wasm|chatw_app_bg.wasm?v=${TS}|g" app/dist/pkg/chatw_app.js
cp -r app/public/. app/dist/
echo "Built Dioxus web bundle (v=${TS}) -> app/dist/"


