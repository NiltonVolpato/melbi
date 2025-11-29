use alloc::boxed::Box;

use hashbrown::HashSet;

use crate::{
    Vec,
    values::RawValue,
    vm::{FunctionAdapter, GenericAdapter, Instruction},
};

pub struct Code<'t> {
    pub constants: Vec<RawValue>,
    /// Function call adapters (specialized for performance).
    pub adapters: Vec<FunctionAdapter<'t>>,
    /// Generic adapters for other operations (Cast, FormatStr, etc.).
    pub generic_adapters: Vec<Box<dyn GenericAdapter + 't>>,
    pub instructions: Vec<Instruction>,
    pub num_locals: usize,
    pub max_stack_size: usize,
}

/// Extract jump offset from an instruction, if it's a jump instruction.
fn get_jump_offset(instr: &Instruction) -> Option<u8> {
    match instr {
        Instruction::JumpForward(offset)
        | Instruction::PopJumpIfFalse(offset)
        | Instruction::PopJumpIfTrue(offset)
        | Instruction::PushOtherwise(offset)
        | Instruction::PopOtherwiseAndJump(offset)
        | Instruction::MatchSomeOrJump(offset)
        | Instruction::MatchNoneOrJump(offset) => Some(*offset),
        _ => None,
    }
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
                writeln!(f, "    [{}] = {:?}", i, constant)?;
            }
            writeln!(f, "  ]")?;
        } else {
            writeln!(f, "  constants: []")?;
        }

        // First pass: collect all jump targets to determine which addresses need labels
        let mut jump_targets: HashSet<usize> = HashSet::new();
        let mut wide_arg: usize = 0;

        for (addr, instr) in self.instructions.iter().enumerate() {
            if let Instruction::WideArg(high) = instr {
                wide_arg = (wide_arg | (*high as usize)) << 8;
                continue;
            }

            if let Some(offset) = get_jump_offset(instr) {
                let full_offset = wide_arg | (offset as usize);
                // Jump is relative to NEXT instruction: target = addr + 1 + offset
                let target = addr + 1 + full_offset;
                jump_targets.insert(target);
            }
            wide_arg = 0;
        }

        // Assign label numbers to targets (sorted for deterministic output)
        let mut sorted_targets: Vec<_> = jump_targets.into_iter().collect();
        sorted_targets.sort();
        let label_map: hashbrown::HashMap<usize, usize> = sorted_targets
            .into_iter()
            .enumerate()
            .map(|(i, addr)| (addr, i))
            .collect();

        // Second pass: print instructions with labels
        writeln!(f, "  instructions:")?;
        wide_arg = 0;

        for (addr, instr) in self.instructions.iter().enumerate() {
            // Print label if this address is a jump target
            let label_prefix = if let Some(&label_num) = label_map.get(&addr) {
                alloc::format!("L{}:", label_num)
            } else {
                alloc::string::String::new()
            };

            // Handle WideArg accumulation
            if let Instruction::WideArg(high) = instr {
                wide_arg = (wide_arg | (*high as usize)) << 8;
                writeln!(f, "    {:4} {:>4}  {:?}", addr, label_prefix, instr)?;
                continue;
            }

            // Format jump instructions with target label
            if let Some(offset) = get_jump_offset(instr) {
                let full_offset = wide_arg | (offset as usize);
                let target = addr + 1 + full_offset;
                let target_label = label_map
                    .get(&target)
                    .map(|l| alloc::format!("L{}", l))
                    .unwrap_or_else(|| alloc::format!("@{}", target));
                writeln!(
                    f,
                    "    {:4} {:>4}  {:?} (to {})",
                    addr, label_prefix, instr, target_label
                )?;
            } else {
                writeln!(f, "    {:4} {:>4}  {:?}", addr, label_prefix, instr)?;
            }
            wide_arg = 0;
        }

        write!(f, "}}")
    }
}
