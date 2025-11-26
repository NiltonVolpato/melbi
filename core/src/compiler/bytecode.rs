//! Bytecode compiler implementation.

use crate::{
    analyzer::typed_expr::{Expr, ExprBuilder},
    format,
    values::dynamic::Value,
    visitor::TreeTransformer,
    vm::{Code, Instruction},
};

/// Bytecode compiler that transforms typed expressions into VM bytecode.
///
/// The compiler implements the TreeTransformer pattern to traverse the AST
/// and emit bytecode instructions. It tracks the operand stack precisely
/// to set exact max_stack_size for debugging.
pub struct BytecodeCompiler<'types, 'arena> {
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
    /// Each scope is a HashMap mapping variable names to their local slot index.
    /// When entering a Where block, we push a new scope.
    /// When exiting, we pop the scope.
    /// This allows proper variable shadowing.
    scopes: alloc::vec::Vec<hashbrown::HashMap<&'arena str, u8>>,

    /// Current stack depth during compilation
    current_stack_depth: usize,

    /// Maximum stack depth observed (exact tracking for debugging)
    max_stack_size: usize,

    /// Phantom data for lifetimes
    _phantom: core::marker::PhantomData<(&'types (), &'arena ())>,
}

impl<'types, 'arena> BytecodeCompiler<'types, 'arena> {
    /// Create a new bytecode compiler.
    pub fn new() -> Self {
        let mut scopes = alloc::vec::Vec::new();
        scopes.push(hashbrown::HashMap::new()); // Global scope

        Self {
            constants: alloc::vec::Vec::new(),
            constant_map: hashbrown::HashMap::new(),
            instructions: alloc::vec::Vec::new(),
            num_locals: 0,
            scopes,
            current_stack_depth: 0,
            max_stack_size: 0,
            _phantom: core::marker::PhantomData,
        }
    }

    /// Finalize compilation and return the bytecode.
    ///
    /// Converts Value constants (with type info) to RawValue for VM execution.
    pub fn finalize(self) -> Code {
        // Convert Values to RawValues for VM
        // TODO: In debug mode, we could keep Values for better error messages
        let raw_constants = self
            .constants
            .into_iter()
            .map(|value| value.as_raw())
            .collect();

        Code {
            constants: raw_constants,
            instructions: self.instructions,
            num_locals: self.num_locals,
            max_stack_size: self.max_stack_size,
        }
    }

