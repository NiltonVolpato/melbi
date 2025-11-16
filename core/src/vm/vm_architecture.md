# Melbi VM Architecture - Fixed 16-bit Instruction Set

## Overview

This document describes Melbi's stack-based VM with **fixed 16-bit instructions**. Every instruction is exactly 2 bytes: one byte for the opcode, one byte for the operand.

## Why Fixed 16-Bit Instructions?

### Advantages

1. **Simpler VM**: No variable-length instruction decoding
2. **Better performance**:
   - Predictable memory access patterns
   - Better instruction cache utilization
   - Branch predictor friendly
3. **Easier debugging**: Fixed addresses for all instructions
4. **Trivial IP arithmetic**: `IP += 2` for every instruction
5. **Simpler jumps**: Offset calculation is straightforward

### Trade-offs

- Limited to 8-bit operands (0-255)
- Need "wide" instructions for values > 255
- Slightly larger bytecode for some patterns

## Instruction Format

```
┌──────────────────┬──────────────────┐
│     OpCode       │     Operand      │
│    (8 bits)      │    (8 bits)      │
│                  │                  │
│   Determines     │  Immediate data  │
│   operation      │  or index/offset │
└──────────────────┴──────────────────┘
      Byte 0             Byte 1
```

### Examples

```
[0x10, 0x00]  // AddInt (operand unused)
[0x02, 0x05]  // ConstInt 5
[0x0C, 0x03]  // LoadLocal 3
[0x48, 0xF0]  // Jump -16 (0xF0 = -16 as signed i8)
```

## Handling Large Values

### Constants > 255

Use `ConstLoadWide` (2 instructions):

```rust
// Load constant at index 1000 (0x03E8)
Instruction::new(ConstLoadWide, 0x03),  // High byte
Instruction::new(..., 0xE8),            // Low byte (any opcode, operand used)
```

The VM reads the next instruction's operand to form a 16-bit index.

### Jumps > ±127 Bytes

Use `JumpWide`:

```rust
// Jump forward 300 instructions (600 bytes = 0x0258)
Instruction::new(JumpWide, 0x02),       // High byte
Instruction::new(..., 0x58),            // Low byte
```

### Many Function Arguments

Pack args in chunks:

```rust
// Call function with 300 arguments
// Use multiple Call instructions, each handling up to 255 args
```

## VM Architecture

### Core Components

```
┌──────────────────────────────────────────────┐
│              VM Instance                     │
├──────────────────────────────────────────────┤
│                                              │
│  ┌────────────┐  ┌────────────┐            │
│  │  Operand   │  │   Call     │            │
│  │   Stack    │  │   Stack    │            │
│  │ (RawValue) │  │  (Frames)  │            │
│  └────────────┘  └────────────┘            │
│                                              │
│  ┌──────────────────────────────────────┐  │
│  │    Instruction Stream                 │  │
│  │    [u16, u16, u16, ...]              │  │
│  │           ↑                           │  │
│  │           IP (instruction pointer)    │  │
│  └──────────────────────────────────────┘  │
│                                              │
│  ┌──────────────────────────────────────┐  │
│  │    Arena Allocator                    │  │
│  │    (heap values)                      │  │
│  └──────────────────────────────────────┘  │
│                                              │
└──────────────────────────────────────────────┘
```

### Execution Loop

