#![allow(dead_code)] // TODO: remove
//! Melbi VM Instructions - Fixed 16-bit Format
//!
//! This module defines the instruction set for Melbi's stack-based virtual machine.
//!
//! # Instruction Format
//!
//! **ALL instructions are exactly 16 bits (2 bytes)**:
//! ```text
//! ┌────────────┬────────────┐
//! │    Tag     │  Operand   │
//! │  (8 bits)  │  (8 bits)  │
//! └────────────┴────────────┘
//! ```
//!
//! Using `#[repr(C, u8)]`, the enum naturally maps to this 2-byte layout:
//! - Discriminant (tag) = 1 byte
//! - Payload (operand) = 0 or 1 byte
//! - Total size = 2 bytes with no padding
//!
//! # Design Principles
//!
//! - **Stack-based**: All operations consume operands from stack and push results
//! - **Fixed-width**: Every instruction is exactly 16 bits (simpler VM, better performance)
//! - **Type-explicit**: Separate instructions for Int vs Float operations (no runtime dispatch)
//! - **Arena-allocated**: All heap values allocated in bump allocator
//! - **Parameterized ops**: Binary operations use operand to encode operation (saves opcodes)
//!
//! # Wide Arguments
//!
//! For values > 255, use the `WideArg` prefix:
//! ```ignore
//! WideArg(high_byte)      // Sets high byte for next instruction
//! ConstLoad(low_byte)     // Combined: (high << 8) | low = 16-bit index
//! ```
//!
//! # Stack Discipline
//!
//! Stack effect notation: `[..., operand1, operand2] -> [..., result]`

use core::fmt;

use crate::{Box, String, Vec};

