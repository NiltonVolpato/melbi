export const NODE_SCOPE_MAP = {
  comment: 'comment.line.melbi',
  boolean: 'constant.language.boolean.melbi',
  integer: 'constant.numeric.integer.melbi',
  float: 'constant.numeric.float.melbi',
  string: 'string.quoted.double.melbi',
  bytes: 'string.quoted.double.bytes.melbi',
  format_string: 'string.quoted.double.format.melbi',
  identifier: 'variable.other.melbi',
  quoted_identifier: 'variable.other.quoted.melbi',
  unquoted_identifier: 'variable.other.melbi',
  type_path: 'entity.name.type.melbi',
};

export const DEFAULT_SCOPE = 'source.melbi';

export const TOKEN_THEME_RULES = [
  { token: DEFAULT_SCOPE, foreground: '1a202c' },
  { token: 'comment.line.melbi', foreground: '718096', fontStyle: 'italic' },
  { token: 'constant.language.boolean.melbi', foreground: '0066cc', fontStyle: 'bold' },
  { token: 'constant.numeric.integer.melbi', foreground: '0451a5' },
  { token: 'constant.numeric.float.melbi', foreground: '0451a5' },
  { token: 'string.quoted.double.melbi', foreground: '22863a' },
  { token: 'string.quoted.double.format.melbi', foreground: '22863a' },
  { token: 'string.quoted.single.format.melbi', foreground: '22863a' },
  { token: 'string.quoted.double.bytes.melbi', foreground: '22863a' },
  { token: 'entity.name.type.melbi', foreground: '6f42c1' },
  { token: 'variable.other.quoted.melbi', foreground: 'e36209' },
  { token: 'variable.other.melbi', foreground: '24292e' },
];

export const COMPLETION_KIND_MAP = {
  function: 'Function',
  variable: 'Variable',
  keyword: 'Keyword',
  snippet: 'Snippet',
  text: 'Text',
};

export class MelbiTokenState {
  constructor(lineNumber = 0, version = 0) {
    this.lineNumber = lineNumber;
    this.version = version;
  }

  clone() {
    return new MelbiTokenState(this.lineNumber, this.version);
  }

  equals(other) {
    return !!other && this.lineNumber === other.lineNumber && this.version === other.version;
  }
}

export function computeNewEndPosition(range, text) {
  const startRow = range.startLineNumber - 1;
  const startColumn = range.startColumn - 1;
  if (!text) {
    return { row: startRow, column: startColumn };
  }
  const lines = text.split('\n');
  if (lines.length === 1) {
    return { row: startRow, column: startColumn + lines[0].length };
  }
  return { row: startRow + lines.length - 1, column: lines[lines.length - 1].length };
}

export function createEmptyTokenLines(model, defaultScope = DEFAULT_SCOPE) {
  const lineCount = typeof model?.getLineCount === 'function' ? model.getLineCount() : 0;
  return Array.from({ length: lineCount }, () => [{ startIndex: 0, scopes: defaultScope }]);
}

export function pushTokenRange(node, scope, lineMap, model, defaultScope = DEFAULT_SCOPE) {
  const start = node.startPosition;
  const end = node.endPosition;
  for (let row = start.row; row <= end.row; row += 1) {
    const tokens = lineMap[row];
    if (!tokens) {
      continue;
    }
    const startColumn = row === start.row ? start.column : 0;
    const endColumn = row === end.row ? end.column : model.getLineLength(row + 1);
    if (startColumn === endColumn) {
      continue;
    }
    tokens.push({ startIndex: startColumn, scopes: scope });
    tokens.push({ startIndex: endColumn, scopes: defaultScope });
  }
}

export function buildTokensFromTree(tree, model, scopeMap = NODE_SCOPE_MAP, defaultScope = DEFAULT_SCOPE) {
  const tokenLines = createEmptyTokenLines(model, defaultScope);
  if (!tree?.rootNode || !model) {
    return tokenLines;
  }
  const stack = [tree.rootNode];
  while (stack.length) {
    const node = stack.pop();
    if (!node) {
      continue;
    }
    const scope = scopeMap[node.type];
    if (scope) {
      pushTokenRange(node, scope, tokenLines, model, defaultScope);
    }
    if (Array.isArray(node.namedChildren)) {
      for (const child of node.namedChildren) {
        stack.push(child);
      }
    }
  }
  return tokenLines.map((lineTokens) => lineTokens.sort((a, b) => a.startIndex - b.startIndex));
}

