use crate::parser::parse;

#[test]
fn test_addition_vs_multiplication() {
    let actual = parse("a + b * c").unwrap();
    let expected = parse("a + (b * c)").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("(a + b) * c").unwrap();
    let expected = parse("(a + b) * c").unwrap(); // Parentheses should preserve grouping
    assert_eq!(actual, expected);
}

#[test]
fn test_and_vs_or() {
    let actual = parse("true and false or true").unwrap();
    let expected = parse("(true and false) or true").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("true and (false or true)").unwrap();
    let expected = parse("true and (false or true)").unwrap(); // Parentheses should preserve grouping
    assert_eq!(actual, expected);
}

#[test]
fn test_unary_vs_binary() {
    let actual = parse("-a + b").unwrap();
    let expected = parse("(-a) + b").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("-(a + b)").unwrap();
    let expected = parse("-(a + b)").unwrap(); // Parentheses should preserve grouping
    assert_eq!(actual, expected);
}

#[test]
fn test_exponentiation_vs_multiplication() {
    let actual = parse("a * b ^ c").unwrap();
    let expected = parse("a * (b ^ c)").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("(a * b) ^ c").unwrap();
    let expected = parse("(a * b) ^ c").unwrap(); // Parentheses should preserve grouping
    assert_eq!(actual, expected);
}

