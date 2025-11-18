import {
  Parser,
  Language,
} from "https://cdn.jsdelivr.net/npm/web-tree-sitter@0.25.10/tree-sitter.js";
import init, { PlaygroundEngine } from "/pkg/playground_worker.js";
import {
  NODE_SCOPE_MAP,
  DEFAULT_SCOPE,
  MelbiTokenState,
  COMPLETION_KIND_MAP,
  computeNewEndPosition,
  buildTokensFromTree,
  collectSyntaxDiagnostics,
  mapCompletionItem,
  spanToRange,
  applyEditsToTree,
} from "./utils.js";

const TREE_SITTER_WASM_URL = "/pkg/tree-sitter-melbi.wasm";
const LANGUAGE_CONFIG_URL = "/language-configuration.json";
const DEFAULT_SOURCE = "1 + 1";
const MARKER_OWNER = "melbi-playground";
const AUTO_RUN_DEBOUNCE_MS = 250;

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
    themeToggle: null,
    timing: null,
  },
  currentTheme: "light",
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
    themeToggle: document.getElementById("theme-toggle"),
    timing: document.getElementById("timing"),
  };
}

// Status element removed - setStatus calls are no-ops

function renderResponse(payload) {
  if (!state.dom.output) {
    return;
  }
  if (payload.status === "ok") {
    const durationText =
      payload.data.duration_ms < 0.01
        ? "<0.01ms"
        : `${payload.data.duration_ms.toFixed(2)}ms`;
    state.dom.output.innerHTML = `${payload.data.value} <span class="type">${payload.data.type_name}</span>`;
    if (state.dom.timing) {
      state.dom.timing.textContent = durationText;
    }
    updateDiagnostics([]);
  } else {
    // Don't show errors in output - squiggly lines are enough
    // Just update diagnostics for the editor
    updateDiagnostics(payload.error.diagnostics || []);
  }
}

