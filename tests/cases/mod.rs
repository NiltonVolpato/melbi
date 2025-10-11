use once_cell::sync::Lazy;
use rhizome::parser;

pub struct TestCase<'a> {
    pub name: &'static str,
    pub expr: &'static str,
    pub ast: parser::Expr<'a>,
}

pub static TEST_CASES: Lazy<Vec<TestCase>> = Lazy::new(|| {
    vec![TestCase {
        name: "simple_addition",
        expr: "1 + 2",
        ast: parser::Expr::Binary {
            op: parser::BinaryOp::Add,
            left: &parser::Expr::Literal(parser::Literal::Int {
                value: 1,
                suffix: None,
            }),
            right: &parser::Expr::Literal(parser::Literal::Int {
                value: 2,
                suffix: None,
            }),
        },
    }]
});
