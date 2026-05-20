#!/usr/bin/env bash
# Build the engine-wasm crate as an npm-consumable package.
#
# Output: engine-wasm/pkg/
#   ├── engine_wasm.js        — JS loader
#   ├── engine_wasm_bg.wasm   — the compiled module
#   ├── engine_wasm.d.ts      — TS types (incl. tsify-derived DTOs)
#   └── package.json
#
# Usage from the visualizer:
#   npm install ../engine-wasm/pkg
#
# Targets:
#   --target web      ES module suitable for direct <script type="module">
#                     and bundlers like Vite. (default here)
#   --target bundler  Webpack-style — use this if the visualizer is Next.js.
#   --target nodejs   For server-side use of the same artifact.
set -euo pipefail

cd "$(dirname "$0")"

if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "wasm-pack not found; install it with:"
    echo "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
fi

TARGET="${1:-web}"
PROFILE="${2:-release}"

case "$PROFILE" in
    release) PROFILE_FLAG="--release" ;;
    dev)     PROFILE_FLAG="--dev" ;;
    *)       echo "Unknown profile '$PROFILE' (expected release|dev)"; exit 1 ;;
esac

wasm-pack build --target "$TARGET" "$PROFILE_FLAG"
echo
echo "Built engine-wasm/pkg/ for target=$TARGET, profile=$PROFILE."
echo "Consume from the visualizer with: npm install ../engine-wasm/pkg"
