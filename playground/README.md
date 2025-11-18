# Melbi Playground

Phase 0 introduces a WebAssembly worker that exposes the public Melbi API to the browser and a web-based playground for interactive testing.

## Quick Start

### Prerequisites

1. Install the WebAssembly target (one-time setup):
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. Install Node.js dependencies:
   ```bash
   cd playground/web
   npm install
   ```

### Development

1. Build the WASM worker:
   ```bash
   wasm-pack build playground/worker --target web --out-dir ../web/pkg --release
   ```

2. Start the development server:
   ```bash
   cd playground/web
   npm run dev
   ```

   This opens http://localhost:5173 with hot module reload. Changes to JavaScript files refresh instantly.

3. When you modify Rust code, rebuild the WASM and the page will auto-refresh:
   ```bash
   wasm-pack build playground/worker --target web --out-dir ../web/pkg --release
   ```

### Production Build

To build for deployment:

```bash
cd playground/web
npm run build
```

This creates a `dist/` directory with all assets ready to deploy to any static hosting service.

You can preview the production build locally:
```bash
npm run preview
```

## One-Step Build (Optional)

For convenience, use the build script to compile WASM and build in one command:

```bash
./playground/build.sh
```

## Project Structure

- `playground/worker/` - Rust WASM worker source
- `playground/web/` - Frontend playground
  - `index.html` - Main HTML page
  - `main.js` - Playground application
  - `pkg/` - Generated WASM output (gitignored)
  - `dist/` - Vite build output (gitignored)
  - `vite.config.js` - Build configuration

## Worker surface area

The WebAssembly module currently exposes two entry points:

- `evaluate(source: &str)` &rarr; returns value + type or structured diagnostics.
- `format_source(source: &str)` &rarr; returns formatted code or formatter errors.

Both responses follow a `{ status: "ok" | "err", ... }` envelope so the UI can add richer features without changing the bindings.

This crate reuses `melbi_core::api::Engine` and `melbi_fmt::format`, ensuring results match the CLI tools byte-for-byte.
