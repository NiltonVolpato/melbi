---
title: Units of Measurement - Technical Design Document
---

# Design Doc: Units of Measurement (Physical Quantities)

**Author**: Claude (with Nilton)

**Date**: 10-31-2025

## Introduction

### Background

Physical quantities with units of measurement are ubiquitous in configuration files, scientific computing, systems programming, and data analysis. Currently, users must handle unit conversions manually and rely on comments or naming conventions to track what units their values represent. This is error-prone and leads to bugs like the infamous Mars Climate Orbiter crash ($327M loss due to meter/feet confusion).

**Problem**: Melbi needs a way to express physical quantities with units (e.g., `` 42`kg` ``, `` 9.81`m/s^2` ``) and validate dimensional consistency at compile-time.

**Solution**: Implement compile-time dimensional analysis through type-level dimensions, similar to Rust's `uom` crate but adapted for Melbi's Hindley-Milner type system.

**Stakeholders**:
- **Language users**: Scientists, engineers, DevOps engineers writing config files
- **Type system**: Requires extension to support dimensional parameters
- **Parser/Analyzer**: Need to validate and normalize suffix expressions
- **Runtime**: Must handle unit conversion efficiently

### Current Functionality

Currently:
- ✅ **Parser**: Fully supports suffix syntax (e.g., `` 42`kg` ``) via PEST grammar
- ✅ **AST**: Stores suffixes as `Literal::Int/Float { value, suffix: Option<&Expr> }`
- ❌ **Analyzer**: Explicitly rejects suffixes with "not yet supported" error
- ❌ **Type System**: No concept of dimensions or physical quantities
- ❌ **Runtime**: No unit handling

### In Scope

**Phase 1** (Milestone 1 - Syntactic Validation):
- Validate suffix expressions (allow only identifiers, integers, `*`, `/`, `^`)
- Pretty-print suffixes compactly (no spaces)
- Comprehensive error messages for invalid suffixes

**Phase 2** (Milestone 2 - Formatter):
- Add comprehensive formatter tests
- Verify idempotency with suffixes

**Phase 3** (Milestone 3 - Semantic Analysis):
- Extend type system with dimensional parameters
- Implement `Dimension` struct with SI base unit exponents
- Build unit registry for known units (SI + common derived units)
- Normalize suffix expressions to dimension records
- Type-check dimensional compatibility (e.g., reject `` 1`m` + 1`kg` ``)
- Handle unit conversion to base units at compile-time
- Runtime value storage in base units

### Out of Scope

- **Temperature units**: Too complex (affine/offset-based), deferred indefinitely
- **Currency**: Requires exchange rates, separate from physical dimensions
- **User-defined dimension systems**: Will not be supported. Units will be built into the language.
- **Formatting flags**: Format spec for quantities with specific units will be considered later.

### Assumptions & Dependencies

**Assumptions**:
- Users understand basic dimensional analysis (mass, length, time, etc.)
- SI base units are sufficient for most use cases
- Type inference can handle dimensional parameters without explicit annotations in common cases
- `i64`/`f64` ranges are sufficient for practical physical quantities

**Dependencies**:
- ✅ PEST parser (already supports suffix syntax)
- ✅ Tree-sitter grammar (already correct, no changes needed)
- ✅ Topiary formatter (already fixed - marks integers/floats as `@leaf`)
- 🔄 Type system extensibility (need to add dimensional parameters)
- 🔄 Error reporting infrastructure (for dimensional mismatch errors)

### Terminology

- **Suffix**: Expression in backticks after a numeric literal (e.g., `` `kg` `` in `` 42`kg` ``)
- **Dimension**: Combination of base unit exponents (e.g., `{length:1, time:-1}` for velocity)
- **Base unit**: Fundamental SI unit (meter, kilogram, second, ampere, kelvin, mole, candela)
- **Derived unit**: Combination of base units (e.g., Newton = kg·m/s²)
- **Quantity**: Numeric value with associated dimensions (e.g., `` 42`kg` `` is a quantity)
- **Dimensionless**: Value with no dimensions (plain number, all exponents are zero)
- **Normalization (syntactic)**: Pretty-printing suffix as written
- **Normalization (semantic)**: Converting suffix expression to canonical dimension record

## Considerations

### Concerns

**Type System Complexity**:
- Adding dimensional parameters to the type system increases complexity significantly
- Type inference must handle dimension unification (e.g., inferring that `` (x) => x + 5`m` `` requires `x` to have length dimension)
- Error messages must clearly explain dimensional mismatches
- Parametric polymorphism interactions (can a generic function `identity(x)` work with quantities?)

**Performance**:
- Dimensional analysis must not add significant overhead to type-checking
- Runtime value storage should be compact (avoid boxing dimensions separately)
- Unit conversion calculations must be compile-time only (no runtime cost)

