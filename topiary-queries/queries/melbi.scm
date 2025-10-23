; === Leaf Nodes (don't format) ===

; Don't format string/bytes content or comments - preserve them as-is
[
  (string)
  (bytes)
  (format_text)
  (format_text_single)
  (comment)
] @leaf

; Note: format_string itself is NOT a leaf - we want to format the
; interpolated expressions inside format_expr nodes

; === Spacing Rules ===

; Add around after keywords
["if" "then" "else" "where" "as" "otherwise" "not"] @prepend_space @append_space

[ (comment) ] @allow_blank_line_before

(comment) @multi_line_indent_all @prepend_input_softline @append_hardline
(comment) @prepend_space

; Add space around binary operators
(binary_expression
  operator: _ @prepend_space @append_space)

; Add space around lambda "=>"
"=>" @append_space @prepend_space

; Add space after commas
"," @append_space

; Ensure 2 spaces before end-of-line comments
; ( _ . (comment) @prepend_delimiter (#delimiter! "  ") )

; Spaces around bindings
(binding
  "=" @prepend_space @append_space)

; Map entry spacing
(map_entry
  ":" @append_space)

; === Indentation Rules ===

; Indent where blocks
(where_expression
  "where"
  "{" @append_begin_scope @append_spaced_scoped_softline @append_indent_start
  (binding_list)
  "}" @prepend_end_scope @prepend_indent_end @prepend_spaced_scoped_softline
  (#scope_id! "where_scope"))

; Empty record - no internal formatting, antispace before {
(record
  "Record" @append_antispace
  .
  "{"
  .
  "}")

; Indent record blocks
(record
  "{" @append_begin_scope @append_spaced_scoped_softline @append_indent_start
  (binding_list)
  "}" @prepend_end_scope @prepend_indent_end @prepend_spaced_scoped_softline
  (#scope_id! "record_scope"))

; Empty map - remove any internal spacing
(map
  "{" @append_antispace
  .
  "}")

; Map with entries
(map
  "{" @append_begin_scope @append_spaced_scoped_softline @append_indent_start
  (map_entry_list)
  "}" @prepend_end_scope @prepend_indent_end @prepend_spaced_scoped_softline
  (#scope_id! "map_scope"))

; Single-line map - remove internal spacing
(map
  (#single_line_scope_only! "map_scope")
  "{" @append_antispace
  "}" @prepend_antispace)

; Indent array elements (multi-line arrays)
(array
  "[" @append_begin_scope @append_spaced_scoped_softline @append_indent_start
  (array_elems)
  "]" @prepend_end_scope @prepend_indent_end @prepend_spaced_scoped_softline
  (#scope_id! "array_scope"))

; Empty array - remove any internal spacing
(array
  "[" @append_antispace
  .
  "]")

; Single-line array - remove internal spacing
(array
  (#single_line_scope_only! "array_scope")
  "[" @append_antispace
  "]" @prepend_antispace)

; === Line Breaks ===

; Remove empty commas after comments.
; There's another rule that will add them back before the comment.
(
  (comment)
  .
  "," @delete
)

; Preserve user's choice: allow line break before "where"
[ "where" "then" "else" ] @prepend_input_softline

; Single-line scope: delete trailing comma
(binding_list
  (#single_line_scope_only! "where_scope")
  (#delimiter! ",")
  "," @delete
  .
)

; Multi-line: expand the whole block if bindings are on separate lines.
(binding_list
  (#scope_id! "where_scope")
  (binding)
  "," @append_spaced_scoped_softline
  .
  (binding)
)

; Multi-line: add trailing comma after the last binding
(binding_list
  (#multi_line_scope_only! "where_scope")
  (#delimiter! ",")
  (binding) @append_delimiter
  .
  ","? @do_nothing
)

; Single-line: delete trailing comma
(binding_list
  (#single_line_scope_only! "record_scope")
  "," @delete
  .
)

; Multi-line: add trailing comma
(binding_list
  (#multi_line_scope_only! "record_scope")
  (#delimiter! ",")
  (binding) @append_delimiter
  .
  ","? @do_nothing
)

; Multi-line: expand the whole block if bindings are on separate lines
(binding_list
  (#multi_line_scope_only! "record_scope")
  (binding)
  "," @append_spaced_softline
  .
  (binding)
)

; Single-line: delete trailing comma
(map_entry_list
  (#single_line_scope_only! "map_scope")
  (#delimiter! ",")
  "," @delete
  .
)

; Multi-line: add trailing comma
(map_entry_list
  (#multi_line_scope_only! "map_scope")
  (#delimiter! ",")
  (map_entry) @append_delimiter
  .
  ","? @do_nothing
)

; Multi-line: expand entries on separate lines
(map_entry_list
  (#multi_line_scope_only! "map_scope")
  (map_entry)
  "," @append_spaced_softline
  .
  (map_entry))

; Single-line: delete trailing comma
(array_elems
  (#single_line_scope_only! "array_scope")
  (#delimiter! ",")
  "," @delete
  .)

; Multi-line: add trailing comma
(array_elems
  (#multi_line_scope_only! "array_scope")
  (#delimiter! ",")
  (expression) @append_delimiter
  .
  ","? @do_nothing
)

; Multi-line: expand entries on separate lines
(array_elems
  (#multi_line_scope_only! "array_scope")
  (expression)
  "," @append_spaced_softline
  .
  (expression)
)

; == Lambdas ==

; Indent lambda argument (multi-line case)
(lambda_expression
  "(" @append_begin_scope @append_empty_scoped_softline @append_indent_start
  _
  ")" @prepend_end_scope @prepend_indent_end @prepend_empty_scoped_softline
  (#scope_id! "lambda_scope"))

; Empty array - remove any internal spacing
(lambda_expression
  "(" @append_antispace
  .
  ")" @prepend_antispace)

; Single-line: delete trailing comma
(lambda_params
  (#single_line_scope_only! "lambda_scope")
  (#delimiter! ",")
  "," @delete @append_antispace
  .
)

; === Format String Interpolations ===

; Add spaces around interpolation braces to avoid ambiguity with escaped braces
; This also maintains consistency with other braced constructs (maps, records, where)
(format_expr
  "{" @append_begin_scope @append_spaced_scoped_softline @append_indent_start
  "}" @prepend_end_scope @prepend_spaced_scoped_softline @prepend_indent_end
  (#scope_id! "format_scope"))

; === Redundant Parentheses ===

(if_expression
  condition: (expression
  	(grouped_expression
      "(" @delete
      ")" @delete)))