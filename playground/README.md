# Melbi Playground

Phase 0 introduces a WebAssembly worker that exposes the public Melbi API to the browser and a static HTML shell for quick testing.

## Building the worker

1. Install the WebAssembly target once (requires network access):

   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. Build the worker bundle into the web folder using `wasm-pack`:

   ```bash
   wasm-pack build playground/worker --target web --out-dir ../web/pkg --release
   ```

   The generated `pkg/` directory is ignored by Git and is what the static page expects.

## Running the playground shell

Open `playground/web/index.html` with any static file server (or your browser's `file://` mode once the bundle exists). The page loads the worker, provides a textarea for Melbi snippets, and exposes **Run** and **Format** buttons wired directly to the worker bindings.

## Worker surface area

The WebAssembly module currently exposes two entry points:

- `evaluate(source: &str)` &rarr; returns value + type or structured diagnostics.
- `format_source(source: &str)` &rarr; returns formatted code or formatter errors.

Both responses follow a `{ status: "ok" | "err", ... }` envelope so the UI can add richer features without changing the bindings.

This crate reuses `melbi_core::api::Engine` and `melbi_fmt::format`, ensuring results match the CLI tools byte-for-byte.
