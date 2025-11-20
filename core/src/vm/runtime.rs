use bumpalo::Bump;

use super::instruction_set::Instruction;

use crate::{
    String,
    Vec,
    format,
    evaluator::{ExecutionError, ExecutionErrorKind, RuntimeError},
    values::{ArrayData, MapData, RawValue, RecordData},
    vm::Stack,
};

pub struct Code {
    pub constants: Vec<RawValue>,
    pub instructions: Vec<Instruction>,
    pub num_locals: usize,
    pub max_stack_size: usize,
}

impl core::fmt::Debug for Code {
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

struct OtherwiseBlock {
    fallback: *const Instruction,
    stack_size: usize,
}

pub struct VM<'a, 'b> {
    arena: &'a Bump,
    code: &'b Code,
    stack: Stack<RawValue>,
    locals: Vec<RawValue>,
    error: Option<ExecutionError>,
    otherwise_stack: Vec<OtherwiseBlock>,
}

impl<'a, 'b> VM<'a, 'b> {
    pub fn new(arena: &'a Bump, code: &'b Code) -> Self {
        VM {
            arena: arena,
            code,
            stack: Stack::new(code.max_stack_size),
            locals: Vec::new(),
            error: None,
            otherwise_stack: Vec::new(),
        }
    }

    pub fn run(&mut self) -> Result<RawValue, ExecutionError> {
        let result = self.run_internal();
        debug_assert!(self.stack.is_empty(), "Stack should be empty.");
        result
    }