/// A single VM instruction (exactly 16 bits)
///
/// The `#[repr(C, u8)]` ensures:
/// - First byte is the discriminant (opcode)
/// - Second byte is the operand (if present)
/// - Total size is exactly 2 bytes
/// - Can be safely transmuted to/from `[u8; 2]`
#[repr(C, u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // ========================================================================
    // Special (0x00)
    // ========================================================================
    /// Halt execution
    ///
    /// Having Halt at 0x00 is a safety feature:
    /// - Uninitialized memory (zeros) will halt instead of executing garbage
    /// - Null-terminated C strings won't accidentally execute
    /// - Better fail-safe behavior on bytecode corruption
    Halt = 0x00,

    // ========================================================================
    // Stack & Constants (0x01 - 0x0F)
    // ========================================================================
    /// Push constant from pool (index 0-255)
    /// Operand: u8 index | Stack: [...] -> [..., value]
    ConstLoad(u8) = 0x01,

    /// Push small signed integer (-128 to 127)
    /// Operand: i8 value | Stack: [...] -> [..., int]
    ConstInt(i8) = 0x02,

    /// Push unsigned byte (0 to 255)
    /// Operand: u8 value | Stack: [...] -> [..., int]
    ConstUInt(u8) = 0x03,

    /// Push true
    /// Stack: [...] -> [..., true]
    ConstTrue = 0x04,

    /// Push false
    /// Stack: [...] -> [..., false]
    ConstFalse = 0x05,

    /// Wide argument prefix - modifies next instruction's operand
    ///
    /// The next instruction will use a 16-bit operand:
    /// `(this_operand << 8) | next_operand`
    ///
    /// Example:
    /// ```ignore
    /// WideArg(0x03)       // High byte
    /// ConstLoad(0xE8)     // Low byte -> loads constant 1000 (0x03E8)
    /// ```
    WideArg(u8) = 0x06,

    /// Duplicate top value
    /// Stack: [..., a] -> [..., a, a]
    Dup = 0x07,

    /// Duplicate value at depth N
    /// Operand: u8 depth | Stack: [..., aN, ...] -> [..., aN, ..., aN]
    DupN(u8) = 0x08,

    /// Pop top value
    /// Stack: [..., a] -> [...]
    Pop = 0x09,

    /// Swap top two values
    /// Stack: [..., a, b] -> [..., b, a]
    Swap = 0x0A,

    /// Load local variable
    /// Operand: u8 index | Stack: [...] -> [..., value]
    LoadLocal(u8) = 0x0B,

    /// Store to local variable
    /// Operand: u8 index | Stack: [..., value] -> [...]
    StoreLocal(u8) = 0x0C,

    /// Load upvalue (captured variable in closure)
    /// Operand: u8 index | Stack: [...] -> [..., value]
    ///
    /// Upvalues are variables captured from enclosing scopes.
    /// Each closure instance has its own upvalue array.
    /// This is how closures "remember" their environment.
    ///
    /// Note: Unlike Python (which captures by reference and is broken),
    /// we capture by value at closure creation time. Each closure gets
    /// its own snapshot of captured variables.
    LoadUpvalue(u8) = 0x0D,

    /// Store to upvalue
    /// Operand: u8 index | Stack: [..., value] -> [...]
    StoreUpvalue(u8) = 0x0E,

    // 0x0F reserved

    // ========================================================================
    // Arithmetic - Integer (0x10 - 0x1F)
    // ========================================================================
    /// Integer binary operation
    ///
    /// Operand encodes the operation:
    /// - `b'+'` (0x2B): Addition
    /// - `b'-'` (0x2D): Subtraction
    /// - `b'*'` (0x2A): Multiplication
    /// - `b'/'` (0x2F): Division (can error)
    /// - `b'%'` (0x25): Modulo (can error)
    /// - `b'^'` (0x5E): Power (can error)
    ///
    /// Stack: [..., a: Int, b: Int] -> [..., result: Int(|!)]
    IntBinOp(u8) = 0x10,

    /// Integer unary negation: -a
    /// Stack: [..., a: Int] -> [..., -a: Int]
    NegInt = 0x11,

    /// Increment: a + 1
    /// Stack: [..., a: Int] -> [..., a+1: Int]
    IncInt = 0x12,

    /// Decrement: a - 1
    /// Stack: [..., a: Int] -> [..., a-1: Int]
    DecInt = 0x13,

    /// Integer comparison operation
    ///
    /// Operand encodes the comparison:
    /// - `b'<'` (0x3C): Less than
    /// - `b'>'` (0x3E): Greater than
    /// - `b'='` (0x3D): Equal (==)
    /// - `b'!'` (0x21): Not equal (!=)
    /// - `b'l'` (0x6C): Less or equal (<=)
    /// - `b'g'` (0x67): Greater or equal (>=)
    ///
    /// Stack: [..., a: Int, b: Int] -> [..., result: Bool]
    IntCmpOp(u8) = 0x14,

    // 0x15-0x1F reserved for future int operations

    // ========================================================================
    // Arithmetic - Float (0x20 - 0x2F)
    // ========================================================================
    /// Float binary operation
    ///
    /// Same operand encoding as IntBinOp:
    /// - `b'+'`: Addition
    /// - `b'-'`: Subtraction
    /// - `b'*'`: Multiplication
    /// - `b'/'`: Division
    /// - `b'^'`: Power
    ///
    /// Stack: [..., a: Float, b: Float] -> [..., result: Float]
    FloatBinOp(u8) = 0x20,

    /// Float unary negation: -a
    /// Stack: [..., a: Float] -> [..., -a: Float]
    NegFloat = 0x21,

    /// Float comparison operation
    ///
    /// Same operand encoding as IntCmpOp:
    /// - `b'<'`: Less than
    /// - `b'>'`: Greater than
    /// - `b'='`: Equal
    /// - `b'!'`: Not equal
    /// - `b'l'`: Less or equal
    /// - `b'g'`: Greater or equal
    ///
    /// Stack: [..., a: Float, b: Float] -> [..., result: Bool]
    FloatCmpOp(u8) = 0x22,

    // 0x23-0x2F reserved for future float operations

    // ========================================================================
    // Logical Operations (0x30 - 0x37)
    // ========================================================================
    /// Logical AND: a && b
    /// Stack: [..., a: Bool, b: Bool] -> [..., a&&b: Bool]
    And = 0x30,

    /// Logical OR: a || b
    /// Stack: [..., a: Bool, b: Bool] -> [..., a||b: Bool]
    Or = 0x31,

    /// Logical NOT: !a
    /// Stack: [..., a: Bool] -> [..., !a: Bool]
    Not = 0x32,

    /// Boolean equality
    /// Stack: [..., a: Bool, b: Bool] -> [..., a==b: Bool]
    EqBool = 0x33,

    // 0x34-0x37 reserved

    // ========================================================================
    // Control Flow (0x38 - 0x4F)
    // ========================================================================
    /// Unconditional jump (signed byte offset in instructions, not bytes)
    ///
    /// Operand: i8 offset (in instructions)
    /// Stack: [...] -> [...]
    ///
    /// Jump is relative to the NEXT instruction.
    /// Offset is in instruction count, not bytes (each instruction is 2 bytes).
    ///
    /// Example: `Jump(3)` skips forward 3 instructions (6 bytes)
    Jump(i8) = 0x38,

    /// Jump if false
    /// Operand: i8 offset | Stack: [..., cond: Bool] -> [...]
    JumpIfFalse(i8) = 0x39,

    /// Jump if true
    /// Operand: i8 offset | Stack: [..., cond: Bool] -> [...]
    JumpIfTrue(i8) = 0x3A,

    /// Jump if false (don't pop condition)
    /// Operand: i8 offset | Stack: [..., cond: Bool] -> [..., cond: Bool]
    /// Used for short-circuit evaluation
    JumpIfFalseNoPop(i8) = 0x3B,

    /// Jump if true (don't pop condition)
    /// Operand: i8 offset | Stack: [..., cond: Bool] -> [..., cond: Bool]
    JumpIfTrueNoPop(i8) = 0x3C,

    /// Jump if error value
    /// Operand: i8 offset | Stack: [..., val!] -> [..., val!]
    /// Used for error propagation and `otherwise` operator
    JumpIfError(i8) = 0x3D,

    /// Return from function
    /// Stack: [..., retval] -> [retval]
    Return = 0x3E,

    /// Call function
    /// Operand: u8 arg count | Stack: [..., args..., func] -> [..., result]
    Call(u8) = 0x3F,

    /// Call native (host) function
    /// Operand: u8 function ID (0-255)
    /// Stack: [..., args...] -> [..., result]
    /// Number of args determined by function signature
    CallNative(u8) = 0x40,

    /// Tail call optimization
    /// Operand: u8 arg count | Stack: [..., args..., func] -> [result]
    TailCall(u8) = 0x41,

    /// Push otherwise error handler
    /// Operand: i8 offset to fallback code
    /// Pushes OtherwiseBlock { fallback: ip + offset, stack_size: current_stack_size } to otherwise_stack
    PushOtherwise(i8) = 0x42,

    /// Pop otherwise error handler (normal cleanup)
    /// Pops the top OtherwiseBlock from otherwise_stack
    /// Used in fallback code to clean up handler
    PopOtherwise = 0x43,

    /// Pop otherwise handler and jump (success case)
    /// Operand: i8 offset to done label
    /// Pops OtherwiseBlock and jumps past fallback code
    /// Used when primary expression succeeds
    PopOtherwiseAndJump(i8) = 0x44,

    // 0x45-0x4F reserved for control flow

    // ========================================================================
    // Function & Closure Operations (0x50 - 0x5F)
    // ========================================================================
    /// Create closure
    ///
    /// Operand: u8 function index in constant pool
    /// Stack: [..., upval1, upval2, ..., upvalN] -> [..., closure]
    ///
    /// Creates a closure by:
    /// 1. Loading function descriptor from constant pool
    /// 2. Popping N upvalues from stack (N specified in function descriptor)
    /// 3. Creating closure object with function + captured upvalues
    /// 4. Pushing closure object onto stack
    ///
    /// The number of upvalues is stored in the FunctionConstant.
    MakeClosure(u8) = 0x50,

    // 0x51-0x5F reserved for function operations

    // ========================================================================
    // Array Operations (0x60 - 0x6F)
    // ========================================================================
    /// Make array with N elements
    /// Operand: u8 count | Stack: [..., e1, ..., eN] -> [..., array]
    MakeArray(u8) = 0x60,

    /// Get array length
    /// Stack: [..., array: Array[T]] -> [..., len: Int]
    ArrayLen = 0x61,

    /// Get array element
    /// Stack: [..., array: Array[T], index: Int] -> [..., elem: T!]
    ArrayGet = 0x62,

    /// Get array element at constant index
    /// Operand: u8 index | Stack: [..., array: Array[T]] -> [..., elem: T!]
    ArrayGetConst(u8) = 0x63,

    /// Concatenate two arrays
    /// Stack: [..., a1: Array[T], a2: Array[T]] -> [..., result: Array[T]]
    ArrayConcat = 0x64,

    /// Slice array
    /// Stack: [..., arr: Array[T], start: Int, end: Int] -> [..., slice: Array[T]!]
    ArraySlice = 0x65,

    /// Append element to array (creates new array)
    /// Stack: [..., array: Array[T], elem: T] -> [..., new_array: Array[T]]
    ArrayAppend = 0x66,

    // 0x67-0x6F reserved for array operations

    // ========================================================================
    // Map Operations (0x70 - 0x7F)
    // ========================================================================
    /// Make map with N key-value pairs
    /// Operand: u8 count | Stack: [..., k1, v1, ..., kN, vN] -> [..., map]
    MakeMap(u8) = 0x70,

    /// Get map size
    /// Stack: [..., map: Map[K,V]] -> [..., size: Int]
    MapLen = 0x71,

    /// Get value from map
    /// Stack: [..., map: Map[K,V], key: K] -> [..., value: V!]
    MapGet = 0x72,

    /// Check if key exists
    /// Stack: [..., map: Map[K,V], key: K] -> [..., exists: Bool]
    MapHas = 0x73,

    /// Insert key-value (creates new map)
    /// Stack: [..., map: Map[K,V], key: K, val: V] -> [..., new_map: Map[K,V]]
    MapInsert = 0x74,

    /// Remove key (creates new map)
    /// Stack: [..., map: Map[K,V], key: K] -> [..., new_map: Map[K,V]]
    MapRemove = 0x75,

    /// Get all keys as array
    /// Stack: [..., map: Map[K,V]] -> [..., keys: Array[K]]
    MapKeys = 0x76,

    /// Get all values as array
    /// Stack: [..., map: Map[K,V]] -> [..., values: Array[V]]
    MapValues = 0x77,

    // 0x78-0x7F reserved for map operations

    // ========================================================================
    // Record Operations (0x80 - 0x8F)
    // ========================================================================
    /// Make record (type descriptor in constant pool)
    /// Operand: u8 type index | Stack: [..., f1, ..., fN] -> [..., record]
    MakeRecord(u8) = 0x80,

    /// Get field by index
    /// Operand: u8 field index | Stack: [..., record] -> [..., value!]
    RecordGet(u8) = 0x81,

    /// Set field (creates new record)
    /// Operand: u8 field index | Stack: [..., record, value] -> [..., new_record]
    RecordSet(u8) = 0x82,

    /// Merge two records
    /// Stack: [..., rec1, rec2] -> [..., merged]
    RecordMerge = 0x83,

    // 0x84-0x8F reserved for record operations

    // ========================================================================
    // String Operations (0x90 - 0x9F)
    // ========================================================================
    /// String operations
    ///
    /// Operand encodes the operation:
    /// - `b'+'` (0x2B): Concatenate
    /// - More operations can be added as needed
    ///
    /// Stack: [..., s1: String, s2: String] -> [..., result: String]
    StringOp(u8) = 0x90,

    /// Get string length (Unicode code points)
    /// Stack: [..., str: String] -> [..., len: Int]
    StringLen = 0x91,

    /// Check if contains substring
    /// Stack: [..., haystack: String, needle: String] -> [..., contains: Bool]
    StringContains = 0x92,

    /// Find substring index
    /// Stack: [..., haystack: String, needle: String] -> [..., index: Int!]
    StringFind = 0x93,

    /// To uppercase
    /// Stack: [..., str: String] -> [..., upper: String]
    StringUpper = 0x94,

    /// To lowercase
    /// Stack: [..., str: String] -> [..., lower: String]
    StringLower = 0x95,

    /// Trim whitespace
    /// Stack: [..., str: String] -> [..., trimmed: String]
    StringTrim = 0x96,

    /// Split by separator
    /// Stack: [..., str: String, sep: String] -> [..., parts: Array[String]]
    StringSplit = 0x97,

    /// Format string (f-string)
    /// Operand: u8 arg count | Stack: [..., args..., template] -> [..., result]
    StringFormat(u8) = 0x98,

    /// String comparison operation
    ///
    /// Same operand encoding as IntCmpOp:
    /// - `b'<'`: Less than (lexicographic)
    /// - `b'>'`: Greater than
    /// - `b'='`: Equal
    /// - `b'!'`: Not equal
    /// - `b'l'`: Less or equal
    /// - `b'g'`: Greater or equal
    ///
    /// Stack: [..., a: String, b: String] -> [..., result: Bool]
    StringCmpOp(u8) = 0x99,

    // 0x9A-0x9F reserved for string operations

    // ========================================================================
    // Bytes Operations (0xA0 - 0xAF)
    // ========================================================================
    /// Concatenate bytes
    /// Stack: [..., b1: Bytes, b2: Bytes] -> [..., result: Bytes]
    BytesConcat = 0xA0,

    /// Get bytes length
    /// Stack: [..., bytes: Bytes] -> [..., len: Int]
    BytesLen = 0xA1,

    /// Get byte at index
    /// Stack: [..., bytes: Bytes, index: Int] -> [..., byte: Int!]
    BytesGet = 0xA2,

    /// Get byte at constant index
    /// Operand: u8 index | Stack: [..., bytes: Bytes] -> [..., byte: Int!]
    BytesGetConst(u8) = 0xA3,

    /// Slice bytes
    /// Stack: [..., bytes: Bytes, start: Int, end: Int] -> [..., slice: Bytes!]
    BytesSlice = 0xA4,

    /// String to bytes (UTF-8 encode)
    /// Stack: [..., str: String] -> [..., bytes: Bytes]
    StringToBytes = 0xA5,

    /// Bytes to string (UTF-8 decode)
    /// Stack: [..., bytes: Bytes] -> [..., str: String!]
    BytesToString = 0xA6,

    /// Bytes comparison (same encoding as StringCmpOp)
    /// Stack: [..., a: Bytes, b: Bytes] -> [..., result: Bool]
    BytesCmpOp(u8) = 0xA7,

    // 0xA8-0xAF reserved for bytes operations

    // ========================================================================
    // Type & Error Operations (0xB0 - 0xBF)
    // ========================================================================
    /// Cast/convert to type
    /// Operand: u8 type index | Stack: [..., value] -> [..., converted!]
    Cast(u8) = 0xB0,

    /// Get type of value
    /// Stack: [..., value] -> [..., type: Type]
    TypeOf = 0xB1,

    /// Check type
    /// Operand: u8 type index | Stack: [..., value] -> [..., matches: Bool]
    TypeCheck(u8) = 0xB2,

    /// Handle error with otherwise
    /// Stack: [..., value!, fallback] -> [..., result]
    Otherwise = 0xB3,

    /// Check if value is error
    /// Stack: [..., value!] -> [..., is_error: Bool]
    IsError = 0xB4,

    /// Generic equality (structural comparison for arrays, maps, records)
    /// Stack: [..., a: T, b: T] -> [..., a==b: Bool]
    Eq = 0xB5,

    /// Generic inequality
    /// Stack: [..., a: T, b: T] -> [..., a!=b: Bool]
    NotEq = 0xB6,

    // 0xB7-0xBF reserved

    // ========================================================================
    // Pattern Matching (0xC0 - 0xCF)
    // ========================================================================
    /// Begin pattern match (duplicate value for matching)
    /// Stack: [..., val] -> [..., val, val]
    MatchBegin = 0xC0,

    /// Match literal constant
    /// Operand: u8 const index | Stack: [..., val] -> [..., matches: Bool]
    MatchLiteral(u8) = 0xC1,

    /// Match constructor
    /// Operand: u8 constructor ID
    /// Stack: [..., val] -> [..., matches: Bool, ...extracted_fields]
    MatchConstructor(u8) = 0xC2,

    /// Match array destructure
    /// Operand: u8 element count
    /// Stack: [..., arr] -> [..., matches: Bool, e1, ..., eN]
    MatchArray(u8) = 0xC3,

    /// Match record destructure
    /// Operand: u8 field count
    /// Stack: [..., rec] -> [..., matches: Bool, f1, ..., fN]
    MatchRecord(u8) = 0xC4,

    /// Wildcard match (always true)
    /// Stack: [...] -> [..., true: Bool]
    MatchWildcard = 0xC5,

    /// Apply guard condition
    /// Stack: [..., match_result: Bool, guard: Bool] -> [..., result: Bool]
    MatchGuard = 0xC6,

    // 0xC7-0xCF reserved for pattern matching

    // ========================================================================
    // Meta & Debug Operations (0xD0 - 0xDF)
    // ========================================================================
    /// No operation
    Nop = 0xD0,

    /// Breakpoint for debugger
    /// Operand: u8 breakpoint ID
    Breakpoint(u8) = 0xD1,

    /// Check execution limits (sandboxing)
    CheckLimits = 0xD2,

    /// Trace execution (profiling)
    /// Operand: u8 trace point ID
    Trace(u8) = 0xD3,

    /// Inline cache hint
    /// Operand: u8 cache ID
    InlineCache(u8) = 0xD4,
    // 0xD5-0xDF reserved

    // 0xE0-0xFF reserved for future expansion
}