```rust
pub struct VM<'a> {
    stack: Vec<RawValue>,
    stack_top: usize,
    frames: Vec<CallFrame>,
    ip: usize,  // Byte offset into bytecode
    bytecode: &'a [u8],
    arena: &'a Bump,
    constants: &'a [Constant],
}

impl<'a> VM<'a> {
    pub fn run(&mut self) -> Result<RawValue, VMError> {
        loop {
            // Fetch instruction (2 bytes)
            let opcode = OpCode::from_u8(self.bytecode[self.ip]);
            let operand = self.bytecode[self.ip + 1];
            self.ip += 2;  // Always advance by 2

            // Dispatch
            match opcode {
                OpCode::AddInt => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(RawValue { int_value: unsafe { a.int_value + b.int_value } })?;
                }

                OpCode::ConstLoad => {
                    let constant = &self.constants[operand as usize];
                    let value = constant.to_raw_value(self.arena);
                    self.push(value)?;
                }

                OpCode::Jump => {
                    let offset = operand as i8 as i16;  // Sign extend
                    self.ip = (self.ip as i32 + offset as i32 * 2) as usize;
                }

                OpCode::Return => {
                    if self.frames.is_empty() {
                        return Ok(self.pop()?);
                    }
                    // ... handle return
                }

                // ... other opcodes
            }
        }
    }
}
```

## Compilation Examples

### Example 1: Simple Arithmetic

**Source**: `1 + 2 * 3`

**Bytecode** (showing byte offsets):
```
Offset  OpCode          Operand    Comment
------  --------------  --------   -----------------
0x00    ConstOne        0          ; Push 1
0x02    ConstInt        2          ; Push 2
0x04    ConstInt        3          ; Push 3
0x06    MulInt          0          ; 2 * 3 = 6
0x08    AddInt          0          ; 1 + 6 = 7
0x0A    Return          0          ; Return 7
```

**Stack Evolution**:
```
After 0x00: [1]
After 0x02: [1, 2]
After 0x04: [1, 2, 3]
After 0x06: [1, 6]
After 0x08: [7]
```

### Example 2: If-Then-Else

**Source**: `if x > 10 then "big" else "small"`

**Bytecode**:
```
Offset  OpCode              Operand    Comment
------  ------------------  --------   -----------------
0x00    LoadLocal           0          ; Load x
0x02    ConstInt            10         ; Push 10
0x04    GtInt               0          ; x > 10?
0x06    JumpIfFalse         6          ; Jump +6 bytes (3 instructions) if false
0x08    ConstLoad           0          ; "big"
0x0A    Jump                4          ; Jump +4 bytes (2 instructions) past else
0x0C    ConstLoad           1          ; "small"
0x0E    Return              0
```

**Control Flow** (if x = 15):
```
0x00: Stack = [15]
0x02: Stack = [15, 10]
0x04: Stack = [true]
0x06: true, so don't jump
0x08: Stack = ["big"]
0x0A: Jump to 0x0E
0x0E: Return "big"
```

### Example 3: Array Indexing with Error Handling

**Source**: `arr[i] otherwise -1`

**Bytecode**:
```
Offset  OpCode              Operand    Comment
------  ------------------  --------   -----------------
0x00    LoadLocal           0          ; Load arr
0x02    LoadLocal           1          ; Load i
0x04    ArrayGet            0          ; arr[i] (can error)
0x06    JumpIfError         4          ; If error, jump to handler
0x08    Jump                6          ; Success, skip handler
0x0A    Pop                 0          ; Discard error
0x0C    ConstInt            -1         ; Push fallback
0x0E    Return              0
```

### Example 4: Loop (while i < 10)

**Source**: Conceptual loop structure

**Bytecode**:
```
Offset  OpCode              Operand    Comment
------  ------------------  --------   -----------------
0x00    ConstZero           0          ; i = 0
0x02    StoreLocal          0

        ; Loop start
0x04    LoadLocal           0          ; Load i
0x06    ConstInt            10         ; Push 10
0x08    LtInt               0          ; i < 10?
0x0A    JumpIfFalse         12         ; Exit loop if false (+12 bytes = +6 inst)

        ; Loop body
0x0C    LoadLocal           0          ; Load i
0x0E    IncInt              0          ; i + 1
0x10    StoreLocal          0          ; Store i
0x12    Jump                -14        ; Jump back to loop start (-14 bytes = -7 inst)

        ; After loop
0x14    LoadLocal           0          ; Load final i
0x16    Return              0
```

