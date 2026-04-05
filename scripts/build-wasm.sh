#!/bin/bash
#
# Build the forge-wasm crate for use in the web frontend.
#
# This script:
# 1. Builds forge-wasm using wasm-pack
# 2. Copies the output to src/wasm for Vite to consume
#
# Prerequisites:
# - Rust toolchain with wasm32-unknown-unknown target
# - wasm-pack: cargo install wasm-pack

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WASM_CRATE="$PROJECT_ROOT/forge-engine/crates/forge-wasm"
OUTPUT_DIR="$PROJECT_ROOT/src/wasm"

echo "Building forge-wasm..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null && ! command -v "$HOME/.cargo/bin/wasm-pack" &> /dev/null; then
    echo "wasm-pack not found. Installing..."
    cargo install wasm-pack
fi

# Use wasm-pack from cargo bin if not in PATH
WASM_PACK="wasm-pack"
if ! command -v wasm-pack &> /dev/null; then
    WASM_PACK="$HOME/.cargo/bin/wasm-pack"
fi

# Build with wasm-pack (output relative to project root)
cd "$PROJECT_ROOT"
$WASM_PACK build --target web --out-dir "$OUTPUT_DIR" --out-name forge_wasm forge-engine/crates/forge-wasm

# Clean up unnecessary files
rm -f "$OUTPUT_DIR/.gitignore"
rm -f "$OUTPUT_DIR/package.json"
rm -f "$OUTPUT_DIR/README.md"

# Bundle card data and preset decks
echo ""
echo "Bundling card data..."
node "$SCRIPT_DIR/bundle-cards.mjs"

echo ""
echo "Build complete!"
echo "WASM output: $OUTPUT_DIR"
echo "Card data:   $PROJECT_ROOT/public/wasm/"