impl Instruction {
    /// Size of an instruction in bytes
    pub const SIZE: usize = 2;

    /// Check if this instruction can produce an error effect
    pub const fn can_error(&self) -> bool {
        matches!(
            self,
            Self::IntBinOp(b'/')
                | Self::IntBinOp(b'%')
                | Self::IntBinOp(b'^')
                | Self::ArrayGet
                | Self::ArrayGetConst(_)
                | Self::ArraySlice
                | Self::MapGet
                | Self::RecordGet(_)
                | Self::StringFind
                | Self::BytesGet
                | Self::BytesGetConst(_)
                | Self::BytesSlice
                | Self::BytesToString
                | Self::Cast(_)
        )
    }

    /// Check if this is a control flow instruction
    pub const fn is_control_flow(&self) -> bool {
        matches!(
            self,
            Self::Jump(_)
                | Self::JumpIfFalse(_)
                | Self::JumpIfTrue(_)
                | Self::JumpIfFalseNoPop(_)
                | Self::JumpIfTrueNoPop(_)
                | Self::JumpIfError(_)
                | Self::Return
                | Self::Call(_)
                | Self::CallNative(_)
                | Self::TailCall(_)
        )
    }

    /// Get the discriminant (opcode byte)
    pub const fn discriminant(&self) -> u8 {
        // Safety: repr(C, u8) guarantees first byte is discriminant
        unsafe { *(self as *const Self as *const u8) }
    }