### Example 5: Large Constant Loading

**Source**: Load constant at index 500

**Bytecode**:
```
Offset  OpCode              Operand    Comment
------  ------------------  --------   -----------------
0x00    ConstLoadWide       0x01       ; High byte of 500 (0x01F4)
0x02    Nop                 0xF4       ; Low byte (Nop opcode ignored, operand used)
```

**VM behavior**:
1. Sees `ConstLoadWide` with operand `0x01`
2. Reads next operand: `0xF4`
3. Combines: `(0x01 << 8) | 0xF4 = 500`
4. Loads constant at index 500
5. Advances IP by 4 (skipped the Nop)

### Example 6: Function Call

**Source**: `max(a, b)`

**Bytecode**:
```
Offset  OpCode              Operand    Comment
------  ------------------  --------   -----------------
0x00    LoadLocal           0          ; Load function 'max'
0x02    LoadLocal           1          ; Load arg 'a'
0x04    LoadLocal           2          ; Load arg 'b'
0x06    Call                2          ; Call with 2 args
0x08    Return              0
```

**Stack Before Call**: `[<func_max>, a_val, b_val]`
**Stack After Call**: `[result]`

### Example 7: Closure Creation

**Source**:
```
x = 10
f = (y) => x + y
```

**Bytecode**:
```
Offset  OpCode              Operand    Comment
------  ------------------  --------   -----------------
0x00    ConstInt            10         ; x = 10
0x02    StoreLocal          0
0x04    LoadLocal           0          ; Load x to capture
0x06    MakeClosure         0          ; Create closure (func at constant 0)
0x08    StoreLocal          1          ; Store closure in 'f'
```

**Function 0 bytecode** (the lambda):
```
Offset  OpCode              Operand    Comment
------  ------------------  --------   -----------------
0x00    LoadLocal           0          ; Load y (parameter)
0x02    LoadUpvalue         0          ; Load captured x
0x04    AddInt              0          ; y + x
0x06    Return              0
```

### Example 8: Record Operations

**Source**: `{ x = 10, y = 20 }.x`

**Bytecode**:
```
Offset  OpCode              Operand    Comment
------  ------------------  --------   -----------------
0x00    ConstInt            10         ; Field x value
0x02    ConstInt            20         ; Field y value
0x04    MakeRecord          0          ; Type descriptor at constant 0
0x06    RecordGet           0          ; Get field at index 0 (x)
0x08    Return              0
```

### Example 9: Pattern Matching

**Source**:
```
value match {
    0 -> "zero",
    1 -> "one",
    _ -> "other"
}
```

**Bytecode**:
```
Offset  OpCode              Operand    Comment
------  ------------------  --------   -----------------
0x00    LoadLocal           0          ; Load value to match

        ; Try pattern 0
0x02    Dup                 0          ; Duplicate for comparison
0x04    ConstZero           0          ; Push 0
0x06    EqInt               0          ; value == 0?
0x08    JumpIfTrue          6          ; Jump to match0 handler

        ; Try pattern 1
0x0A    Dup                 0          ; Duplicate again
0x0C    ConstOne            0          ; Push 1
0x0E    EqInt               0          ; value == 1?
0x10    JumpIfTrue          8          ; Jump to match1 handler

        ; Wildcard (default)
0x12    Pop                 0          ; Discard value
0x14    ConstLoad           2          ; "other"
0x16    Jump                14         ; Jump to end

        ; match0:
0x18    Pop                 0          ; Discard value
0x1A    ConstLoad           0          ; "zero"
0x1C    Jump                8          ; Jump to end

        ; match1:
0x1E    Pop                 0          ; Discard value
0x20    ConstLoad           1          ; "one"

        ; end:
0x22    Return              0
```

### Example 10: Short-Circuit AND

