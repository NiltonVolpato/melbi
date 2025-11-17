import {
  Parser,
  Language,
} from "https://cdn.jsdelivr.net/npm/web-tree-sitter@0.25.10/tree-sitter.js";
import init, { PlaygroundEngine } from "./pkg/playground_worker.js";
import {
  NODE_SCOPE_MAP,
  DEFAULT_SCOPE,
  TOKEN_THEME_RULES,
  MelbiTokenState,
  COMPLETION_KIND_MAP,
  computeNewEndPosition,
  buildTokensFromTree,
  collectSyntaxDiagnostics,
  mapCompletionItem,
  spanToRange,
  applyEditsToTree,
} from "./src/melbi-playground-utils.js";

const TREE_SITTER_WASM_URL = "./pkg/tree-sitter-melbi.wasm";
const DEFAULT_SOURCE = "1 + 1";
const MARKER_OWNER = "melbi-playground";
const AUTO_RUN_DEBOUNCE_MS = 750;

const state = {
  editor: null,
  monacoApi: null,
  enginePromise: null,
  parserPromise: null,
  parserInstance: null,
  currentTree: null,
  currentTokensByLine: [],
  tokenStateVersion: 0,
  lastWorkerDiagnostics: [],
  lastSyntaxDiagnostics: [],
  dom: {
    editorContainer: null,
    output: null,
    status: null,
    runButton: null,
  },
  autoRunHandle: null,
  pendingAutoRunAfterInFlight: false,
  inFlightEvaluation: null,
};

function getDomRefs() {
  if (typeof document === "undefined") {
    return state.dom;
  }
  return {
    editorContainer: document.getElementById("editor-container"),
    output: document.getElementById("output"),
    status: document.getElementById("status"),
    runButton: document.getElementById("run"),
  };
}

function setStatus(message) {
  if (state.dom.status) {
    state.dom.status.textContent = message;
  }
}

function renderResponse(payload) {
  if (!state.dom.output) {
    return;
  }
  if (payload.status === "ok") {
    state.dom.output.innerHTML = `${payload.data.value} <span class="type">${payload.data.type_name}</span>`;
    updateDiagnostics([]);
  } else {
    const diagnostics = payload.error.diagnostics
      ?.map((diag) => {
        const spanText = diag.span
          ? ` [${diag.span.start}, ${diag.span.end}]`
          : "";
        return `${diag.severity}: ${diag.message}${spanText}`;
      })
      .join("\n");
    state.dom.output.textContent =
      `Error (${payload.error.kind}): ${payload.error.message}` +
      (diagnostics ? `\n${diagnostics}` : "");
    updateDiagnostics(payload.error.diagnostics || []);
  }
}

async function ensureEngine() {
  if (!state.enginePromise) {
    state.enginePromise = (async () => {
      try {
        await init();
        const instance = new PlaygroundEngine();
        setStatus("Ready to run Melbi snippets.");
        if (state.dom.runButton) {
          state.dom.runButton.disabled = false;
        }
        return instance;
      } catch (err) {
        console.error(err);
        setStatus("Failed to initialize worker. See console for details.");
        if (state.dom.runButton) {
          state.dom.runButton.disabled = true;
        }
        throw err;
      }
    })();
  }
  return state.enginePromise;
}

async function ensureParser() {
  if (!state.parserPromise) {
    state.parserPromise = (async () => {
      await Parser.init({
        locateFile(scriptName, scriptDirectory) {
          return `https://cdn.jsdelivr.net/npm/web-tree-sitter@0.25.10/${scriptName}`;
        },
      });
      const language = await Language.load(TREE_SITTER_WASM_URL);
      const parser = new Parser();
      parser.setLanguage(language);
      return parser;
    })();
  }
  return state.parserPromise;
}

function loadMonaco() {
  return new Promise((resolve, reject) => {
    if (window.monaco) {
      resolve(window.monaco);
      return;
    }
    if (!window.require) {
      reject(new Error("Monaco loader missing."));
      return;
    }
    window.require(
      ["vs/editor/editor.main"],
      (monaco) => resolve(monaco),
      reject,
    );
  });
}

