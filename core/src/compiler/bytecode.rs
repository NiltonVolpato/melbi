//! Bytecode compiler implementation.

use crate::{
    analyzer::typed_expr::{Expr, ExprBuilder},
    format,
    scope_stack::{CompleteScope, IncompleteScope, ScopeStack},
    types::manager::TypeManager,
    values::dynamic::Value,
    visitor::TreeTransformer,
    vm::{Code, FunctionAdapter, Instruction},
};
use bumpalo::Bump;

/// Entry in the scope stack: either a local slot index or a global value.
#[derive(Clone, Copy)]
enum ScopeEntry<'types, 'arena> {
    /// Local variable slot index
    Local(u32),
    /// Global value (e.g., Math package) to add to constants
    Global(Value<'types, 'arena>),
}

/// Bytecode compiler that transforms typed expressions into VM bytecode.
///
/// The compiler implements the TreeTransformer pattern to traverse the AST
/// and emit bytecode instructions. It tracks the operand stack precisely
/// to set exact max_stack_size for debugging.
pub struct BytecodeCompiler<'types, 'arena> {
    /// Type manager for creating function adapters
    type_mgr: &'types TypeManager<'types>,

    /// Arena for allocations
    arena: &'arena Bump,

    /// Constant pool for literal values
    ///
    /// We store `Value` (not `RawValue`) to preserve type information for debugging.
    /// At runtime, the VM will extract the RawValue when loading constants.
    ///
    /// Future: In release mode, we could strip types and store only RawValue.
    constants: alloc::vec::Vec<Value<'types, 'arena>>,

    /// Constant deduplication map: Value -> index
    ///
    /// Maps values to their index in the constants pool to avoid duplicates.
    constant_map: hashbrown::HashMap<Value<'types, 'arena>, usize>,

    /// Bytecode instructions
    instructions: alloc::vec::Vec<Instruction>,

    /// Number of local variables
    num_locals: usize,

    /// Scope stack for lexical scoping
    ///
    /// Uses ScopeStack from scope_stack.rs which handles:
    /// - Globals (Math, String packages, etc.) at the bottom
    /// - Expression params (future) in the middle
    /// - Where bindings (pushed/popped dynamically) at the top
    scope_stack: ScopeStack<'arena, ScopeEntry<'types, 'arena>>,

    /// Function adapters for FFI calls
    ///
    /// Each adapter stores parameter types for a call site.
    /// TODO: Deduplicate adapters with same parameter types.
    adapters: alloc::vec::Vec<FunctionAdapter<'types>>,

    /// Current stack depth during compilation
    current_stack_depth: usize,

    /// Maximum stack depth observed (exact tracking for debugging)
    max_stack_size: usize,
}