**User Experience**:
- Users must understand dimensional analysis concepts
- Error messages must be pedagogical, not cryptic
- Common mistakes (like `` 1`cm` `` with Int base unit `m`) must have helpful suggestions

**Precision Loss**:
- `` 1`cm` `` with Int type and meter base unit would require 0.01, which is impossible
- Must be a clear type error, not silent rounding or runtime error
- Users must understand when to use `Float` vs `Int`

### Operational Readiness Considerations

**Deployment**:
- This is a language feature, not a service deployment
- Rolled out with a new language version
- Requires documentation, examples, and migration guide

**Metrics**: Not applicable (compiler/language feature)

**Testing Strategy**:
- Unit tests for dimension arithmetic (`{length:1} * {time:-1} = {length:1, time:-1}`)
- Integration tests for type-checking with dimensions
- Error message tests (verify helpful output for common mistakes)
- Property-based tests for dimension normalization (commutative, associative)
- Formatter idempotency tests with suffixes

**Debugging**:
- Type errors show both user's original suffix and normalized dimensions
- Debug mode can show full type inference trace for quantities
- Pretty-printed type signatures include dimensions

### Open Questions

None - all design decisions have been finalized.

### Cross-Region Considerations

Not applicable (single-region compiler/language implementation).

## Proposed Design

### Solution

Implement compile-time dimensional analysis through **type-level dimensions** integrated with Melbi's Hindley-Milner type system.

**High-level approach**:

1. **Syntactic layer** (Milestone 1): Validate suffix expressions contain only identifiers, integers, and operators (`*`, `/`, `^`)

2. **Type system extension** (Milestone 3):
   - Add `Dimension` struct with SI base unit exponents
   - Extend `Type` enum with quantity types: `Quantity<T, Dimension>`
   - Modify type inference to handle dimensional unification

3. **Semantic normalization** (Milestone 3):
   - Parse suffix expressions into dimension records
   - Normalize values to SI base units at compile-time
   - Type-check dimensional compatibility

4. **Runtime representation** (Milestone 3):
   - Store values in base units (no runtime dimension tracking needed)
   - Values are plain `i64`/`f64` after compilation

### System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Source Code                             │
│                      42`kg` + 10`kg`                            │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      PEST Parser                                │
│   Literal::Int { value: 42, suffix: Some(Ident("kg")) }         │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Suffix Validator (Milestone 1)                 │
│   - Check: only identifiers, ints, *, /, ^                      │
│   - Reject: parentheses, quoted idents, other exprs             │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                Type Checker / Analyzer (Milestone 3)            │
│   1. Parse suffix `kg` → lookup in unit registry → {mass: 1}    │
│   2. Convert value to base unit: 42 kg → 42 (already in kg)     │
│   3. Type: Quantity<Int, mass=1>                                │
│   4. Check compatibility: Quantity<Int, mass=1> + Quantity<...> │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Typed Expression                           │
│   Type: Quantity<Int, mass=1>                                   │
│   Value: 52 (in base units: kilograms)                          │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Runtime (VM)                             │
│   Plain i64 value: 52                                           │
│   (No dimension tracking at runtime - caught by type checker)   │
└─────────────────────────────────────────────────────────────────┘
```

**Component interactions**:
- Parser → Suffix Validator → Type Checker → Runtime
- Type Checker maintains `UnitRegistry` (singleton)
- `TypeManager` extended to intern `Quantity<T, Dimension>` types
- Error reporting at every stage with clear messages

### Data Model

#### Dimension Struct

```rust
/// Represents a physical dimension as exponents of SI base units.
///
/// Examples:
/// - Velocity (m/s): { length: 1, time: -1, ...rest: 0 }
/// - Force (kg·m/s²): { mass: 1, length: 1, time: -2, ...rest: 0 }
/// - Data rate (MB/s): { information: 1, time: -1, ...rest: 0 }
/// - Dimensionless: { all fields: 0 }
///
/// Note: Field order (information, mass, length, time) determines display order.
/// This ensures natural representations like "MB/s" instead of "s^-1*MB".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dimension {
    pub information: i8,   // bit (base unit for bytes/bits)
    pub mass: i8,          // kilogram (kg)
    pub length: i8,        // meter (m)
    pub time: i8,          // second (s)
}

impl Dimension {
    /// Dimensionless (all exponents zero)
    pub const DIMENSIONLESS: Self = Dimension {
        information: 0,
        mass: 0,
        length: 0,
        time: 0,
    };

    /// Common dimensions
    pub const INFORMATION: Self = Dimension { information: 1, ..DIMENSIONLESS };
    pub const MASS: Self = Dimension { mass: 1, ..DIMENSIONLESS };
    pub const LENGTH: Self = Dimension { length: 1, ..DIMENSIONLESS };
    pub const TIME: Self = Dimension { time: 1, ..DIMENSIONLESS };

