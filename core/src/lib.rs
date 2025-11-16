#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
// #![cfg_attr(not(test), no_std)]

extern crate alloc;

// Re-export for convenience so other modules don't need alloc:: prefix
#[allow(unused_imports)]
pub(crate) use alloc::{boxed::Box, format, string::String, string::ToString, vec, vec::Vec};

pub mod analyzer;
pub mod api;
pub mod casting;
pub mod diagnostics;
pub mod evaluator;
pub mod parser;
pub mod scope_stack;
pub mod syntax;
pub mod teeny_vec;
pub mod types;
pub mod values;
pub mod vm;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}

/// Test utilities for enabling logging in tests
#[cfg(test)]
pub mod test_utils {
    /// Initialize tracing subscriber for tests with DEBUG level
    /// Call this at the start of tests where you want to see logging output
    ///
    /// # Example
    /// ```ignore
    /// #[test]
    /// fn test_type_inference() {
    ///     test_utils::init_test_logging();
    ///     // ... your test code
    /// }
    /// ```
    pub fn init_test_logging() {
        use tracing_subscriber::{fmt, EnvFilter};

        // Try to initialize, ignore error if already initialized
        let _ = fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
            )
            .with_test_writer()
            .try_init();
    }
}
