---
title: "Let Bindings & Scope"
code: |
  let x = 10;
  let y = 20;
  let sum = x + y;
  sum * 2
---

# Let Bindings and Scope

The `let` keyword allows you to bind names to values, making your code more readable and reusable.

## Basic Let Bindings

You can bind a name to any value:

```melbi
let answer = 42;
answer
```

## Multiple Bindings

Chain multiple `let` bindings together:

```melbi
let x = 10;
let y = 20;
x + y
```

Each binding is separated by a semicolon `;`.

## Using Previous Bindings

Later bindings can use earlier ones:

```melbi
let x = 5;
let doubled = x * 2;
let quadrupled = doubled * 2;
quadrupled
```

## Scope

Bindings are scoped - they're only available after they're defined:

```melbi
let outer = 1;
let inner = outer + 2;
inner  // This works - outer is visible
```

## Try It!

Modify the code on the right:
- Add more variables
- Create a calculation using multiple steps
- Try referring to a variable defined later (you'll see an error!)
