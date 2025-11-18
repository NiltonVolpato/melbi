#!/usr/bin/env bash
set -euo pipefail

echo "🖖 Building Melbi Playground..."
echo ""

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack not found. Install it with:"
    echo "   cargo install wasm-pack"
    exit 1
fi

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo "❌ npm not found. Please install Node.js"
    exit 1
fi

# Get the directory where this script lives
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLAYGROUND_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DIST_DIR="$PLAYGROUND_DIR/dist"

echo "📦 Step 1/4: Cleaning dist directory..."
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

echo ""
echo "📦 Step 2/4: Building WASM worker..."
cd "$PLAYGROUND_DIR"
wasm-pack build worker --target web --out-dir ../dist/pkg --release

# Copy tree-sitter WASM
cp ../tree-sitter/tree-sitter-melbi.wasm dist/pkg/

echo ""
echo "🔧 Step 3/4: Installing dependencies (if needed)..."
cd "$PLAYGROUND_DIR"
if [ ! -d "node_modules" ]; then
    npm install
else
    echo "   ✓ Dependencies already installed"
fi

echo ""
echo "🏗️  Step 4/4: Building playground with Vite..."
npm run build

echo ""
echo "✅ Build complete!"
echo ""
echo "📂 Output directory: $DIST_DIR"
echo ""
echo "To preview locally:"
echo "   cd $PLAYGROUND_DIR && npm run preview"
echo ""
echo "To deploy, upload the contents of $DIST_DIR to your hosting service."
