use alloc::sync::Arc;

use snafu::Snafu;

use crate::{Box, String, Vec, parser::Span, types::unification};

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

    #[snafu(display("Closures not yet supported"))]
    ClosuresNotSupported {
        src: String,

        span: Span,

        captured: Vec<String>,
    },

    #[snafu(display("Maps not yet implemented"))]
    MapsNotYetImplemented { src: String, span: Span },

    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn core::error::Error + Send + Sync>, Some)))]
        source: Option<Box<dyn core::error::Error + Send + Sync>>,
    },
}
