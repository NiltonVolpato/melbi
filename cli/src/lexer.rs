use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token {
    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[regex(r"//.*")]
    Comment,

    // Strict Quoted Identifier (must end with `)
    #[regex(r"`[A-Za-z0-9\-_.:\/]+`")]
    QuotedId,

    // Strict Double Quote String (must end with ")
    #[regex(r#"(?:b|f)?"(?:[^"\\]|\\.)*""#)]
    StringDouble,

    // Strict Single Quote String (must end with ')
    #[regex(r#"(?:b|f)?'(?:[^'\\]|\\.)*'"#)]
    StringSingle,

    #[regex(r#"[^ \t\n\f\{\}\[\]\(\)\"'`]+"#)]
    Other,
}

pub fn calculate_depth(buffer: &str) -> Option<usize> {
    let mut depth: isize = 0;

    // We use .spanned() or just iteration.
    // Logos returns Result<Token, _> where Err means "could not match".
    for token_res in Token::lexer(buffer) {
        match token_res {
            Ok(Token::LBrace) | Ok(Token::LBracket) | Ok(Token::LParen) => depth += 1,
            Ok(Token::RBrace) | Ok(Token::RBracket) | Ok(Token::RParen) => depth -= 1,

            // Valid tokens that don't affect depth
            Ok(_) => {}

            // STRICT BEHAVIOR:
            // If we hit an unclosed string (or any unknown char), abort immediately.
            Err(_) => {
                return None;
            }
        }
    }

    if depth < 0 {
        Some(0)
    } else {
        Some(depth as usize)
    }
}
