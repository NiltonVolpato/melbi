; Keywords
["if" "then" "else" "where" "as" "otherwise"] @keyword

; Operators
["and" "or" "not"] @keyword.operator

; Literals
(boolean) @constant.builtin

(integer) @number
(float) @number.float

; Strings
(string) @string
(bytes) @string.special
(format_string) @string
(format_expr) @embedded

; Comments
(comment) @comment

; Functions
(lambda_expression) @function
(call_expression function: (expression) @function)

; Types
(type_path) @type
"Record" @type.builtin

; Type in cast expressions - anything after "as"
(cast_expression
  type: (_) @type)

; Type fields
(type_field
  name: (identifier) @property
  type: (_) @type)

; Identifiers
(identifier) @variable
(field_expression
  field: (identifier) @property)

; Operators
["+" "-" "*" "/" "^"] @operator
["=" ":" "=>"] @punctuation.special
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
["," "."] @punctuation.delimiter

(ERROR) @error
