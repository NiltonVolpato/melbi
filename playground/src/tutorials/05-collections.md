---
title: "Arrays & Records"
code: |
  let numbers = [1, 2, 3, 4, 5];
  let person = { name: "Alice", age: 30 };
  person.name
---

# Collections: Arrays and Records

Melbi provides two main collection types: arrays for ordered sequences and records for labeled data.

## Arrays

Arrays hold multiple values of the same type:

```melbi
let numbers = [1, 2, 3, 4, 5];
numbers
```

### Array Operations

Access elements by index:

```melbi
let first = numbers[0];  // Gets 1
```

Arrays are zero-indexed, so the first element is at position 0.

## Records

Records group related data with labeled fields:

```melbi
let person = { name: "Alice", age: 30 };
person
```

### Accessing Fields

Use dot notation to access record fields:

```melbi
let name = person.name;   // Gets "Alice"
let age = person.age;     // Gets 30
```

## Nested Structures

You can combine arrays and records:

```melbi
let users = [
  { name: "Alice", age: 30 },
  { name: "Bob", age: 25 }
];
users[0].name  // Gets "Alice"
```

## Try It!

Experiment with the code on the right:
- Create an array with different numbers
- Add more fields to the person record
- Try accessing different fields
- Create a nested structure combining arrays and records
