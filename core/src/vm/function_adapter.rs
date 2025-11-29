use bumpalo::Bump;

use crate::{
    Vec,
    evaluator::ExecutionErrorKind,
    types::{Type, manager::TypeManager},
    values::{Function, RawValue, dynamic::Value},
    vm::GenericAdapter,
};

/// Melbi's VM doesn't have knowledge of types: it just executes instructions
/// over data in memory. However, to provide a type-safe API to FFI authors
/// we use a `FunctionAdapter` to add/remove types at the boundary between the
/// VM and the host language. From VM -> host language: add types, and from
/// host language -> VM: remove types.
pub struct FunctionAdapter<'t> {
    type_mgr: &'t TypeManager<'t>,
    types: Vec<&'t Type<'t>>,
}

impl<'t> FunctionAdapter<'t> {
    pub fn new(type_mgr: &'t TypeManager<'t>, types: Vec<&'t Type<'t>>) -> Self {
        FunctionAdapter { type_mgr, types }
    }
}

impl<'t> GenericAdapter for FunctionAdapter<'t> {
    fn num_args(&self) -> usize {
        // +1 for the function itself (last element in args)
        self.types.len() + 1
    }

    #[allow(unsafe_code)]
    fn call(&self, arena: &Bump, args: &[RawValue]) -> Result<RawValue, ExecutionErrorKind> {
        // Last element is the function, rest are arguments
        let (func, arguments) = args.split_last().expect("args should contain at least the function");
        let func = *func;

        let typed_args: Vec<_> = arguments
            .iter()
            .zip(self.types.iter())
            .map(|(arg, ty)| unsafe { Value::from_raw_unchecked(ty, *arg) })
            .collect();

        unsafe {
            let storage_ptr = func.function as *const *const dyn Function<'_, '_>;
            let func_ptr = *storage_ptr;
            let func = &*func_ptr;
            func.call_unchecked(arena, self.type_mgr, typed_args.as_slice())
                .map(|value| value.as_raw())
                .map_err(|e| e.kind)
        }
    }
}
