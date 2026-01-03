#!/usr/bin/env bash
#
# Build skia-rs for WebAssembly
#
# Usage:
#   ./scripts/build-wasm.sh          # Build for web
#   ./scripts/build-wasm.sh --nodejs # Build for Node.js
#   ./scripts/build-wasm.sh --all    # Build for both
#

set -euo pipefail

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT_DIR="$WORKSPACE_ROOT/pkg"

# Parse arguments
TARGET="web"
if [[ "${1:-}" == "--nodejs" ]]; then
    TARGET="nodejs"
elif [[ "${1:-}" == "--all" ]]; then
    TARGET="all"
fi

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║            skia-rs WebAssembly Build Script                ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check for wasm-pack
if ! command -v wasm-pack &> /dev/null; then
    echo -e "${YELLOW}Installing wasm-pack...${NC}"
    cargo install wasm-pack
fi

# Check for wasm32 target
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo -e "${YELLOW}Adding wasm32-unknown-unknown target...${NC}"
    rustup target add wasm32-unknown-unknown
fi

# Clean output
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Build function
build_wasm() {
    local target="$1"
    echo -e "${BLUE}Building for $target...${NC}"

    cd "$WORKSPACE_ROOT/crates/skia-rs-safe"

    case "$target" in
        web)
            wasm-pack build \
                --target web \
                --out-dir "$OUTPUT_DIR/web" \
                --features "std" \
                -- --features "std"
            ;;
        nodejs)
            wasm-pack build \
                --target nodejs \
                --out-dir "$OUTPUT_DIR/nodejs" \
                --features "std" \
                -- --features "std"
            ;;
        bundler)
            wasm-pack build \
                --target bundler \
                --out-dir "$OUTPUT_DIR/bundler" \
                --features "std" \
                -- --features "std"
            ;;
    esac

    echo -e "${GREEN}✓ Built for $target${NC}"
}

# Build based on target
case "$TARGET" in
    web)
        build_wasm web
        ;;
    nodejs)
        build_wasm nodejs
        ;;
    all)
        build_wasm web
        build_wasm nodejs
        build_wasm bundler
        ;;
esac

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                    Build Complete!                          ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "Output directory: ${BLUE}$OUTPUT_DIR${NC}"
echo ""
echo -e "${YELLOW}Usage in HTML:${NC}"
echo "  <script type=\"module\">"
echo "    import init, { WasmSurface } from './pkg/web/skia_rs_safe.js';"
echo "    await init();"
echo "    const surface = new WasmSurface(800, 600);"
echo "  </script>"
echo ""
echo -e "${YELLOW}Usage in Node.js:${NC}"
echo "  const { WasmSurface } = require('./pkg/nodejs/skia_rs_safe');"
echo "  const surface = new WasmSurface(800, 600);"
