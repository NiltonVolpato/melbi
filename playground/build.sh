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
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "📦 Step 1/3: Building WASM worker..."
cd "$PROJECT_ROOT"
wasm-pack build playground/worker --target web --out-dir ../web/pkg --release

echo ""
echo "🔧 Step 2/3: Installing dependencies (if needed)..."
cd "$PROJECT_ROOT/playground/web"
if [ ! -d "node_modules" ]; then
    npm install
else
    echo "   ✓ Dependencies already installed"
fi

echo ""
echo "🏗️  Step 3/3: Building playground with Vite..."
npm run build

echo ""
echo "✅ Build complete!"
echo ""
echo "📂 Output directory: playground/web/dist/"
echo ""
echo "To preview locally:"
echo "   cd playground/web && npm run preview"
echo ""
echo "To deploy, upload the contents of playground/web/dist/ to your hosting service."
