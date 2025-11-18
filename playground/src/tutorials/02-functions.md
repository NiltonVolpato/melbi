---
title: "Functions"
code: |
  let double = fn x => x * 2;
  double(5)
---

# Functions in Melbi

Functions are the heart of Melbi. They let you package up computations and reuse them.

## Defining Functions

Use the `fn` keyword to create a function:

```melbi
fn x => x * 2
```

This creates a function that takes one parameter `x` and returns `x * 2`.

## Naming Functions

To use a function multiple times, give it a name with `let`:

```melbi
let double = fn x => x * 2;
double(5)
```

The semicolon `;` separates the definition from the expression that uses it.

## Multiple Parameters

Functions can take multiple parameters:

```melbi
let add = fn x => fn y => x + y;
add(3)(4)
```

This is called **currying** - each function takes one parameter and returns another function.

## Try It!

Experiment with the example on the right:
- Change the multiplier in the `double` function
- Try calling `double` with different numbers
- Create your own function that adds 10 to a number