    /// Multiply dimensions (add exponents)
    /// Returns error if any exponent would overflow i8 range
    pub fn multiply(&self, other: &Dimension) -> Result<Dimension, DimensionError> { /* ... */ }

    /// Divide dimensions (subtract exponents)
    /// Returns error if any exponent would overflow i8 range
    pub fn divide(&self, other: &Dimension) -> Result<Dimension, DimensionError> { /* ... */ }

    /// Power of dimension (multiply exponents)
    /// Returns error if any exponent would overflow i8 range
    pub fn pow(&self, exp: i8) -> Result<Dimension, DimensionError> { /* ... */ }

    /// Check if dimensionless
    pub fn is_dimensionless(&self) -> bool {
        *self == Self::DIMENSIONLESS
    }
}

#[derive(Debug)]
pub enum DimensionError {
    /// Exponent overflow (exceeds i8 range: -128 to 127)
    ExponentOverflow {
        dimension_name: &'static str,  // e.g., "length", "mass", "time"
    },
}
```

#### Unit Registry

```rust
/// Maps unit names (like "kg", "m", "Hz") to their dimension and conversion factors.
pub struct UnitRegistry {
    /// Map from dimension to its base unit name
    /// e.g., Dimension { length: 1, .. } -> "m"
    ///       Dimension { mass: 1, .. } -> "kg"
    base_units: HashMap<Dimension, &'static str>,

    /// Map from unit name to its conversion info
    units: HashMap<&'static str, ConversionInfo>,
}

/// Represents a unit's dimension and conversion factors to base units.
/// Used both for individual units in the registry and for compound units after parsing.
#[derive(Debug, Clone)]
pub struct ConversionInfo {
    /// Dimension of this unit (e.g., "kg" has {mass: 1}, "m/s" has {length: 1, time: -1})
    pub dimension: Dimension,

    /// Conversion factor for Float types (always available)
    /// e.g., "km" has factor 1000.0, "miles/minute^2" has factor 0.44704
    pub float_factor: f64,

    /// Conversion factor for Int types (if representable exactly)
    /// None means this unit requires Float type (e.g., "inch" = 2.54cm, or after operations that lose exactness)
    pub int_factor: Option<IntConversion>,
}

/// Integer conversion to base unit (avoids floating-point rounding errors)
#[derive(Debug, Clone, Copy)]
pub enum IntConversion {
    /// Multiply by this value (e.g., "km" multiplies by 1000)
    Multiply(i64),

    /// Divide by this value (e.g., "cm" divides by 100)
    /// Conversion fails if value is not exactly divisible
    Divide(i64),
}

impl ConversionInfo {
    /// Multiply two conversion infos (for `unit1 * unit2`)
    pub fn multiply(&self, other: &ConversionInfo) -> Result<ConversionInfo, DimensionError> {
        Ok(ConversionInfo {
            dimension: self.dimension.multiply(&other.dimension)?,
            float_factor: self.float_factor * other.float_factor,
            int_factor: combine_int_multiply(self.int_factor, other.int_factor),
        })
    }

    /// Divide two conversion infos (for `unit1 / unit2`)
    pub fn divide(&self, other: &ConversionInfo) -> Result<ConversionInfo, DimensionError> {
        Ok(ConversionInfo {
            dimension: self.dimension.divide(&other.dimension)?,
            float_factor: self.float_factor / other.float_factor,
            int_factor: combine_int_divide(self.int_factor, other.int_factor),
        })
    }

    /// Raise conversion info to a power (for `unit^n`)
    pub fn pow(&self, exp: i8) -> Result<ConversionInfo, DimensionError> {
        Ok(ConversionInfo {
            dimension: self.dimension.pow(exp)?,
            float_factor: self.float_factor.powi(exp as i32),
            int_factor: combine_int_pow(self.int_factor, exp),
        })
    }
}

impl UnitRegistry {
    /// Get the global unit registry instance
    pub fn global() -> &'static UnitRegistry;

    /// Parse a suffix expression into full conversion info (dimension + factors)
    /// Example: `miles/minute^2` → ConversionInfo {
    ///     dimension: {length:1, time:-2},
    ///     float_factor: 0.44704,
    ///     int_factor: None,
    /// }
    pub fn parse_suffix(&self, expr: &Expr) -> Result<ConversionInfo, UnitError> {
        match expr {
            Expr::Ident(name) => {
                self.units.get(name)
                    .ok_or_else(|| UnitError::UnknownUnit {
                        name: name.to_string(),
                        suggestions: vec![], // TODO: fuzzy match suggestions
                    })
                    .cloned()
            }
            Expr::Binary { op: BinaryOp::Mul, left, right } => {
                let left_info = self.parse_suffix(left)?;
                let right_info = self.parse_suffix(right)?;
                left_info.multiply(&right_info)
                    .map_err(|e| UnitError::DimensionError(e))
            }
            Expr::Binary { op: BinaryOp::Div, left, right } => {
                let left_info = self.parse_suffix(left)?;
                let right_info = self.parse_suffix(right)?;
                left_info.divide(&right_info)
                    .map_err(|e| UnitError::DimensionError(e))
            }
            Expr::Binary { op: BinaryOp::Pow, left, right } => {
                let base_info = self.parse_suffix(left)?;
                // Extract exponent from integer literal
                let exp = match right {
                    Expr::Literal(Literal::Int { value, suffix: None }) => {
                        i8::try_from(*value).map_err(|_| UnitError::ExponentTooLarge)?
                    }
                    _ => return Err(UnitError::InvalidExponent),
                };
                base_info.pow(exp)
                    .map_err(|e| UnitError::DimensionError(e))
            }
            _ => Err(UnitError::InvalidSuffixExpression),
        }
    }

