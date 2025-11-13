# Melbi Playground

Phase 2 brings persistence, collaboration, and polish to the Melbi playground. It still relies on the WebAssembly worker from Pha
se 0 for evaluation/formatting, but the browser shell now layers richer tooling on top.

## Building the worker

1. Install the WebAssembly target once (requires network access):

   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. Build the worker bundle into the web folder using `wasm-pack`:

   ```bash
   wasm-pack build playground/worker --target web --out-dir playground/web/pkg --release
   ```

   The generated `pkg/` directory is ignored by Git and is what the static page expects.

## Running the playground shell

Open `playground/web/index.html` with any static file server (or your browser's `file://` mode once the bundle exists). The page
 loads the worker, provides a textarea for Melbi snippets, and exposes **Run**, **Format**, **Share**, and **Copy Issue Template
** buttons wired directly to the worker bindings and collaboration helpers.

## Collaboration & polish features

Phase 2 adds a few niceties on top of the original shell:

- **Shareable snippets:** The **Share** button encodes the current snippet directly into the URL fragment so it can be sent to co
lleagues without any backend services. Opening a link with a `#snippet=...` fragment hydrates the editor immediately.
- **Analytics log:** The UI now records evaluations, formatting runs, and share attempts inside a local analytics stream to help
 debug user experiences. These events never leave the browser but make it easy to reason about what happened in a session.
- **Issue template helper:** Clicking **Copy Issue Template** places a prefilled Markdown template on your clipboard with the cur
rent snippet, last error diagnostics, and local analytics log so filing GitHub issues is painless.

Because the entire experience remains static assets + WASM, you can deploy this folder directly to any static host.

## Keeping the branch up to date

Phase 2 now lives on the main Melbi branch, so follow-up work should never reintroduce the original collaboration diff. Use the
`scripts/update-branch.sh` helper whenever you need to sync with upstream:

```bash
scripts/update-branch.sh
```

The script tries to fetch `main` from `https://github.com/NiltonVolpato/melbi` and rebases your current branch on top of it. In
sandboxed environments where GitHub access is blocked, the helper prints a clear error so you can re-run it from a machine with
network access before copying the refreshed tree back into the workspace.