    /// Safely decode from bytes
    pub fn from_bytes(bytes: [u8; 2]) -> Result<Self, InvalidInstruction> {
        // We could validate the discriminant here, but for now
        // we trust that bytecode is well-formed (validated at load time)
        Ok(unsafe { core::mem::transmute(bytes) })
    }

    /// Encode to bytes
    pub fn to_bytes(self) -> [u8; 2] {
        unsafe { core::mem::transmute(self) }
    }
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Binary operations - show operator as char
            Self::IntBinOp(op) => write!(f, "IntBinOp({})", *op as char),
            Self::FloatBinOp(op) => write!(f, "FloatBinOp({})", *op as char),
            Self::StringOp(op) => write!(f, "StringOp({})", *op as char),

            // Comparisons - show operator
            Self::IntCmpOp(b'<') => write!(f, "IntCmpOp(<)"),
            Self::IntCmpOp(b'>') => write!(f, "IntCmpOp(>)"),
            Self::IntCmpOp(b'=') => write!(f, "IntCmpOp(==)"),
            Self::IntCmpOp(b'!') => write!(f, "IntCmpOp(!=)"),
            Self::IntCmpOp(b'l') => write!(f, "IntCmpOp(<=)"),
            Self::IntCmpOp(b'g') => write!(f, "IntCmpOp(>=)"),
            Self::IntCmpOp(op) => write!(f, "IntCmpOp(0x{:02X})", op),

