/**
 * @file Rhizome grammar for tree-sitter
 * @author Nilton Volpato <nilton@volpa.to>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "rhizome",

  rules: {
    // TODO: add the actual grammar rules
    source_file: $ => "hello"
  }
});