export function spanToRange(model, span) {
  if (!span) {
    const position = model.getPositionAt(0);
    return {
      startLineNumber: position.lineNumber,
      startColumn: position.column,
      endLineNumber: position.lineNumber,
      endColumn: position.column,
    };
  }
  const start = model.getPositionAt(span.start ?? 0);
  const end = model.getPositionAt(span.end ?? span.start ?? 0);
  return {
    startLineNumber: start.lineNumber,
    startColumn: start.column,
    endLineNumber: end.lineNumber,
    endColumn: end.column,
  };
}

export function tsPointToOffset(model, point) {
  return model.getOffsetAt({ lineNumber: point.row + 1, column: point.column + 1 });
}

export function nodeToSpan(model, node) {
  const startIndex = tsPointToOffset(model, node.startPosition);
  const endIndex = tsPointToOffset(model, node.endPosition);
  if (startIndex === endIndex) {
    const fallbackEnd = Math.min(startIndex + 1, model.getValueLength());
    return { start: startIndex, end: fallbackEnd };
  }
  return { start: startIndex, end: endIndex };
}

export function collectSyntaxDiagnostics(tree, model) {
  if (!tree?.rootNode || !model) {
    return [];
  }
  const diagnostics = [];
  const stack = [tree.rootNode];
  while (stack.length) {
    const node = stack.pop();
    if (!node) {
      continue;
    }
    if (typeof node.isMissing === 'function' && node.isMissing()) {
      diagnostics.push({
        message: `Missing ${node.type}`,
        severity: 'error',
        code: 'missing',
        span: nodeToSpan(model, node),
        source: 'parser',
      });
    } else if (
      typeof node.isError === 'function' &&
      node.isError() &&
      !(node.parent && typeof node.parent.isError === 'function' && node.parent.isError())
    ) {
      diagnostics.push({
        message: 'Syntax error',
        severity: 'error',
        code: 'syntax',
        span: nodeToSpan(model, node),
        source: 'parser',
      });
    }
    if (Array.isArray(node.children)) {
      for (const child of node.children) {
        stack.push(child);
      }
    }
  }
  diagnostics.sort((a, b) => (a.span?.start ?? 0) - (b.span?.start ?? 0));
  return diagnostics;
}

export function mapCompletionItem(monaco, model, position, item, kindMap = COMPLETION_KIND_MAP) {
  const word = model.getWordUntilPosition(position);
  const range = new monaco.Range(
    position.lineNumber,
    word.startColumn,
    position.lineNumber,
    word.endColumn,
  );
  const kindKey = (item.kind || item.type || 'text').toString().toLowerCase();
  const kindName = kindMap[kindKey] || kindMap.text;
  const kind = monaco.languages.CompletionItemKind[kindName] ||
    monaco.languages.CompletionItemKind.Text;
  const insertText = item.insert_text || item.snippet || item.text || item.label || '';
  const isSnippet = Boolean(item.snippet || item.is_snippet);
  const insertTextRules = isSnippet
    ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet
    : undefined;
  return {
    label: item.label || item.text || insertText,
    kind,
    detail: item.detail || item.documentation,
    documentation: item.documentation,
    insertText,
    insertTextRules,
    range,
  };
}

export function applyEditsToTree(tree, changes, computeFn = computeNewEndPosition) {
  if (!tree || !Array.isArray(changes) || changes.length === 0) {
    return;
  }
  const ordered = [...changes].sort((a, b) => a.rangeOffset - b.rangeOffset);
  for (const change of ordered) {
    tree.edit({
      startIndex: change.rangeOffset,
      oldEndIndex: change.rangeOffset + change.rangeLength,
      newEndIndex: change.rangeOffset + change.text.length,
      startPosition: {
        row: change.range.startLineNumber - 1,
        column: change.range.startColumn - 1,
      },
      oldEndPosition: {
        row: change.range.endLineNumber - 1,
        column: change.range.endColumn - 1,
      },
      newEndPosition: computeFn(change.range, change.text),
    });
  }
}
