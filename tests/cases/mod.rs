use once_cell::sync::Lazy;
use rhizome::ast::*;

pub struct TestCase<'a> {
    pub name: &'static str,
    pub expr: &'static str,
    pub ast: Expr<'a>,
}

pub static TEST_CASES: Lazy<Vec<TestCase>> = Lazy::new(|| {
    vec![TestCase {
        name: "simple_addition",
        expr: "1 + 2",
        ast: Expr::Binary {
            op: BinaryOp::Add,
            left: &Expr::Literal(Literal::Int(1)),
            right: &Expr::Literal(Literal::Int(2)),
        },
    }]
});
