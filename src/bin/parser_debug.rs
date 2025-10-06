use rhizome::parser;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let parsed = parser::parse(&args[1]);
    match parsed {
        Err(e) => {
            eprintln!("Error:\n{}", e);
            return;
        }
        Ok(ast) => {
            println!(
                "Parsed AST:\n{:#?}\n\nPretty printed:\n{}",
                ast,
                ast.render(80)
            );
        }
    }
}