    /// Convert Float value to base unit using conversion info
    pub fn convert_float_to_base(&self, value: f64, info: &ConversionInfo) -> f64 {
        value * info.float_factor
    }

    /// Convert Int value to base unit using conversion info (may fail if not exactly representable)
    pub fn convert_int_to_base(&self, value: i64, info: &ConversionInfo) -> Result<i64, ConversionError> {
        match info.int_factor {
            Some(IntConversion::Multiply(factor)) => {
                value.checked_mul(factor)
                    .ok_or(ConversionError::Overflow {
                        value,
                        factor
                    })
            }
            Some(IntConversion::Divide(divisor)) => {
                if value % divisor != 0 {
                    Err(ConversionError::FractionalResult {
                        value,
                        divisor
                    })
                } else {
                    Ok(value / divisor)
                }
            }
            None => {
                Err(ConversionError::RequiresFloat)
            }
        }
    }
}

// Helper functions to combine integer conversion factors
fn combine_int_multiply(a: Option<IntConversion>, b: Option<IntConversion>) -> Option<IntConversion> {
    match (a, b) {
        (Some(IntConversion::Multiply(x)), Some(IntConversion::Multiply(y))) => {
            x.checked_mul(y).map(IntConversion::Multiply)
        }
        (Some(IntConversion::Divide(x)), Some(IntConversion::Divide(y))) => {
            x.checked_mul(y).map(IntConversion::Divide)
        }
        _ => None,  // Mixed or None, requires Float
    }
}

fn combine_int_divide(a: Option<IntConversion>, b: Option<IntConversion>) -> Option<IntConversion> {
    match (a, b) {
        (Some(IntConversion::Multiply(x)), Some(IntConversion::Multiply(y))) => {
            // x / y - check if evenly divisible
            if x % y == 0 {
                Some(IntConversion::Multiply(x / y))
            } else if y % x == 0 {
                Some(IntConversion::Divide(y / x))
            } else {
                None
            }
        }
        (Some(IntConversion::Divide(x)), Some(IntConversion::Divide(y))) => {
            // (1/x) / (1/y) = y/x
            if y % x == 0 {
                Some(IntConversion::Multiply(y / x))
            } else if x % y == 0 {
                Some(IntConversion::Divide(x / y))
            } else {
                None
            }
        }
        (Some(IntConversion::Multiply(x)), Some(IntConversion::Divide(y))) => {
            // x / (1/y) = x*y
            x.checked_mul(y).map(IntConversion::Multiply)
        }
        (Some(IntConversion::Divide(x)), Some(IntConversion::Multiply(y))) => {
            // (1/x) / y = 1/(x*y)
            x.checked_mul(y).map(IntConversion::Divide)
        }
        _ => None,
    }
}

fn combine_int_pow(base: Option<IntConversion>, exp: i8) -> Option<IntConversion> {
    match (base, exp) {
        (Some(IntConversion::Multiply(x)), exp) if exp > 0 => {
            (x as i64).checked_pow(exp as u32).map(IntConversion::Multiply)
        }
        (Some(IntConversion::Divide(x)), exp) if exp > 0 => {
            (x as i64).checked_pow(exp as u32).map(IntConversion::Divide)
        }
        (Some(IntConversion::Multiply(x)), exp) if exp < 0 => {
            (x as i64).checked_pow((-exp) as u32).map(IntConversion::Divide)
        }
        (Some(IntConversion::Divide(x)), exp) if exp < 0 => {
            (x as i64).checked_pow((-exp) as u32).map(IntConversion::Multiply)
        }
        _ => None,  // exp == 0 or overflow
    }
}

#[derive(Debug)]
pub enum UnitError {
    UnknownUnit { name: String, suggestions: Vec<String> },
    DimensionError(DimensionError),
    ExponentTooLarge,
    InvalidExponent,
    InvalidSuffixExpression,
}

#[derive(Debug)]
pub enum ConversionError {
    /// Value overflow when multiplying to base unit
    Overflow { value: i64, factor: i64 },

    /// Value not exactly divisible (results in fraction)
    FractionalResult { value: i64, divisor: i64 },

