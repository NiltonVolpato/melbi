use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("melbi error")]
pub enum Error {
    #[diagnostic(code(melbi_core::parse_error))]
    #[error("parse error")]
    Parse {
        #[source_code]
        src: String,

        #[label("parse error here")]
        err_span: SourceSpan,

        #[help]
        help: Option<String>,
    },

    #[diagnostic(code(melbi_core::type_checking_error))]
    #[error("Type checking error")]
    TypeChecking {
        #[source_code]
        src: String,

        #[label("type mismatch here")]
        span: Option<SourceSpan>,

        #[help]
        help: Option<String>,
    },

    #[diagnostic(code(melbi_core::type_conversion_error))]
    #[error("Type conversion error")]
    TypeConversion {
        #[source_code]
        src: String,

        #[label("invalid type here")]
        span: SourceSpan,

        #[help]
        help: String,
    },

    #[error("unknown error")]
    Unknown,
}
