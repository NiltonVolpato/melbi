#set page(
  paper: "us-letter",
  margin: (x: 0.5in, y: 0.5in),
)

#set text(
  font: "IBM Plex Sans",
  size: 9pt,
)

#set par(
  justify: false,
  leading: 0.5em,
)

#set heading(
  numbering: none,
)

// Enable Melbi syntax highlighting
#set raw(
  syntaxes: "melbi.sublime-syntax",
  theme: "github.tmTheme"
)

#show heading.where(level: 1): it => [
  #set text(size: 14pt, weight: "bold", fill: rgb("#cc3333"))
  #block(below: 0.5em, it.body)
]

#show heading.where(level: 2): it => [
  #set text(size: 10pt, weight: "bold")
  #block(above: 0.7em, below: 0.3em, it.body)
]

#show raw.where(block: true): it => [
  #block(
    fill: luma(250),
    inset: 4pt,
    radius: 2pt,
    width: 100%,
  )[
    #set text(size: 8pt, weight: "regular")
    #it
  ]
]

#show raw.where(block: false): it => [
  #box(
    fill: luma(250),
    inset: (x: 2pt, y: 0pt),
    radius: 1pt,
    baseline: 0.1em,
  )[
    #set text(size: 8.5pt, weight: "regular")
    #it
  ]
]

// Page 1: Essential Quick Reference
#align(center)[
  #text(size: 18pt, weight: "bold")[🖖 Melbi Language Cheat Sheet]
  #v(0.2em)
  #text(size: 9pt, style: "italic")[Quick Reference for Programmers]
]

#v(0.5em)

#columns(2, gutter: 1em)[

= Literals

== Integers
```melbi
42          -123        0b101010
0o52        0x2A        999_999_999
```

== Floats
```melbi
3.14        0.5         .5
3.          1.5e10      1.5E-10
```

== Booleans & Options
```melbi
true        false
some 42     none
```

== Strings & Bytes
```melbi
"hello"     'world'     b"bytes"
f"Hello {name}"         // f-string
```

== Arrays & Records
```melbi
[1, 2, 3]              // Array
{x = 1, y = 2}         // Record
Record{}               // Empty record
```

== Maps
```melbi
{}                     // Empty map
{a: 1, b: 2}          // String keys
{1: "one", 2: "two"}  // Int keys
```

= Operators

== Arithmetic
```melbi
2 ^ 3       // Power
5 * 6       7 / 8      // Mul, Div
1 + 2       3 - 4      // Add, Sub
-5                     // Negation
```

== Comparison
```melbi
5 == 5      5 != 3
3 < 5       10 > 5
5 <= 5      7 >= 3
```

== Logical
```melbi
not true
true and false
true or false
```

== Membership
```melbi
5 in [1, 2, 3]
"lo" in "hello"
5 not in [1, 2]
```

== Fallback
```melbi
x otherwise 0
x + y otherwise fallback
```

#colbreak()

= Control Flow

== If Expression
```melbi
if condition then value1 else value2

if x > 0
then x
else -x
```

== Where Bindings
```melbi
result where { x = 1, y = 2 }

result where {
    x = 1,
    y = 2,
}
```

== Pattern Matching
```melbi
value match {
    some x -> x * 2,
    none -> 0
}

flag match {
    true -> "yes",
    false -> "no"
}

x match {
    1 -> "one",
    2 -> "two",
    _ -> "other"
}
```

= Functions

== Lambda Syntax
```melbi
(x) => x + 1
(x, y) => x + y
() => 42

(x) => (y) => x + y  // Currying
```

== Function Calls
```melbi
double(21)
add(1, 2)
func()
```

= Postfix Operations

```melbi
record.field        // Field access
array[0]            // Indexing
map[key]            // Map lookup
value as Int        // Type cast
```

= Type System

```melbi
Int     Float   Bool   String   Bytes
Array[T]        Map[K, V]
(T1, T2) => R           // Function
Option[T]               // Optional
Record[x: Int, y: Int]  // Record type
```

]

#pagebreak()

// Page 2: Detailed Reference
#align(center)[
  #text(size: 16pt, weight: "bold")[Melbi Language - Detailed Reference]
]

#v(0.5em)

#columns(2, gutter: 1em)[

= Operator Precedence

#table(
  columns: (auto, 1fr),
  stroke: none,
  align: (right, left),
  [*1.*], [`()` `[]` `.` `where` `as` `match`],
  [*2.*], [`-` `not` `if` `()=>` `some`],
  [*3.*], [`^`],
  [*4.*], [`*` `/`],
  [*5.*], [`+` `-`],
  [*6.*], [`==` `!=` `<` `>` `<=` `>=`],
  [*7.*], [`in` `not in`],
  [*8.*], [`and`],
  [*9.*], [`or`],
  [*10.*], [`otherwise`],
)

= Escape Sequences

== Strings & Format Strings
```melbi
\n  \r  \t  \0  \\  \"  \'
\uXXXX              // Unicode (4 hex)
\UXXXXXXXX          // Unicode (8 hex)
\                   // Line continuation
```

== Bytes
```melbi
\xXX                // Hex byte (2 hex)
```

== Format Strings
```melbi
{{  }}              // Literal braces
```

= Pattern Matching Details

*Exhaustiveness checking:*
- `Bool`: Must cover `true` and `false` (or `_`)
- `Option[T]`: Must cover `some _` and `none` (or `_`)
- Other types: Require `_` wildcard

```melbi
// Nested patterns
opt match {
    some (some x) -> x,
    some none -> -1,
    none -> 0
}

// Variable binding
x match { value -> value + 1 }
```

#colbreak()

= Complete Examples

== Calculation with Discount
```melbi
price * quantity * (1.0 - discount)
where {
    discount = if premium
               then 0.2
               else 0.1
}
```

== Format String
```melbi
f"Hello {name}, score: {score * 100}!"
where {
    name = "Alice",
    score = 0.95,
}
```

== Quadratic Formula
```melbi
(a, b, c) => roots where {
    delta = b ^ 2 - 4 * a * c,
    r0 = (-b + delta ^ 0.5) / (2 * a),
    r1 = (-b - delta ^ 0.5) / (2 * a),
    roots = [r0, r1],
}
```

== Safe Array Access
```melbi
array[index] otherwise -1

data match {
    some value -> value * 2,
    none -> 0
} otherwise fallback
```

= Key Features

*Type Inference*
- Hindley-Milner type inference
- No annotations needed
- Compile-time type checking

*Immutability*
- All values immutable
- No variable reassignment
- Pure functional

*No Null*
- Uses `Option[T]` instead
- Pattern matching required
- Safe by construction

*Comments*
```melbi
// Single-line only
x + y  // Inline comment
```

]
