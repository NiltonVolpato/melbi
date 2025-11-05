#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
// #![cfg_attr(not(test), no_std)]

extern crate alloc;

// Re-export for convenience so other modules don't need alloc:: prefix
#[allow(unused_imports)]
pub(crate) use alloc::{boxed::Box, format, string::String, string::ToString, vec::Vec};

pub mod analyzer;
pub mod casting;
pub mod errors;
pub mod evaluator;
pub mod parser;
pub mod scope_stack;
pub mod syntax;
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
