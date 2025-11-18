//! Math Package
//!
//! Provides mathematical functions and constants for Melbi.
//!
//! Constants: PI, E, TAU, INFINITY, NAN
//! Functions: Abs, Min, Max, Clamp, Floor, Ceil, Round, Sqrt, Pow,
//!            Sin, Cos, Tan, Asin, Acos, Atan, Atan2, Log, Log10, Exp

use crate::{
    evaluator::ExecutionError,
    types::manager::TypeManager,
    values::{dynamic::Value, from_raw::TypeError, function::NativeFunction},
};
use bumpalo::Bump;

// ============================================================================
// Basic Operations
// ============================================================================

/// Absolute value of a float
fn math_abs<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::float(type_mgr, value.abs()))
}

/// Minimum of two floats
fn math_min<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let a = args[0].as_float().unwrap();
    let b = args[1].as_float().unwrap();
    Ok(Value::float(type_mgr, a.min(b)))
}

/// Maximum of two floats
fn math_max<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let a = args[0].as_float().unwrap();
    let b = args[1].as_float().unwrap();
    Ok(Value::float(type_mgr, a.max(b)))
}

/// Clamp a value between min and max
fn math_clamp<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    let min = args[1].as_float().unwrap();
    let max = args[2].as_float().unwrap();
    Ok(Value::float(type_mgr, value.clamp(min, max)))
}

// ============================================================================
// Rounding Functions
// ============================================================================

/// Floor function - returns largest integer <= x
fn math_floor<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::int(type_mgr, value.floor() as i64))
}

/// Ceiling function - returns smallest integer >= x
fn math_ceil<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::int(type_mgr, value.ceil() as i64))
}

/// Round to nearest integer
fn math_round<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::int(type_mgr, value.round() as i64))
}

// ============================================================================
// Exponentiation
// ============================================================================

/// Square root
fn math_sqrt<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    // Note: sqrt of negative returns NaN (IEEE 754 semantics)
    Ok(Value::float(type_mgr, value.sqrt()))
}

/// Power function - base^exp
fn math_pow<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let base = args[0].as_float().unwrap();
    let exp = args[1].as_float().unwrap();
    Ok(Value::float(type_mgr, base.powf(exp)))
}

// ============================================================================
// Trigonometry
// ============================================================================

/// Sine function
fn math_sin<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::float(type_mgr, value.sin()))
}

/// Cosine function
fn math_cos<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::float(type_mgr, value.cos()))
}

/// Tangent function
fn math_tan<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::float(type_mgr, value.tan()))
}

/// Arc sine function
fn math_asin<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::float(type_mgr, value.asin()))
}

/// Arc cosine function
fn math_acos<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::float(type_mgr, value.acos()))
}

/// Arc tangent function
fn math_atan<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::float(type_mgr, value.atan()))
}

/// Two-argument arc tangent function
fn math_atan2<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let y = args[0].as_float().unwrap();
    let x = args[1].as_float().unwrap();
    Ok(Value::float(type_mgr, y.atan2(x)))
}

// ============================================================================
// Logarithms
// ============================================================================

/// Natural logarithm (base e)
fn math_log<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::float(type_mgr, value.ln()))
}

/// Base-10 logarithm
fn math_log10<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::float(type_mgr, value.log10()))
}

/// Exponential function (e^x)
fn math_exp<'types, 'arena>(
    _arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    let value = args[0].as_float().unwrap();
    Ok(Value::float(type_mgr, value.exp()))
}

// ============================================================================
// Package Builder
// ============================================================================