    pub fn run_internal(&mut self) -> Result<RawValue, ExecutionError> {
        let mut wide_arg: u64 = 0;
        let mut p = unsafe { self.code.instructions.as_ptr().sub(1) };
        loop {
            loop {
                p = unsafe { p.add(1) };

                use Instruction::*;
                match unsafe { *p } {
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

                    // Integer unary operations
                    NegInt => {
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            int_value: unsafe { -a.int_value },
                        });
                    }
                    IncInt => {
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            int_value: unsafe { a.int_value + 1 },
                        });
                    }
                    DecInt => {
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            int_value: unsafe { a.int_value - 1 },
                        });
                    }

                    // Integer comparisons
                    IntCmpOp(b'<') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.int_value < b.int_value },
                        });
                    }
                    IntCmpOp(b'>') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.int_value > b.int_value },
                        });
                    }
                    IntCmpOp(b'=') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.int_value == b.int_value },
                        });
                    }
                    IntCmpOp(b'!') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.int_value != b.int_value },
                        });
                    }
                    IntCmpOp(b'l') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.int_value <= b.int_value },
                        });
                    }
                    IntCmpOp(b'g') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.int_value >= b.int_value },
                        });
                    }

                    // Float binary operations
                    FloatBinOp(b'+') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            float_value: unsafe { a.float_value + b.float_value },
                        });
                    }
                    FloatBinOp(b'-') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            float_value: unsafe { a.float_value - b.float_value },
                        });
                    }
                    FloatBinOp(b'*') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            float_value: unsafe { a.float_value * b.float_value },
                        });
                    }
                    FloatBinOp(b'/') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            float_value: unsafe { a.float_value / b.float_value },
                        });
                    }
                    FloatBinOp(b'^') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            float_value: unsafe { a.float_value.powf(b.float_value) },
                        });
                    }

                    NegFloat => {
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            float_value: unsafe { -a.float_value },
                        });
                    }

                    // Float comparisons
                    FloatCmpOp(b'<') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.float_value < b.float_value },
                        });
                    }
                    FloatCmpOp(b'>') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.float_value > b.float_value },
                        });
                    }
                    FloatCmpOp(b'=') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.float_value == b.float_value },
                        });
                    }
                    FloatCmpOp(b'!') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.float_value != b.float_value },
                        });
                    }
                    FloatCmpOp(b'l') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.float_value <= b.float_value },
                        });
                    }
                    FloatCmpOp(b'g') => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.float_value >= b.float_value },
                        });
                    }

                    // Logical operations
                    And => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.bool_value && b.bool_value },
                        });
                    }
                    Or => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.bool_value || b.bool_value },
                        });
                    }
                    Not => {
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { !a.bool_value },
                        });
                    }
                    EqBool => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(RawValue {
                            bool_value: unsafe { a.bool_value == b.bool_value },
                        });
                    }

                    // Stack operations
                    Dup => {
                        let val = *self.stack.peek().unwrap();
                        self.stack.push(val);
                    }
                    DupN(depth) => {
                        let val = *self.stack.peek_at(depth as usize).unwrap();
                        self.stack.push(val);
                    }
                    Pop => {
                        self.stack.pop().unwrap();
                    }
                    Swap => {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack.push(b);
                        self.stack.push(a);
                    }

                    // Local variables
                    LoadLocal(index) => {
                        self.stack.push(self.locals[index as usize]);
                    }
                    StoreLocal(index) => {
                        let val = self.stack.pop().unwrap();
                        if self.locals.len() <= index as usize {
                            self.locals
                                .resize(index as usize + 1, RawValue { int_value: 0 });
                        }
                        self.locals[index as usize] = val;
                    }

                    // Control flow
                    Jump(offset) => {
                        p = unsafe { p.offset(offset as isize) };
                    }
                    JumpIfFalse(offset) => {
                        let cond = self.stack.pop().unwrap();
                        if unsafe { !cond.bool_value } {
                            p = unsafe { p.offset(offset as isize) };
                        }
                    }
                    JumpIfTrue(offset) => {
                        let cond = self.stack.pop().unwrap();
                        if unsafe { cond.bool_value } {
                            p = unsafe { p.offset(offset as isize) };
                        }
                    }
                    JumpIfFalseNoPop(offset) => {
                        let cond = *self.stack.peek().unwrap();
                        if unsafe { !cond.bool_value } {
                            p = unsafe { p.offset(offset as isize) };
                        }
                    }
                    JumpIfTrueNoPop(offset) => {
                        let cond = *self.stack.peek().unwrap();
                        if unsafe { cond.bool_value } {
                            p = unsafe { p.offset(offset as isize) };
                        }
                    }

                    Halt => {
                        break;
                    }
                    Return => {
                        break;
                    }

                    // === Otherwise Error Handling ===
                    PushOtherwise(delta) => {
                        // Calculate fallback instruction pointer
                        let fallback_ip = unsafe { p.offset(delta as isize) };

                        // Push handler onto otherwise_stack
                        self.otherwise_stack.push(OtherwiseBlock {
                            fallback: fallback_ip,
                            stack_size: self.stack.len(),
                        });
                    }

                    PopOtherwise => {
                        // Remove the top otherwise handler (called in fallback code)
                        self.otherwise_stack.pop()
                            .expect("PopOtherwise called with empty otherwise_stack");
                    }

                    PopOtherwiseAndJump(delta) => {
                        // Remove the otherwise handler (not needed, primary succeeded)
                        self.otherwise_stack.pop()
                            .expect("PopOtherwiseAndJump called with empty otherwise_stack");

                        // Jump past fallback code to done label
                        p = unsafe { p.offset(delta as isize) };
                    }

                    Nop => {
                        // No operation
                    }

                    MakeArray(len) => {
                        let len = len as usize;
                        let array = ArrayData::new_with(self.arena, self.stack.top_n(len).unwrap());
                        self.stack.pop_n(len);
                        self.stack.push(array.as_raw_value());
                    }

                    // TODO: Complex operations to implement later
                    LoadUpvalue(_) | StoreUpvalue(_) => todo!("Upvalues for closures"),
                    Call(_) | CallNative(_) | TailCall(_) => todo!("Function calls"),
                    MakeClosure(_) => todo!("Closure creation"),

                    // === Array Operations ===
                    ArrayGet => {
                        // Stack: [..., array, index] -> [..., element]
                        let index_raw = self.stack.pop().expect("Stack underflow");
                        let array_raw = self.stack.pop().expect("Stack underflow");

                        let array = ArrayData::from_raw_value(array_raw);
                        let index = unsafe { index_raw.int_value };

                        // Check bounds
                        if index < 0 || index as usize >= array.length() {
                            self.error = Some(ExecutionError {
                                kind: RuntimeError::IndexOutOfBounds {
                                    index,
                                    len: array.length(),
                                }.into(),
                                source: String::new(),
                                span: crate::parser::Span(0..0),
                            });
                            break;
                        }

                        let element = unsafe { array.get(index as usize) };
                        self.stack.push(element);
                    }

                    ArrayGetConst(const_index) => {
                        // Stack: [..., array] -> [..., element]
                        let array_raw = self.stack.pop().expect("Stack underflow");
                        let array = ArrayData::from_raw_value(array_raw);
                        let index = const_index as usize;

                        // Check bounds
                        if index >= array.length() {
                            self.error = Some(ExecutionError {
                                kind: RuntimeError::IndexOutOfBounds {
                                    index: index as i64,
                                    len: array.length(),
                                }.into(),
                                source: String::new(),
                                span: crate::parser::Span(0..0),
                            });
                            break;
                        }

                        let element = unsafe { array.get(index) };
                        self.stack.push(element);
                    }

                    ArrayLen | ArrayConcat | ArraySlice | ArrayAppend => {
                        todo!("Other array operations")
                    }

                    // === Map Operations ===
                    MapGet => {
                        // Stack: [..., map, key] -> [..., value]
                        let key = self.stack.pop().expect("Stack underflow");
                        let map_raw = self.stack.pop().expect("Stack underflow");
                        let map = MapData::from_raw_value(map_raw);

                        // Linear search for the key
                        // TODO: Use binary search since map is sorted
                        let mut found = None;
                        for i in 0..map.length() {
                            let entry_key = unsafe { map.get_key(i) };
                            // For now, do simple bitwise comparison
                            // This works for Int, Bool, and other primitive types
                            // TODO: Proper value equality for complex types
                            if unsafe { entry_key.int_value == key.int_value } {
                                found = Some(unsafe { map.get_value(i) });
                                break;
                            }
                        }

                        match found {
                            Some(value) => self.stack.push(value),
                            None => {
                                // Format key for error message (simple int display for now)
                                let key_display = format!("{}", unsafe { key.int_value });
                                self.error = Some(ExecutionError {
                                    kind: RuntimeError::KeyNotFound { key_display }.into(),
                                    source: String::new(),
                                    span: crate::parser::Span(0..0),
                                });
                                break;
                            }
                        }
                    }

                    MakeMap(num_pairs) => {
                        // Stack: [..., key1, val1, key2, val2, ..., keyN, valN] -> [..., map]
                        use crate::Vec;
                        use crate::values::raw::MapEntry;

                        let num_pairs = num_pairs as usize;
                        let num_values = num_pairs * 2;

                        // Get all key-value pairs from stack
                        let values = self.stack.top_n(num_values).unwrap();

                        // Create MapEntry structs
                        let mut entries: Vec<MapEntry> = Vec::with_capacity(num_pairs);
                        for i in 0..num_pairs {
                            let key_idx = i * 2;
                            let val_idx = i * 2 + 1;
                            entries.push(MapEntry {
                                key: values[key_idx],
                                value: values[val_idx],
                            });
                        }

                        // Sort entries by key (integer comparison for now)
                        // TODO: Proper multi-type key comparison
                        entries.sort_by(|a, b| unsafe { a.key.int_value.cmp(&b.key.int_value) });

                        // Create the map
                        let map = MapData::new_with_sorted(self.arena, &entries);

                        // Pop the 2*N elements
                        self.stack.pop_n(num_values);

                        // Push the map result
                        self.stack.push(map.as_raw_value());
                    }

                    MapLen | MapHas | MapInsert | MapRemove | MapKeys | MapValues => {
                        todo!("Other map operations")
                    }

                    // === Record Operations ===
                    MakeRecord(num_fields) => {
                        // Stack: [..., val0, val1, ..., valN] -> [..., record]
                        let num_fields = num_fields as usize;
                        // Get the top N elements to create the record
                        let record =
                            RecordData::new_with(self.arena, self.stack.top_n(num_fields).unwrap());
                        // Pop the N elements that were used to create the record
                        self.stack.pop_n(num_fields);
                        // Push the record result
                        self.stack.push(record.as_raw_value());
                    }

                    RecordGet(field_index) => {
                        // Stack: [..., record] -> [..., field_value]
                        let record_raw = self.stack.pop().expect("Stack underflow");
                        let record = RecordData::from_raw_value(record_raw);
                        let index = field_index as usize;

                        // Check bounds
                        if index >= record.length() {
                            self.error = Some(ExecutionError {
                                kind: RuntimeError::IndexOutOfBounds {
                                    index: index as i64,
                                    len: record.length(),
                                }.into(),
                                source: String::new(),
                                span: crate::parser::Span(0..0),
                            });
                            break;
                        }

                        let field_value = unsafe { record.get(index) };
                        self.stack.push(field_value);
                    }

                    RecordSet(_) | RecordMerge => {
                        todo!("Other record operations")
                    }
                    StringOp(_) | StringLen | StringContains | StringFind | StringUpper
                    | StringLower | StringTrim | StringSplit | StringFormat(_) | StringCmpOp(_) => {
                        todo!("String operations")
                    }
                    BytesConcat | BytesLen | BytesGet | BytesGetConst(_) | BytesSlice
                    | StringToBytes | BytesToString | BytesCmpOp(_) => todo!("Bytes operations"),
                    Cast(_) | TypeOf | TypeCheck(_) | Otherwise | IsError | Eq | NotEq => {
                        todo!("Type/error operations")
                    }
                    MatchBegin | MatchLiteral(_) | MatchConstructor(_) | MatchArray(_)
                    | MatchRecord(_) | MatchWildcard | MatchGuard => todo!("Pattern matching"),
                    Breakpoint(_) | CheckLimits | Trace(_) | InlineCache(_) => {
                        todo!("Debug/meta operations")
                    }

                    _ => {
                        panic!("Unsupported operation: {:?}", unsafe { *p });
                    }
                }
                wide_arg = 0;
            }
            match self.error.take() {
                Some(e) => {
                    // If we are within an area that is covered by an `otherwise` block
                    // then `otherwise_stack` will be non empty.
                    if let Some(block) = self.otherwise_stack.last() {
                        match e.kind {
                            ExecutionErrorKind::Runtime(_) => {
                                p = block.fallback;
                                self.stack.pop_n(self.stack.len() - block.stack_size);
                                continue;
                            }
                            _ => {} // `otherwise` can only handle `Runtime` error kind.
                        }
                    }
                    self.stack.clear();
                    return Err(e);
                }
                None => {
                    return Ok(self.stack.pop().unwrap());
                }
            }
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

    #[test]
    fn test_int_comparisons() {
        use Instruction::*;

        // Test <
        let code = Code {
            constants: vec![],
            instructions: vec![ConstInt(5), ConstInt(10), IntCmpOp(b'<'), Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().bool_value, true) };

        // Test ==
        let code = Code {
            constants: vec![],
            instructions: vec![ConstInt(42), ConstInt(42), IntCmpOp(b'='), Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().bool_value, true) };
    }

    #[test]
    fn test_float_ops() {
        use Instruction::*;

        let code = Code {
            constants: vec![RawValue { float_value: 3.5 }, RawValue { float_value: 2.0 }],
            instructions: vec![ConstLoad(0), ConstLoad(1), FloatBinOp(b'+'), Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().float_value, 5.5) };
    }

    #[test]
    fn test_logical_ops() {
        use Instruction::*;

        // Test AND
        let code = Code {
            constants: vec![],
            instructions: vec![ConstTrue, ConstFalse, And, Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().bool_value, false) };

        // Test OR
        let code = Code {
            constants: vec![],
            instructions: vec![ConstTrue, ConstFalse, Or, Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().bool_value, true) };

        // Test NOT
        let code = Code {
            constants: vec![],
            instructions: vec![ConstFalse, Not, Return],
            num_locals: 0,
            max_stack_size: 1,
        };
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().bool_value, true) };
    }

    #[test]
    fn test_stack_ops() {
        use Instruction::*;

        // Test Dup
        let code = Code {
            constants: vec![],
            instructions: vec![ConstInt(42), Dup, IntBinOp(b'+'), Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 84) };

        // Test Swap
        let code = Code {
            constants: vec![],
            instructions: vec![ConstInt(10), ConstInt(5), Swap, IntBinOp(b'-'), Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, -5) };
    }

    #[test]
    fn test_local_vars() {
        use Instruction::*;

        // Store and load local variable
        let code = Code {
            constants: vec![],
            instructions: vec![
                ConstInt(42),
                StoreLocal(0),
                ConstInt(10),
                LoadLocal(0),
                IntBinOp(b'+'),
                Return,
            ],
            num_locals: 1,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 52) };
    }

    #[test]
    fn test_jumps() {
        use Instruction::*;

        // Unconditional jump
        let code = Code {
            constants: vec![],
            instructions: vec![
                ConstInt(1),
                Jump(2),      // Skip next 2 instructions
                ConstInt(50), // Skipped
                ConstInt(60), // Skipped
                ConstInt(3),
                IntBinOp(b'+'),
                Return,
            ],
            num_locals: 0,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 4) };
    }

    #[test]
    fn test_conditional_jumps() {
        use Instruction::*;

        // JumpIfTrue - should jump
        let code = Code {
            constants: vec![],
            instructions: vec![
                ConstTrue,
                JumpIfTrue(1), // Skip next instruction
                ConstInt(99),  // Skipped
                ConstInt(42),
                Return,
            ],
            num_locals: 0,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 42) };

        // JumpIfFalse - should not jump
        let code = Code {
            constants: vec![],
            instructions: vec![
                ConstTrue,
                JumpIfFalse(1), // Don't jump
                ConstInt(42),
                Return,
                ConstInt(99),
            ],
            num_locals: 0,
            max_stack_size: 2,
        };
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 42) };
    }

    #[test]
    fn test_unary_ops() {
        use Instruction::*;

        // NegInt
        let code = Code {
            constants: vec![],
            instructions: vec![ConstInt(42), NegInt, Return],
            num_locals: 0,
            max_stack_size: 1,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, -42) };

        // IncInt
        let code = Code {
            constants: vec![],
            instructions: vec![ConstInt(41), IncInt, Return],
            num_locals: 0,
            max_stack_size: 1,
        };
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 42) };

        // DecInt
        let code = Code {
            constants: vec![],
            instructions: vec![ConstInt(43), DecInt, Return],
            num_locals: 0,
            max_stack_size: 1,
        };
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 42) };
    }
}
