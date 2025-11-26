use crate::{
    Vec,
    values::RawValue,
    vm::{FunctionAdapter, Instruction},
};

pub struct Code<'t> {
    pub constants: Vec<RawValue>,
    pub adapters: Vec<FunctionAdapter<'t>>,
    pub instructions: Vec<Instruction>,
    pub num_locals: usize,
    pub max_stack_size: usize,
}

impl core::fmt::Debug for Code<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Code {{")?;
        writeln!(f, "  num_locals: {}", self.num_locals)?;
        writeln!(f, "  max_stack_size: {}", self.max_stack_size)?;

        // Print constants pool
        if !self.constants.is_empty() {
            writeln!(f, "  constants: [")?;
            for (i, constant) in self.constants.iter().enumerate() {
                // Print raw value as hex since RawValue is a union
                writeln!(f, "    [{}] = 0x{:016x}", i, unsafe {
                    constant.int_value as u64
                })?;
            }
            writeln!(f, "  ]")?;
        } else {
            writeln!(f, "  constants: []")?;
        }

        // Print instructions in assembly style
        writeln!(f, "  instructions: [")?;
        for (addr, instr) in self.instructions.iter().enumerate() {
            writeln!(f, "    {:3}  {:?}", addr, instr)?;
        }
        writeln!(f, "  ]")?;
        write!(f, "}}")
    }
}
