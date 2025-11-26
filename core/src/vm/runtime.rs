use bumpalo::Bump;

use super::instruction_set::Instruction;

use crate::{
    String, Vec,
    evaluator::{ExecutionError, ExecutionErrorKind, RuntimeError},
    format,
    parser::Span,
    values::{ArrayData, MapData, RawValue, RecordData},
    vm::{Code, Stack},
};

struct OtherwiseBlock {
    fallback: *const Instruction,
    stack_size: usize,
}

pub struct VM<'a, 'c> {
    arena: &'a Bump,
    code: &'c Code<'a>,
    ip: *const Instruction,
    stack: Stack<RawValue>,
    locals: Vec<RawValue>,
    otherwise_stack: Vec<OtherwiseBlock>,
}

impl<'a, 'c> VM<'a, 'c> {
    pub fn new(arena: &'a Bump, code: &'c Code<'a>) -> Self {
        VM {
            arena: arena,
            code,
            ip: unsafe { code.instructions.as_ptr().sub(1) },
            stack: Stack::new(code.max_stack_size),
            locals: Vec::new(),
            otherwise_stack: Vec::new(),
        }
    }

    pub fn execute(arena: &'a Bump, code: &Code<'a>) -> Result<RawValue, ExecutionError> {
        let mut vm = VM::new(arena, code);
        vm.run()
    }

    pub fn run(&mut self) -> Result<RawValue, ExecutionError> {
        let result = self.run_control_loop();
        debug_assert!(self.stack.is_empty(), "Stack should be empty.");
        result
    }

    #[inline(always)]
    fn run_control_loop(&mut self) -> Result<RawValue, ExecutionError> {
        loop {
            let result = self.run_main_loop();
            match result {
                Err(e) => {
                    // If we are within an area that is covered by an `otherwise` block
                    // then `otherwise_stack` will be non empty.
                    if let Some(block) = self.otherwise_stack.last() {
                        match e {
                            ExecutionErrorKind::Runtime(_) => {
                                self.ip = block.fallback;
                                self.stack.pop_n(self.stack.len() - block.stack_size);
                                continue;
                            }
                            _ => {} // `otherwise` can only handle `Runtime` error kind.
                        }
                    }
                    self.stack.clear();
                    return Err(e).map_err(|e| ExecutionError {
                        kind: e,
                        // TODO: Add source and span information.
                        source: String::new(),
                        span: Span(0..0),
                    });
                }
                Ok(()) => {
                    return Ok(self.stack.pop());
                }
            }
        }
    }

