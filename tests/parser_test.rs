#[cfg(test)]
use pretty_assertions::assert_eq;
use rhizome::parser::parse;

mod cases;

#[test]
fn test_works() {
    assert!(true);
}

#[test]
fn test_all_cases() {
    let arena = bumpalo::Bump::new();
    for case in cases::TEST_CASES.iter() {
        let parsed = parse(&arena, case.expr).unwrap();
        assert_eq!(*parsed.expr, case.ast, "Test case '{}' failed", case.name);
    }
}
