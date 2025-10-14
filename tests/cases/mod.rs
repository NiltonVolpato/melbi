#[macro_export]
macro_rules! test_case {
    (
        $mod_name:ident,
        input: $input:expr
        $(, ast: $ast:pat)?
        $(, formatted: $fmt:pat)?
        $(,)?
    ) => {
        #[cfg(test)]
        mod $mod_name {
            #![allow(unused_imports, dead_code)]

            use super::*;
            use bumpalo::Bump;
            use melbi_core::parser::{self, Expr, Literal};
            use $crate::test_case;

            const INPUT: &str = $input;

            $(
                #[test]
                fn validate_ast() {
                    let arena = Bump::new();
                    let result = parser::parse(&arena, INPUT).map(|p| p.expr);
                    let $ast = result else {
                        panic!("Pattern didn't match result: {:?}", result);
                    };
                }
            )?

            $(
                #[test]
                fn validate_formatted() {
                    let formatted = melbi_fmt::format(INPUT, false, false);
                    let result = formatted
                        .as_ref()
                        .map(|s| s.as_str())
                        .map_err(|e| format!("{:?}", e));
                    let $fmt = result else {
                        panic!("Pattern didn't match result: {:?}", result);
                    };
                }
            )?
        }
    };
}
