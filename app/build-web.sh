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
sed "s|/pkg/chatw_app.js|/pkg/chatw_app.js?v=${TS}|g" app/web/boot.js > app/dist/boot.js
sed -i "s|chatw_app_bg.wasm|chatw_app_bg.wasm?v=${TS}|g" app/dist/pkg/chatw_app.js
cp -r app/public/. app/dist/
echo "Built Dioxus web bundle (v=${TS}) -> app/dist/"


