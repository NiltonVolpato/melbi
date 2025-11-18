//! String Package
//!
//! Provides string manipulation functions for Melbi.
//!
//! Design notes:
//! - String.Len returns UTF-8 codepoint count (not byte count)
//! - Upper/Lower are ASCII-only to keep binary size minimal
//! - For full Unicode support, use the Unicode package
//! - Format strings (f"...") are built into the language, not library functions

use crate::{
    String, Vec,
    evaluator::ExecutionError,
    types::manager::TypeManager,
    values::{dynamic::Value, from_raw::TypeError, function::NativeFunction, typed::Str},
};
use bumpalo::Bump;
use melbi_macros::melbi_fn;

// ============================================================================
// Inspection Functions
// ============================================================================

/// Get the length of a string (number of UTF-8 codepoints, not bytes)
fn string_len<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let len = s.chars().count() as i64;
    Ok(Value::int(type_mgr, len))
}

/// Check if string is empty
fn string_is_empty<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    Ok(Value::bool(type_mgr, s.is_empty()))
}

/// Check if haystack contains needle
fn string_contains<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let haystack = args[0].as_str().unwrap();
    let needle = args[1].as_str().unwrap();
    Ok(Value::bool(type_mgr, haystack.contains(needle)))
}

/// Check if string starts with prefix
fn string_starts_with<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let prefix = args[1].as_str().unwrap();
    Ok(Value::bool(type_mgr, s.starts_with(prefix)))
}

/// Check if string ends with suffix
fn string_ends_with<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let suffix = args[1].as_str().unwrap();
    Ok(Value::bool(type_mgr, s.ends_with(suffix)))
}

// ============================================================================
// Transformation Functions (ASCII-only for minimal binary size)
// ============================================================================

/// Convert string to uppercase (ASCII-only)
#[melbi_fn(name = "Upper")]
fn string_upper<'a, 't>(arena: &'a Bump, _type_mgr: &'t TypeManager, s: Str<'a>) -> Str<'a> {
    let upper = s.to_ascii_uppercase();
    Str::from_str(arena, &upper)
}

/// Convert string to lowercase (ASCII-only)
fn string_lower<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let lower = s.to_ascii_lowercase();
    Ok(Value::str(arena, type_mgr.str(), &lower))
}

/// Trim whitespace from both ends
fn string_trim<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let trimmed = s.trim();
    Ok(Value::str(arena, type_mgr.str(), trimmed))
}

/// Trim whitespace from start
fn string_trim_start<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let trimmed = s.trim_start();
    Ok(Value::str(arena, type_mgr.str(), trimmed))
}

/// Trim whitespace from end
fn string_trim_end<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let trimmed = s.trim_end();
    Ok(Value::str(arena, type_mgr.str(), trimmed))
}

/// Replace all occurrences of pattern with replacement
fn string_replace<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let from = args[1].as_str().unwrap();
    let to = args[2].as_str().unwrap();
    let replaced = s.replace(from, to);
    Ok(Value::str(arena, type_mgr.str(), &replaced))
}

/// Replace first N occurrences of pattern with replacement
fn string_replace_n<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let from = args[1].as_str().unwrap();
    let to = args[2].as_str().unwrap();
    let count = args[3].as_int().unwrap() as usize;
    let replaced = s.replacen(from, to, count);
    Ok(Value::str(arena, type_mgr.str(), &replaced))
}

// ============================================================================
// Splitting and Joining
// ============================================================================

/// Split string by delimiter
///
/// Special case: empty delimiter splits into individual characters (codepoints)
fn string_split<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let delimiter = args[1].as_str().unwrap();

    let parts: Vec<Value> = if delimiter.is_empty() {
        // Empty delimiter: split into individual characters (codepoints)
        s.chars()
            .map(|c| {
                let char_str = alloc::string::String::from(c);
                Value::str(arena, type_mgr.str(), &char_str)
            })
            .collect()
    } else {
        // Non-empty delimiter: use standard split
        s.split(delimiter)
            .map(|part| Value::str(arena, type_mgr.str(), part))
            .collect()
    };

    let array_ty = type_mgr.array(type_mgr.str());
    Ok(Value::array(arena, array_ty, &parts)
        .expect("String.Split: array construction should not fail with correct types"))
}

