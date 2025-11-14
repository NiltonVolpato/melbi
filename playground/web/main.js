import Parser from 'https://cdn.jsdelivr.net/npm/web-tree-sitter@0.20.8/dist/tree-sitter.js';
import init, { PlaygroundEngine } from './pkg/playground_worker.js';
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
} from './src/melbi-playground-utils.js';

const TREE_SITTER_WASM_URL = './pkg/melbi.wasm';
const DEFAULT_SOURCE = '1 + 1';
const MARKER_OWNER = 'melbi-playground';

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
    formatButton: null,
  },
};

function getDomRefs() {
  if (typeof document === 'undefined') {
    return state.dom;
  }
  return {
    editorContainer: document.getElementById('editor-container'),
    output: document.getElementById('output'),
    status: document.getElementById('status'),
    runButton: document.getElementById('run'),
    formatButton: document.getElementById('format'),
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
  if (payload.status === 'ok') {
    if (payload.data.value !== undefined) {
      state.dom.output.textContent = `Result: ${payload.data.value}\nType: ${payload.data.type_name}`;
    } else if (payload.data.formatted !== undefined) {
      state.editor?.setValue(payload.data.formatted);
      state.dom.output.textContent = 'Source formatted successfully.';
    }
    updateDiagnostics([]);
  } else {
    const diagnostics = payload.error.diagnostics
      ?.map((diag) => {
        const spanText = diag.span ? ` [${diag.span.start}, ${diag.span.end}]` : '';
        return `${diag.severity}: ${diag.message}${spanText}`;
      })
      .join('\n');
    state.dom.output.textContent = `Error (${payload.error.kind}): ${payload.error.message}` +
      (diagnostics ? `\n${diagnostics}` : '');
    updateDiagnostics(payload.error.diagnostics || []);
  }
}

async function ensureEngine() {
  if (!state.enginePromise) {
    state.enginePromise = (async () => {
      try {
        await init();
        const instance = new PlaygroundEngine();
        setStatus('Ready to run Melbi snippets.');
        if (state.dom.runButton) {
          state.dom.runButton.disabled = false;
        }
        if (state.dom.formatButton) {
          state.dom.formatButton.disabled = false;
        }
        return instance;
      } catch (err) {
        console.error(err);
        setStatus('Failed to initialize worker. See console for details.');
        if (state.dom.runButton) {
          state.dom.runButton.disabled = true;
        }
        if (state.dom.formatButton) {
          state.dom.formatButton.disabled = true;
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
      await Parser.init();
      const language = await Parser.Language.load(TREE_SITTER_WASM_URL);
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
      reject(new Error('Monaco loader missing.'));
      return;
    }
    window.require(['vs/editor/editor.main'], (monaco) => resolve(monaco), reject);
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
  const combinedDiagnostics = [...state.lastSyntaxDiagnostics, ...state.lastWorkerDiagnostics];
  const markers = combinedDiagnostics.map((diag) => {
    const range = spanToRange(model, diag.span);
    const severity = (diag.severity || '').toLowerCase();
    const markerSeverity =
      severity === 'error'
        ? state.monacoApi.MarkerSeverity.Error
        : severity === 'warning'
          ? state.monacoApi.MarkerSeverity.Warning
          : state.monacoApi.MarkerSeverity.Info;
    return {
      ...range,
      message: diag.message,
      severity: markerSeverity,
      code: diag.code,
      source: diag.source || 'melbi',
    };
  });
  state.monacoApi.editor.setModelMarkers(model, MARKER_OWNER, markers);
}

async function getHoverFromWorker(model, position) {
  const offset = model.getOffsetAt(position);
  const response = await callWorkerMethod(
    ['hover_at_position', 'hover_at', 'hover'],
    model.getValue(),
    offset,
  );
  if (!response || response.status !== 'ok') {
    return null;
  }
  const contents = response.data?.contents || response.data?.text || response.data?.value;
  if (!contents) {
    return null;
  }
  const range = response.data?.span ? spanToRange(model, response.data.span) : null;
  return {
    contents: [{ value: contents }],
    range,
  };
}

async function getCompletionsFromWorker(model, position) {
  const offset = model.getOffsetAt(position);
  const response = await callWorkerMethod(
    ['completions_at_position', 'completions_at', 'completion_items', 'complete'],
    model.getValue(),
    offset,
  );
  if (!response || response.status !== 'ok') {
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
    if (typeof fn === 'function') {
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
  monaco.languages.register({ id: 'melbi' });
  monaco.languages.setTokensProvider('melbi', createTokensProvider());
  monaco.editor.defineTheme('melbi-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: TOKEN_THEME_RULES,
    colors: {
      'editor.background': '#050816',
    },
  });
  monaco.editor.setTheme('melbi-dark');

  monaco.languages.registerHoverProvider('melbi', {
    provideHover: async (model, position) => {
      try {
        return await getHoverFromWorker(model, position);
      } catch (err) {
        console.error('Hover provider failed', err);
        return null;
      }
    },
  });

  monaco.languages.registerCompletionItemProvider('melbi', {
    triggerCharacters: [' ', '.', ':', '('],
    provideCompletionItems: async (model, position) => {
      try {
        const workerItems = await getCompletionsFromWorker(model, position);
        const suggestions = workerItems.map((item) =>
          mapCompletionItem(monaco, model, position, item, COMPLETION_KIND_MAP),
        );
        return { suggestions };
      } catch (err) {
        console.error('Completion provider failed', err);
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

function setupEditor(monaco) {
  registerLanguageProviders(monaco);
  state.editor = monaco.editor.create(state.dom.editorContainer, {
    value: DEFAULT_SOURCE,
    language: 'melbi',
    minimap: { enabled: false },
    fontSize: 15,
    theme: 'melbi-dark',
    automaticLayout: true,
    renderWhitespace: 'none',
    scrollbar: { vertical: 'hidden' },
  });
  state.editor.onDidChangeModelContent((event) => {
    handleModelContentChange(event);
  });
}

function handleModelContentChange(event) {
  if (!state.parserInstance || !state.editor) {
    updateDiagnostics();
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
  state.currentTree = state.parserInstance.parse(model.getValue(), state.currentTree);
  if (previousTree) {
    previousTree.delete();
  }
  updateSyntaxArtifacts(model);
}

function updateSyntaxArtifacts(model) {
  if (!model) {
    state.lastSyntaxDiagnostics = [];
    state.currentTokensByLine = [];
    updateDiagnostics();
    return;
  }
  state.currentTokensByLine = buildTokensFromTree(state.currentTree, model, NODE_SCOPE_MAP, DEFAULT_SCOPE);
  refreshTokensForModel(model);
  state.lastSyntaxDiagnostics = collectSyntaxDiagnostics(state.currentTree, model);
  updateDiagnostics();
}

function refreshTokensForModel(model) {
  state.tokenStateVersion += 1;
  if (typeof model?.forceTokenization === 'function') {
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
    console.error('Failed to initialize Tree-sitter', err);
  }
}

function attachButtonHandlers() {
  if (state.dom.runButton && !state.dom.runButton.__melbiBound) {
    state.dom.runButton.__melbiBound = true;
    state.dom.runButton.addEventListener('click', async () => {
      const engine = await ensureEngine().catch(() => null);
      if (!engine || !state.editor) {
        return;
      }
      state.dom.runButton.disabled = true;
      setStatus('Evaluating…');
      try {
        const payload = await engine.evaluate(state.editor.getValue());
        renderResponse(payload);
        setStatus('Evaluation finished.');
      } catch (err) {
        console.error(err);
        if (state.dom.output) {
          state.dom.output.textContent = `Evaluation failed: ${err}`;
        }
        setStatus('Evaluation failed.');
      } finally {
        state.dom.runButton.disabled = false;
      }
    });
  }

  if (state.dom.formatButton && !state.dom.formatButton.__melbiBound) {
    state.dom.formatButton.__melbiBound = true;
    state.dom.formatButton.addEventListener('click', async () => {
      const engine = await ensureEngine().catch(() => null);
      if (!engine || !state.editor) {
        return;
      }
      state.dom.formatButton.disabled = true;
      setStatus('Formatting…');
      try {
        const payload = await engine.format_source(state.editor.getValue());
        renderResponse(payload);
        setStatus('Formatting finished.');
      } catch (err) {
        console.error(err);
        if (state.dom.output) {
          state.dom.output.textContent = `Format failed: ${err}`;
        }
        setStatus('Format failed.');
      } finally {
        state.dom.formatButton.disabled = false;
      }
    });
  }
}

export async function initializePlayground() {
  if (typeof window === 'undefined' || typeof document === 'undefined') {
    return;
  }
  state.dom = getDomRefs();
  if (!state.dom.editorContainer || !state.dom.output || !state.dom.status) {
    console.error('Playground DOM elements missing.');
    return;
  }
  attachButtonHandlers();
  try {
    state.monacoApi = await loadMonaco();
    setupEditor(state.monacoApi);
  } catch (err) {
    console.error('Failed to load Monaco', err);
    setStatus('Failed to load code editor.');
    return;
  }
  await setupParser();
  ensureEngine().catch(() => {});
}

if (typeof window !== 'undefined' && typeof document !== 'undefined') {
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