/// Build the Math package as a record containing all math functions and constants.
///
/// The package includes:
/// - Constants: PI, E, TAU, INFINITY, NAN
/// - Basic operations: Abs, Min, Max, Clamp
/// - Rounding: Floor, Ceil, Round
/// - Exponentiation: Sqrt, Pow
/// - Trigonometry: Sin, Cos, Tan, Asin, Acos, Atan, Atan2
/// - Logarithms: Log, Log10, Exp
///
/// # Example
///
/// ```ignore
/// let math = build_math_package(arena, type_mgr)?;
/// env.register("Math", math)?;
/// ```
pub fn build_math_package<'arena>(
    arena: &'arena Bump,
    type_mgr: &'arena TypeManager<'arena>,
) -> Result<Value<'arena, 'arena>, TypeError> {
    let float_ty = type_mgr.float();
    let int_ty = type_mgr.int();

    let mut builder = Value::record_builder(type_mgr);

    // Constants
    builder = builder.field("PI", Value::float(type_mgr, core::f64::consts::PI));
    builder = builder.field("E", Value::float(type_mgr, core::f64::consts::E));
    builder = builder.field("TAU", Value::float(type_mgr, core::f64::consts::TAU));
    builder = builder.field("INFINITY", Value::float(type_mgr, f64::INFINITY));
    builder = builder.field("NAN", Value::float(type_mgr, f64::NAN));

    // Basic operations
    let abs_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Abs",
        Value::function(arena, NativeFunction::new(abs_ty, math_abs)).unwrap(),
    );

    let min_ty = type_mgr.function(&[float_ty, float_ty], float_ty);
    builder = builder.field(
        "Min",
        Value::function(arena, NativeFunction::new(min_ty, math_min)).unwrap(),
    );

    let max_ty = type_mgr.function(&[float_ty, float_ty], float_ty);
    builder = builder.field(
        "Max",
        Value::function(arena, NativeFunction::new(max_ty, math_max)).unwrap(),
    );

    let clamp_ty = type_mgr.function(&[float_ty, float_ty, float_ty], float_ty);
    builder = builder.field(
        "Clamp",
        Value::function(arena, NativeFunction::new(clamp_ty, math_clamp)).unwrap(),
    );

    // Rounding functions
    let floor_ty = type_mgr.function(&[float_ty], int_ty);
    builder = builder.field(
        "Floor",
        Value::function(arena, NativeFunction::new(floor_ty, math_floor)).unwrap(),
    );

    let ceil_ty = type_mgr.function(&[float_ty], int_ty);
    builder = builder.field(
        "Ceil",
        Value::function(arena, NativeFunction::new(ceil_ty, math_ceil)).unwrap(),
    );

    let round_ty = type_mgr.function(&[float_ty], int_ty);
    builder = builder.field(
        "Round",
        Value::function(arena, NativeFunction::new(round_ty, math_round)).unwrap(),
    );

    // Exponentiation
    let sqrt_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Sqrt",
        Value::function(arena, NativeFunction::new(sqrt_ty, math_sqrt)).unwrap(),
    );

    let pow_ty = type_mgr.function(&[float_ty, float_ty], float_ty);
    builder = builder.field(
        "Pow",
        Value::function(arena, NativeFunction::new(pow_ty, math_pow)).unwrap(),
    );

    // Trigonometry
    let sin_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Sin",
        Value::function(arena, NativeFunction::new(sin_ty, math_sin)).unwrap(),
    );

    let cos_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Cos",
        Value::function(arena, NativeFunction::new(cos_ty, math_cos)).unwrap(),
    );

    let tan_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Tan",
        Value::function(arena, NativeFunction::new(tan_ty, math_tan)).unwrap(),
    );

    let asin_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Asin",
        Value::function(arena, NativeFunction::new(asin_ty, math_asin)).unwrap(),
    );

    let acos_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Acos",
        Value::function(arena, NativeFunction::new(acos_ty, math_acos)).unwrap(),
    );

    let atan_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Atan",
        Value::function(arena, NativeFunction::new(atan_ty, math_atan)).unwrap(),
    );

    let atan2_ty = type_mgr.function(&[float_ty, float_ty], float_ty);
    builder = builder.field(
        "Atan2",
        Value::function(arena, NativeFunction::new(atan2_ty, math_atan2)).unwrap(),
    );

    // Logarithms
    let log_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Log",
        Value::function(arena, NativeFunction::new(log_ty, math_log)).unwrap(),
    );

    let log10_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Log10",
        Value::function(arena, NativeFunction::new(log10_ty, math_log10)).unwrap(),
    );

    let exp_ty = type_mgr.function(&[float_ty], float_ty);
    builder = builder.field(
        "Exp",
        Value::function(arena, NativeFunction::new(exp_ty, math_exp)).unwrap(),
    );

    builder.build(arena)
}

#[cfg(test)]
#[path = "math_test.rs"]
mod math_test;
