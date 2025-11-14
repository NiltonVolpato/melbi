import { describe, expect, it } from 'vitest';
import {
  DEFAULT_SCOPE,
  buildTokensFromTree,
  collectSyntaxDiagnostics,
  computeNewEndPosition,
  pushTokenRange,
  spanToRange,
} from '../src/melbi-playground-utils.js';

class MockModel {
  constructor(text) {
    this.text = text;
    this.lines = text.split('\n');
  }

  getLineCount() {
    return this.lines.length;
  }

  getLineLength(lineNumber) {
    return this.lines[lineNumber - 1]?.length ?? 0;
  }

  getPositionAt(offset) {
    const clamped = Math.max(0, Math.min(offset, this.text.length));
    let remaining = clamped;
    for (let index = 0; index < this.lines.length; index += 1) {
      const lineLength = this.lines[index].length;
      if (remaining <= lineLength) {
        return { lineNumber: index + 1, column: remaining + 1 };
      }
      remaining -= lineLength + 1;
    }
    const lastIndex = this.lines.length - 1;
    return { lineNumber: this.lines.length, column: this.lines[lastIndex].length + 1 };
  }

  getOffsetAt(position) {
    const { lineNumber, column } = position;
    let offset = 0;
    for (let index = 1; index < lineNumber; index += 1) {
      offset += this.getLineLength(index) + 1;
    }
    return offset + (column - 1);
  }

  getValueLength() {
    return this.text.length;
  }

  getValue() {
    return this.text;
  }
}

function createNode({
  startRow,
  startColumn,
  endRow,
  endColumn,
  type,
  children = [],
  namedChildren = [],
  isError = () => false,
  isMissing = () => false,
  parent = null,
}) {
  const node = {
    type,
    startPosition: { row: startRow, column: startColumn },
    endPosition: { row: endRow, column: endColumn },
    children,
    namedChildren,
    isError,
    isMissing,
    parent,
  };
  node.children?.forEach((child) => {
    child.parent = node;
  });
  node.namedChildren?.forEach((child) => {
    child.parent = node;
  });
  return node;
}

describe('computeNewEndPosition', () => {
  it('handles single-line insertions', () => {
    const result = computeNewEndPosition({ startLineNumber: 2, startColumn: 4 }, 'abc');
    expect(result).toEqual({ row: 1, column: 6 });
  });

  it('handles multi-line insertions', () => {
    const result = computeNewEndPosition({ startLineNumber: 3, startColumn: 1 }, 'foo\nbar');
    expect(result).toEqual({ row: 3, column: 3 });
  });
});

describe('pushTokenRange', () => {
  it('adds scope boundaries for a node span', () => {
    const model = new MockModel('abcdef');
    const lineMap = [[{ startIndex: 0, scopes: DEFAULT_SCOPE }]];
    const node = createNode({ startRow: 0, startColumn: 1, endRow: 0, endColumn: 4, type: 'string' });
    pushTokenRange(node, 'example.scope', lineMap, model, DEFAULT_SCOPE);
    expect(lineMap[0]).toEqual([
      { startIndex: 0, scopes: DEFAULT_SCOPE },
      { startIndex: 1, scopes: 'example.scope' },
      { startIndex: 4, scopes: DEFAULT_SCOPE },
    ]);
  });
});

describe('buildTokensFromTree', () => {
  it('emits tokens for mapped node scopes', () => {
    const model = new MockModel('let x = 1\nprint(x)');
    const child = createNode({
      startRow: 0,
      startColumn: 0,
      endRow: 0,
      endColumn: 3,
      type: 'identifier',
    });
    const root = createNode({
      startRow: 0,
      startColumn: 0,
      endRow: 1,
      endColumn: 6,
      type: 'source',
      namedChildren: [child],
    });
    const tree = { rootNode: root };
    const tokens = buildTokensFromTree(tree, model);
    expect(tokens[0].some((token) => token.scopes === 'variable.other.melbi')).toBe(true);
  });
});

describe('spanToRange', () => {
  it('maps spans to Monaco-style ranges', () => {
    const model = new MockModel('abc\ndef');
    const range = spanToRange(model, { start: 2, end: 5 });
    expect(range).toEqual({ startLineNumber: 1, startColumn: 3, endLineNumber: 2, endColumn: 2 });
  });
});

describe('collectSyntaxDiagnostics', () => {
  it('creates diagnostics for missing and error nodes', () => {
    const model = new MockModel('value');
    const missingNode = createNode({
      startRow: 0,
      startColumn: 0,
      endRow: 0,
      endColumn: 0,
      type: 'identifier',
      isMissing: () => true,
    });
    const errorNode = createNode({
      startRow: 0,
      startColumn: 2,
      endRow: 0,
      endColumn: 3,
      type: 'ERROR',
      isError: () => true,
    });
    const root = createNode({
      startRow: 0,
      startColumn: 0,
      endRow: 0,
      endColumn: 4,
      type: 'root',
      children: [missingNode, errorNode],
    });
    const diagnostics = collectSyntaxDiagnostics({ rootNode: root }, model);
    expect(diagnostics).toHaveLength(2);
    expect(diagnostics.map((d) => d.code)).toEqual(['missing', 'syntax']);
  });
});