    /// Unit requires Float type (no exact integer conversion)
    RequiresFloat,
}
```

#### Type System Extension

```rust
// Current Type enum in core/src/types/types.rs
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type<'a> {
    Int,
    Float,
    // ... other types

    // NEW: Physical quantity with dimensions
    Quantity {
        base_type: &'a Type<'a>,  // Int or Float
        dimension: Dimension,
    },
}

impl Display for Type<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Type::Quantity { base_type: Type::Int, dimension } => {
                write!(f, "Int{}", dimension)  // e.g., "Int[m/s]"
            }
            Type::Quantity { base_type: Type::Float, dimension } => {
                write!(f, "Float{}", dimension)  // e.g., "Float[kg*m/s^2]"
            }
            // ...
        }
    }
}

// Dimension display (compact suffix-like format)
impl Display for Dimension {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.is_dimensionless() {
            return Ok(());  // Don't display anything for dimensionless
        }

        write!(f, "[")?;
        // Build compact representation: [kg*m/s^2]
        // ...implementation details
        write!(f, "]")
    }
}
```

### Interface / API Definitions

#### Suffix Validation API (Milestone 1)

```rust
// Module: core/src/parser/suffix.rs

/// Validate that a suffix expression contains only allowed constructs.
pub fn validate_suffix(expr: &Expr) -> Result<(), SuffixError>;

/// Pretty-print a suffix expression (compact, no spaces).
pub fn print_suffix(expr: &Expr) -> String;

#[derive(Debug)]
pub enum SuffixError {
    /// Disallowed expression type in suffix
    InvalidExpression {
        found: String,           // e.g., "lambda expression"
        suggestion: String,      // e.g., "use only identifiers and operators *, /, ^"
    },

    /// Parentheses are not allowed
    ParenthesesNotAllowed {
        message: String,
    },

    /// Quoted identifiers are not allowed
    QuotedIdentifierNotAllowed {
        identifier: String,
    },
}
```

#### Unit Registry API (Milestone 3)

```rust
// Module: core/src/units/registry.rs

impl UnitRegistry {
    /// Get the global unit registry instance
    pub fn global() -> &'static UnitRegistry;

    /// Parse a suffix expression into a Dimension
    pub fn parse_suffix<'a>(
        &self,
        expr: &Expr<'a>,
    ) -> Result<DimensionInfo, UnitError>;

    /// Check if a value can be represented with Int type in given unit
    pub fn check_precision(
        &self,
        value: i64,
        unit: &str,
        base_unit: &str,
    ) -> Result<(), PrecisionError>;
}

pub struct DimensionInfo {
    pub dimension: Dimension,
    pub conversion_factor: f64,  // To convert original value to base units
}

#[derive(Debug)]
pub enum UnitError {
    UnknownUnit { name: String, suggestions: Vec<String> },
    PrecisionLoss { value: i64, from_unit: String, to_unit: String },
    DimensionalMismatch { expected: Dimension, got: Dimension },
}
```

#### Type Checker Integration (Milestone 3)

```rust
// Extends: core/src/analyzer/analyzer.rs

impl<'types, 'arena> Analyzer<'types, 'arena> {
    /// Analyze a literal with optional suffix
    fn analyze_literal(
        &mut self,
        literal: &parser::Literal<'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        match literal {
            Literal::Int { value, suffix: Some(suffix_expr) } => {
                // 1. Validate suffix syntax
                validate_suffix(suffix_expr)?;

                // 2. Parse suffix to dimension
                let dim_info = self.unit_registry.parse_suffix(suffix_expr)?;

                // 3. Check precision (can we represent in Int with base units?)
                if dim_info.conversion_factor != 1.0 {
                    // e.g., 1cm → 0.01m (requires Float)
                    return Err(PrecisionError { ... });
                }

                // 4. Create Quantity type
                let qty_type = self.type_manager.quantity(
                    self.type_manager.int(),
                    dim_info.dimension,
                );

                // 5. Create typed expression with converted value
                Ok(self.alloc(qty_type, ExprInner::Constant(value)))
            }
            // ... similar for Float
        }
    }

    /// Type-check binary operations with quantities
    fn analyze_binary_add(
        &mut self,
        left: &'arena Expr<'types, 'arena>,
        right: &'arena Expr<'types, 'arena>,
    ) -> Result<&'arena Expr<'types, 'arena>, Error> {
        let left_type = left.ty();
        let right_type = right.ty();

        match (left_type, right_type) {
            (Type::Quantity { dimension: dim1, .. },
             Type::Quantity { dimension: dim2, .. }) => {
                // Dimensions must match for addition
                if dim1 != dim2 {
                    return Err(DimensionalMismatchError {
                        operation: "addition",
                        left_dim: dim1,
                        right_dim: dim2,
                    });
                }
                // Result has same dimension
                Ok(...)
            }
            _ => // ... handle other cases
        }
    }
}
```

### Business Logic

#### Exponent Overflow Validation

Dimension exponents are stored as `i8` (range: -128 to 127). All dimension arithmetic operations must validate that results stay within this range:

```rust
// During multiplication: m^100 * m^50
let left_dim = Dimension { length: 100, ..DIMENSIONLESS };
let right_dim = Dimension { length: 50, ..DIMENSIONLESS };
// 100 + 50 = 150 > 127 → ERROR

