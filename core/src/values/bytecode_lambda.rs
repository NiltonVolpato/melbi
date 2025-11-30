//! Bytecode lambda function implementation for closures.
//!
//! This module defines `BytecodeLambda` which represents Melbi lambdas compiled to bytecode.
//! When called, it creates a new VM to execute the lambda's Code with captures.

use super::dynamic::Value;
use super::function::Function;
use crate::evaluator::ExecutionError;
use crate::types::{Type, manager::TypeManager};
use crate::values::RawValue;
use crate::vm::{Code, VM};
use bumpalo::Bump;

/// A bytecode-compiled lambda function value.
///
/// Stores the lambda's type signature, compiled Code, and captured values.
/// When called, it creates a new VM with captures and executes the Code.
///
/// # Closure Support
///
/// Lambdas can capture variables from their enclosing scope. Captured variables are stored
/// as a slice of RawValues and passed to the VM when the lambda is called.
pub struct BytecodeLambda<'types, 'arena> {
    /// The function's type signature (Function type)
    ty: &'types Type<'types>,

    /// The compiled lambda Code
    code: &'arena Code<'types>,

    /// Captured values from the enclosing scope
    captures: &'arena [RawValue],
}

impl<'types, 'arena> BytecodeLambda<'types, 'arena> {
    /// Create a new bytecode lambda.
    ///
    /// # Parameters
    ///
    /// - `ty`: The function's type (must be a Function type)
    /// - `code`: The compiled lambda Code
    /// - `captures`: Captured values from the enclosing scope
    pub fn new(
        ty: &'types Type<'types>,
        code: &'arena Code<'types>,
        captures: &'arena [RawValue],
    ) -> Self {
        debug_assert!(
            matches!(ty, Type::Function { .. }),
            "BytecodeLambda type must be Function"
        );

        Self { ty, code, captures }
    }
}

impl<'types, 'arena> Function<'types, 'arena> for BytecodeLambda<'types, 'arena> {
    fn ty(&self) -> &'types Type<'types> {
        self.ty
    }

    #[allow(unsafe_code)]
    unsafe fn call_unchecked(
        &self,
        arena: &'arena Bump,
        _type_mgr: &'types TypeManager<'types>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        // Collect arguments as locals
        let locals = args.iter().map(|arg| arg.as_raw()).collect();

        // Create VM with locals and captures, then execute
        let mut vm = VM::new(arena, self.code, locals, self.captures);
        let result = vm.run()?;

        // Convert RawValue back to Value using function return type
        use crate::types::traits::{TypeKind, TypeView};
        let return_type = match self.ty.view() {
            TypeKind::Function { ret, .. } => ret,
            _ => unreachable!("BytecodeLambda type must be Function"),
        };

        // SAFETY: The type is derived from self.ty's return type, which is correct
        // for the result of evaluating the lambda body.
        Ok(unsafe { Value::from_raw(return_type, result) })
    }
}