    /// Convenience method to compile an expression in one call.
    pub fn compile(expr: &'arena Expr<'types, 'arena>) -> Code {
        let mut compiler = Self::new();
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

    /// Emit an instruction.
    fn emit(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    // === Local Variable Management ===

    /// Look up a variable in the current scope chain.
    ///
    /// Searches from the innermost scope outward.
    fn lookup_local(&self, name: &'arena str) -> Option<u8> {
        // Search from innermost scope to outermost
        for scope in self.scopes.iter().rev() {
            if let Some(&index) = scope.get(name) {
                return Some(index);
            }
        }
        None
    }

    /// Allocate a new local variable slot in the current scope.
    ///
    /// Always creates a new slot, even if the same name exists in an outer scope.
    /// This enables proper variable shadowing.
    fn allocate_local(&mut self, name: &'arena str) -> Result<u8, &'static str> {
        let index = self.num_locals;
        let index_u8: u8 = index
            .try_into()
            .map_err(|_| "Too many local variables (>255)")?;

        // Add to current (innermost) scope
        self.scopes.last_mut().unwrap().insert(name, index_u8);
        self.num_locals += 1;

        Ok(index_u8)
    }

    /// Push a new scope for where bindings.
    fn push_scope(&mut self) {
        self.scopes.push(hashbrown::HashMap::new());
    }

    /// Pop the current scope when exiting a where block.
    fn pop_scope(&mut self) {
        self.scopes.pop();
        debug_assert!(!self.scopes.is_empty(), "Cannot pop global scope");
    }

    // === Constant Pool Management ===

    /// Add a constant to the pool (or reuse existing) and return its index.
    ///
    /// Deduplicates constants by value equality.
    /// Returns an error if the constant pool exceeds 255 entries (u8 limit).
    ///
    /// TODO: Support WideArg prefix for constants > 255
    fn add_constant(&mut self, value: Value<'types, 'arena>) -> Result<u8, &'static str> {
        // Check if this constant already exists
        if let Some(&existing_index) = self.constant_map.get(&value) {
            return existing_index
                .try_into()
                .map_err(|_| "Constant pool index overflow (>255)");
        }

        // Add new constant
        let index = self.constants.len();
        self.constants.push(value);
        self.constant_map.insert(value, index);

        index
            .try_into()
            .map_err(|_| "Constant pool overflow: more than 255 constants")
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
        F: FnOnce(i8) -> Instruction,
    {
        // Calculate the offset from the jump instruction to the target
        // The VM loop automatically increments the instruction pointer after each instruction,
        // so: offset = target - current - 1
        let offset = target_label as isize - placeholder_index as isize - 1;

        // For now, we'll use single-instruction jumps (i8 range: -128 to 127)
        // TODO: Support wider range with two-instruction encoding
        let offset_i8 = offset.try_into().expect(&format!(
            "Jump offset {} out of range for i8 (-128 to 127)",
            offset
        ));

        // Patch the placeholder with the actual jump instruction
        self.instructions[placeholder_index] = make_jump(offset_i8);
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
                        let const_index = self
                            .add_constant(value)
                            .expect("Constant pool overflow - TODO: support WideArg");
                        self.emit(Instruction::ConstLoad(const_index));
                        self.push_stack();
                    }
                } else if let Ok(b) = value.as_bool() {
                    // Use immediate encoding for booleans
                    if b {
                        self.emit(Instruction::ConstTrue);
                    } else {
                        self.emit(Instruction::ConstFalse);
                    }
                    self.push_stack();
                } else {
                    // Other types (float, string, etc.) - use constant pool
                    let const_index = self
                        .add_constant(value)
                        .expect("Constant pool overflow - TODO: support WideArg");
                    self.emit(Instruction::ConstLoad(const_index));
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
                self.pop_stack(); // Condition consumed by JumpIfFalse

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
                self.patch_jump(else_jump, else_label, Instruction::JumpIfFalse);

                // Reset stack depth for else branch
                // (only one branch executes at runtime, so they share the same stack space)
                self.current_stack_depth = depth_before_branches;

                // Compile else branch
                self.transform(else_branch);
                // Else branch leaves one result on stack

                // Patch the end jump to point here
                let end_label = self.label();
                self.patch_jump(end_jump, end_label, Instruction::Jump);

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
                let count_u8: u8 = count
                    .try_into()
                    .expect("Array has more than 255 elements - TODO: support larger arrays");
                self.emit(Instruction::MakeArray(count_u8));
                self.push_stack();
            }

            // === Variable Access ===
            ExprInner::Ident(name) => {
                // Look up the variable in the scope chain
                let index = self
                    .lookup_local(name)
                    .expect("Undefined variable (should be caught by type checker)");
                self.emit(Instruction::LoadLocal(index));
                self.push_stack();
            }

            // === Where Bindings ===
            ExprInner::Where { expr, bindings } => {
                // Push a new scope for the bindings
                self.push_scope();

                // Compile all bindings first (in order)
                for (name, value_expr) in bindings.iter() {
                    // Compile the value expression
                    self.transform(value_expr);
                    self.pop_stack();

                    // Allocate a NEW local slot (even if name exists in outer scope)
                    let index = self
                        .allocate_local(name)
                        .expect("Failed to allocate local variable");
                    self.emit(Instruction::StoreLocal(index));
                }

                // Then compile the main expression (which can reference the bindings)
                self.transform(expr);
                // Result is left on stack

                // Pop the scope when done
                self.pop_scope();
            }

            // === Index Operations ===
            ExprInner::Index { value, index } => {
                use crate::types::traits::TypeKind;

                // Compile the value expression (array or map)
                self.transform(value);

                // Check if index is a constant for optimization
                if let ExprInner::Constant(idx_val) = index.view() {
                    if let Ok(i) = idx_val.as_int() {
                        if i >= 0 && i <= u8::MAX as i64 {
                            // Use constant index optimization for arrays
                            match value.type_view() {
                                TypeKind::Array(_) => {
                                    self.pop_stack(); // Pop array
                                    self.emit(Instruction::ArrayGetConst(i as u8));
                                    self.push_stack(); // Push result
                                    return;
                                }
                                _ => {} // Fall through to dynamic case for maps
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

                // Convert to u8 (field indices should be < 256)
                let field_index_u8: u8 = field_index
                    .try_into()
                    .expect("Field index too large (>255)");

                // Emit RecordGet instruction
                self.pop_stack(); // Pop record
                self.emit(Instruction::RecordGet(field_index_u8));
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
                let count_u8: u8 = count.try_into().expect("Record has more than 255 fields");
                self.emit(Instruction::MakeRecord(count_u8));
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
                let num_pairs_u8: u8 = num_pairs
                    .try_into()
                    .expect("Map has more than 255 key-value pairs");
                self.emit(Instruction::MakeMap(num_pairs_u8));
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
                let push_delta = (fallback_offset as i32 - push_placeholder_idx as i32) as i8;
                self.instructions[push_placeholder_idx] = Instruction::PushOtherwise(push_delta);

                // Patch PopOtherwiseAndJump to done
                let pop_jump_delta =
                    (done_offset as i32 - pop_and_jump_placeholder_idx as i32 - 1) as i8;
                self.instructions[pop_and_jump_placeholder_idx] =
                    Instruction::PopOtherwiseAndJump(pop_jump_delta);

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

            // TODO: Add tests for Call and implement VM instructions..
            ExprInner::Call { callable, args } => {
                for arg in args.iter() {
                    self.transform(arg);
                }
                self.transform(callable);

                self.pop_stack_n(args.len() + 1);
                self.emit(Instruction::Call(args.len().try_into().unwrap()));
                self.push_stack();
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