// Error message:
Error: Dimension exponent overflow
  Expression: m^100 * m^50
  The 'length' dimension would have exponent 150, which exceeds the maximum of 127
```

**Validation points**:
- During `Dimension::multiply()` (addition of exponents)
- During `Dimension::divide()` (subtraction of exponents)
- During `Dimension::pow()` (multiplication of exponents)
- During suffix parsing (e.g., `m^200` directly in suffix)

**Rationale**: Using i8 for exponents is sufficient for all practical physical quantities. Overflow indicates either a bug or nonsensical expression (e.g., `m^1000` has no physical meaning).

#### Dimensional Arithmetic

Core rules for operations on quantities:

**Addition/Subtraction**: Dimensions must match exactly
```
Quantity<T, D> + Quantity<T, D> → Quantity<T, D>
5`m` + 3`m` → 8`m`  ✅
5`m` + 3`kg` → ERROR  ❌
```

**Multiplication**: Dimensions add (exponents add)
```
Quantity<T, D1> * Quantity<T, D2> → Quantity<T, D1 + D2>
5`m` * 3`s` → 15`m*s`  (dimension: {length:1, time:1})
5`m` * 3`m` → 15`m^2`  (dimension: {length:2})
```

**Division**: Dimensions subtract (exponents subtract)
```
Quantity<T, D1> / Quantity<T, D2> → Quantity<T, D1 - D2>
10`m` / 2`s` → 5`m/s`  (dimension: {length:1, time:-1})
10`m` / 2`m` → 5       (dimension: dimensionless)
```

**Exponentiation**: Dimension exponents multiply
```
Quantity<T, D> ^ n → Quantity<T, D * n>
(5`m`)^2 → 25`m^2`  (dimension: {length:2})
```

**Dimensionless results**: When dimensions cancel out, result is plain numeric type
```
5`m` / 2`m` → 2  (Int, dimensionless)
```

#### Base Unit Conversion Algorithm

**For Float types:**
```
1. Parse suffix expression to Dimension using unit registry
2. Look up float_factor for the unit
3. Convert: value_in_base = value * float_factor
4. Type: Quantity<Float, dimension>
```

**For Int types:**
```
1. Parse suffix expression to Dimension using unit registry
2. Look up int_factor for the unit
3. If int_factor is None:
   → ERROR: "Unit 'X' requires Float type (not exactly representable as Int)"
4. If int_factor is Some(Multiply(factor)):
   a. Check overflow: value.checked_mul(factor)
   b. If overflow → ERROR: "Value overflow when converting to base unit"
   c. Otherwise: value_in_base = value * factor
5. If int_factor is Some(Divide(divisor)):
   a. Check exact division: value % divisor == 0
   b. If not exact → ERROR: "Cannot represent {value} {unit} as Int (fractional result)"
   c. Otherwise: value_in_base = value / divisor
6. Type: Quantity<Int, dimension>
```

**Examples:**

```melbi
100`km`  (Int)
→ dimension: {length: 1}
→ int_factor: Some(Multiply(1000))
→ 100 * 1000 = 100000 ✅ (no overflow)
→ value_in_base: 100000
→ type: Quantity<Int, length=1>
```

```melbi
1`cm`  (Int)
→ dimension: {length: 1}
→ int_factor: Some(Divide(100))
→ 1 % 100 != 0 → fractional result ❌
→ ERROR: "Cannot represent 1cm as Int (would be 0.01m). Use Float or multiple of 100cm."
```

```melbi
100`cm`  (Int)
→ dimension: {length: 1}
→ int_factor: Some(Divide(100))
→ 100 % 100 == 0 ✅ (exactly divisible)
→ value_in_base: 100 / 100 = 1
→ type: Quantity<Int, length=1>
```

```melbi
1`inch`  (Int)
→ dimension: {length: 1}
→ int_factor: None (1 inch = 2.54cm, not exact integer conversion)
→ ERROR: "Unit 'inch' requires Float type (not exactly representable as Int)"
```

**Note on divisibility checking:**
Allowing `100`cm`` but rejecting `1`cm`` with Int type is intentional. While users might be surprised that `100cm / 2 == 0cm` (integer division), this is consistent with regular integer arithmetic and the error from `100cm + 1cm` will make the limitation clear.

#### Bits/Bytes Special Case

Information quantity has special display logic:

**Storage**: Always in bits (base unit)
```
1B → 8 bits (stored as 8)
1bit → 1 bit (stored as 1)
```

**Display**: Smart formatting
```
if value % 8 == 0:
    display as bytes: 8bit → 1B, 16bit → 2B