#[test]
fn test_if_vs_binary() {
    let actual = parse("if a then b + c else d").unwrap();
    let expected = parse("if a then (b + c) else d").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("if a then b else c + d").unwrap();
    let expected = parse("if a then b else (c + d)").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_where_vs_binary() {
    let actual = parse("a + b where { x = 1 }").unwrap();
    let expected = parse("(a + b) where { x = 1 }").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("a where { x = 1 } + b").unwrap();
    let expected = parse("(a where { x = 1 }) + b").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_otherwise_vs_binary() {
    let actual = parse("a + b otherwise c").unwrap();
    let expected = parse("(a + b) otherwise c").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("a otherwise b + c").unwrap();
    let expected = parse("a otherwise (b + c)").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_cast_vs_binary() {
    let actual = parse("a + b as c").unwrap();
    let expected = parse("(a + b) as c").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("a as b + c").unwrap();
    let expected = parse("a as (b + c)").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_grouped_vs_binary() {
    let actual = parse("a + (b * c)").unwrap();
    let expected = parse("a + (b * c)").unwrap(); // Parentheses should preserve grouping
    assert_eq!(actual, expected);

    let actual = parse("(a + b) * c").unwrap();
    let expected = parse("(a + b) * c").unwrap(); // Parentheses should preserve grouping
    assert_eq!(actual, expected);
}

#[test]
fn test_record_vs_binary() {
    let actual = parse("a + Record { x = 1 }").unwrap();
    let expected = parse("a + (Record { x = 1 })").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("Record { x = 1 } + a").unwrap();
    let expected = parse("(Record { x = 1 }) + a").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_map_vs_binary() {
    let actual = parse("a + { x: 1, y: 2 }").unwrap();
    let expected = parse("a + ({ x: 1, y: 2 })").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("{ x: 1, y: 2 } + a").unwrap();
    let expected = parse("({ x: 1, y: 2 }) + a").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_array_vs_binary() {
    let actual = parse("a + [1, 2, 3]").unwrap();
    let expected = parse("a + ([1, 2, 3])").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("[1, 2, 3] + a").unwrap();
    let expected = parse("([1, 2, 3]) + a").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_attr_access_vs_binary() {
    let actual = parse("a + obj.field").unwrap();
    let expected = parse("a + (obj.field)").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("obj.field + a").unwrap();
    let expected = parse("(obj.field) + a").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_index_access_vs_binary() {
    let actual = parse("a + arr[0]").unwrap();
    let expected = parse("a + (arr[0])").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("arr[0] + a").unwrap();
    let expected = parse("(arr[0]) + a").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_function_call_vs_binary() {
    let actual = parse("a + foo(1, 2)").unwrap();
    let expected = parse("a + (foo(1, 2))").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("foo(1, 2) + a").unwrap();
    let expected = parse("(foo(1, 2)) + a").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_otherwise_vs_if() {
    let actual = parse("if a then b otherwise c").unwrap();
    let expected = parse("(if a then b) otherwise c").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("if a then (b otherwise c)").unwrap();
    let expected = parse("if a then (b otherwise c)").unwrap(); // Parentheses should preserve grouping
    assert_eq!(actual, expected);
}

#[test]
fn test_otherwise_vs_where() {
    let actual = parse("a where { x = 1 } otherwise b").unwrap();
    let expected = parse("(a where { x = 1 }) otherwise b").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("a otherwise b where { x = 1 }").unwrap();
    let expected = parse("a otherwise (b where { x = 1 })").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_if_vs_where() {
    let actual = parse("if a then b where { x = 1 } else c").unwrap();
    let expected = parse("(if a then b where { x = 1 }) else c").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("if a then b else c where { x = 1 }").unwrap();
    let expected = parse("if a then b else (c where { x = 1 })").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_otherwise_vs_cast() {
    let actual = parse("a as b otherwise c").unwrap();
    let expected = parse("(a as b) otherwise c").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("a otherwise b as c").unwrap();
    let expected = parse("a otherwise (b as c)").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_if_vs_cast() {
    let actual = parse("if a then b as c else d").unwrap();
    let expected = parse("if a then (b as c) else d").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("if a then b else c as d").unwrap();
    let expected = parse("if a then b else (c as d)").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_where_vs_cast() {
    let actual = parse("a as b where { x = 1 }").unwrap();
    let expected = parse("(a as b) where { x = 1 }").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("a where { x = 1 } as b").unwrap();
    let expected = parse("(a where { x = 1 }) as b").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_otherwise_vs_grouped() {
    let actual = parse("a + (b otherwise c)").unwrap();
    let expected = parse("a + (b otherwise c)").unwrap(); // Parentheses should preserve grouping
    assert_eq!(actual, expected);

    let actual = parse("(a + b) otherwise c").unwrap();
    let expected = parse("(a + b) otherwise c").unwrap(); // Parentheses should preserve grouping
    assert_eq!(actual, expected);
}

#[test]
fn test_otherwise_vs_division_and_addition() {
    let actual = parse("a / b otherwise b + c").unwrap();
    let expected = parse("(a / b) otherwise (b + c)").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_otherwise_vs_and_or() {
    let actual = parse("a and b otherwise c or d").unwrap();
    let expected = parse("(a and b) otherwise (c or d)").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("a otherwise b and c or d").unwrap();
    let expected = parse("a otherwise ((b and c) or d)").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_complex_nested_expression() {
    let actual = parse("if a then b + c where { x = 1 } otherwise d and e or f").unwrap();
    let expected = parse("((if a then (b + c) where { x = 1 }) otherwise (d and e)) or f").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_excessive_parentheses() {
    let actual = parse("(((a + b)))").unwrap();
    let expected = parse("a + b").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_exponentiation_associativity() {
    let actual = parse("a ^ b ^ c").unwrap();
    let expected = parse("a ^ (b ^ c)").unwrap(); // Exponentiation is right-associative
    assert_eq!(actual, expected);
}

#[test]
fn test_function_call_with_complex_arguments() {
    let actual = parse("foo(a + b, c * d)").unwrap();
    let expected = parse("foo((a + b), (c * d))").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_chained_constructs() {
    let actual = parse("if a then b where { x = 1 } otherwise c").unwrap();
    let expected = parse("(if a then b where { x = 1 }) otherwise c").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_not_vs_and_or() {
    let actual = parse("not a and b").unwrap();
    let expected = parse("(not a) and b").unwrap();
    assert_eq!(actual, expected);

    let actual = parse("a or not b").unwrap();
    let expected = parse("a or (not b)").unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_deeply_nested_expressions() {
    let actual = parse("a + (b * (c - (d / e)))").unwrap();
    let expected = parse("a + (b * (c - (d / e)))").unwrap();
    assert_eq!(actual, expected);
}