            Self::FloatCmpOp(b'<') => write!(f, "FloatCmpOp(<)"),
            Self::FloatCmpOp(b'>') => write!(f, "FloatCmpOp(>)"),
            Self::FloatCmpOp(b'=') => write!(f, "FloatCmpOp(==)"),
            Self::FloatCmpOp(b'!') => write!(f, "FloatCmpOp(!=)"),
            Self::FloatCmpOp(b'l') => write!(f, "FloatCmpOp(<=)"),
            Self::FloatCmpOp(b'g') => write!(f, "FloatCmpOp(>=)"),
            Self::FloatCmpOp(op) => write!(f, "FloatCmpOp(0x{:02X})", op),

            Self::StringCmpOp(b'<') => write!(f, "StringCmpOp(<)"),
            Self::StringCmpOp(b'>') => write!(f, "StringCmpOp(>)"),
            Self::StringCmpOp(b'=') => write!(f, "StringCmpOp(==)"),
            Self::StringCmpOp(b'!') => write!(f, "StringCmpOp(!=)"),
            Self::StringCmpOp(b'l') => write!(f, "StringCmpOp(<=)"),
            Self::StringCmpOp(b'g') => write!(f, "StringCmpOp(>=)"),
            Self::StringCmpOp(op) => write!(f, "StringCmpOp(0x{:02X})", op),