**Source**: `a and b` (short-circuit: if a is false, don't evaluate b)

**Bytecode**:
```
Offset  OpCode              Operand    Comment
------  ------------------  --------   -----------------
0x00    LoadLocal           0          ; Load a
0x02    Dup                 0          ; Duplicate a
0x04    JumpIfFalseNoPop    6          ; If false, skip to end (keeping false on stack)
0x06    Pop                 0          ; Pop the duplicate (a was true)
0x08    LoadLocal           1          ; Load b
0x0A    Return              0          ; Return b's value
```

**If a = false**:
```
Stack after 0x02: [false, false]
Stack after 0x04: [false]         (jumped, keeping one false)
Stack after 0x0A: [false]         (returned)
```

**If a = true**:
```
Stack after 0x02: [true, true]
Stack after 0x06: [true]          (popped duplicate)
Stack after 0x08: [true, <b>]
Stack after 0x0A: [<b>]           (returned b)
```

## Optimization Strategies

### 1. Constant Optimizations

Special opcodes for common constants save space:

```
ConstZero         vs     ConstInt, 0
ConstOne          vs     ConstInt, 1
ConstTrue         vs     ConstLoad, <index>
```

### 2. Constant Operations

Operations with small constant operands:

```
AddIntConst, 1    vs     ConstOne, AddInt
IncInt            vs     ConstOne, AddInt
DecInt            vs     ConstInt, -1, AddInt
```

### 3. Specialized Instructions

```
ArrayGetConst, 0  vs     ConstZero, ArrayGet
RecordGet, 2      vs     ConstInt, 2, RecordGet
```

### 4. Instruction Fusion (Future)

Detect common patterns and fuse:

```
LoadLocal, 0
ConstOne          →      IncLocal, 0
AddInt
StoreLocal, 0
```

### 5. Peephole Optimization

Remove redundant operations:

```
Dup
Pop               →      (delete both)

Jump, 0           →      (delete)

ConstZero
AddInt            →      (delete both)
```

## Performance Characteristics

### Instruction Decoding

**Fixed 16-bit**:
```rust
let opcode = bytecode[ip];
let operand = bytecode[ip + 1];
ip += 2;
// Fast: no decoding logic, just two loads
```

**Variable-length** (for comparison):
```rust
let opcode = bytecode[ip];
ip += 1;
let operand_size = opcode.operand_bytes();
let operand = match operand_size {
    0 => 0,
    1 => { ip += 1; bytecode[ip] },
    2 => { ip += 2; read_u16(...) },
    4 => { ip += 4; read_u32(...) },
};
// Slower: branching, variable IP increment
```

### Bytecode Size Comparison

| Pattern | Fixed 16-bit | Variable-length |
|---------|--------------|-----------------|
| Simple ops (Add, Sub) | 2 bytes | 1 byte |
| Load local 0-255 | 2 bytes | 2 bytes |
| Load local 256+ | 4 bytes | 3 bytes |
| Small jump (±127) | 2 bytes | 2 bytes |
| Large jump | 4 bytes | 5 bytes |
| **Average** | ~5-10% larger | Baseline |

**Verdict**: Fixed format trades ~5-10% larger bytecode for significantly simpler and faster execution.

## Sandboxing Implementation

### Instruction Counting

```rust
pub struct VM<'a> {
    instruction_count: u64,
    max_instructions: u64,
    check_interval: u64,
}

impl<'a> VM<'a> {
    fn run(&mut self) -> Result<RawValue, VMError> {
        loop {
            // Check limits every N instructions
            if self.instruction_count % self.check_interval == 0 {
                if self.instruction_count >= self.max_instructions {
                    return Err(VMError::InstructionLimitExceeded);
                }
            }
            self.instruction_count += 1;

            // Execute instruction
            // ...
        }
    }
}
```

Or use the explicit `CheckLimits` opcode at loop back-edges.

### Memory Limits

```rust
impl<'a> VM<'a> {
    fn allocate(&mut self, size: usize) -> Result<*mut u8, VMError> {
        self.allocated_bytes += size;
        if self.allocated_bytes > self.max_memory {
            return Err(VMError::OutOfMemory);
        }
        Ok(self.arena.alloc_layout(...).as_ptr())
    }
}
```

### Stack Depth Limits

```rust
const MAX_STACK_SIZE: usize = 1024;

fn push(&mut self, value: RawValue) -> Result<(), VMError> {
    if self.stack_top >= MAX_STACK_SIZE {
        return Err(VMError::StackOverflow);
    }
    self.stack[self.stack_top] = value;
    self.stack_top += 1;
    Ok(())
}
```

## Debugging Support

### Breakpoints

```
Instruction::new(Breakpoint, <bp_id>)
```

VM checks breakpoint table and halts if debugger attached.

### Source Maps

Map instruction offsets to source locations:

```rust
pub struct SourceMap {
    mappings: Vec<(usize, SourceSpan)>,  // (byte_offset, source_location)
}

impl SourceMap {
    fn lookup(&self, ip: usize) -> Option<SourceSpan> {
        // Binary search to find source location
    }
}
```

### Tracing

```
Instruction::new(Trace, <trace_id>)
```

Records execution if profiling enabled.

## Future Extensions

### JIT Compilation Triggers

```rust
struct HotSpot {
    address: usize,
    execution_count: u64,
    threshold: u64,
}

// When threshold exceeded, compile to native code
if hotspot.execution_count > hotspot.threshold {
    let native_fn = jit_compile(bytecode_range);
    replace_with_native_call(address, native_fn);
}
```

### Inline Caching

```
Instruction::new(InlineCache, <cache_id>)

struct InlineCache {
    last_type: TypeId,
    cached_offset: u16,  // For field access
}
```

### Superinstructions

Combine common sequences into single instructions:

```
LoadLocal_0_AddInt = custom opcode for "LoadLocal 0; AddInt"
```

Requires profiling to identify hot sequences.

## Bytecode Generation

### Compiler Output

```rust
struct BytecodeCompiler<'a> {
    output: Vec<Instruction>,
    constants: Vec<Constant>,
    local_map: HashMap<&'a str, u8>,
}

impl<'a> BytecodeCompiler<'a> {
    fn compile_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Binary { op: BinaryOp::Add, left, right } => {
                self.compile_expr(left);
                self.compile_expr(right);
                self.emit(Instruction::simple(OpCode::AddInt));
            }

            Expr::Literal(Literal::Int { value, .. }) => {
                if *value >= -128 && *value <= 127 {
                    self.emit(Instruction::new(OpCode::ConstInt, *value as i8 as u8));
                } else if *value >= 0 && *value <= 255 {
                    self.emit(Instruction::new(OpCode::ConstUInt, *value as u8));
                } else {
                    let idx = self.add_constant(Constant::Int(*value));
                    self.emit_load_const(idx);
                }
            }

            // ... other expressions
        }
    }

    fn emit_load_const(&mut self, index: u16) {
        if index <= 255 {
            self.emit(Instruction::new(OpCode::ConstLoad, index as u8));
        } else {
            let (high, low) = (index >> 8, index & 0xFF);
            self.emit(Instruction::new(OpCode::ConstLoadWide, high as u8));
            self.emit(Instruction::new(OpCode::Nop, low as u8));
        }
    }
}
```

## Conclusion

The fixed 16-bit instruction format provides an excellent balance:

✅ **Simpler VM implementation**
✅ **Better performance** (no variable-length decoding)
✅ **Predictable execution** (easier to optimize)
✅ **Good enough operand range** (8 bits covers most cases)
✅ **Escape hatch for large values** (wide instructions)

The ~5-10% bytecode size increase is a worthwhile trade-off for the execution speed and implementation simplicity gains.