async function ensureEngine() {
  if (!state.enginePromise) {
    state.enginePromise = (async () => {
      try {
        await init();
        const instance = new PlaygroundEngine();

        // Run a warmup evaluation to JIT-compile WASM functions
        // This ensures the first user evaluation shows real performance
        try {
          await instance.evaluate("1 + 1");
        } catch (err) {
          // Ignore warmup errors
        }

        // Ready to run
        return instance;
      } catch (err) {
        console.error(err);
        console.error("Failed to initialize worker");
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
      () => resolve(window.monaco),
      reject,
    );
  });
}

async function loadLanguageConfig() {
  try {
    const response = await fetch(LANGUAGE_CONFIG_URL);
    if (!response.ok) {
      throw new Error(`Failed to load language config: ${response.statusText}`);
    }
    const config = await response.json();

    // Convert string regex patterns to RegExp objects
    // (JSON can only store strings, but Monaco expects RegExp objects)
    if (config.folding?.markers) {
      if (typeof config.folding.markers.start === 'string') {
        config.folding.markers.start = new RegExp(config.folding.markers.start);
      }
      if (typeof config.folding.markers.end === 'string') {
        config.folding.markers.end = new RegExp(config.folding.markers.end);
      }
    }
    if (config.wordPattern && typeof config.wordPattern === 'string') {
      config.wordPattern = new RegExp(config.wordPattern);
    }
    if (config.indentationRules) {
      if (typeof config.indentationRules.increaseIndentPattern === 'string') {
        config.indentationRules.increaseIndentPattern = new RegExp(config.indentationRules.increaseIndentPattern);
      }
      if (typeof config.indentationRules.decreaseIndentPattern === 'string') {
        config.indentationRules.decreaseIndentPattern = new RegExp(config.indentationRules.decreaseIndentPattern);
      }
    }

    return config;
  } catch (err) {
    console.warn("Failed to load language configuration:", err);
    return null;
  }
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

function registerLanguageProviders(monaco, languageConfig) {
  monaco.languages.register({ id: "melbi" });
  if (languageConfig) {
    monaco.languages.setLanguageConfiguration("melbi", languageConfig);
  }
  monaco.languages.setTokensProvider("melbi", createTokensProvider());
  // Theme will be set by applyTheme()

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
    console.log("[updateEditorHeight] Early return: no editor or container");
    return;
  }
  const model = state.editor.getModel();
  if (!model) {
    console.log("[updateEditorHeight] Early return: no model");
    return;
  }

  const lineCount = model.getLineCount();
  const lineHeight = state.editor.getOption(
    state.monacoApi.editor.EditorOption.lineHeight,
  );
  const padding = 16; // Top + bottom padding
  const currentHeight = state.dom.editorContainer.style.height;
  const contentHeight = state.editor.getContentHeight();

  const maxHeight = 400;
  const idealHeight = lineCount * lineHeight + padding;
  const newHeight = Math.max(
    lineHeight + padding,
    Math.min(maxHeight, idealHeight),
  );
  const isAtMaxHeight = idealHeight >= maxHeight;

  console.log("[updateEditorHeight]", {
    lineCount,
    lineHeight,
    padding,
    currentHeight,
    contentHeight,
    idealHeight,
    calculatedHeight: newHeight,
    isAtMaxHeight,
    scrollTop: state.editor.getScrollTop(),
  });

  state.dom.editorContainer.style.height = `${newHeight}px`;

  // Show scrollbar only when we've reached maximum height
  state.editor.updateOptions({
    scrollbar: {
      vertical: isAtMaxHeight ? "auto" : "hidden",
      horizontal: "auto",
      verticalScrollbarSize: 8,
      horizontalScrollbarSize: 8,
    },
  });

  // Force Monaco to layout with explicit dimensions
  const width = state.dom.editorContainer.clientWidth;
  state.editor.layout({ width, height: newHeight });

  // Reset scroll position to prevent first line from getting hidden
  // Only reset if we're not at max height (so users can scroll when needed)
  if (!isAtMaxHeight) {
    state.editor.setScrollTop(0);
  }

  console.log("[updateEditorHeight] After layout:", {
    newScrollTop: state.editor.getScrollTop(),
    containerHeight: state.dom.editorContainer.style.height,
  });
}

async function setupEditor(monaco) {
  const languageConfig = await loadLanguageConfig();
  registerLanguageProviders(monaco, languageConfig);

  // Define custom themes with distinct line numbers
  monaco.editor.defineTheme("melbi-light", {
    base: "vs",
    inherit: true,
    rules: [],
    colors: {
      "editorGutter.background": "#f1f4f5",
      "editorLineNumber.foreground": "#a0aec0",
      "editorLineNumber.activeForeground": "#2d3748",
    },
  });

  monaco.editor.defineTheme("melbi-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [],
    colors: {
      "editorGutter.background": "#1a202c",
      "editorLineNumber.foreground": "#4a5568",
      "editorLineNumber.activeForeground": "#cbd5e0",
    },
  });

  state.editor = monaco.editor.create(state.dom.editorContainer, {
    value: "",
    language: "melbi",
    minimap: { enabled: false },
    fontSize: 22,
    fontFamily:
      "'JetBrains Mono', 'Fira Code', 'SF Mono', 'Cascadia Code', 'Consolas', monospace",
    fontLigatures: true,
    theme: state.currentTheme === "dark" ? "melbi-dark" : "melbi-light",
    automaticLayout: false,
    lineNumbers: "off",
    glyphMargin: false,
    folding: false,
    renderLineHighlight: "line",
    scrollbar: {
      vertical: "hidden",
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
    console.log("[onDidChangeModelContent] Event:", event);
    updateEditorHeight();
    handleModelContentChange(event);
  });

  console.log(
    "[setupEditor] Editor created, calling initial updateEditorHeight",
  );
  updateEditorHeight();

  // Trigger initial auto-run for the default code
  scheduleAutoRun();
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
    // Syntax errors present - auto-run disabled
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
    // Syntax errors present - auto-run disabled
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
      // Evaluating...
      const payload = await engine.evaluate(state.editor.getValue());
      renderResponse(payload);
      // Evaluation complete
      return payload;
    } catch (err) {
      console.error(err);
      if (state.dom.output) {
        state.dom.output.textContent = `Evaluation failed: ${err}`;
      }
      console.error("Evaluation failed");
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
      // Set default content after parser is ready
      if (model.getValue() === "") {
        state.editor.setValue(DEFAULT_SOURCE);
      }

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

function toggleTheme() {
  state.currentTheme = state.currentTheme === "light" ? "dark" : "light";
  applyTheme();
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("melbi-theme", state.currentTheme);
  }
}

function applyTheme() {
  const isDark = state.currentTheme === "dark";

  // Update body class
  if (typeof document !== "undefined") {
    document.body.classList.toggle("dark", isDark);
  }

  // Update Monaco theme
  if (state.monacoApi && state.editor) {
    state.monacoApi.editor.setTheme(isDark ? "melbi-dark" : "melbi-light");
  }

  // Update button text
  if (state.dom.themeToggle) {
    state.dom.themeToggle.textContent = isDark ? "☀️ Light" : "🌙 Dark";
  }
}

function loadThemePreference() {
  if (typeof localStorage !== "undefined") {
    const saved = localStorage.getItem("melbi-theme");
    if (saved === "dark" || saved === "light") {
      state.currentTheme = saved;
    }
  }
  applyTheme();
}

function attachButtonHandlers() {
  if (state.dom.themeToggle && !state.dom.themeToggle.__melbiBound) {
    state.dom.themeToggle.__melbiBound = true;
    state.dom.themeToggle.addEventListener("click", toggleTheme);
  }
}

export async function initializePlayground() {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return;
  }
  state.dom = getDomRefs();
  if (!state.dom.editorContainer || !state.dom.output) {
    console.error("Playground DOM elements missing.");
    return;
  }

  // Load theme preference before setting up editor
  loadThemePreference();

  attachButtonHandlers();
  try {
    state.monacoApi = await loadMonaco();
    await setupEditor(state.monacoApi);
  } catch (err) {
    console.error("Failed to load Monaco", err);
    console.error("Failed to load code editor");
    return;
  }
  await setupParser();
  ensureEngine().catch(() => {});
}

// Export state for tutorial.js to access
if (typeof window !== "undefined") {
  window.playgroundState = state;
}

if (typeof window !== "undefined" && typeof document !== "undefined") {
  initializePlayground();
}

export {
  NODE_SCOPE_MAP,
  DEFAULT_SCOPE,
  MelbiTokenState,
  COMPLETION_KIND_MAP,
  computeNewEndPosition,
  buildTokensFromTree,
  collectSyntaxDiagnostics,
  mapCompletionItem,
  spanToRange,
  applyEditsToTree,
};