            Self::BytesCmpOp(b'<') => write!(f, "BytesCmpOp(<)"),
            Self::BytesCmpOp(b'>') => write!(f, "BytesCmpOp(>)"),
            Self::BytesCmpOp(b'=') => write!(f, "BytesCmpOp(==)"),
            Self::BytesCmpOp(b'!') => write!(f, "BytesCmpOp(!=)"),
            Self::BytesCmpOp(b'l') => write!(f, "BytesCmpOp(<=)"),
            Self::BytesCmpOp(b'g') => write!(f, "BytesCmpOp(>=)"),
            Self::BytesCmpOp(op) => write!(f, "BytesCmpOp(0x{:02X})", op),

            // Default formatting for everything else
            Self::Halt => write!(f, "Halt"),
            Self::ConstLoad(idx) => write!(f, "ConstLoad({})", idx),
            Self::ConstInt(val) => write!(f, "ConstInt({})", val),
            Self::ConstUInt(val) => write!(f, "ConstUInt({})", val),
            Self::ConstTrue => write!(f, "ConstTrue"),
            Self::ConstFalse => write!(f, "ConstFalse"),
            Self::WideArg(high) => write!(f, "WideArg(0x{:02X})", high),
            Self::Dup => write!(f, "Dup"),
            Self::DupN(depth) => write!(f, "DupN({})", depth),
            Self::Pop => write!(f, "Pop"),
            Self::Swap => write!(f, "Swap"),
            Self::LoadLocal(idx) => write!(f, "LoadLocal({})", idx),
            Self::StoreLocal(idx) => write!(f, "StoreLocal({})", idx),
            Self::LoadUpvalue(idx) => write!(f, "LoadUpvalue({})", idx),
            Self::StoreUpvalue(idx) => write!(f, "StoreUpvalue({})", idx),
            Self::NegInt => write!(f, "NegInt"),
            Self::IncInt => write!(f, "IncInt"),
            Self::DecInt => write!(f, "DecInt"),
            Self::NegFloat => write!(f, "NegFloat"),
            Self::And => write!(f, "And"),
            Self::Or => write!(f, "Or"),
            Self::Not => write!(f, "Not"),
            Self::EqBool => write!(f, "EqBool"),
            Self::Jump(offset) => write!(f, "Jump({:+})", offset),
            Self::JumpIfFalse(offset) => write!(f, "{:18} {:+3}", "JumpIfFalse", offset),
            Self::JumpIfTrue(offset) => write!(f, "JumpIfTrue({:+})", offset),
            Self::JumpIfFalseNoPop(offset) => write!(f, "JumpIfFalseNoPop({:+})", offset),
            Self::JumpIfTrueNoPop(offset) => write!(f, "JumpIfTrueNoPop({:+})", offset),
            Self::JumpIfError(offset) => write!(f, "JumpIfError({:+})", offset),
            Self::Return => write!(f, "Return"),
            Self::Call(argc) => write!(f, "Call({})", argc),
            Self::CallNative(id) => write!(f, "CallNative({})", id),
            Self::TailCall(argc) => write!(f, "TailCall({})", argc),
            Self::MakeClosure(idx) => write!(f, "MakeClosure({})", idx),
            Self::MakeArray(count) => write!(f, "MakeArray({})", count),
            Self::ArrayLen => write!(f, "ArrayLen"),
            Self::ArrayGet => write!(f, "ArrayGet"),
            Self::ArrayGetConst(idx) => write!(f, "ArrayGetConst({})", idx),
            Self::ArrayConcat => write!(f, "ArrayConcat"),
            Self::ArraySlice => write!(f, "ArraySlice"),
            Self::ArrayAppend => write!(f, "ArrayAppend"),
            Self::MakeMap(count) => write!(f, "MakeMap({})", count),
            Self::MapLen => write!(f, "MapLen"),
            Self::MapGet => write!(f, "MapGet"),
            Self::MapHas => write!(f, "MapHas"),
            Self::MapInsert => write!(f, "MapInsert"),
            Self::MapRemove => write!(f, "MapRemove"),
            Self::MapKeys => write!(f, "MapKeys"),
            Self::MapValues => write!(f, "MapValues"),
            Self::MakeRecord(ty_idx) => write!(f, "MakeRecord({})", ty_idx),
            Self::RecordGet(idx) => write!(f, "RecordGet({})", idx),
            Self::RecordSet(idx) => write!(f, "RecordSet({})", idx),
            Self::RecordMerge => write!(f, "RecordMerge"),
            Self::StringLen => write!(f, "StringLen"),
            Self::StringContains => write!(f, "StringContains"),
            Self::StringFind => write!(f, "StringFind"),
            Self::StringUpper => write!(f, "StringUpper"),
            Self::StringLower => write!(f, "StringLower"),
            Self::StringTrim => write!(f, "StringTrim"),
            Self::StringSplit => write!(f, "StringSplit"),
            Self::StringFormat(argc) => write!(f, "StringFormat({})", argc),
            Self::BytesConcat => write!(f, "BytesConcat"),
            Self::BytesLen => write!(f, "BytesLen"),
            Self::BytesGet => write!(f, "BytesGet"),
            Self::BytesGetConst(idx) => write!(f, "BytesGetConst({})", idx),
            Self::BytesSlice => write!(f, "BytesSlice"),
            Self::StringToBytes => write!(f, "StringToBytes"),
            Self::BytesToString => write!(f, "BytesToString"),
            Self::Cast(ty_idx) => write!(f, "Cast({})", ty_idx),
            Self::TypeOf => write!(f, "TypeOf"),
            Self::TypeCheck(ty_idx) => write!(f, "TypeCheck({})", ty_idx),
            Self::Otherwise => write!(f, "Otherwise"),
            Self::IsError => write!(f, "IsError"),
            Self::Eq => write!(f, "Eq"),
            Self::NotEq => write!(f, "NotEq"),
            Self::MatchBegin => write!(f, "MatchBegin"),
            Self::MatchLiteral(idx) => write!(f, "MatchLiteral({})", idx),
            Self::MatchConstructor(id) => write!(f, "MatchConstructor({})", id),
            Self::MatchArray(count) => write!(f, "MatchArray({})", count),
            Self::MatchRecord(count) => write!(f, "MatchRecord({})", count),
            Self::MatchWildcard => write!(f, "MatchWildcard"),
            Self::MatchGuard => write!(f, "MatchGuard"),
            Self::Nop => write!(f, "Nop"),
            Self::Breakpoint(id) => write!(f, "Breakpoint({})", id),
            Self::CheckLimits => write!(f, "CheckLimits"),
            Self::Trace(id) => write!(f, "Trace({})", id),
            Self::InlineCache(id) => write!(f, "InlineCache({})", id),
            Self::PushOtherwise(offset) => write!(f, "PushOtherwise({:+})", offset),
            Self::PopOtherwise => write!(f, "PopOtherwise"),
            Self::PopOtherwiseAndJump(offset) => write!(f, "PopOtherwiseAndJump({:+})", offset),
        }
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub struct InvalidInstruction(pub u8);