    #[inline(always)]
    pub fn run_main_loop(&mut self) -> Result<(), ExecutionErrorKind> {
        let mut wide_arg: usize = 0;
        loop {
            self.ip = unsafe { self.ip.add(1) };

            use Instruction::*;
            match unsafe { *self.ip } {
                ConstLoad(arg) => {
                    let index = wide_arg | arg as usize;
                    self.stack.push(self.code.constants[index]);
                }
                ConstInt(value) => {
                    self.stack.push(RawValue {
                        int_value: value as i64,
                    });
                }
                ConstUInt(value) => {
                    self.stack.push(RawValue {
                        int_value: value as i64,
                    });
                }
                ConstBool(value) => {
                    self.stack.push(RawValue {
                        bool_value: value != 0,
                    });
                }
                WideArg(arg) => {
                    wide_arg |= arg as usize;
                    wide_arg <<= 8;
                    continue;
                }
                IntBinOp(b'+') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value + b.int_value },
                    });
                }
                IntBinOp(b'-') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value - b.int_value },
                    });
                }
                IntBinOp(b'*') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value * b.int_value },
                    });
                }
                IntBinOp(b'/') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();

                    // Check for division by zero
                    if unsafe { b.int_value } == 0 {
                        return Err(RuntimeError::DivisionByZero {}.into());
                    }

                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value / b.int_value },
                    });
                }
                IntBinOp(b'%') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();

                    // Check for modulo by zero
                    if unsafe { b.int_value } == 0 {
                        return Err(RuntimeError::DivisionByZero {}.into());
                    }

                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value % b.int_value },
                    });
                }
                IntBinOp(b'^') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value.pow(b.int_value.try_into().unwrap()) },
                    });
                }

                // Integer unary operations
                NegInt => {
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        int_value: unsafe { -a.int_value },
                    });
                }
                IncInt => {
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value + 1 },
                    });
                }
                DecInt => {
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        int_value: unsafe { a.int_value - 1 },
                    });
                }

                // Integer comparisons
                IntCmpOp(b'<') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.int_value < b.int_value },
                    });
                }
                IntCmpOp(b'>') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.int_value > b.int_value },
                    });
                }
                IntCmpOp(b'=') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.int_value == b.int_value },
                    });
                }
                IntCmpOp(b'!') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.int_value != b.int_value },
                    });
                }
                IntCmpOp(b'l') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.int_value <= b.int_value },
                    });
                }
                IntCmpOp(b'g') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.int_value >= b.int_value },
                    });
                }

                // Float binary operations
                FloatBinOp(b'+') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        float_value: unsafe { a.float_value + b.float_value },
                    });
                }
                FloatBinOp(b'-') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        float_value: unsafe { a.float_value - b.float_value },
                    });
                }
                FloatBinOp(b'*') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        float_value: unsafe { a.float_value * b.float_value },
                    });
                }
                FloatBinOp(b'/') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        float_value: unsafe { a.float_value / b.float_value },
                    });
                }
                FloatBinOp(b'^') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        float_value: unsafe { a.float_value.powf(b.float_value) },
                    });
                }

                NegFloat => {
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        float_value: unsafe { -a.float_value },
                    });
                }

                // Float comparisons
                FloatCmpOp(b'<') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.float_value < b.float_value },
                    });
                }
                FloatCmpOp(b'>') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.float_value > b.float_value },
                    });
                }
                FloatCmpOp(b'=') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.float_value == b.float_value },
                    });
                }
                FloatCmpOp(b'!') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.float_value != b.float_value },
                    });
                }
                FloatCmpOp(b'l') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.float_value <= b.float_value },
                    });
                }
                FloatCmpOp(b'g') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.float_value >= b.float_value },
                    });
                }

                // Logical operations
                And => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.bool_value && b.bool_value },
                    });
                }
                Or => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.bool_value || b.bool_value },
                    });
                }
                Not => {
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { !a.bool_value },
                    });
                }
                EqBool => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue {
                        bool_value: unsafe { a.bool_value == b.bool_value },
                    });
                }

                // Stack operations
                DupN(depth) => {
                    let val = *self.stack.peek_at(depth as usize).unwrap();
                    self.stack.push(val);
                }
                Pop => {
                    self.stack.pop();
                }
                Swap => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(b);
                    self.stack.push(a);
                }

                // Local variables
                LoadLocal(index) => {
                    self.stack.push(self.locals[index as usize]);
                }
                StoreLocal(index) => {
                    let val = self.stack.pop();
                    if self.locals.len() <= index as usize {
                        self.locals
                            .resize(index as usize + 1, RawValue { int_value: 0 });
                    }
                    self.locals[index as usize] = val;
                }

                // Control flow
                JumpForward(delta) => {
                    self.ip = unsafe { self.ip.add(delta as usize) };
                }
                PopJumpIfFalse(delta) => {
                    let cond = self.stack.pop();
                    if unsafe { !cond.bool_value } {
                        self.ip = unsafe { self.ip.add(delta as usize) };
                    }
                }
                PopJumpIfTrue(delta) => {
                    let cond = self.stack.pop();
                    if unsafe { cond.bool_value } {
                        self.ip = unsafe { self.ip.add(delta as usize) };
                    }
                }

                Halt => {
                    return Ok(());
                }
                Return => {
                    return Ok(());
                }

                // === Otherwise Error Handling ===
                PushOtherwise(delta) => {
                    // Calculate fallback instruction pointer
                    let fallback_ip = unsafe { self.ip.add(delta as usize) };

                    // Push handler onto otherwise_stack
                    self.otherwise_stack.push(OtherwiseBlock {
                        fallback: fallback_ip,
                        stack_size: self.stack.len(),
                    });
                }

                PopOtherwise => {
                    // Remove the top otherwise handler (called in fallback code)
                    self.otherwise_stack
                        .pop()
                        .expect("PopOtherwise called with empty otherwise_stack");
                }

                PopOtherwiseAndJump(delta) => {
                    // Remove the otherwise handler (not needed, primary succeeded)
                    self.otherwise_stack
                        .pop()
                        .expect("PopOtherwiseAndJump called with empty otherwise_stack");

                    // Jump past fallback code to done label
                    self.ip = unsafe { self.ip.add(delta as usize) };
                }

                Nop => {
                    // No operation
                }

                MakeArray(len) => {
                    let len = len as usize;
                    let array = ArrayData::new_with(self.arena, self.stack.top_n(len));
                    self.stack.pop_n(len);
                    self.stack.push(array.as_raw_value());
                }

                Call(adapter_index) => {
                    let adapter_index = adapter_index as usize;
                    let func = self.stack.pop();

                    let adapter = &self.code.adapters[adapter_index];
                    let num_args = adapter.num_args();
                    let args = self.stack.top_n(num_args);

                    let result = adapter.call(self.arena, func, args)?;

                    // Pop arguments from stack after the call
                    self.stack.pop_n(num_args);

                    // Push the result
                    self.stack.push(result);
                }

                // TODO: Complex operations to implement later
                LoadUpvalue(_) | StoreUpvalue(_) => todo!("Upvalues for closures"),
                MakeClosure(_) => todo!("Closure creation"),

                // === Array Operations ===
                ArrayGet => {
                    // Stack: [..., array, index] -> [..., element]
                    let index_raw = self.stack.pop();
                    let array_raw = self.stack.pop();

                    let array = ArrayData::from_raw_value(array_raw);
                    let index = unsafe { index_raw.int_value };

                    // Handle negative indices (Python-style: -1 is last element, -2 is second-to-last, etc.)
                    let actual_index = if index < 0 {
                        let len_i64 = array.length() as i64;
                        let converted = len_i64 + index;

                        if converted < 0 {
                            return Err(RuntimeError::IndexOutOfBounds {
                                index,
                                len: array.length(),
                            }
                            .into());
                        }
                        converted as usize
                    } else {
                        // Safe conversion from i64 to usize, avoiding truncation on 32-bit platforms
                        match usize::try_from(index) {
                            Ok(idx) => idx,
                            Err(_) => {
                                return Err(RuntimeError::IndexOutOfBounds {
                                    index,
                                    len: array.length(),
                                }
                                .into());
                            }
                        }
                    };

                    // Check bounds
                    if actual_index >= array.length() {
                        return Err(RuntimeError::IndexOutOfBounds {
                            index,
                            len: array.length(),
                        }
                        .into());
                    }

                    let element = unsafe { array.get(actual_index) };
                    self.stack.push(element);
                }

                ArrayGetConst(const_index) => {
                    // Stack: [..., array] -> [..., element]
                    let array_raw = self.stack.pop();
                    let array = ArrayData::from_raw_value(array_raw);
                    let index = const_index as usize;

                    // Check bounds
                    if index >= array.length() {
                        return Err(RuntimeError::IndexOutOfBounds {
                            index: index as i64,
                            len: array.length(),
                        }
                        .into());
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
                    let key = self.stack.pop();
                    let map_raw = self.stack.pop();
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
                            return Err(RuntimeError::KeyNotFound { key_display }.into());
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
                    let values = self.stack.top_n(num_values);

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
                    let record = RecordData::new_with(self.arena, self.stack.top_n(num_fields));
                    // Pop the N elements that were used to create the record
                    self.stack.pop_n(num_fields);
                    // Push the record result
                    self.stack.push(record.as_raw_value());
                }

                RecordGet(field_index) => {
                    // Stack: [..., record] -> [..., field_value]
                    let record_raw = self.stack.pop();
                    let record = RecordData::from_raw_value(record_raw);
                    let index = field_index as usize;
                    debug_assert!(index < record.length());

                    let field_value = unsafe { record.get(index) };
                    self.stack.push(field_value);
                }

                RecordMerge => {
                    todo!("Other record operations")
                }

                // === Option Construction ===
                MakeOption(is_some) => {
                    let option_value = match is_some {
                        0 => None,
                        1 => {
                            let value = self.stack.pop();
                            Some(value)
                        }
                        _ => panic!("Invalid MakeOption operand: {}", is_some),
                    };
                    self.stack
                        .push(RawValue::make_optional(self.arena, option_value));
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
                    panic!("Unsupported operation: {:?}", unsafe { *self.ip });
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
            adapters: vec![],
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
        let mut code = Code {
            constants: vec![RawValue { int_value: 2 }],
            adapters: vec![],
            instructions: vec![
                ConstLoad(0),
                WideArg(0x01),
                ConstLoad(0x00),
                IntBinOp(b'*'),
                Return,
            ],
            num_locals: 0,
            max_stack_size: 2,
        };
        code.constants.resize(257, RawValue { int_value: 0 });
        code.constants[256] = RawValue { int_value: 42 };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 84) };
    }

    #[test]
    fn test_int_comparisons() {
        use Instruction::*;

        // Test <
        let code = Code {
            constants: vec![],
            adapters: vec![],
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
            adapters: vec![],
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
            adapters: vec![],
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
            adapters: vec![],
            instructions: vec![ConstBool(1), ConstBool(0), And, Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().bool_value, false) };

        // Test OR
        let code = Code {
            constants: vec![],
            adapters: vec![],
            instructions: vec![ConstBool(1), ConstBool(0), Or, Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().bool_value, true) };

        // Test NOT
        let code = Code {
            constants: vec![],
            adapters: vec![],
            instructions: vec![ConstBool(0), Not, Return],
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
            adapters: vec![],
            instructions: vec![ConstInt(42), DupN(0), IntBinOp(b'+'), Return],
            num_locals: 0,
            max_stack_size: 2,
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 84) };

        // Test Swap
        let code = Code {
            constants: vec![],
            adapters: vec![],
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
            adapters: vec![],
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
            adapters: vec![],
            instructions: vec![
                ConstInt(1),
                JumpForward(2), // Skip next 2 instructions
                ConstInt(50),   // Skipped
                ConstInt(60),   // Skipped
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
            adapters: vec![],
            instructions: vec![
                ConstBool(1),
                PopJumpIfTrue(1), // Skip next instruction
                ConstInt(99),     // Skipped
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
            adapters: vec![],
            instructions: vec![
                ConstBool(1),
                PopJumpIfFalse(1), // Don't jump
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
            adapters: vec![],
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
            adapters: vec![],
            instructions: vec![ConstInt(41), IncInt, Return],
            num_locals: 0,
            max_stack_size: 1,
        };
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 42) };

        // DecInt
        let code = Code {
            constants: vec![],
            adapters: vec![],
            instructions: vec![ConstInt(43), DecInt, Return],
            num_locals: 0,
            max_stack_size: 1,
        };
        let mut vm = VM::new(&arena, &code);
        unsafe { assert_eq!(vm.run().unwrap().int_value, 42) };
    }
}
