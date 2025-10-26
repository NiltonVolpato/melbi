#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
// #![cfg_attr(not(test), no_std)]

extern crate alloc;

// Re-export for convenience so other modules don't need alloc:: prefix
pub use alloc::{boxed::Box, format, string::String, string::ToString, vec, vec::Vec};

pub mod analyzer;
pub mod errors;
pub mod parser;
pub mod syntax;
pub mod types;
pub mod values;
pub use types::Type;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
