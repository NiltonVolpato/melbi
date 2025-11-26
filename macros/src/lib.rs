//! Procedural macros for Melbi FFI functions
//!
//! This crate provides the `#[melbi_fn]` attribute macro for generating
//! type-safe FFI bindings between Rust and Melbi.

extern crate proc_macro;

use proc_macro::TokenStream;

mod melbi_fn;

/// Generate a type-safe FFI function for Melbi.
///
/// This attribute macro transforms a clean Rust function into a struct that
/// implements both `Function` and `AnnotatedFunction` traits, enabling
/// zero-cost type-safe calls from Melbi code.
///
/// # Example
///
/// ```ignore
/// #[melbi_fn(name = "Len")]
/// fn string_len(_arena: &Bump, _type_mgr: &TypeManager, s: Str) -> i64 {
///     s.chars().count() as i64
/// }
/// ```
///
/// This generates:
/// - Implementation function `string_len_impl`
/// - Struct `Len` with metadata (name, location, doc)
/// - `Function` trait implementation for runtime execution
/// - `AnnotatedFunction` trait implementation for registration
///
/// # Required Attribute
///
/// - `name`: The Melbi function name (string literal). This becomes the struct name.
///
/// # Parameters
///
/// Functions can accept any type that implements the `Bridge` trait:
/// - Primitives: `i64`, `f64`, `bool`
/// - Strings: `Str` (zero-copy wrapper)
/// - Collections: `Array<T>`, `Map<K, V>`
/// - Options: `Optional<T>`
///
/// The first two parameters should be `_arena: &Bump` and `_type_mgr: &TypeManager`
/// (can be omitted if unused).
///
/// # Returns
///
/// Functions must return a type that implements `Bridge`.
///
/// # Registration
///
/// ```ignore
/// // In package builder:
/// Len::new(type_mgr).register(arena, type_mgr, env)?;
/// ```
#[proc_macro_attribute]
pub fn melbi_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    melbi_fn::melbi_fn_impl(attr, item)
}