function updateDiagnostics(workerDiagnostics) {
  if (Array.isArray(workerDiagnostics)) {
    state.lastWorkerDiagnostics = workerDiagnostics;
  } else if (workerDiagnostics === null) {
    state.lastWorkerDiagnostics = [];
  }
  if (!state.monacoApi || !state.editor) {
    return;
  }
  const model = state.editor.getModel();
  if (!model) {
    return;
  }
  const combinedDiagnostics = [
    ...state.lastSyntaxDiagnostics,
    ...state.lastWorkerDiagnostics,
  ];
  const markers = combinedDiagnostics.map((diag) => {
    const range = spanToRange(model, diag.span);
    const severity = (diag.severity || "").toLowerCase();
    const markerSeverity =
      severity === "error"
        ? state.monacoApi.MarkerSeverity.Error
        : severity === "warning"
          ? state.monacoApi.MarkerSeverity.Warning
          : state.monacoApi.MarkerSeverity.Info;
    return {
      ...range,
      message: diag.message,
      severity: markerSeverity,
      code: diag.code,
      source: diag.source || "melbi",
    };
  });
  state.monacoApi.editor.setModelMarkers(model, MARKER_OWNER, markers);
}

async function getHoverFromWorker(model, position) {
  const offset = model.getOffsetAt(position);
  const response = await callWorkerMethod(
    ["hover_at_position", "hover_at", "hover"],
    model.getValue(),
    offset,
  );
  if (!response || response.status !== "ok") {
    return null;
  }
  const contents =
    response.data?.contents || response.data?.text || response.data?.value;
  if (!contents) {
    return null;
  }
  const range = response.data?.span
    ? spanToRange(model, response.data.span)
    : null;
  return {
    contents: [{ value: contents }],
    range,
  };
}

async function getCompletionsFromWorker(model, position) {
  const offset = model.getOffsetAt(position);
  const response = await callWorkerMethod(
    [
      "completions_at_position",
      "completions_at",
      "completion_items",
      "complete",
    ],
    model.getValue(),
    offset,
  );
  if (!response || response.status !== "ok") {
    return [];
  }
  const suggestions = response.data?.items || response.data?.suggestions || [];
  return suggestions;
}

async function callWorkerMethod(methodNames, ...args) {
  const engine = await ensureEngine();
  const names = Array.isArray(methodNames) ? methodNames : [methodNames];
  for (const name of names) {
    const fn = engine?.[name];
    if (typeof fn === "function") {
      try {
        return await fn.apply(engine, args);
      } catch (err) {
        console.error(`Worker method ${name} failed`, err);
        return null;
      }
    }
  }
  return null;
}

function registerLanguageProviders(monaco) {
  monaco.languages.register({ id: "melbi" });
  monaco.languages.setTokensProvider("melbi", createTokensProvider());
  monaco.editor.defineTheme("melbi-light", {
    base: "vs",
    inherit: true,
    rules: TOKEN_THEME_RULES,
    colors: {
      "editor.background": "#f8f9fa",
      "editor.lineHighlightBackground": "#edf2f7",
    },
  });
  monaco.editor.setTheme("melbi-light");

  monaco.languages.registerHoverProvider("melbi", {
    provideHover: async (model, position) => {
      try {
        return await getHoverFromWorker(model, position);
      } catch (err) {
        console.error("Hover provider failed", err);
        return null;
      }
    },
  });

  monaco.languages.registerCompletionItemProvider("melbi", {
    triggerCharacters: [" ", ".", ":", "("],
    provideCompletionItems: async (model, position) => {
      try {
        const workerItems = await getCompletionsFromWorker(model, position);
        const suggestions = workerItems.map((item) =>
          mapCompletionItem(monaco, model, position, item, COMPLETION_KIND_MAP),
        );
        return { suggestions };
      } catch (err) {
        console.error("Completion provider failed", err);
        return { suggestions: [] };
      }
    },
  });
}

