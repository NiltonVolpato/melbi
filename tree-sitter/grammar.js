/**
 * @file Rhizome grammar for tree-sitter
 * @author Nilton Volpato <nilton@volpa.to>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "rhizome",

  extras: ($) => [
    /\s/, // Whitespace
    $.comment,
  ],

  rules: {
    source_file: ($) => $.expression,

    // Comments
    comment: ($) => token(seq("//", /.*/)),

    // === Expressions ===
    expression: ($) =>
      choice(
        // $.binary_expression,
        // $.unary_expression,
        // $.if_expression,
        // $.lambda_expression,
        // $.where_expression,
        // $.otherwise_expression,
        // $.call_expression,
        // $.index_expression,
        // $.field_expression,
        // $.cast_expression,
        // $.grouped_expression,
        $.literal,
        $.identifier
      ),

    // We'll fill these in step by step...
    // Start with literals

    literal: ($) =>
      choice(
        $.integer,
        $.float,
        $.boolean,
        $.string
        // We'll add more later
      ),

    // === Simple Literals ===

    integer: ($) =>
      token(
        choice(
          seq("0b", /[01_]+/), // Binary
          seq("0o", /[0-7_]+/), // Octal
          seq("0x", /[0-9a-fA-F_]+/), // Hex
          /[0-9][0-9_]*/ // Decimal
        )
      ),

    float: ($) =>
      token(
        choice(
          // 3.14, 3., .5
          seq(
            optional(/[0-9][0-9_]*/),
            ".",
            /[0-9_]+/,
            optional(seq(/[eE]/, optional(/[+-]/), /[0-9_]+/))
          ),
          // 3e10, 3e-10
          seq(/[0-9][0-9_]*/, /[eE]/, optional(/[+-]/), /[0-9_]+/)
        )
      ),

    boolean: ($) => choice("true", "false"),

    string: ($) =>
      choice(
        seq('"', repeat(choice(/[^"\\]/, /\\./)), '"'),
        seq("'", repeat(choice(/[^'\\]/, /\\./)), "'")
      ),

    // === Identifiers ===

    identifier: ($) => choice($.quoted_identifier, $.unquoted_identifier),

    quoted_identifier: ($) => /`[A-Za-z0-9\-_.:\/]+`/,

    unquoted_identifier: ($) => token(seq(/[a-z_]/, /[a-zA-Z0-9_]*/)),

    // === Expressions (stubs for now) ===

    grouped_expression: ($) => seq("(", $.expression, ")"),

    // TODO: We'll implement these next
    binary_expression: ($) => "TODO",
    unary_expression: ($) => "TODO",
    if_expression: ($) => "TODO",
    lambda_expression: ($) => "TODO",
    where_expression: ($) => "TODO",
    otherwise_expression: ($) => "TODO",
    call_expression: ($) => "TODO",
    index_expression: ($) => "TODO",
    field_expression: ($) => "TODO",
    cast_expression: ($) => "TODO",
  },
});
