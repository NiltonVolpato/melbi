# Melbi Playground

Phase 0 introduces a WebAssembly worker that exposes the public Melbi API to the browser and a web-based playground for interactive testing.

## Quick Start

### Prerequisites

1. Install the WebAssembly target (one-time setup):
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. Install `wasm-pack`:
   ```bash
   cargo install wasm-pack
   ```

3. Install Node.js dependencies:
   ```bash
   cd playground
   npm install
   ```

### Development

1. Build the WASM worker:
   ```bash
   cd playground
   wasm-pack build worker --target web --out-dir ../dist/pkg --release
   cp ../tree-sitter/tree-sitter-melbi.wasm dist/pkg/
   ```

2. Start the development server:
   ```bash
   npm run dev
   ```

   This opens http://localhost:5173 with hot module reload. Changes to JavaScript files refresh instantly.

3. When you modify Rust code, rebuild the WASM and the page will auto-refresh:
   ```bash
   wasm-pack build worker --target web --out-dir ../dist/pkg --release
   ```

### Production Build

**Recommended:** Use the build script to compile WASM and build in one command:

```bash
cd playground
bash scripts/build.sh
```

Or build manually:

```bash
cd playground
npm run build
```

This creates a `dist/` directory with all assets ready to deploy to any static hosting service.

You can preview the production build locally:
```bash
npm run preview
```

## Project Structure

```
playground/
├── worker/              # Rust WASM worker source
├── src/                 # Web playground source
│   ├── *.html          # HTML pages (index, tutorial, embed)
│   ├── *.js            # JavaScript (main, tutorial, utils)
│   ├── styles/         # CSS files
│   └── tutorials/      # Tutorial markdown files
├── scripts/            # Build scripts
│   ├── build.sh        # Complete build script
│   └── build-tutorials.js
├── tests/              # Tests
├── dist/               # Build output (gitignored)
│   ├── pkg/           # WASM files
│   └── assets/        # Bundled JS/CSS
├── package.json
└── vite.config.js     # Build configuration
```

## Playground Versions

The playground has three versions:

- **index.html** - Main playground with editor and output
- **tutorial.html** - Interactive tutorial with step-by-step lessons
- **embed.html** - Minimal embeddable version

All versions are built from the same codebase and deployed together.

## Worker API

The WebAssembly module exposes the Melbi evaluation engine:

- `evaluate(source: &str)` → returns value + type or structured diagnostics

Responses follow a `{ status: "ok" | "err", ... }` envelope for structured error handling.

This crate reuses `melbi_core::api::Engine`, ensuring results match the CLI tools byte-for-byte.