function createTokensProvider() {
  return {
    getInitialState: () => new MelbiTokenState(0, state.tokenStateVersion),
    tokenize: (_line, tokenState) => {
      const lineIndex = tokenState.lineNumber;
      const lineTokens = state.currentTokensByLine[lineIndex] || [];
      return {
        tokens: lineTokens.map((token) => ({
          startIndex: token.startIndex,
          scopes: token.scopes,
        })),
        endState: new MelbiTokenState(lineIndex + 1, state.tokenStateVersion),
      };
    },
  };
}

function updateEditorHeight() {
  if (!state.editor || !state.dom.editorContainer) {
    return;
  }
  const model = state.editor.getModel();
  if (!model) {
    return;
  }
  const lineCount = model.getLineCount();
  const lineHeight = 21;
  const padding = 8;
  const newHeight = Math.max(42, Math.min(400, lineCount * lineHeight + padding * 2));
  state.dom.editorContainer.style.height = `${newHeight}px`;
  state.editor.layout();
}

function setupEditor(monaco) {
  registerLanguageProviders(monaco);
  state.editor = monaco.editor.create(state.dom.editorContainer, {
    value: DEFAULT_SOURCE,
    language: "melbi",
    minimap: { enabled: false },
    fontSize: 15,
    theme: "melbi-light",
    automaticLayout: false,
    lineNumbers: "on",
    glyphMargin: false,
    folding: false,
    renderLineHighlight: "line",
    scrollbar: {
      vertical: "auto",
      horizontal: "auto",
      verticalScrollbarSize: 8,
      horizontalScrollbarSize: 8,
    },
    overviewRulerLanes: 0,
    hideCursorInOverviewRuler: true,
    scrollBeyondLastLine: false,
    wordWrap: "on",
    padding: {
      top: 8,
      bottom: 8,
    },
  });
  state.editor.onDidChangeModelContent((event) => {
    handleModelContentChange(event);
    updateEditorHeight();
  });
  updateEditorHeight();
}

function handleModelContentChange(event) {
  if (!state.parserInstance || !state.editor) {
    updateDiagnostics();
    scheduleAutoRun();
    return;
  }
  const model = state.editor.getModel();
  if (!model) {
    return;
  }
  if (state.currentTree && event?.changes?.length) {
    applyEditsToTree(state.currentTree, event.changes, computeNewEndPosition);
  }
  const previousTree = state.currentTree;
  state.currentTree = state.parserInstance.parse(
    model.getValue(),
    state.currentTree,
  );
  if (previousTree) {
    previousTree.delete();
  }
  updateSyntaxArtifacts(model);
  scheduleAutoRun();
}

function updateSyntaxArtifacts(model) {
  if (!model) {
    state.lastSyntaxDiagnostics = [];
    state.currentTokensByLine = [];
    updateDiagnostics();
    return;
  }
  state.currentTokensByLine = buildTokensFromTree(
    state.currentTree,
    model,
    NODE_SCOPE_MAP,
    DEFAULT_SCOPE,
  );
  refreshTokensForModel(model);
  state.lastSyntaxDiagnostics = collectSyntaxDiagnostics(
    state.currentTree,
    model,
  );
  updateDiagnostics();
}

function hasBlockingSyntaxErrors() {
  return state.lastSyntaxDiagnostics.some(
    (diag) => (diag?.severity || "error").toLowerCase() === "error",
  );
}

function cancelScheduledAutoRun() {
  if (state.autoRunHandle) {
    clearTimeout(state.autoRunHandle);
    state.autoRunHandle = null;
  }
}

function scheduleAutoRun() {
  if (!state.editor) {
    return;
  }
  if (state.autoRunHandle) {
    clearTimeout(state.autoRunHandle);
  }
  state.autoRunHandle = globalThis.setTimeout(() => {
    state.autoRunHandle = null;
    attemptAutoRun();
  }, AUTO_RUN_DEBOUNCE_MS);
}

