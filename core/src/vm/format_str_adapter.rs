//! Format string adapter for f-string interpolation in the VM.

use alloc::boxed::Box;
use bumpalo::Bump;
use core::fmt::Write;

use crate::{
    String, Vec,
    evaluator::ExecutionErrorKind,
    types::{Type, manager::TypeManager},
    values::{RawValue, dynamic::Value},
    vm::GenericAdapter,
};

/// Adapter for format string operations (`f"Hello {name}"`).
///
/// Stores the expression types and string parts needed to format the string at runtime.
/// String parts are stored as owned `Box<str>` (immutable) to avoid lifetime constraints from the AST.
pub struct FormatStrAdapter<'t> {
    type_mgr: &'t TypeManager<'t>,
    /// Types of each expression to format (for Display conversion)
    expr_types: Vec<&'t Type<'t>>,
    /// String parts to interleave (len = expr_types.len() + 1), owned to avoid AST lifetime
    strs: Vec<Box<str>>,
}

impl<'t> FormatStrAdapter<'t> {
    pub fn new(type_mgr: &'t TypeManager<'t>, expr_types: &[&'t Type<'t>], strs: &[&str]) -> Self {
        debug_assert_eq!(
            strs.len(),
            expr_types.len() + 1,
            "strs.len() must be expr_types.len() + 1"
        );
        FormatStrAdapter {
            type_mgr,
            expr_types: expr_types.to_vec(),
            strs: strs.iter().map(|s| Box::from(*s)).collect(),
        }
    }
}

impl<'t> GenericAdapter for FormatStrAdapter<'t> {
    fn num_args(&self) -> usize {
        self.expr_types.len()
    }

    #[allow(unsafe_code)]
    fn call(&self, arena: &Bump, args: &[RawValue]) -> Result<RawValue, ExecutionErrorKind> {
        // Build result string: strs[0] + format(args[0]) + strs[1] + ...
        let mut result = String::new();
        result.push_str(&self.strs[0]);

        for (i, (raw, ty)) in args.iter().zip(self.expr_types.iter()).enumerate() {
            // Convert RawValue to Value for formatting
            let value = Value::from_raw_unchecked(ty, *raw);
            // Use Display trait (outputs strings without quotes)
            write!(result, "{}", value).expect("Writing to String should not fail");
            result.push_str(&self.strs[i + 1]);
        }

        // Allocate result in arena and return as RawValue
        let result_str = arena.alloc_str(&result);
        Ok(Value::str(arena, self.type_mgr.str(), result_str).as_raw())
    }
}