/// Join array of strings with separator
fn string_join<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let parts_array = args[0].as_array().unwrap();
    let separator = args[1].as_str().unwrap();

    // First collect the Values, then extract &str from each
    let values: Vec<Value> = (0..parts_array.len())
        .map(|i| parts_array.get(i).unwrap())
        .collect();

    let strings: Vec<&str> = values.iter().map(|v| v.as_str().unwrap()).collect();

    let joined = strings.join(separator);
    Ok(Value::str(arena, type_mgr.str(), &joined))
}

// ============================================================================
// Extraction
// ============================================================================

/// Extract substring by codepoint indices (not byte indices)
fn string_substring<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();
    let start = args[1].as_int().unwrap() as usize;
    let end = args[2].as_int().unwrap() as usize;

    // Convert to char indices
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    // Clamp indices
    let start = start.min(len);
    let end = end.min(len);

    if start >= end {
        return Ok(Value::str(arena, type_mgr.str(), ""));
    }

    let substring: String = chars[start..end].iter().collect();
    Ok(Value::str(arena, type_mgr.str(), &substring))
}

// ============================================================================
// Parsing
// ============================================================================

/// Parse string to integer
fn string_to_int<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();

    match s.parse::<i64>() {
        Ok(value) => {
            let int_val = Value::int(type_mgr, value);
            let option_ty = type_mgr.option(type_mgr.int());
            Ok(Value::optional(arena, option_ty, Some(int_val))
                .expect("String.ToInt: Option construction should not fail"))
        }
        Err(_) => {
            let option_ty = type_mgr.option(type_mgr.int());
            Ok(Value::optional(arena, option_ty, None)
                .expect("String.ToInt: Option construction should not fail"))
        }
    }
}

/// Parse string to float
fn string_to_float<'types, 'arena>(
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let s = args[0].as_str().unwrap();

    match s.parse::<f64>() {
        Ok(value) => {
            let float_val = Value::float(type_mgr, value);
            let option_ty = type_mgr.option(type_mgr.float());
            Ok(Value::optional(arena, option_ty, Some(float_val))
                .expect("String.ToFloat: Option construction should not fail"))
        }
        Err(_) => {
            let option_ty = type_mgr.option(type_mgr.float());
            Ok(Value::optional(arena, option_ty, None)
                .expect("String.ToFloat: Option construction should not fail"))
        }
    }
}

// ============================================================================
// Package Builder
// ============================================================================