impl<'types, 'arena> BytecodeCompiler<'types, 'arena> {
    /// Create a new bytecode compiler.
    ///
    /// # Arguments
    /// * `type_mgr` - Type manager for creating function adapters
    /// * `arena` - Arena for allocations
    /// * `globals` - Global values (e.g., Math package) sorted by name
    pub fn new(
        type_mgr: &'types TypeManager<'types>,
        arena: &'arena Bump,
        globals: &'arena [(&'arena str, Value<'types, 'arena>)],
    ) -> Self {
        // Convert globals slice to ScopeEntry format
        let globals_entries: &'arena [(&'arena str, ScopeEntry<'types, 'arena>)] = arena
            .alloc_slice_fill_iter(
                globals
                    .iter()
                    .map(|(name, value)| (*name, ScopeEntry::Global(*value))),
            );

        // Initialize scope stack with globals at the bottom
        let mut scope_stack = ScopeStack::new();
        scope_stack.push(CompleteScope::from_sorted(globals_entries));

        Self {
            type_mgr,
            arena,
            constants: alloc::vec::Vec::new(),
            constant_map: hashbrown::HashMap::new(),
            instructions: alloc::vec::Vec::new(),
            num_locals: 0,
            scope_stack,
            adapters: alloc::vec::Vec::new(),
            current_stack_depth: 0,
            max_stack_size: 0,
        }
    }

    /// Finalize compilation and return the bytecode.
    ///
    /// Converts Value constants (with type info) to RawValue for VM execution.
    pub fn finalize(self) -> Code<'types> {
        // Convert Values to RawValues for VM
        // TODO: In debug mode, we could keep Values for better error messages
        let raw_constants = self
            .constants
            .into_iter()
            .map(|value| value.as_raw())
            .collect();

        Code {
            constants: raw_constants,
            adapters: self.adapters,
            instructions: self.instructions,
            num_locals: self.num_locals,
            max_stack_size: self.max_stack_size,
        }
    }

    /// Convenience method to compile an expression in one call.
    ///
    /// # Arguments
    /// * `type_mgr` - Type manager for creating function adapters
    /// * `arena` - Arena for allocations
    /// * `globals` - Global values (e.g., Math package) sorted by name
    /// * `expr` - The typed expression to compile
    pub fn compile(
        type_mgr: &'types TypeManager<'types>,
        arena: &'arena Bump,
        globals: &'arena [(&'arena str, Value<'types, 'arena>)],
        expr: &'arena Expr<'types, 'arena>,
    ) -> Code<'types> {
        let mut compiler = Self::new(type_mgr, arena, globals);
        compiler.transform(expr);
        // Emit Return instruction to signal end of execution
        compiler.emit(Instruction::Return);
        compiler.finalize()
    }

    // === Stack Management ===

    /// Push a value onto the stack (increases depth by 1).
    fn push_stack(&mut self) {
        self.current_stack_depth += 1;
        if self.current_stack_depth > self.max_stack_size {
            self.max_stack_size = self.current_stack_depth;
        }
    }

    /// Pop a value from the stack (decreases depth by 1).
    fn pop_stack(&mut self) {
        debug_assert!(self.current_stack_depth > 0, "Stack underflow");
        self.current_stack_depth -= 1;
    }

    /// Pop N values from the stack.
    fn pop_stack_n(&mut self, n: usize) {
        debug_assert!(
            self.current_stack_depth >= n,
            "Stack underflow: trying to pop {} but depth is {}",
            n,
            self.current_stack_depth
        );
        self.current_stack_depth -= n;
    }

    // === Instruction Emission ===

    /// Emit an instruction without an argument.
    fn emit(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    /// Emit an instruction with a u32 argument, handling WideArg automatically.
    ///
    /// This is the non-generic implementation to avoid code bloat from monomorphization.
    /// For arg 0x00_12_34_56:
    ///   - 0x56 goes in the instruction itself (passed in `instruction`)
    ///   - 0x00 is not emitted (leading zero)
    ///   - Emit WideArg(0x12), WideArg(0x34) before the instruction
    fn emit_with_arg_impl(&mut self, instruction: Instruction, mut remaining: u32) {
        // Max 3 WideArgs for u32
        let mut wide_bytes = alloc::vec::Vec::with_capacity(3);
        while remaining > 0 {
            wide_bytes.push((remaining & 0xFF) as u8);
            remaining >>= 8;
        }
        // Emit in reverse (most significant byte first)
        for &byte in wide_bytes.iter().rev() {
            self.instructions.push(Instruction::WideArg(byte));
        }
        self.instructions.push(instruction);
    }

    /// Emit an instruction with a u32 argument.
    ///
    /// The generic wrapper constructs the instruction with the low byte,
    /// then delegates to emit_with_arg_impl for WideArg handling.
    fn emit_with_arg<F>(&mut self, make_instr: F, arg: u32)
    where
        F: FnOnce(u8) -> Instruction,
    {
        self.emit_with_arg_impl(make_instr((arg & 0xFF) as u8), arg >> 8);
    }

    // === Local Variable Management ===

    /// Allocate a new local variable slot.
    ///
    /// Always creates a new slot (does not add to scope - that's done separately).
    /// This enables proper variable shadowing.
    fn allocate_local(&mut self) -> u32 {
        let index = self.num_locals;
        self.num_locals += 1;
        index.try_into().expect("Local index overflow")
    }

    // === Constant Pool Management ===

    /// Add a constant to the pool (or reuse existing) and return its index.
    ///
    /// Deduplicates constants by value equality.
    /// Returns the index as u32 - emit_with_arg handles WideArg if needed.
    fn add_constant(&mut self, value: Value<'types, 'arena>) -> u32 {
        // Check if this constant already exists
        if let Some(&existing_index) = self.constant_map.get(&value) {
            return existing_index as u32;
        }

        // Add new constant
        let index = self.constants.len();
        self.constants.push(value);
        self.constant_map.insert(value, index);
        index.try_into().expect("Constant index overflow")
    }

    // === Jump Patching Infrastructure ===

    /// Reserve space for a jump instruction and return its index.
    ///
    /// The jump target will be patched later with `patch_jump`.
    /// We reserve 2 instructions to support 64K jump range.
    fn jump_placeholder(&mut self) -> usize {
        let placeholder_index = self.instructions.len();
        // Reserve space - we'll patch these instructions later
        self.emit(Instruction::Nop);
        self.emit(Instruction::Nop);
        placeholder_index
    }

    /// Get the current instruction index (for use as a jump label).
    fn label(&self) -> usize {
        self.instructions.len()
    }

    /// Patch a jump placeholder with the actual jump instruction.
    ///
    /// # Arguments
    /// * `placeholder_index` - The index returned by `jump_placeholder()`
    /// * `target_label` - The target instruction index from `label()`
    /// * `make_jump` - Function that creates the jump instruction with the offset
    fn patch_jump<F>(&mut self, placeholder_index: usize, target_label: usize, make_jump: F)
    where
        F: FnOnce(u8) -> Instruction,
    {
        // Calculate the offset from the jump instruction to the target
        // The VM loop automatically increments the instruction pointer after each instruction,
        // so: offset = target - current - 1
        debug_assert!(target_label >= placeholder_index);
        let offset = target_label as usize - placeholder_index as usize - 1;

        // For now, we'll use single-instruction jumps (i8 range: -128 to 127)
        // TODO: Support wider range with two-instruction encoding
        let offset_u8: u8 = offset.try_into().expect(&format!(
            "Jump offset {} out of range for u8 (0 to 255)",
            offset
        ));

        // Patch the placeholder with the actual jump instruction
        self.instructions[placeholder_index] = make_jump(offset_u8);
        // Second instruction stays as Nop for now (could be used for extended offset)
    }
}

