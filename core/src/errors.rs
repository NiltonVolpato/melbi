use alloc::sync::Arc;

use snafu::Snafu;

use crate::{String, Vec, parser::Span, types::unification};

#[derive(Debug, Snafu)]
pub struct Error {
    pub kind: Arc<ErrorKind>,
    pub context: Vec<String>,
}

#[derive(Debug, Snafu)]
pub enum ErrorKind {
    #[snafu(display("Parse error"))]
    Parse {
        src: String,

        err_span: Span,

        help: Option<String>,
    },

    #[snafu(display("Type checking error"))]
    TypeChecking {
        src: String,

        span: Option<Span>,

        help: Option<String>,

        unification_context: Option<unification::Error>,
    },

    #[snafu(display("Type conversion error"))]
    TypeConversion {
        src: String,

        span: Span,

        help: String,
    },
}
