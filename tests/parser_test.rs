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
    for case in cases::TEST_CASES.iter() {
        let parsed = parse(case.expr).unwrap();
        assert_eq!(parsed, case.ast, "Test case '{}' failed", case.name);
    }
}