impl fmt::Display for InvalidInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid instruction discriminant: 0x{:02X}", self.0)
    }
}

// ============================================================================
// Constant Pool & Program Structure
// ============================================================================

/// Types of constants in the constant pool
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
    Function(FunctionConstant),
    Type(TypeDescriptor),
}

/// Function bytecode and metadata
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionConstant {
    /// Function name (for debugging)
    pub name: String,

    /// Number of parameters
    pub param_count: u8,

    /// Number of upvalues to capture
    pub upvalue_count: u8,

    /// Number of local variables
    pub local_count: u8,

    /// Bytecode (array of Instructions)
    pub bytecode: Vec<Instruction>,

    /// Constants used by this function
    pub constants: Vec<Constant>,
}

/// Type descriptor for runtime type operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDescriptor {
    Int,
    Float,
    Bool,
    String,
    Bytes,
    Array {
        element_type: Box<TypeDescriptor>,
    },
    Map {
        key_type: Box<TypeDescriptor>,
        value_type: Box<TypeDescriptor>,
    },
    Record {
        fields: Vec<(String, TypeDescriptor)>,
    },
    Function {
        param_types: Vec<TypeDescriptor>,
        return_type: Box<TypeDescriptor>,
    },
}

/// Complete bytecode program
#[derive(Debug, Clone)]
pub struct BytecodeProgram {
    /// Entry point function
    pub entry_point: FunctionConstant,

