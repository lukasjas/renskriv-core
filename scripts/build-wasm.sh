#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Bygger WASM..."
cd "$PROJECT_ROOT"
wasm-pack build crates/renskriv-wasm \
    --target web \
    --out-dir "$PROJECT_ROOT/extension/src/wasm" \
    --out-name renskriv_wasm

# Fjern unodvendige filer generert av wasm-pack
rm -f "$PROJECT_ROOT/extension/src/wasm/.gitignore"
rm -f "$PROJECT_ROOT/extension/src/wasm/package.json"

echo "WASM-bygg ferdig. Output i extension/src/wasm/"
ls -lh "$PROJECT_ROOT/extension/src/wasm/"
