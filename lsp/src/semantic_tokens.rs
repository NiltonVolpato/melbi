use tower_lsp::lsp_types::*;

/// Semantic token types used by Melbi LSP
/// The order here determines the indices used in semantic token encoding
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::TYPE,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::NUMBER,
    SemanticTokenType::STRING,
    SemanticTokenType::COMMENT,
    SemanticTokenType::OPERATOR,
];

// Indices for token types (derived from position in TOKEN_TYPES array)
pub const KEYWORD: u32 = 0;
pub const VARIABLE: u32 = 1;
pub const FUNCTION: u32 = 2;
pub const PARAMETER: u32 = 3;
pub const TYPE: u32 = 4;
pub const PROPERTY: u32 = 5;
pub const NUMBER: u32 = 6;
pub const STRING: u32 = 7;
pub const COMMENT: u32 = 8;
pub const OPERATOR: u32 = 9;

/// Get the semantic token legend for LSP registration
pub fn get_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: vec![],
    }
}
