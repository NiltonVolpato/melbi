use bumpalo::Bump;
use rhizome::parser;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let arena = Bump::new();
    let parsed = parser::parse(&arena, &args[1]);
    match parsed {
        Err(e) => {
            eprintln!("Error:\n{}", e);
            return;
        }
        Ok(ast) => {
            println!("Parsed AST:\n{:#?}", ast.root);
        }
    }
}
