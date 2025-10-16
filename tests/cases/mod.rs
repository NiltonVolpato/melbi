#[macro_export]
macro_rules! test_case {
    (
        $mod_name:ident,
        input: $input:expr
        $(, ast: $ast:expr)?
        $(, formatted: $fmt:expr)?
        $(,)?
    ) => {
        #[cfg(test)]
        mod $mod_name {
            #![allow(unused_imports, dead_code)]

            use super::*;
            use bumpalo::Bump;
            use melbi_core::parser::{self, Expr, Literal};
            use $crate::test_case;

            const INPUT: &str = ($input);

            $(
                test_case!(@for, ast, $ast);
            )?

            $(
                test_case!(@for, formatted, $fmt);
            )?
        }
    };

    // Each field in test_case! can implement its own test function.
    // So, when adding a new field, you can define how to test it here.
    ( @for, ast, $expected:expr ) => {
        #[test]
        fn validate_ast() {
            let arena = Bump::new();
            let result = parser::parse(&arena, INPUT).map(|p| p.expr);
            test_case!(@assert, result, $expected);
        }
    };
    ( @for, formatted, $expected:expr ) => {
        #[test]
        fn validate_formatted() {
            let formatted = melbi_fmt::format(INPUT, false, false);
            let result = formatted
                .as_ref()
                .map(|s| s.as_str())
                .map_err(|e| format!("{:#?}", e));
            test_case!(@assert, result, $expected);
        }
    };

    // @assertto be used in the implementation of test cases.
    // Syntax: assert_case!(result, expected);
    // Where `expected` can be `Ok(pattern)`, `Err(pattern)`, or
    // a direct value to compare against.
    ( @assert, $result:expr, Ok($expected:pat) ) => {
        let _result = $result;
        let Ok($expected) = _result else {
            panic!("OK: Pattern didn't match result: {:#?}", _result);
        };
    };
    ( @assert, $result:expr, Err($expected:pat) ) => {
        let _result = $result;
        let Err($expected) = _result else {
            panic!("ERR: Pattern didn't match result: {:#?}", _result);
        };
    };
    ( @assert, $result:expr, $expected:expr ) => {
        let _result = $result;
        let Ok(_actual) = _result else {
            panic!("EXPR: Expected succes, but got: {:#?}", _result);
        };
        pretty_assertions::assert_eq!($expected, _actual);
    };
}