    /// Global constants
    pub constants: Vec<Constant>,

    /// Source map for debugging
    pub source_map: Option<SourceMap>,
}

/// Source map for bytecode debugging
#[derive(Debug, Clone)]
pub struct SourceMap {
    /// Maps instruction index to source location
    pub mappings: Vec<(usize, SourceSpan)>,
}

/// Location in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub column: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_size() {
        // Critical: instructions must be exactly 2 bytes
        assert_eq!(core::mem::size_of::<Instruction>(), 2);
        assert_eq!(Instruction::SIZE, 2);
    }

    #[test]
    fn test_instruction_alignment() {
        // Should have alignment of 1 (no padding)
        assert_eq!(core::mem::align_of::<Instruction>(), 1);
    }

    #[test]
    fn test_instruction_encoding() {
        let inst = Instruction::IntBinOp(b'+');
        let bytes = inst.to_bytes();
        let decoded = Instruction::from_bytes(bytes).unwrap();
        assert_eq!(inst, decoded);
    }

    #[test]
    fn test_halt_is_zero() {
        assert_eq!(Instruction::Halt.discriminant(), 0x00);
    }

    #[test]
    fn test_parameterized_ops() {
        // Test that parameterized ops work correctly
        let add = Instruction::IntBinOp(b'+');
        let sub = Instruction::IntBinOp(b'-');
        assert_ne!(add, sub);

        let lt = Instruction::IntCmpOp(b'<');
        let gt = Instruction::IntCmpOp(b'>');
        assert_ne!(lt, gt);
    }

    #[test]
    fn test_can_error() {
        assert!(Instruction::IntBinOp(b'/').can_error());
        assert!(!Instruction::IntBinOp(b'+').can_error());
        assert!(Instruction::ArrayGet.can_error());
        assert!(!Instruction::ArrayLen.can_error());
    }

    #[test]
    fn test_control_flow() {
        assert!(Instruction::Jump(10).is_control_flow());
        assert!(Instruction::Return.is_control_flow());
        assert!(!Instruction::IntBinOp(b'+').is_control_flow());
    }

    #[test]
    fn test_debug_formatting() {
        let inst = Instruction::IntBinOp(b'+');
        let debug = format!("{:?}", inst);
        assert_eq!(debug, "IntBinOp(+)");

        let cmp = Instruction::IntCmpOp(b'<');
        let debug = format!("{:?}", cmp);
        assert_eq!(debug, "IntCmpOp(<)");
    }
}