impl<'types, 'arena> TreeTransformer<ExprBuilder<'types, 'arena>>
    for BytecodeCompiler<'types, 'arena>
where
    'types: 'arena,
{
    type Output = ();

    fn transform(&mut self, tree: &'arena Expr<'types, 'arena>) -> Self::Output {
        use crate::{
            analyzer::typed_expr::ExprInner,
            parser::{BinaryOp, BoolOp, ComparisonOp},
            visitor::TreeView,
        };

        match tree.view() {
            // === Constants ===
            ExprInner::Constant(value) => {
                if let Ok(i) = value.as_int() {
                    // Use immediate encoding for small integers
                    if i >= i8::MIN as i64 && i <= i8::MAX as i64 {
                        self.emit(Instruction::ConstInt(i as i8));
                        self.push_stack();
                    } else if i >= 0 && i <= u8::MAX as i64 {
                        self.emit(Instruction::ConstUInt(i as u8));
                        self.push_stack();
                    } else {
                        // Large integer - use constant pool
                        let const_index = self.add_constant(value);
                        self.emit_with_arg(Instruction::ConstLoad, const_index);
                        self.push_stack();
                    }
                } else if let Ok(b) = value.as_bool() {
                    // Use immediate encoding for booleans
                    if b {
                        self.emit(Instruction::ConstBool(1));
                    } else {
                        self.emit(Instruction::ConstBool(0));
                    }
                    self.push_stack();
                } else {
                    // Other types (float, string, etc.) - use constant pool
                    let const_index = self.add_constant(value);
                    self.emit_with_arg(Instruction::ConstLoad, const_index);
                    self.push_stack();
                }
            }

            // === Binary Operations ===
            ExprInner::Binary { op, left, right } => {
                // Compile left operand
                self.transform(left);

                // Compile right operand
                self.transform(right);

                // Emit operation instruction (pops 2, pushes 1)
                self.pop_stack_n(2);
                let op_byte = match op {
                    BinaryOp::Add => b'+',
                    BinaryOp::Sub => b'-',
                    BinaryOp::Mul => b'*',
                    BinaryOp::Div => b'/',
                    BinaryOp::Pow => b'^',
                };

                // Check if this is a float or int operation based on the result type
                use crate::types::traits::{TypeKind, TypeView};
                match tree.0.view() {
                    TypeKind::Float => self.emit(Instruction::FloatBinOp(op_byte)),
                    TypeKind::Int => self.emit(Instruction::IntBinOp(op_byte)),
                    _ => panic!("Binary operation on non-numeric type"),
                }
                self.push_stack();
            }

            // === Unary Operations ===
            ExprInner::Unary { op, expr } => {
                use crate::parser::UnaryOp;
                use crate::types::traits::{TypeKind, TypeView};

                // Compile operand
                self.transform(expr);

                // Emit operation (pops 1, pushes 1)
                self.pop_stack();
                match op {
                    UnaryOp::Neg => {
                        // Check if this is float or int negation based on operand type
                        match expr.0.view() {
                            TypeKind::Float => self.emit(Instruction::NegFloat),
                            TypeKind::Int => self.emit(Instruction::NegInt),
                            _ => panic!("Negation on non-numeric type"),
                        }
                    }
                    UnaryOp::Not => {
                        self.emit(Instruction::Not);
                    }
                }
                self.push_stack();
            }

            // === Comparison Operations ===
            ExprInner::Comparison { op, left, right } => {
                use crate::types::traits::{TypeKind, TypeView};

                // Compile left operand
                self.transform(left);

                // Compile right operand
                self.transform(right);

                // Emit comparison instruction (pops 2, pushes 1)
                self.pop_stack_n(2);
                let op_byte = match op {
                    ComparisonOp::Lt => b'<',
                    ComparisonOp::Gt => b'>',
                    ComparisonOp::Eq => b'=',
                    ComparisonOp::Neq => b'!',
                    ComparisonOp::Le => b'l',
                    ComparisonOp::Ge => b'g',
                    ComparisonOp::In | ComparisonOp::NotIn => {
                        todo!("Implement 'in' operator")
                    }
                };

                // Check if we're comparing floats or ints based on operand type
                match left.0.view() {
                    TypeKind::Float => self.emit(Instruction::FloatCmpOp(op_byte)),
                    TypeKind::Int => self.emit(Instruction::IntCmpOp(op_byte)),
                    _ => panic!("Comparison on non-numeric type"),
                }
                self.push_stack();
            }

            // === Boolean Operations ===
            ExprInner::Boolean { op, left, right } => {
                // For And/Or, we need short-circuit evaluation
                // For now, implement eager evaluation
                // TODO: Implement short-circuit with JumpIfFalseNoPop/JumpIfTrueNoPop
                // todo!("Implement short-circuit evaluation");

                // Compile left operand
                self.transform(left);

                // Compile right operand
                self.transform(right);

                // Emit operation (pops 2, pushes 1)
                self.pop_stack_n(2);
                match op {
                    BoolOp::And => self.emit(Instruction::And),
                    BoolOp::Or => self.emit(Instruction::Or),
                }
                self.push_stack();
            }

            // === If Expressions ===
            ExprInner::If {
                cond,
                then_branch,
                else_branch,
            } => {
                // Compile condition
                self.transform(cond);
                self.pop_stack(); // Condition consumed by PopJumpIfFalse

                // Reserve space for jump to else branch
                let else_jump = self.jump_placeholder();

                // Save stack depth before branches
                // Both branches will leave exactly one result on the stack
                let depth_before_branches = self.current_stack_depth;

                // Compile then branch
                self.transform(then_branch);
                // Then branch leaves one result on stack

                // Reserve space for jump over else branch
                let end_jump = self.jump_placeholder();

                // Patch the else jump to point here
                let else_label = self.label();
                self.patch_jump(else_jump, else_label, Instruction::PopJumpIfFalse);

                // Reset stack depth for else branch
                // (only one branch executes at runtime, so they share the same stack space)
                self.current_stack_depth = depth_before_branches;

                // Compile else branch
                self.transform(else_branch);
                // Else branch leaves one result on stack

                // Patch the end jump to point here
                let end_label = self.label();
                self.patch_jump(end_jump, end_label, Instruction::JumpForward);

                // After if expression, exactly one result is on stack
                self.current_stack_depth = depth_before_branches + 1;
            }

            // === Array Construction ===
            ExprInner::Array { elements } => {
                // Compile all element expressions
                // They will be pushed onto the stack in order
                for element in elements.iter() {
                    self.transform(element);
                }

                // MakeArray pops N elements and pushes 1 array
                let count = elements.len();
                self.pop_stack_n(count);

                // Emit MakeArray instruction
                self.emit_with_arg(Instruction::MakeArray, count as u32);
                self.push_stack();
            }

            // === Variable Access ===
            ExprInner::Ident(name) => {
                // Look up the variable in the scope stack (locals and globals)
                match self.scope_stack.lookup(name) {
                    Some(ScopeEntry::Local(index)) => {
                        self.emit_with_arg(Instruction::LoadLocal, *index);
                    }
                    Some(ScopeEntry::Global(value)) => {
                        // Global value - add to constants and load
                        let const_index = self.add_constant(*value);
                        self.emit_with_arg(Instruction::ConstLoad, const_index);
                    }
                    None => panic!(
                        "Undefined variable '{}' (should be caught by type checker)",
                        name
                    ),
                }
                self.push_stack();
            }

            // === Where Bindings ===
            ExprInner::Where { expr, bindings } => {
                // Collect binding names for the incomplete scope
                let names: alloc::vec::Vec<_> = bindings.iter().map(|(name, _)| *name).collect();

                // Push an incomplete scope for the bindings
                self.scope_stack.push(
                    IncompleteScope::new(self.arena, &names)
                        .expect("Duplicate binding names (should be caught by type checker)"),
                );

                // Compile all bindings first (in order)
                for (name, value_expr) in bindings.iter() {
                    // Compile the value expression
                    self.transform(value_expr);
                    self.pop_stack();

                    // Allocate a new local slot
                    let index = self.allocate_local();
                    self.emit_with_arg(Instruction::StoreLocal, index);

                    // Bind the name to the local slot in the current scope
                    self.scope_stack
                        .bind_in_current(name, ScopeEntry::Local(index))
                        .expect("Failed to bind variable (should not happen)");
                }

                // Then compile the main expression (which can reference the bindings)
                self.transform(expr);
                // Result is left on stack

                // Pop the scope when done
                self.scope_stack.pop().expect("Scope stack underflow");
            }

            // === Index Operations ===
            ExprInner::Index { value, index } => {
                use crate::types::traits::TypeKind;

                // Compile the value expression (array or map)
                self.transform(value);

                // Check if index is a constant for optimization
                if let ExprInner::Constant(idx_val) = index.view() {
                    if let Ok(i) = idx_val.as_int() {
                        if 0 <= i && i <= 127 {
                            // Use constant index optimization for arrays
                            match value.type_view() {
                                TypeKind::Array(_) => {
                                    self.pop_stack(); // Pop array
                                    self.emit_with_arg(Instruction::ArrayGetConst, i as u32);
                                    self.push_stack(); // Push result
                                    return;
                                }
                                _ => {} // Fall through to generic case (including maps)
                            }
                        }
                    }
                }

                // Dynamic index: compile index expression
                self.transform(index);

                // Emit appropriate get instruction based on value type
                self.pop_stack_n(2); // Pop index and container
                match value.type_view() {
                    TypeKind::Array(_) => {
                        self.emit(Instruction::ArrayGet);
                    }
                    TypeKind::Map(_, _) => {
                        self.emit(Instruction::MapGet);
                    }
                    _ => panic!("Index operation on non-indexable type"),
                }
                self.push_stack(); // Push result
            }

            // === Field Access ===
            ExprInner::Field { value, field } => {
                use crate::types::traits::TypeKind;

                // Compile the record expression
                self.transform(value);

                // Look up field index in the record type
                let field_index = match value.type_view() {
                    TypeKind::Record(fields) => {
                        // Fields are sorted by name, find the index
                        let mut idx = None;
                        for (i, (name, _ty)) in fields.enumerate() {
                            if name == field {
                                idx = Some(i);
                                break;
                            }
                        }
                        idx.expect(
                            "Field not found in record type (should be caught by type checker)",
                        )
                    }
                    _ => panic!("Field access on non-record type"),
                };

                // Emit RecordGet instruction
                self.pop_stack(); // Pop record
                self.emit_with_arg(Instruction::RecordGet, field_index as u32);
                self.push_stack(); // Push field value
            }

            // === Record Construction ===
            ExprInner::Record { fields } => {
                // Compile all field values in order
                // Fields are already sorted by name in the typed representation
                for (_name, value_expr) in fields.iter() {
                    self.transform(value_expr);
                }

                // MakeRecord pops N values and pushes 1 record
                let count = fields.len();
                self.pop_stack_n(count);

                // Emit MakeRecord instruction
                self.emit_with_arg(Instruction::MakeRecord, count as u32);
                self.push_stack();
            }

            // === Map Construction ===
            ExprInner::Map { elements } => {
                // Compile all key-value pairs
                // Each pair pushes key then value onto the stack
                for (key_expr, value_expr) in elements.iter() {
                    self.transform(key_expr); // Push key
                    self.transform(value_expr); // Push value
                }

                // MakeMap pops 2*N values (N key-value pairs) and pushes 1 map
                let num_pairs = elements.len();
                self.pop_stack_n(num_pairs * 2);

                // Emit MakeMap instruction
                self.emit_with_arg(Instruction::MakeMap, num_pairs as u32);
                self.push_stack();
            }

            ExprInner::Otherwise { primary, fallback } => {
                // Reserve placeholder for PushOtherwise (will patch with offset to fallback)
                let push_placeholder_idx = self.instructions.len();
                self.emit(Instruction::PushOtherwise(0)); // Placeholder

                // Compile primary expression (may error)
                self.transform(primary);

                // Reserve placeholder for PopOtherwiseAndJump (will patch with offset to done)
                let pop_and_jump_placeholder_idx = self.instructions.len();
                self.emit(Instruction::PopOtherwiseAndJump(0)); // Placeholder

                // Fallback label (VM jumps here on error)
                let fallback_offset = self.instructions.len();

                // Pop the otherwise handler
                self.emit(Instruction::PopOtherwise);

                // Compile fallback expression
                self.transform(fallback);

                // Done label
                let done_offset = self.instructions.len();

                // Patch PushOtherwise jump to fallback
                let push_delta = fallback_offset - push_placeholder_idx;
                let push_delta_u8: u8 = push_delta
                    .try_into()
                    .expect("Otherwise jump offset too large (>255)");
                self.instructions[push_placeholder_idx] = Instruction::PushOtherwise(push_delta_u8);

                // Patch PopOtherwiseAndJump to done
                let pop_jump_delta = done_offset - pop_and_jump_placeholder_idx - 1;
                let pop_jump_delta_u8: u8 = pop_jump_delta
                    .try_into()
                    .expect("Otherwise jump offset too large (>255)");
                self.instructions[pop_and_jump_placeholder_idx] =
                    Instruction::PopOtherwiseAndJump(pop_jump_delta_u8);

                // Stack depth: same as primary/fallback (both have same type)
                // No change needed
            }

            // === Option Construction ===
            ExprInner::Option { inner } => {
                match inner {
                    Some(value_expr) => {
                        // some expr: compile the inner expression, then wrap with MakeOption(1)
                        self.transform(value_expr);
                        // MakeOption(1) pops 1 value and pushes 1 option
                        self.pop_stack();
                        self.emit(Instruction::MakeOption(1));
                        self.push_stack();
                    }
                    None => {
                        // none: just create a None value with MakeOption(0)
                        self.emit(Instruction::MakeOption(0));
                        self.push_stack();
                    }
                }
            }

            // === Function Calls ===
            ExprInner::Call { callable, args } => {
                use crate::types::traits::{TypeKind, TypeView};

                // 1. Compile arguments first (they go on stack before function)
                for arg in args.iter() {
                    self.transform(arg);
                }

                // 2. Compile the callable (pushes function value on stack)
                self.transform(callable);

                // 3. Extract parameter types from callable's function type
                let param_types: alloc::vec::Vec<_> = match callable.0.view() {
                    TypeKind::Function { params, .. } => params.collect(),
                    _ => panic!("Call on non-function (should be caught by type checker)"),
                };

                // 4. Create and store the adapter
                // TODO: Deduplicate adapters with same parameter types
                let adapter = FunctionAdapter::new(self.type_mgr, param_types);
                let adapter_index = self.adapters.len();
                self.adapters.push(adapter);

                // 5. Emit Call instruction
                self.pop_stack_n(args.len() + 1); // Pop args + function
                self.emit_with_arg(Instruction::Call, adapter_index as u32);
                self.push_stack(); // Push result
            }

            // TODO: Add tests for Cast and implement VM instructions.
            ExprInner::Cast { expr } => {
                self.transform(expr);
                self.pop_stack();
                self.emit(Instruction::Cast(0)); // TODO: This makes no sense.
                self.push_stack();
            }

            ExprInner::Lambda { .. } => {
                // TODO: Monomorphize the lambda for all instantiations.
                todo!("Implement Lambda");
            }

            ExprInner::Match { .. } => {
                todo!("Implement Match");
            }

            ExprInner::FormatStr { .. } => {
                todo!("Implement FormatStr");
            }
        }
    }
}