else:
    display as bits: 7bit → 7bit, 9bit → 9bit
```

**Type checking**: Same as other dimensions
```
8bit + 8bit → 16bit (displays as 2B)
1B + 1bit → 9bit
```

### Migration Strategy

Not applicable (new feature, no existing code to migrate).

Users can adopt gradually:
1. Start using suffixes in new code
2. Optionally add suffixes to existing numeric literals for better type safety
3. No breaking changes to existing code without suffixes

### Work Required

#### Milestone 1: Syntactic Validation (~2-3 days)

**Scope**: Create suffix validation module and pretty printer

**Key components**:
- `core/src/parser/suffix.rs` module
- `validate_suffix()` function - allows only identifiers, integers, and operators `*`, `/`, `^`
- `print_suffix()` function - compact output with no spaces
- `SuffixError` enum with clear error messages

**Team**: 1 developer
**Dependencies**: None

#### Milestone 2: Formatter Testing (~1 day)

**Scope**: Add comprehensive formatter tests for suffix support

**Key components**:
- Test cases covering simple, complex, and edge cases
- Idempotency verification
- Documentation of formatting behavior

**Team**: 1 developer
**Dependencies**: Milestone 1

#### Milestone 3: Semantic Analysis (~1-2 weeks)

**Phase 3a: Dimension & Unit Registry (~3-4 days)**
- `Dimension` struct with arithmetic operations and overflow validation
- `UnitRegistry` with SI base units and common derived units
- Suffix parsing to dimension conversion

**Phase 3b: Type System Extension (~3-4 days)**
- Extend `Type` enum with `Quantity` variant
- Update `TypeManager` for quantity type interning
- Implement type unification for quantities

**Phase 3c: Analyzer Integration (~4-5 days)**
- Update literal analysis to handle suffixes
- Dimensional type checking for operations
- Precision loss detection (Int with fractional base units)
- Rich error messages showing dimensions

**Phase 3d: Runtime Support (~1-2 days)**
- Verify compiled code stores values in base units
- No runtime dimension tracking needed

**Team**: 1-2 developers
**Dependencies**: Type system expertise, error reporting infrastructure

### Work Sequence

**Sequential (must complete in order)**:
1. Milestone 1 → Milestone 2 → Milestone 3
2. Within Milestone 3: Phase 3a → 3b → 3c → 3d

**Can be parallelized**:
- Documentation writing can happen alongside Milestone 3 implementation
- Example code can be written during Milestone 2

### High-level Test Plan

#### Unit Tests
- **Suffix validation**: All valid cases pass, all invalid cases error with clear messages
- **Pretty printing**: Round-trip property: parse → print → parse produces same AST
- **Dimension arithmetic**: All operations (multiply, divide, pow) correct
- **Unit registry**: All SI units defined correctly, conversion factors accurate
- **Type unification**: Quantities unify correctly, dimensionless handled properly

#### Integration Tests
- **Type checking**: Valid expressions type-check, invalid expressions error
- **Error messages**: Dimensional mismatches show helpful messages
- **Precision checking**: `1cm` with Int errors, `1cm` with Float succeeds
- **Formatter**: Idempotency holds with suffixes, no extra spaces

#### Property-based Tests
- **Commutativity**: `a*b == b*a` for dimensions
- **Associativity**: `(a*b)*c == a*(b*c)` for dimensions
- **Distributivity**: Unit conversion is linear

#### End-to-end Tests
```melbi
// Should succeed
let speed = 100`km/h`
let distance = 50`km`
let time = distance / speed  // Type: Quantity<Int, time=1>

// Should fail: dimensional mismatch
let invalid = 5`m` + 3`kg`  // ERROR: Cannot add length and mass

// Should fail: precision loss
let cm_int = 1`cm`  // ERROR: Cannot represent 1cm as Int with base unit m

