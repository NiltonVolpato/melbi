use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser/expression.pest"]
pub struct ExpressionParser;
