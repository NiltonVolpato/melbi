use bumpalo::Bump;
use melbi_core::{errors::ErrorKind, types::Type, types::manager::TypeManager, values::RawValue};

pub struct Engine<'arena> {
    type_manager: &'arena TypeManager<'arena>,
}

impl<'arena> Engine<'arena> {
    pub fn new(arena: &'arena Bump) -> &'arena Self {
        arena.alloc(Self {
            type_manager: arena.alloc(TypeManager::new(arena)),
        })
    }

    pub fn type_manager(&self) -> &'arena TypeManager<'arena> {
        self.type_manager
    }

    pub fn compile(
        &self,
        _arena: &'arena Bump,
        _source: &str,
        _params: &[(&'arena str, &'arena Type<'arena>)],
    ) -> Result<CompiledExpression<'arena>, ErrorKind> {
        // Compilation logic would go here
        todo!()
    }
}

pub struct CompiledExpression<'arena> {
    source: &'arena str,
    params: &'arena [(&'arena str, &'arena Type<'arena>)],
    return_type: &'arena Type<'arena>,
}

impl<'arena> CompiledExpression<'arena> {
    pub fn run<'value>(
        &self,
        _arena: &'arena Bump,
        _args: &'value [RawValue],
    ) -> Result<RawValue, ErrorKind> {
        // Execution logic would go here
        todo!()
    }

    pub fn source(&self) -> &'arena str {
        self.source
    }

    pub fn params(&self) -> &'arena [(&'arena str, &'arena Type<'arena>)] {
        self.params
    }

    pub fn return_type(&self) -> &'arena Type<'arena> {
        self.return_type
    }
}