function attemptAutoRun() {
  if (state.inFlightEvaluation) {
    state.pendingAutoRunAfterInFlight = true;
    return;
  }
  if (hasBlockingSyntaxErrors()) {
    setStatus("Fix syntax errors to run automatically.");
    return;
  }
  runEvaluation({ reason: "auto", skipIfSyntaxErrors: true }).catch((err) => {
    console.warn("Auto-run evaluation failed.", err);
  });
}

async function runEvaluation({
  reason = "manual",
  skipIfSyntaxErrors = false,
} = {}) {
  if (!state.editor) {
    return null;
  }
  if (reason === "manual") {
    cancelScheduledAutoRun();
    state.pendingAutoRunAfterInFlight = false;
  }
  if (skipIfSyntaxErrors && hasBlockingSyntaxErrors()) {
    setStatus("Fix syntax errors to run automatically.");
    return null;
  }
  if (state.inFlightEvaluation) {
    if (reason === "manual") {
      try {
        await state.inFlightEvaluation;
      } catch (err) {
        console.error("Previous evaluation failed", err);
      }
    } else {
      state.pendingAutoRunAfterInFlight = true;
      return state.inFlightEvaluation;
    }
  }
  const evaluationPromise = (async () => {
    const statusLabel = reason === "auto" ? "Auto-running…" : "Evaluating…";
    try {
      const engine = await ensureEngine();
      setStatus(statusLabel);
      const payload = await engine.evaluate(state.editor.getValue());
      renderResponse(payload);
      setStatus("Evaluation finished.");
      return payload;
    } catch (err) {
      console.error(err);
      if (state.dom.output) {
        state.dom.output.textContent = `Evaluation failed: ${err}`;
      }
      setStatus("Evaluation failed.");
      throw err;
    }
  })();
  state.inFlightEvaluation = evaluationPromise;
  try {
    return await evaluationPromise;
  } finally {
    if (state.inFlightEvaluation === evaluationPromise) {
      state.inFlightEvaluation = null;
    }
    if (state.pendingAutoRunAfterInFlight) {
      state.pendingAutoRunAfterInFlight = false;
      attemptAutoRun();
    }
  }
}

function refreshTokensForModel(model) {
  state.tokenStateVersion += 1;
  if (typeof model?.forceTokenization === "function") {
    model.forceTokenization(model.getLineCount());
  }
}

async function setupParser() {
  try {
    state.parserInstance = await ensureParser();
    const model = state.editor?.getModel();
    if (state.parserInstance && model) {
      const previousTree = state.currentTree;
      state.currentTree = state.parserInstance.parse(model.getValue());
      if (previousTree) {
        previousTree.delete();
      }
      updateSyntaxArtifacts(model);
    }
  } catch (err) {
    console.error("Failed to initialize Tree-sitter", err);
  }
}

function attachButtonHandlers() {
  // Run button removed - auto-run handles everything
}

export async function initializePlayground() {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return;
  }
  state.dom = getDomRefs();
  if (!state.dom.editorContainer || !state.dom.output || !state.dom.status) {
    console.error("Playground DOM elements missing.");
    return;
  }
  attachButtonHandlers();
  try {
    state.monacoApi = await loadMonaco();
    setupEditor(state.monacoApi);
  } catch (err) {
    console.error("Failed to load Monaco", err);
    setStatus("Failed to load code editor.");
    return;
  }
  await setupParser();
  ensureEngine().catch(() => {});
}

if (typeof window !== "undefined" && typeof document !== "undefined") {
  initializePlayground();
}

export {
  NODE_SCOPE_MAP,
  DEFAULT_SCOPE,
  TOKEN_THEME_RULES,
  MelbiTokenState,
  COMPLETION_KIND_MAP,
  computeNewEndPosition,
  buildTokensFromTree,
  collectSyntaxDiagnostics,
  mapCompletionItem,
  spanToRange,
  applyEditsToTree,
};
