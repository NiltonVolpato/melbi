use bumpalo::Bump;

use super::instruction_set::Instruction;

use crate::{Vec, evaluator::ExecutionError, values::RawValue, vm::Stack};

pub struct Code {
    pub constants: Vec<RawValue>,
    pub instructions: Vec<Instruction>,
    pub num_locals: usize,
    pub max_stack_size: usize,
}

pub struct VM<'a, 'b> {
    arena: &'a Bump,
    code: &'b Code,
    ip: usize,
    stack: Stack<RawValue>,
    locals: Vec<RawValue>,
}

impl<'a, 'b> VM<'a, 'b> {
    pub fn new(arena: &'a Bump, code: &'b Code) -> Self {
        VM {
            arena,
            code,
            ip: 0,
            stack: Stack::new(code.max_stack_size),
            locals: Vec::new(),
        }
    }

    pub fn run(&mut self) -> Result<RawValue, ExecutionError> {
        let mut wide_arg: u64 = 0;
        loop {
            let instruction = self.code.instructions[self.ip];
            self.ip += 1;

            use Instruction::*;
            match instruction {
                ConstLoad(index) => {
                    self.stack.push(self.code.constants[index as usize]);
                }
                ConstInt(value) => {
                    self.stack.push(RawValue {
                        int_value: wide_arg as i64 | value as i64,
                    });
                }
                ConstUInt(value) => {
                    self.stack.push(RawValue {
                        int_value: (wide_arg | value as u64) as i64,
                    });
                }
                ConstTrue => {
                    self.stack.push(RawValue { bool_value: true });
                }
                ConstFalse => {
                    self.stack.push(RawValue { bool_value: false });
                }
                WideArg(arg) => {
                    wide_arg |= arg as u64;
                    wide_arg <<= 8;
                    continue;
                }
                IntBinOp(b'+') => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value + b.int_value },
                    });
                }
                IntBinOp(b'-') => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value - b.int_value },
                    });
                }
                IntBinOp(b'*') => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value * b.int_value },
                    });
                }
                IntBinOp(b'/') => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value / b.int_value },
                    });
                }
                IntBinOp(b'%') => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value % b.int_value },
                    });
                }
                IntBinOp(b'^') => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value.pow(b.int_value.try_into().unwrap()) },
                    });
                }
                Return => {
                    return Ok(self.stack.pop().unwrap());
                }
                _ => {
                    panic!("Unsupported operation");
                }
            }
            wide_arg = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_works() {
        use Instruction::*;
        let code = Code {
            constants: vec![RawValue { int_value: 42 }],
            instructions: vec![ConstLoad(0), ConstInt(2), IntBinOp(b'*'), Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 84) };
    }

    #[test]
    fn test_wide() {
        use Instruction::*;
        let code = Code {
            constants: vec![RawValue { int_value: 2 }],
            instructions: vec![
                ConstLoad(0),
                WideArg(255),
                ConstUInt(255),
                IntBinOp(b'*'),
                Return,
            ],
            num_locals: 0,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 131070) };
    }
}
