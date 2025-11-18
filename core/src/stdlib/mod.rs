//! Melbi Standard Library
//!
//! This module provides the standard library packages for Melbi, including:
//! - Math: Mathematical functions and constants
//! - String: String manipulation (future)
//! - Array: Array operations (future)
//! - Option: Option utilities (future)
//!
//! Each package is implemented as a record containing functions and constants.
//! Packages are built using native Rust functions (FFI) and registered in the
//! global environment before user code executes.

pub mod math;

// Re-export for convenience
pub use math::build_math_package;