/// Build the String package as a record containing all string functions.
///
/// The package includes:
/// - Inspection: Len (codepoints), IsEmpty, Contains, StartsWith, EndsWith
/// - Transformation: Upper (ASCII), Lower (ASCII), Trim variants, Replace
/// - Splitting/Joining: Split, Join
/// - Extraction: Substring
/// - Parsing: ToInt, ToFloat
///
/// # Example
///
/// ```ignore
/// let string = build_string_package(arena, type_mgr)?;
/// env.register("String", string)?;
/// ```
pub fn build_string_package<'arena>(
    arena: &'arena Bump,
    type_mgr: &'arena TypeManager<'arena>,
) -> Result<Value<'arena, 'arena>, TypeError> {
    use crate::values::function::AnnotatedFunction;

    let string_ty = type_mgr.str();
    let int_ty = type_mgr.int();
    let bool_ty = type_mgr.bool();
    let string_array_ty = type_mgr.array(string_ty);

    let mut builder = Value::record_builder(type_mgr);

    // Inspection
    let len_ty = type_mgr.function(&[string_ty], int_ty);
    builder = builder.field(
        "Len",
        Value::function(arena, NativeFunction::new(len_ty, string_len)).unwrap(),
    );

    let is_empty_ty = type_mgr.function(&[string_ty], bool_ty);
    builder = builder.field(
        "IsEmpty",
        Value::function(arena, NativeFunction::new(is_empty_ty, string_is_empty)).unwrap(),
    );

    let contains_ty = type_mgr.function(&[string_ty, string_ty], bool_ty);
    builder = builder.field(
        "Contains",
        Value::function(arena, NativeFunction::new(contains_ty, string_contains)).unwrap(),
    );

    let starts_with_ty = type_mgr.function(&[string_ty, string_ty], bool_ty);
    builder = builder.field(
        "StartsWith",
        Value::function(
            arena,
            NativeFunction::new(starts_with_ty, string_starts_with),
        )
        .unwrap(),
    );

    let ends_with_ty = type_mgr.function(&[string_ty, string_ty], bool_ty);
    builder = builder.field(
        "EndsWith",
        Value::function(arena, NativeFunction::new(ends_with_ty, string_ends_with)).unwrap(),
    );

    // Transformation
    builder = Upper::new(type_mgr).register(arena, builder)?;

    let lower_ty = type_mgr.function(&[string_ty], string_ty);
    builder = builder.field(
        "Lower",
        Value::function(arena, NativeFunction::new(lower_ty, string_lower)).unwrap(),
    );

    let trim_ty = type_mgr.function(&[string_ty], string_ty);
    builder = builder.field(
        "Trim",
        Value::function(arena, NativeFunction::new(trim_ty, string_trim)).unwrap(),
    );

    let trim_start_ty = type_mgr.function(&[string_ty], string_ty);
    builder = builder.field(
        "TrimStart",
        Value::function(arena, NativeFunction::new(trim_start_ty, string_trim_start)).unwrap(),
    );

    let trim_end_ty = type_mgr.function(&[string_ty], string_ty);
    builder = builder.field(
        "TrimEnd",
        Value::function(arena, NativeFunction::new(trim_end_ty, string_trim_end)).unwrap(),
    );

    let replace_ty = type_mgr.function(&[string_ty, string_ty, string_ty], string_ty);
    builder = builder.field(
        "Replace",
        Value::function(arena, NativeFunction::new(replace_ty, string_replace)).unwrap(),
    );

    let replace_n_ty = type_mgr.function(&[string_ty, string_ty, string_ty, int_ty], string_ty);
    builder = builder.field(
        "ReplaceN",
        Value::function(arena, NativeFunction::new(replace_n_ty, string_replace_n)).unwrap(),
    );

    // Splitting and Joining
    let split_ty = type_mgr.function(&[string_ty, string_ty], string_array_ty);
    builder = builder.field(
        "Split",
        Value::function(arena, NativeFunction::new(split_ty, string_split)).unwrap(),
    );

    let join_ty = type_mgr.function(&[string_array_ty, string_ty], string_ty);
    builder = builder.field(
        "Join",
        Value::function(arena, NativeFunction::new(join_ty, string_join)).unwrap(),
    );

    // Extraction
    let substring_ty = type_mgr.function(&[string_ty, int_ty, int_ty], string_ty);
    builder = builder.field(
        "Substring",
        Value::function(arena, NativeFunction::new(substring_ty, string_substring)).unwrap(),
    );

    // Parsing
    let to_int_ty = type_mgr.function(&[string_ty], type_mgr.option(int_ty));
    builder = builder.field(
        "ToInt",
        Value::function(arena, NativeFunction::new(to_int_ty, string_to_int)).unwrap(),
    );

    let to_float_ty = type_mgr.function(&[string_ty], type_mgr.option(type_mgr.float()));
    builder = builder.field(
        "ToFloat",
        Value::function(arena, NativeFunction::new(to_float_ty, string_to_float)).unwrap(),
    );

    builder.build(arena)
}

#[cfg(test)]
#[path = "string_test.rs"]
mod string_test;
