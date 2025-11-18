---
title: "Type Classes"
code: |
  typeclass Show a where
    show : a -> String
  end;

  instance Show Int where
    show = fn n => "Number"
  end;

  show(42)
---

# Type Classes

Type classes are one of Melbi's most powerful features. They let you define behavior that works across different types.

## What are Type Classes?

A type class defines a set of functions that types can implement. Think of it as an interface or protocol.

## Defining a Type Class

Use the `typeclass` keyword:

```melbi
typeclass Show a where
  show : a -> String
end
```

This says: "Any type `a` that wants to be `Show`-able must provide a `show` function that converts values to strings."

## Creating Instances

To make a type support a type class, create an instance:

```melbi
instance Show Int where
  show = fn n => "Number"
end
```

Now integers can be "shown" as strings!

## Using Type Classes

Once you've defined a type class and its instances, you can use the functions:

```melbi
show(42)  // Returns "Number"
```

## Common Type Classes

Melbi includes several built-in type classes:
- `Eq` - for equality comparison
- `Ord` - for ordering
- `Num` - for numeric operations

## Try It!

The code on the right shows a simple `Show` type class. Try:
- Changing what the `show` function returns
- Creating an instance for another type