// Should succeed with Float
let cm_float = 1.0`cm`  // OK: Quantity<Float, length=1> = 0.01m
```

### Deployment Sequence

1. **Milestone 1**: Released as "syntax support" (parser + validation only)
   - Users can write suffixes, get validation errors
   - Analyzer still rejects (with updated error message)

2. **Milestone 2**: Formatter updates
   - Silent release (just fixes)

3. **Milestone 3**: Full semantic support
   - Major version bump (new language feature)
   - Documentation, examples, migration guide
   - Blog post explaining dimensional analysis

## Impact

### Performance

**Compile-time**:
- Type checking adds overhead for dimension unification
- Estimated <5% increase in type-check time (dimensions are small structs)
- Unit registry lookup is O(1) hash map access

**Runtime**:
- **Zero overhead**: Quantities compile to plain `i64`/`f64` values
- No runtime dimension tracking
- Conversion to base units happens at compile-time
- Same performance as hand-written code with manual conversions

**Memory**:
- `Dimension` struct: 8 bytes (8 × i8)
- `Type::Quantity`: 16 bytes (pointer + Dimension)
- Type interning keeps memory usage low

### Security

**No security impact**: Purely compile-time feature, no runtime attack surface.

**Potential benefits**:
- Prevents unit confusion bugs that could lead to unsafe behavior
- Strong typing reduces likelihood of misconfiguration

### Correctness

**Significantly improved correctness**:
- Compile-time detection of dimensional errors
- Prevents unit confusion bugs (Mars Climate Orbiter-style failures)
- Forces explicit conversions when mixing units

### Cost Analysis

**Development cost**: ~2-3 weeks of engineering time

**Ongoing cost**:
- Maintenance of unit registry (add new units as needed)
- Documentation updates
- Minimal ongoing cost (feature is self-contained)

**User cost**:
- Learning curve for dimensional analysis concepts
- Slight verbosity (must add suffixes)
- Offset by fewer bugs and better documentation

## Alternatives

### Alternative 1: Runtime Dimension Tracking (Value-level)

**Approach**: Store dimensions with values at runtime
```rust
struct Quantity {
    value: f64,
    dimension: Dimension,
}
```

**Pros**:
- Simpler type system changes
- Can inspect dimensions at runtime
- More flexible (dynamic dimension checking)

**Cons**:
- Runtime overhead (8 extra bytes per value)
- Errors detected late (at runtime, not compile-time)
- Doesn't leverage Melbi's static type system
- Incompatible with Melbi's "no runtime errors" philosophy

**Why rejected**: Melbi is statically typed with the goal of catching all errors at compile-time. Runtime dimension tracking contradicts this philosophy.

### Alternative 2: String-based Units (Like TOML/YAML)

**Approach**: Units as string annotations, no type-level tracking
```melbi
let timeout = "5s"  // Just a string
```

**Pros**:
- No type system changes
- Easy to implement

**Cons**:
- No type safety whatsoever
- Manual parsing required
- No dimensional validation
- Defeats purpose of units

**Why rejected**: Provides no value over comments, doesn't solve the core problem.

### Alternative 3: Macro/Preprocessor Approach

**Approach**: Units handled by macro expansion before type-checking
```melbi
units!(42`kg`)  // Expands to runtime check or plain number
```

**Pros**:
- Flexible
- No core language changes

**Cons**:
- Not first-class language feature
- Poor error messages
- Requires macro system (Melbi doesn't have one)
- Still doesn't leverage type system

**Why rejected**: Macros add complexity, and we want first-class language support.

### Alternative 4: No Normalization (Keep Original Units)

**Approach**: Store values in original units, convert on-demand

**Pros**:
- Preserves user intent
- No precision loss from conversion

**Cons**:
- Type equality becomes complex (`1m` != `100cm` as types?)
- Runtime conversion overhead
- Complicates type checking significantly

**Why rejected**: Normalization to base units is standard practice (uom crate), simplifies type system.

## Looking into the Future

### Next Steps (Post-MVP)

**Short-term**:
1. **More derived units**: Add common engineering units (psi, mph, kWh, etc.)
2. **Unit prefixes**: Support SI prefixes declaratively (kilo, mega, milli, etc.)
3. **Explicit conversion function**: `Convert(value, target_unit)` for output
4. **Dimension inference**: Infer dimensions from context when possible

**Medium-term**:
5. **Binary/hexadecimal suffixes**: `0xFF`bytes`` for byte counts
6. **Custom unit systems**: Allow users to define domain-specific units
7. **Linter rules**: Warn about uncommon units, suggest alternatives
8. **IDE support**: Auto-completion for common units

**Long-term**:
9. **Temperature support**: Absolute vs relative (like date vs duration)
10. **Currency**: Special handling (exchange rates, not physical dimensions)
11. **Compile-time unit optimization**: Display `m*m` as `m^2`
12. **Cross-quantity operations**: Energy-mass equivalence (E=mc²)

### Nice to Haves

- **Unit documentation**: Hover over `kg` in IDE shows "kilogram, SI unit of mass"
- **Automatic unit suggestion**: If user writes `42m`, suggest adding suffix: `42`m``
- **Physics equations**: Built-in constants with units (`SPEED_OF_LIGHT = 299792458`m/s``)
- **Dimensional analysis teaching mode**: Explain why `1m + 1kg` is invalid
- **Unit conversion table**: Generate documentation of all supported units

### Evolutionary Path

The design allows natural evolution:
1. Start simple (SI base units only)
2. Add derived units incrementally
3. Extend to domain-specific units later
4. Optional features (temperature, currency) can be added without breaking existing code

The architecture is extensible and follows Melbi's philosophy of compile-time correctness.
