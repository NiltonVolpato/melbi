//! Cast adapter for type conversions in the VM.

use bumpalo::Bump;

use crate::{
    evaluator::ExecutionErrorKind,
    types::{Type, manager::TypeManager},
    values::{RawValue, dynamic::Value},
    vm::GenericAdapter,
};

/// Adapter for type cast operations (`value as Type`).
///
/// Stores the source and target types needed to perform the cast at runtime.
pub struct CastAdapter<'t> {
    type_mgr: &'t TypeManager<'t>,
    source_type: &'t Type<'t>,
    target_type: &'t Type<'t>,
}

impl<'t> CastAdapter<'t> {
    pub fn new(
        type_mgr: &'t TypeManager<'t>,
        source_type: &'t Type<'t>,
        target_type: &'t Type<'t>,
    ) -> Self {
        CastAdapter {
            type_mgr,
            source_type,
            target_type,
        }
    }
}

impl<'t> GenericAdapter for CastAdapter<'t> {
    fn num_args(&self) -> usize {
        1 // Just the value to cast
    }

    fn call(&self, arena: &Bump, args: &[RawValue]) -> Result<RawValue, ExecutionErrorKind> {
        let raw_value = args[0];

        // Convert RawValue to Value using source type
        let value = Value::from_raw_unchecked(self.source_type, raw_value);

        // Perform the cast using the casting library
        crate::casting::perform_cast(arena, value, self.target_type, self.type_mgr)
            .map(|v| v.as_raw())
            .map_err(ExecutionErrorKind::from)
    }
}
