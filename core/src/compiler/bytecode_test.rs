//! Tests for the bytecode compiler.

use crate::{
    analyzer,
    compiler::BytecodeCompiler,
    evaluator::ExecutionError,
    parser,
    stdlib::math::build_math_package,
    types::manager::TypeManager,
    values::dynamic::Value,
    vm::{Code, Instruction, VM},
};
use bumpalo::Bump;

/// Helper function to compile and run a source expression.
/// Returns the compiled bytecode and the VM execution result as a safe Value.
///
/// This helper includes the Math package by default, so all tests can use
/// `Math.Sin`, `Math.Sqrt`, etc. without any extra setup.
fn compile_and_run<'a>(
    arena: &'a Bump,
    type_manager: &'a TypeManager<'a>,
    source: &str,
) -> (Code<'a>, Result<Value<'a, 'a>, ExecutionError>) {
    // Build Math package (available to all tests)
    let math = build_math_package(arena, type_manager).unwrap();

    // Globals for analyzer (types only)
    let globals_types = &[("Math", math.ty)];
    // Globals for compiler (values)
    let globals_values = arena.alloc_slice_copy(&[("Math", math)]);

    let parsed = parser::parse(arena, source).unwrap();
    let typed = analyzer::analyze(type_manager, arena, &parsed, globals_types, &[]).unwrap();
    let result_type = typed.expr.0;
    let code = BytecodeCompiler::compile(type_manager, arena, globals_values, typed.expr).unwrap();
    let result =
        VM::execute(arena, &code).map(|raw| unsafe { Value::from_raw_unchecked(result_type, raw) });
    (code, result)
}

#[test]
fn test_compile_simple_integer() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "42");

    // Verify bytecode: ConstInt(42), Return
    assert_eq!(code.instructions.len(), 2);
    assert_eq!(code.instructions[0], Instruction::ConstInt(42));
    assert_eq!(code.instructions[1], Instruction::Return);
    assert_eq!(code.max_stack_size, 1);
    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 42);
}

#[test]
fn test_compile_addition() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "2 + 3");

    // Verify bytecode: ConstInt(2), ConstInt(3), IntBinOp('+'), Return
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(2));
    assert_eq!(code.instructions[1], Instruction::ConstInt(3));
    assert_eq!(code.instructions[2], Instruction::IntBinOp(b'+'));
    assert_eq!(code.instructions[3], Instruction::Return);
    assert_eq!(
        code.max_stack_size, 2,
        "Stack depth should be 2 (two operands)"
    );
    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 5);
}

#[test]
fn test_compile_subtraction() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "10 - 3");

    // Verify bytecode: ConstInt(10), ConstInt(3), IntBinOp('-'), Return
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(10));
    assert_eq!(code.instructions[1], Instruction::ConstInt(3));
    assert_eq!(code.instructions[2], Instruction::IntBinOp(b'-'));
    assert_eq!(code.instructions[3], Instruction::Return);
    assert_eq!(code.max_stack_size, 2);
    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 7);
}

#[test]
fn test_compile_multiplication() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "5 * 7");

    // Verify bytecode: ConstInt(5), ConstInt(7), IntBinOp('*'), Return
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(5));
    assert_eq!(code.instructions[1], Instruction::ConstInt(7));
    assert_eq!(code.instructions[2], Instruction::IntBinOp(b'*'));
    assert_eq!(code.instructions[3], Instruction::Return);
    assert_eq!(code.max_stack_size, 2);
    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 35);
}

#[test]
fn test_compile_negation() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "-(5)");

    // Verify bytecode: ConstInt(5), NegInt, Return
    assert_eq!(code.instructions.len(), 3);
    assert_eq!(code.instructions[0], Instruction::ConstInt(5));
    assert_eq!(code.instructions[1], Instruction::NegInt);
    assert_eq!(code.instructions[2], Instruction::Return);
    assert_eq!(code.max_stack_size, 1);
    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), -5);
}

#[test]
fn test_compile_complex_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "(2 + 3) * 4");

    // Verify bytecode:
    // ConstInt(2), ConstInt(3), IntBinOp('+'), ConstInt(4), IntBinOp('*'), Return
    assert_eq!(code.instructions.len(), 6);
    assert_eq!(code.instructions[0], Instruction::ConstInt(2));
    assert_eq!(code.instructions[1], Instruction::ConstInt(3));
    assert_eq!(code.instructions[2], Instruction::IntBinOp(b'+'));
    assert_eq!(code.instructions[3], Instruction::ConstInt(4));
    assert_eq!(code.instructions[4], Instruction::IntBinOp(b'*'));
    assert_eq!(code.instructions[5], Instruction::Return);
    assert_eq!(code.max_stack_size, 2, "Stack depth should be 2");
    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 20);
}

#[test]
fn test_stack_depth_tracking() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "1 + 2 + 3");

    // Stack never grows beyond 2 because we evaluate left-to-right
    assert_eq!(
        code.max_stack_size, 2,
        "Stack should never exceed 2 for left-associative operations"
    );
    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 6);
}

#[test]
fn test_debug_output() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "(2 + 3) * 4");

    // Print debug output to demonstrate assembly-style listing
    println!("\n{:?}\n", code);

    // Verify it compiled correctly
    assert_eq!(code.instructions.len(), 6);
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_convenience_compile_method() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "10 - 3");

    // Verify it works the same as the manual approach
    // ConstInt(10), ConstInt(3), IntBinOp('-'), Return
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(10));
    assert_eq!(code.instructions[1], Instruction::ConstInt(3));
    assert_eq!(code.instructions[2], Instruction::IntBinOp(b'-'));
    assert_eq!(code.instructions[3], Instruction::Return);
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_constant_deduplication() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "1000 + 1000 + 1000");

    // Verify that 1000 only appears once in the constant pool
    assert_eq!(
        code.constants.len(),
        1,
        "Should only have 1 unique constant (1000 deduplicated)"
    );

    // Verify the bytecode uses the same constant index three times
    // Expected: ConstLoad(0), ConstLoad(0), IntBinOp('+'), ConstLoad(0), IntBinOp('+')
    assert_eq!(code.instructions.len(), 6);
    assert_eq!(code.instructions[0], Instruction::ConstLoad(0));
    assert_eq!(code.instructions[1], Instruction::ConstLoad(0));
    assert_eq!(code.instructions[2], Instruction::IntBinOp(b'+'));
    assert_eq!(code.instructions[3], Instruction::ConstLoad(0));
    assert_eq!(code.instructions[4], Instruction::IntBinOp(b'+'));
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_comparison_operations() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "5 < 10");

    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(5));
    assert_eq!(code.instructions[1], Instruction::ConstInt(10));
    assert_eq!(code.instructions[2], Instruction::IntCmpOp(b'<'));
    assert_eq!(code.max_stack_size, 2);
    // Verify result
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_boolean_not() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "not (5 < 10)");

    // Expected: ConstInt(5), ConstInt(10), IntCmpOp('<'), Not
    assert_eq!(code.instructions.len(), 5);
    assert_eq!(code.instructions[0], Instruction::ConstInt(5));
    assert_eq!(code.instructions[1], Instruction::ConstInt(10));
    assert_eq!(code.instructions[2], Instruction::IntCmpOp(b'<'));
    assert_eq!(code.instructions[3], Instruction::Not);
    assert_eq!(code.max_stack_size, 2);
    // Verify result
    assert_eq!(result.unwrap().as_bool().unwrap(), false);
}

#[test]
fn test_boolean_and() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "true and false");

    // Expected: ConstTrue, ConstFalse, And, Return
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstBool(1));
    assert_eq!(code.instructions[1], Instruction::ConstBool(0));
    assert_eq!(code.instructions[2], Instruction::And);
    assert_eq!(code.instructions[3], Instruction::Return);
    assert_eq!(code.max_stack_size, 2);
    // Verify result
    assert_eq!(result.unwrap().as_bool().unwrap(), false);
}

#[test]
fn test_if_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "if true then 42 else 99");

    // Print debug output to see the generated bytecode
    println!("\n{:?}\n", code);

    // Verify structure (exact offsets depend on jump patching implementation)
    // Should have: ConstTrue, JumpIfFalse, ConstInt(42), Jump, ConstInt(99)
    assert!(code.instructions.len() >= 6);
    assert_eq!(code.instructions[0], Instruction::ConstBool(1));
    assert_eq!(
        code.max_stack_size, 1,
        "If expressions should have stack depth of 1"
    );
    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 42);
}

#[test]
fn test_all_comparison_operators() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test all comparison operators
    let tests = vec![
        ("1 == 1", b'='),
        ("1 != 2", b'!'),
        ("1 < 2", b'<'),
        ("2 > 1", b'>'),
        ("1 <= 2", b'l'),
        ("2 >= 1", b'g'),
    ];

    for (expr, expected_op) in tests {
        let (code, _result) = compile_and_run(&arena, &type_manager, expr);

        assert_eq!(
            code.instructions[2],
            Instruction::IntCmpOp(expected_op),
            "Failed for expression: {}",
            expr
        );
    }
}

#[test]
fn test_boolean_or() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "false or true");

    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstBool(0));
    assert_eq!(code.instructions[1], Instruction::ConstBool(1));
    assert_eq!(code.instructions[2], Instruction::Or);
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_complex_boolean_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "(5 < 10) and (3 > 1)");

    // Should compile to:
    // ConstInt(5), ConstInt(10), IntCmpOp(<),
    // ConstInt(3), ConstInt(1), IntCmpOp(>),
    // And
    assert_eq!(code.instructions.len(), 8);
    assert_eq!(code.instructions[2], Instruction::IntCmpOp(b'<'));
    assert_eq!(code.instructions[5], Instruction::IntCmpOp(b'>'));
    assert_eq!(code.instructions[6], Instruction::And);
    // Stack depth is 3: first comparison leaves result (1), then second comparison needs 2 more slots
    assert_eq!(code.max_stack_size, 3);
}

#[test]
fn test_nested_if_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(
        &arena,
        &type_manager,
        "if true then (if false then 1 else 2) else 3",
    );

    println!("\nNested if bytecode:\n{:?}\n", code);

    // Should have nested jump structure
    assert!(
        code.instructions.len() >= 10,
        "Nested if should have multiple jumps"
    );
    assert_eq!(
        code.max_stack_size, 1,
        "Nested if should still have stack depth of 1"
    );
}

#[test]
fn test_if_with_complex_condition() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(
        &arena,
        &type_manager,
        "if (5 < 10) and (3 > 1) then 100 else 200",
    );

    println!("\nIf with complex condition:\n{:?}\n", code);

    // Complex condition evaluates two comparisons (depth 3), then jumps based on result
    assert_eq!(
        code.max_stack_size, 3,
        "Complex condition with And needs stack depth 3"
    );
}

#[test]
fn test_chained_comparisons() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "1 < 2 and 2 < 3");

    // Verify it compiles successfully and produces logical And of two comparisons
    assert_eq!(code.instructions.len(), 8);
    assert_eq!(code.instructions[2], Instruction::IntCmpOp(b'<'));
    assert_eq!(code.instructions[5], Instruction::IntCmpOp(b'<'));
    assert_eq!(code.instructions[6], Instruction::And);
}

#[test]
fn test_not_equals() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "5 != 10");

    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(5));
    assert_eq!(code.instructions[1], Instruction::ConstInt(10));
    assert_eq!(code.instructions[2], Instruction::IntCmpOp(b'!'));
}

#[test]
fn test_empty_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "[]");

    // Should just be MakeArray(0)
    assert_eq!(code.instructions.len(), 2);
    assert_eq!(code.instructions[0], Instruction::MakeArray(0));
    assert_eq!(
        code.max_stack_size, 1,
        "Empty array still produces one value"
    );
}

#[test]
fn test_array_with_constants() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "[1, 2, 3]");

    // Should be: ConstInt(1), ConstInt(2), ConstInt(3), MakeArray(3)
    assert_eq!(code.instructions.len(), 5);
    assert_eq!(code.instructions[0], Instruction::ConstInt(1));
    assert_eq!(code.instructions[1], Instruction::ConstInt(2));
    assert_eq!(code.instructions[2], Instruction::ConstInt(3));
    assert_eq!(code.instructions[3], Instruction::MakeArray(3));
    assert_eq!(
        code.max_stack_size, 3,
        "Need to hold all elements before array creation"
    );
}

#[test]
fn test_array_with_expressions() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "[1 + 2, 3 * 4]");

    println!("\nArray with expressions:\n{:?}\n", code);

    // Should evaluate each expression and then make array
    // ConstInt(1), ConstInt(2), IntBinOp(+),
    // ConstInt(3), ConstInt(4), IntBinOp(*),
    // MakeArray(2)
    assert_eq!(code.instructions.len(), 8);
    assert_eq!(code.instructions[2], Instruction::IntBinOp(b'+'));
    assert_eq!(code.instructions[5], Instruction::IntBinOp(b'*'));
    assert_eq!(code.instructions[6], Instruction::MakeArray(2));
    // Max stack: 2 for first add, then result + 2 for second multiply = 3, then collapse to 1 array
    assert_eq!(code.max_stack_size, 3);
}

#[test]
fn test_nested_arrays() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "[[1, 2], [3, 4]]");

    println!("\nNested arrays:\n{:?}\n", code);

    // Should have two MakeArray(2) for inner arrays, then MakeArray(2) for outer
    let make_array_count = code
        .instructions
        .iter()
        .filter(|inst| matches!(inst, Instruction::MakeArray(_)))
        .count();
    assert_eq!(make_array_count, 3, "Should have 3 MakeArray instructions");

    // Second-to-last instruction should be MakeArray(2) for outer array, then Return
    assert_eq!(
        code.instructions[code.instructions.len() - 2],
        Instruction::MakeArray(2)
    );
    assert_eq!(
        code.instructions[code.instructions.len() - 1],
        Instruction::Return
    );
}

#[test]
fn test_array_of_booleans() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "[true, false, 5 < 10]");

    println!("\nArray of booleans:\n{:?}\n", code);

    // Should compile each element, create array, then return
    assert_eq!(code.instructions[0], Instruction::ConstBool(1));
    assert_eq!(code.instructions[1], Instruction::ConstBool(0));
    assert_eq!(code.instructions[4], Instruction::IntCmpOp(b'<'));
    assert_eq!(
        code.instructions[code.instructions.len() - 2],
        Instruction::MakeArray(3)
    );
    assert_eq!(
        code.instructions[code.instructions.len() - 1],
        Instruction::Return
    );
}

#[test]
fn test_single_element_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "[42]");

    assert_eq!(code.instructions.len(), 3);
    assert_eq!(code.instructions[0], Instruction::ConstInt(42));
    assert_eq!(code.instructions[1], Instruction::MakeArray(1));
    assert_eq!(code.max_stack_size, 1);
}

#[test]
fn test_float_addition() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "1.5 + 2.5");

    println!("\nFloat addition:\n{:?}\n", code);

    // Should use FloatBinOp instead of IntBinOp
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[2], Instruction::FloatBinOp(b'+'));
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_float_operations() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let tests = vec![
        ("3.14 + 2.71", Instruction::FloatBinOp(b'+')),
        ("5.0 - 2.0", Instruction::FloatBinOp(b'-')),
        ("2.5 * 4.0", Instruction::FloatBinOp(b'*')),
        ("10.0 / 2.5", Instruction::FloatBinOp(b'/')),
        ("2.0 ^ 3.0", Instruction::FloatBinOp(b'^')),
    ];

    for (expr, expected_instr) in tests {
        let (code, _result) = compile_and_run(&arena, &type_manager, expr);

        assert_eq!(
            code.instructions[2], expected_instr,
            "Failed for expression: {}",
            expr
        );
    }
}

#[test]
fn test_float_negation() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "-(3.14)");

    println!("\nFloat negation:\n{:?}\n", code);

    // Should use NegFloat instead of NegInt
    assert_eq!(code.instructions.len(), 3);
    assert_eq!(code.instructions[1], Instruction::NegFloat);
}

#[test]
fn test_float_comparisons() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let tests = vec![
        ("1.5 < 2.5", b'<'),
        ("2.5 > 1.5", b'>'),
        ("1.0 == 1.0", b'='),
        ("1.0 != 2.0", b'!'),
        ("1.5 <= 2.5", b'l'),
        ("2.5 >= 1.5", b'g'),
    ];

    for (expr, expected_op) in tests {
        let (code, _result) = compile_and_run(&arena, &type_manager, expr);

        assert_eq!(
            code.instructions[2],
            Instruction::FloatCmpOp(expected_op),
            "Failed for expression: {}",
            expr
        );
    }
}

#[test]
fn test_mixed_float_expressions() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "(1.5 + 2.5) * 3.0");

    println!("\nMixed float expression:\n{:?}\n", code);

    // Should have two FloatBinOp instructions
    assert_eq!(code.instructions.len(), 6);
    assert_eq!(code.instructions[2], Instruction::FloatBinOp(b'+'));
    assert_eq!(code.instructions[4], Instruction::FloatBinOp(b'*'));
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_float_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "[1.0, 2.0, 3.0]");

    println!("\nFloat array:\n{:?}\n", code);

    // Should load 3 float constants, make array, then return
    // ConstLoad(0), ConstLoad(1), ConstLoad(2), MakeArray(3), Return
    assert_eq!(code.instructions.len(), 5);
    assert_eq!(code.instructions[3], Instruction::MakeArray(3));
    assert_eq!(code.instructions[4], Instruction::Return);
    assert_eq!(code.constants.len(), 3, "Should have 3 float constants");
}

#[test]
fn test_simple_where_binding() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "x + 1 where { x = 5 }");

    println!("\nSimple where binding:\n{:?}\n", code);

    // Should: ConstInt(5), StoreLocal(0), LoadLocal(0), ConstInt(1), IntBinOp(+)
    assert!(
        code.instructions
            .iter()
            .any(|i| matches!(i, Instruction::StoreLocal(0)))
    );
    assert!(
        code.instructions
            .iter()
            .any(|i| matches!(i, Instruction::LoadLocal(0)))
    );
    assert_eq!(code.num_locals, 1, "Should have 1 local variable");
}

#[test]
fn test_multiple_where_bindings() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "x + y where { x = 10, y = 20 }");

    println!("\nMultiple where bindings:\n{:?}\n", code);

    // Should have 2 local variables
    assert_eq!(code.num_locals, 2, "Should have 2 local variables");

    // Should have StoreLocal for both variables
    let store_count = code
        .instructions
        .iter()
        .filter(|i| matches!(i, Instruction::StoreLocal(_)))
        .count();
    assert_eq!(store_count, 2, "Should have 2 StoreLocal instructions");

    // Should load both variables and add them
    assert!(
        code.instructions
            .iter()
            .any(|i| matches!(i, Instruction::LoadLocal(0)))
    );
    assert!(
        code.instructions
            .iter()
            .any(|i| matches!(i, Instruction::LoadLocal(1)))
    );
}

#[test]
fn test_nested_where_bindings() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(
        &arena,
        &type_manager,
        "y + 1 where { y = (x * 2 where { x = 5 }) }",
    );

    println!("\nNested where bindings:\n{:?}\n", code);

    // Should have 2 local variables (x and y)
    assert_eq!(
        code.num_locals, 2,
        "Should have 2 local variables (x and y)"
    );
}

#[test]
fn test_where_with_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(&arena, &type_manager, "result where { result = 2 + 3 }");

    println!("\nWhere with expression:\n{:?}\n", code);

    // The binding should evaluate the expression first
    // Expected: ConstInt(2), ConstInt(3), IntBinOp(+), StoreLocal(0), LoadLocal(0)
    let add_pos = code
        .instructions
        .iter()
        .position(|i| matches!(i, Instruction::IntBinOp(b'+')));
    let store_pos = code
        .instructions
        .iter()
        .position(|i| matches!(i, Instruction::StoreLocal(0)));

    assert!(add_pos.is_some() && store_pos.is_some());
    assert!(
        add_pos.unwrap() < store_pos.unwrap(),
        "Addition should happen before store"
    );
}

#[test]
fn test_where_in_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) =
        compile_and_run(&arena, &type_manager, "[x, x + 1, x + 2] where { x = 10 }");

    println!("\nWhere in array:\n{:?}\n", code);

    // Should load x multiple times
    let load_count = code
        .instructions
        .iter()
        .filter(|i| matches!(i, Instruction::LoadLocal(0)))
        .count();
    assert_eq!(
        load_count, 3,
        "Should load x three times for array elements"
    );

    // Should end with MakeArray(3), Return
    assert_eq!(
        code.instructions[code.instructions.len() - 2],
        Instruction::MakeArray(3)
    );
    assert_eq!(
        code.instructions[code.instructions.len() - 1],
        Instruction::Return
    );
}

#[test]
fn test_where_with_shadowing() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) =
        compile_and_run(&arena, &type_manager, "x where { x = (x where { x = 5 }) }");

    println!("\nWhere with shadowing:\n{:?}\n", code);

    // Both bindings use the same variable name 'x'
    // The compiler will allocate SEPARATE slots for proper shadowing
    // Inner x gets slot 0, outer x gets slot 1
    assert_eq!(
        code.num_locals, 2,
        "Should allocate separate slots for shadowed variables"
    );
}

#[test]
fn test_where_scope_unshadowing() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(
        &arena,
        &type_manager,
        "x + (x where { x = 10 }) where { x = 5 }",
    );

    println!("\nWhere scope unshadowing:\n{:?}\n", code);

    // Should allocate 2 slots: one for outer x, one for inner x
    assert_eq!(
        code.num_locals, 2,
        "Should allocate 2 slots for outer and inner x"
    );

    // Verify we're loading from both slots
    assert!(
        code.instructions
            .iter()
            .any(|i| matches!(i, Instruction::StoreLocal(0))),
        "Should store to slot 0 (outer x)"
    );
    assert!(
        code.instructions
            .iter()
            .any(|i| matches!(i, Instruction::StoreLocal(1))),
        "Should store to slot 1 (inner x)"
    );
    assert!(
        code.instructions
            .iter()
            .any(|i| matches!(i, Instruction::LoadLocal(0))),
        "Should load from slot 0 (outer x for addition)"
    );
    assert!(
        code.instructions
            .iter()
            .any(|i| matches!(i, Instruction::LoadLocal(1))),
        "Should load from slot 1 (inner x)"
    );

    // Should end with an addition
    assert!(
        code.instructions
            .iter()
            .any(|i| matches!(i, Instruction::IntBinOp(b'+'))),
        "Should add the two x values"
    );
}

#[test]
fn test_where_scope_restoration() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, _result) = compile_and_run(
        &arena,
        &type_manager,
        "[ x, x where { x = 10 }, x ] where { x = 1 }",
    );

    println!("\nWhere scope restoration:\n{:?}\n", code);

    // Should allocate 2 slots: one for outer x, one for inner x
    assert_eq!(
        code.num_locals, 2,
        "Should allocate 2 slots for outer and inner x"
    );

    // Should load from slot 0 twice (first and third array elements)
    let load_local_0_count = code
        .instructions
        .iter()
        .filter(|i| matches!(i, Instruction::LoadLocal(0)))
        .count();
    assert_eq!(
        load_local_0_count, 2,
        "Should load from slot 0 twice (first and third array elements)"
    );

    // Should load from slot 1 once (for the inner x reference)
    let load_local_1_count = code
        .instructions
        .iter()
        .filter(|i| matches!(i, Instruction::LoadLocal(1)))
        .count();
    assert_eq!(
        load_local_1_count, 1,
        "Should load from slot 1 once (inner x)"
    );

    // Should create an array with 3 elements
    assert!(
        code.instructions
            .iter()
            .any(|i| matches!(i, Instruction::MakeArray(3))),
        "Should create array with 3 elements"
    );
}

// ============================================================================
// VM Execution Tests
// ============================================================================

#[test]
fn test_vm_simple_integer() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "42");

    // Result should be 42
    assert_eq!(result.unwrap().as_int().unwrap(), 42);
}

#[test]
fn test_vm_arithmetic() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "10 + 5 * 2");

    // Result should be 20 (10 + (5 * 2))
    assert_eq!(result.unwrap().as_int().unwrap(), 20);
}

#[test]
fn test_vm_boolean_operations() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "(5 < 10) and (3 > 1)");

    // Result should be true
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_vm_if_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "if true then 42 else 99");

    // Result should be 42
    assert_eq!(result.unwrap().as_int().unwrap(), 42);
}

#[test]
fn test_vm_where_binding() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "x + 1 where { x = 5 }");

    // Result should be 6 (5 + 1)
    assert_eq!(result.unwrap().as_int().unwrap(), 6);
}

#[test]
fn test_vm_scope_restoration() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "[ x, x where { x = 10 }, x ] where { x = 1 }",
    );

    // Result should be an array [1, 10, 1]
    let array = result.unwrap().as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 10);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 1);
}

#[test]
fn test_vm_shadowing_unshadowing() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "x + (x where { x = 10 }) where { x = 5 }",
    );

    // Result should be 15
    assert_eq!(result.unwrap().as_int().unwrap(), 15);
}

// ============================================================================
// Index Expression Tests
// ============================================================================

#[test]
fn test_array_index_constant() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "[10, 20, 30][1]");

    // Expected bytecode:
    // ConstInt(10), ConstInt(20), ConstInt(30), MakeArray(3), ArrayGetConst(1), Return
    assert_eq!(code.instructions.len(), 6);
    assert_eq!(code.instructions[0], Instruction::ConstInt(10));
    assert_eq!(code.instructions[1], Instruction::ConstInt(20));
    assert_eq!(code.instructions[2], Instruction::ConstInt(30));
    assert_eq!(code.instructions[3], Instruction::MakeArray(3));
    assert_eq!(code.instructions[4], Instruction::ArrayGetConst(1));
    assert_eq!(code.instructions[5], Instruction::Return);

    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 20);
}

#[test]
fn test_array_index_dynamic() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "[10, 20, 30][x] where { x = 2 }");

    // Expected bytecode:
    // ConstInt(2), StoreLocal(0),  -- where binding
    // ConstInt(10), ConstInt(20), ConstInt(30), MakeArray(3),  -- array
    // LoadLocal(0),  -- load x
    // ArrayGet, Return
    assert_eq!(code.instructions.len(), 9);
    assert_eq!(code.instructions[0], Instruction::ConstInt(2));
    assert_eq!(code.instructions[1], Instruction::StoreLocal(0));
    assert_eq!(code.instructions[2], Instruction::ConstInt(10));
    assert_eq!(code.instructions[3], Instruction::ConstInt(20));
    assert_eq!(code.instructions[4], Instruction::ConstInt(30));
    assert_eq!(code.instructions[5], Instruction::MakeArray(3));
    assert_eq!(code.instructions[6], Instruction::LoadLocal(0));
    assert_eq!(code.instructions[7], Instruction::ArrayGet);
    assert_eq!(code.instructions[8], Instruction::Return);

    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 30);
}

#[test]
fn test_vm_array_index_constant() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "[100, 200, 300][0]");

    // Result should be 100
    assert_eq!(result.unwrap().as_int().unwrap(), 100);
}

#[test]
fn test_vm_array_index_constant_last() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "[100, 200, 300][2]");

    // Result should be 300
    assert_eq!(result.unwrap().as_int().unwrap(), 300);
}

#[test]
fn test_vm_array_index_dynamic() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) =
        compile_and_run(&arena, &type_manager, "[10, 20, 30, 40][i] where { i = 2 }");

    // Result should be 30
    assert_eq!(result.unwrap().as_int().unwrap(), 30);
}

#[test]
fn test_vm_array_index_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "[5, 10, 15, 20][1 + 1]");

    // Result should be 15 (index 2)
    assert_eq!(result.unwrap().as_int().unwrap(), 15);
}

// ============================================================================
// Record and Field Expression Tests
// ============================================================================

#[test]
fn test_record_construction() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "{ x = 10, y = 20 }");

    // Expected bytecode:
    // Fields are sorted by name, so 'x' comes before 'y'
    // ConstInt(10), ConstInt(20), MakeRecord(2), Return
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(10));
    assert_eq!(code.instructions[1], Instruction::ConstInt(20));
    assert_eq!(code.instructions[2], Instruction::MakeRecord(2));
    assert_eq!(code.instructions[3], Instruction::Return);

    // Verify result
    let record = result.unwrap().as_record().unwrap();
    assert_eq!(record.len(), 2);
    assert_eq!(record.get("x").unwrap().as_int().unwrap(), 10);
    assert_eq!(record.get("y").unwrap().as_int().unwrap(), 20);
}

#[test]
fn test_field_access() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "{ x = 10, y = 20 }.x");

    // Expected bytecode:
    // ConstInt(10), ConstInt(20), MakeRecord(2), RecordGet(0), Return
    // Field 'x' is at index 0 (sorted order)
    assert_eq!(code.instructions.len(), 5);
    assert_eq!(code.instructions[0], Instruction::ConstInt(10));
    assert_eq!(code.instructions[1], Instruction::ConstInt(20));
    assert_eq!(code.instructions[2], Instruction::MakeRecord(2));
    assert_eq!(code.instructions[3], Instruction::RecordGet(0)); // 'x' is first
    assert_eq!(code.instructions[4], Instruction::Return);

    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 10);
}

#[test]
fn test_field_access_second_field() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "{ x = 10, y = 20 }.y");

    // Expected bytecode:
    // ConstInt(10), ConstInt(20), MakeRecord(2), RecordGet(1), Return
    // Field 'y' is at index 1 (sorted order)
    assert_eq!(code.instructions.len(), 5);
    assert_eq!(code.instructions[0], Instruction::ConstInt(10));
    assert_eq!(code.instructions[1], Instruction::ConstInt(20));
    assert_eq!(code.instructions[2], Instruction::MakeRecord(2));
    assert_eq!(code.instructions[3], Instruction::RecordGet(1)); // 'y' is second
    assert_eq!(code.instructions[4], Instruction::Return);

    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 20);
}

#[test]
fn test_vm_record_field_access() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "{ x = 100, y = 200 }.x");

    // Result should be 100
    assert_eq!(result.unwrap().as_int().unwrap(), 100);
}

#[test]
fn test_vm_record_field_access_second() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "{ a = 5, b = 10, c = 15 }.b");

    // Result should be 10
    assert_eq!(result.unwrap().as_int().unwrap(), 10);
}

#[test]
fn test_vm_nested_record_field_access() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "{ x = { y = 42 } }.x.y");

    // Result should be 42
    assert_eq!(result.unwrap().as_int().unwrap(), 42);
}

#[test]
fn test_vm_record_in_where() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "rec.x + rec.y where { rec = { x = 3, y = 4 } }",
    );

    // Result should be 3 + 4 = 7
    assert_eq!(result.unwrap().as_int().unwrap(), 7);
}

// ============================================================================
// Map Expression Tests
// ============================================================================

#[test]
fn test_map_construction() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "{ 1: 10, 2: 20 }");

    // Expected bytecode:
    // ConstInt(1), ConstInt(10), ConstInt(2), ConstInt(20), MakeMap(2), Return
    assert_eq!(code.instructions.len(), 6);
    assert_eq!(code.instructions[0], Instruction::ConstInt(1));
    assert_eq!(code.instructions[1], Instruction::ConstInt(10));
    assert_eq!(code.instructions[2], Instruction::ConstInt(2));
    assert_eq!(code.instructions[3], Instruction::ConstInt(20));
    assert_eq!(code.instructions[4], Instruction::MakeMap(2));
    assert_eq!(code.instructions[5], Instruction::Return);

    // Verify result
    let map = result.unwrap().as_map().unwrap();
    assert_eq!(map.len(), 2);
}

#[test]
fn test_map_indexing() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (code, result) = compile_and_run(&arena, &type_manager, "{ 1: 100, 2: 200 }[1]");

    // Expected bytecode:
    // ConstInt(1), ConstInt(100), ConstInt(2), ConstUInt(200), MakeMap(2),
    // ConstInt(1), MapGet, Return
    assert_eq!(code.instructions.len(), 8);
    assert_eq!(code.instructions[0], Instruction::ConstInt(1));
    assert_eq!(code.instructions[1], Instruction::ConstInt(100));
    assert_eq!(code.instructions[2], Instruction::ConstInt(2));
    assert_eq!(code.instructions[3], Instruction::ConstUInt(200));
    assert_eq!(code.instructions[4], Instruction::MakeMap(2));
    assert_eq!(code.instructions[5], Instruction::ConstInt(1));
    assert_eq!(code.instructions[6], Instruction::MapGet);
    assert_eq!(code.instructions[7], Instruction::Return);

    // Verify result
    assert_eq!(result.unwrap().as_int().unwrap(), 100);
}

#[test]
fn test_vm_map_indexing() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(&arena, &type_manager, "{ 1: 100, 2: 200, 3: 300 }[2]");

    // Result should be 200
    assert_eq!(result.unwrap().as_int().unwrap(), 200);
}

#[test]
fn test_vm_map_with_variable_key() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "m[k] where { m = { 10: 100, 20: 200 }, k = 20 }",
    );

    // Result should be 200
    assert_eq!(result.unwrap().as_int().unwrap(), 200);
}

// ============================================================================
// Ignored Tests for Unimplemented Features
// ============================================================================

#[test]
fn test_vm_array_negative_index_last() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [10, 20, 30][-1] should return 30 (last element)
    let (_code, result) = compile_and_run(&arena, &type_manager, "[10, 20, 30][-1]");
    assert_eq!(result.unwrap().as_int().unwrap(), 30);
}

#[test]
fn test_vm_array_negative_index_first() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [10, 20, 30][-3] should return 10 (first element)
    let (_code, result) = compile_and_run(&arena, &type_manager, "[10, 20, 30][-3]");
    assert_eq!(result.unwrap().as_int().unwrap(), 10);
}

#[test]
#[ignore = "Map string keys not implemented in VM (only integer keys work)"]
fn test_vm_map_string_keys() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: { "a": 100, "b": 200 }["a"] should return 100
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"{ "a": 100, "b": 200 }["a"]"#);
    assert_eq!(result.unwrap().as_int().unwrap(), 100);
}

#[test]
#[ignore = "Map[Str, Str] not implemented in VM"]
fn test_vm_map_string_to_string() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: { "greeting": "hello", "farewell": "goodbye" }["greeting"]
    let (_code, _result) = compile_and_run(
        &arena,
        &type_manager,
        r#"{ "greeting": "hello", "farewell": "goodbye" }["greeting"]"#,
    );
    // Result should be the string "hello" but string extraction not implemented yet
}

#[test]
#[ignore = "Array[Float] indexing not tested"]
fn test_vm_float_array_index() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [1.5, 2.5, 3.5][1] should return 2.5
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1.5, 2.5, 3.5][1]");
    assert_eq!(result.unwrap().as_float().unwrap(), 2.5);
}

#[test]
#[ignore = "Array[Str] indexing not tested"]
fn test_vm_string_array_index() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: ["a", "b", "c"][0] should return "a"
    let (_code, _result) = compile_and_run(&arena, &type_manager, r#"["a", "b", "c"][0]"#);
    // Result should be the string "a" but string extraction not implemented yet
}

#[test]
#[ignore = "Array[Bool] indexing not tested"]
fn test_vm_bool_array_index() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [true, false, true][2] should return true
    let (_code, result) = compile_and_run(&arena, &type_manager, "[true, false, true][2]");
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
#[ignore = "Empty map construction not tested"]
fn test_vm_empty_map() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: {} (in map context) should create empty map
    // Note: This may require type annotation to distinguish from empty record
    let (_code, result) = compile_and_run(&arena, &type_manager, "{:}");
    let map = result.unwrap().as_map().unwrap();
    assert_eq!(map.len(), 0);
}

#[test]
#[ignore = "Nested map access not tested"]
fn test_vm_nested_map_access() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: { 1: { 2: 42 } }[1][2] should return 42
    let (_code, result) = compile_and_run(&arena, &type_manager, "{ 1: { 2: 42 } }[1][2]");
    assert_eq!(result.unwrap().as_int().unwrap(), 42);
}

#[test]
#[ignore = "Map with large number of entries not tested"]
fn test_vm_large_map() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test map with 10 entries
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "{ 1: 10, 2: 20, 3: 30, 4: 40, 5: 50, 6: 60, 7: 70, 8: 80, 9: 90, 10: 100 }[7]",
    );
    assert_eq!(result.unwrap().as_int().unwrap(), 70);
}

#[test]
#[ignore = "Array of records not tested"]
fn test_vm_array_of_records() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [{ x = 1 }, { x = 2 }, { x = 3 }][1].x should return 2
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "[{ x = 1 }, { x = 2 }, { x = 3 }][1].x",
    );
    assert_eq!(result.unwrap().as_int().unwrap(), 2);
}

#[test]
#[ignore = "Record with many fields not tested"]
fn test_vm_large_record() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test record with 10 fields
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "{ a = 1, b = 2, c = 3, d = 4, e = 5, f = 6, g = 7, h = 8, i = 9, j = 10 }.g",
    );
    assert_eq!(result.unwrap().as_int().unwrap(), 7);
}

#[test]
#[ignore = "Deeply nested records not tested"]
fn test_vm_deeply_nested_records() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: { a = { b = { c = 42 } } }.a.b.c should return 42
    let (_code, result) =
        compile_and_run(&arena, &type_manager, "{ a = { b = { c = 42 } } }.a.b.c");
    assert_eq!(result.unwrap().as_int().unwrap(), 42);
}

// ============================================================================
// Otherwise Operator Tests
// ============================================================================

#[test]
fn test_vm_otherwise_array_out_of_bounds() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Array index out of bounds should use fallback
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1, 2, 3][10] otherwise 99");
    assert_eq!(result.unwrap().as_int().unwrap(), 99);
}

#[test]
fn test_vm_otherwise_array_negative_index() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Negative array index should use fallback
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1, 2, 3][-5] otherwise 42");
    assert_eq!(result.unwrap().as_int().unwrap(), 42);
}

#[test]
fn test_vm_otherwise_array_success() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Valid array index should NOT use fallback
    let (_code, result) = compile_and_run(&arena, &type_manager, "[10, 20, 30][1] otherwise 99");
    assert_eq!(result.unwrap().as_int().unwrap(), 20);
}

#[test]
fn test_vm_otherwise_map_key_not_found() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Map key not found should use fallback
    let (_code, result) = compile_and_run(&arena, &type_manager, "{1: 10, 2: 20}[5] otherwise 99");
    assert_eq!(result.unwrap().as_int().unwrap(), 99);
}

#[test]
fn test_vm_otherwise_map_success() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Valid map key should NOT use fallback
    let (_code, result) = compile_and_run(&arena, &type_manager, "{1: 10, 2: 20}[2] otherwise 99");
    assert_eq!(result.unwrap().as_int().unwrap(), 20);
}

#[test]
fn test_vm_otherwise_complex_primary_expr() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Complex expression as primary (with error)
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "([1, 2][0] + [3, 4][10]) otherwise 100",
    );
    assert_eq!(result.unwrap().as_int().unwrap(), 100);
}

#[test]
fn test_vm_otherwise_complex_fallback_expr() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Complex expression as fallback
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "[1, 2][10] otherwise ([5, 6][0] + [7, 8][1])",
    );
    assert_eq!(result.unwrap().as_int().unwrap(), 13); // 5 + 8
}

#[test]
fn test_vm_otherwise_nested() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Nested otherwise: inner error should be caught by inner otherwise
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "([1, 2][10] otherwise 50) otherwise 99",
    );
    assert_eq!(result.unwrap().as_int().unwrap(), 50);
}

#[test]
fn test_vm_otherwise_nested_fallback_error() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Nested otherwise: inner succeeds, outer catches error in fallback
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "[1, 2][5] otherwise ([3, 4][10] otherwise 77)",
    );
    assert_eq!(result.unwrap().as_int().unwrap(), 77);
}

#[test]
fn test_vm_otherwise_in_arithmetic() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Otherwise in arithmetic expression
    let (_code, result) = compile_and_run(&arena, &type_manager, "10 + ([1, 2][5] otherwise 5)");
    assert_eq!(result.unwrap().as_int().unwrap(), 15);
}

#[test]
fn test_vm_otherwise_bool_result() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Otherwise with boolean result
    let (_code, result) =
        compile_and_run(&arena, &type_manager, "[true, false][10] otherwise true");
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_vm_otherwise_chained() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Multiple otherwise operators chained
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "[1][5] otherwise [2][5] otherwise [3][5] otherwise 42",
    );
    assert_eq!(result.unwrap().as_int().unwrap(), 42);
}

// ============================================================================
// Error Tests (Without Otherwise Handlers)
// ============================================================================

#[test]
fn test_vm_array_index_error_no_otherwise() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Array index out of bounds without otherwise should return error
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1, 2, 3][10]");

    assert!(result.is_err(), "Expected error for out of bounds access");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            crate::evaluator::ExecutionErrorKind::Runtime(
                crate::evaluator::RuntimeError::IndexOutOfBounds { index: 10, len: 3 }
            )
        ),
        "Expected IndexOutOfBounds error, got: {:?}",
        err.kind
    );
}

#[test]
fn test_vm_map_key_error_no_otherwise() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Map key not found without otherwise should return error
    let (_code, result) = compile_and_run(&arena, &type_manager, "{1: 10, 2: 20}[99]");

    assert!(result.is_err(), "Expected error for key not found");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            crate::evaluator::ExecutionErrorKind::Runtime(
                crate::evaluator::RuntimeError::KeyNotFound { .. }
            )
        ),
        "Expected KeyNotFound error, got: {:?}",
        err.kind
    );
}

#[test]
fn test_vm_integer_division_by_zero_no_otherwise() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Integer division by zero without otherwise should return error
    let (_code, result) = compile_and_run(&arena, &type_manager, "10 / 0");

    assert!(result.is_err(), "Expected error for division by zero");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            crate::evaluator::ExecutionErrorKind::Runtime(
                crate::evaluator::RuntimeError::DivisionByZero {}
            )
        ),
        "Expected DivisionByZero error, got: {:?}",
        err.kind
    );
}

#[test]
fn test_vm_float_division_by_zero_returns_inf() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Float division by zero should return infinity (IEEE 754), not error
    let (_code, result) = compile_and_run(&arena, &type_manager, "10.0 / 0.0");

    assert!(result.is_ok(), "Float division by zero should not error");
    let value = result.unwrap().as_float().unwrap();
    assert!(
        value.is_infinite() && value.is_sign_positive(),
        "Expected positive infinity"
    );
}

// ============================================================================
// Negative Array Indexing Tests
// ============================================================================

#[test]
fn test_vm_negative_index_last_element() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [1, 2, 3][-1] should return 3 (last element)
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1, 2, 3][-1]");
    assert_eq!(result.unwrap().as_int().unwrap(), 3);
}

#[test]
fn test_vm_negative_index_second_to_last() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [1, 2, 3][-2] should return 2 (second to last)
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1, 2, 3][-2]");
    assert_eq!(result.unwrap().as_int().unwrap(), 2);
}

#[test]
fn test_vm_negative_index_first_element() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [10, 20, 30][-3] should return 10 (first element, counting from end)
    let (_code, result) = compile_and_run(&arena, &type_manager, "[10, 20, 30][-3]");
    assert_eq!(result.unwrap().as_int().unwrap(), 10);
}

#[test]
fn test_vm_negative_index_out_of_bounds() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [1, 2][-3] should error (too negative)
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1, 2][-3]");

    assert!(
        result.is_err(),
        "Expected error for out of bounds negative index"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            crate::evaluator::ExecutionErrorKind::Runtime(
                crate::evaluator::RuntimeError::IndexOutOfBounds { index: -3, len: 2 }
            )
        ),
        "Expected IndexOutOfBounds error, got: {:?}",
        err.kind
    );
}

#[test]
fn test_vm_negative_index_way_out_of_bounds() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [1, 2][-100] should error (way too negative)
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1, 2][-100]");

    assert!(
        result.is_err(),
        "Expected error for way out of bounds negative index"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            crate::evaluator::ExecutionErrorKind::Runtime(
                crate::evaluator::RuntimeError::IndexOutOfBounds {
                    index: -100,
                    len: 2
                }
            )
        ),
        "Expected IndexOutOfBounds error, got: {:?}",
        err.kind
    );
}

#[test]
fn test_vm_negative_index_dynamic() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Dynamic negative index via variable
    let (_code, result) = compile_and_run(&arena, &type_manager, "[10, 20][i] where { i = -1 }");
    assert_eq!(result.unwrap().as_int().unwrap(), 20);
}

#[test]
fn test_vm_negative_index_with_otherwise_success() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Valid negative index should NOT use fallback
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1, 2][-1] otherwise 99");
    assert_eq!(result.unwrap().as_int().unwrap(), 2);
}

#[test]
fn test_vm_negative_index_with_otherwise_error() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Invalid negative index should use fallback
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1, 2][-5] otherwise 99");
    assert_eq!(result.unwrap().as_int().unwrap(), 99);
}

#[test]
fn test_vm_negative_index_single_element_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Single element array with -1 should return that element
    let (_code, result) = compile_and_run(&arena, &type_manager, "[42][-1]");
    assert_eq!(result.unwrap().as_int().unwrap(), 42);
}

#[test]
fn test_vm_negative_index_float_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Negative indexing works with Array[Float]
    let (_code, result) = compile_and_run(&arena, &type_manager, "[1.5, 2.5, 3.5][-1]");
    assert_eq!(result.unwrap().as_float().unwrap(), 3.5);
}

#[test]
fn test_vm_negative_index_string_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Negative indexing works with Array[Str]
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"["a", "b", "c"][-2]"#);
    assert_eq!(result.unwrap().as_str().unwrap(), "b");
}

#[test]
fn test_vm_negative_index_bool_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Negative indexing works with Array[Bool]
    let (_code, result) = compile_and_run(&arena, &type_manager, "[true, false, true][-1]");
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_vm_negative_index_nested_arrays() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Negative indexing works with nested arrays
    let (_code, result) = compile_and_run(&arena, &type_manager, "[[1, 2], [3, 4]][-1][-1]");
    assert_eq!(result.unwrap().as_int().unwrap(), 4);
}

#[test]
fn test_vm_negative_index_boundary_last() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Boundary case - exactly at the first element via negative index
    let (_code, result) = compile_and_run(&arena, &type_manager, "[100, 200, 300, 400][-4]");
    assert_eq!(result.unwrap().as_int().unwrap(), 100);
}

// ============================================================================
// Empty Array Edge Cases
// ============================================================================

#[test]
fn test_vm_empty_array_positive_index_error() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Indexing empty array with positive index should error
    let (_code, result) = compile_and_run(&arena, &type_manager, "[][0]");

    assert!(result.is_err(), "Expected error for indexing empty array");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            crate::evaluator::ExecutionErrorKind::Runtime(
                crate::evaluator::RuntimeError::IndexOutOfBounds { index: 0, len: 0 }
            )
        ),
        "Expected IndexOutOfBounds error, got: {:?}",
        err.kind
    );
}

#[test]
fn test_vm_empty_array_negative_index_error() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Indexing empty array with negative index should error
    let (_code, result) = compile_and_run(&arena, &type_manager, "[][-1]");

    assert!(
        result.is_err(),
        "Expected error for negative indexing empty array"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            crate::evaluator::ExecutionErrorKind::Runtime(
                crate::evaluator::RuntimeError::IndexOutOfBounds { index: -1, len: 0 }
            )
        ),
        "Expected IndexOutOfBounds error, got: {:?}",
        err.kind
    );
}

#[test]
fn test_vm_empty_array_with_otherwise() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Empty array indexing should use otherwise fallback
    let (_code, result) = compile_and_run(&arena, &type_manager, "[][0] otherwise 42");
    assert_eq!(result.unwrap().as_int().unwrap(), 42);

    let (_code, result) = compile_and_run(&arena, &type_manager, "[][-1] otherwise 99");
    assert_eq!(result.unwrap().as_int().unwrap(), 99);
}

// ============================================================================
// Option Constructor Tests
// ============================================================================

#[test]
fn test_vm_none_literal() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: none should compile to MakeOption(0)
    let (code, result) = compile_and_run(&arena, &type_manager, "none");

    // Bytecode: MakeOption(0), Return
    assert_eq!(code.instructions.len(), 2);
    assert_eq!(code.instructions[0], Instruction::MakeOption(0));

    // VM execution: should produce None value
    let value = result.unwrap();
    assert_eq!(value.as_option().unwrap(), None, "Expected None value");
}

#[test]
fn test_vm_some_integer() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: some 42 should compile to ConstInt(42), MakeOption(1)
    let (code, result) = compile_and_run(&arena, &type_manager, "some 42");

    // Bytecode: ConstInt(42), MakeOption(1), Return
    assert_eq!(code.instructions.len(), 3);
    assert_eq!(code.instructions[0], Instruction::ConstInt(42));
    assert_eq!(code.instructions[1], Instruction::MakeOption(1));

    // VM execution: should produce Some(42)
    let value = result.unwrap();
    let option_value = value.as_option().unwrap();
    assert!(option_value.is_some(), "Expected Some value");
    assert_eq!(option_value.unwrap().as_int().unwrap(), 42);
}

#[test]
fn test_vm_some_float() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: some 3.14
    let (_code, result) = compile_and_run(&arena, &type_manager, "some 3.14");

    let value = result.unwrap();
    let option_value = value.as_option().unwrap();
    assert!(option_value.is_some(), "Expected Some value");
    assert_eq!(option_value.unwrap().as_float().unwrap(), 3.14);
}

#[test]
fn test_vm_some_bool() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: some true
    let (_code, result) = compile_and_run(&arena, &type_manager, "some true");

    let value = result.unwrap();
    let option_value = value.as_option().unwrap();
    assert!(option_value.is_some(), "Expected Some value");
    assert_eq!(option_value.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_vm_some_string() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: some "hello"
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"some "hello""#);

    let value = result.unwrap();
    let option_value = value.as_option().unwrap();
    assert!(option_value.is_some(), "Expected Some value");
    assert_eq!(option_value.unwrap().as_str().unwrap(), "hello");
}

#[test]
fn test_vm_nested_some() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: some (some 10) - nested options
    let (_code, result) = compile_and_run(&arena, &type_manager, "some (some 10)");

    let value = result.unwrap();
    let outer_option = value.as_option().unwrap();
    assert!(outer_option.is_some(), "Expected outer Some value");

    let inner = outer_option.unwrap();
    let inner_option = inner.as_option().unwrap();
    assert!(inner_option.is_some(), "Expected inner Some value");
    assert_eq!(inner_option.unwrap().as_int().unwrap(), 10);
}

#[test]
fn test_vm_some_with_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: some (1 + 2) - some with complex expression
    let (_code, result) = compile_and_run(&arena, &type_manager, "some (1 + 2)");

    let value = result.unwrap();
    let option_value = value.as_option().unwrap();
    assert!(option_value.is_some(), "Expected Some value");
    assert_eq!(option_value.unwrap().as_int().unwrap(), 3);
}

#[test]
fn test_vm_some_with_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: some [1, 2, 3]
    let (_code, result) = compile_and_run(&arena, &type_manager, "some [1, 2, 3]");

    let value = result.unwrap();
    let option_value = value.as_option().unwrap();
    assert!(option_value.is_some(), "Expected Some value");

    let array = option_value.unwrap();
    let array_data = array.as_array().unwrap();
    assert_eq!(array_data.len(), 3);
}

#[test]
fn test_vm_some_with_record() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: some { x = 1, y = 2 }
    let (_code, result) = compile_and_run(&arena, &type_manager, "some { x = 1, y = 2 }");

    let value = result.unwrap();
    let option_value = value.as_option().unwrap();
    assert!(option_value.is_some(), "Expected Some value");

    let record = option_value.unwrap();
    assert!(record.as_record().is_ok(), "Expected record inside Some");
}

// ============================================================================
// FFI Function Call Tests
// ============================================================================

#[test]
fn test_ffi_math_sin() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Sin(0.0) should return 0.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Sin(0.0)");
    let value = result.unwrap().as_float().unwrap();
    assert!(value.abs() < 1e-10, "Expected ~0.0, got {}", value);
}

#[test]
fn test_ffi_math_sin_pi() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Sin(Math.PI) should return ~0.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Sin(Math.PI)");
    let value = result.unwrap().as_float().unwrap();
    assert!(value.abs() < 1e-10, "Expected ~0.0, got {}", value);
}

#[test]
fn test_ffi_math_sqrt() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Sqrt(4.0) should return 2.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Sqrt(4.0)");
    let value = result.unwrap().as_float().unwrap();
    assert!((value - 2.0).abs() < 1e-10, "Expected 2.0, got {}", value);
}

#[test]
fn test_ffi_math_sqrt_with_expression_arg() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Sqrt(2.0 + 2.0) should return 2.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Sqrt(2.0 + 2.0)");
    let value = result.unwrap().as_float().unwrap();
    assert!((value - 2.0).abs() < 1e-10, "Expected 2.0, got {}", value);
}

#[test]
fn test_ffi_in_where_binding() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Sin(x) where { x = 0.0 }
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Sin(x) where { x = 0.0 }");
    let value = result.unwrap().as_float().unwrap();
    assert!(value.abs() < 1e-10, "Expected ~0.0, got {}", value);
}

#[test]
fn test_ffi_nested_calls() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Sqrt(Math.Abs(-4.0)) should return 2.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Sqrt(Math.Abs(-4.0))");
    let value = result.unwrap().as_float().unwrap();
    assert!((value - 2.0).abs() < 1e-10, "Expected 2.0, got {}", value);
}

#[test]
fn test_ffi_math_abs_positive() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Abs(5.0) should return 5.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Abs(5.0)");
    let value = result.unwrap().as_float().unwrap();
    assert!((value - 5.0).abs() < 1e-10, "Expected 5.0, got {}", value);
}

#[test]
fn test_ffi_math_abs_negative() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Abs(-5.0) should return 5.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Abs(-5.0)");
    let value = result.unwrap().as_float().unwrap();
    assert!((value - 5.0).abs() < 1e-10, "Expected 5.0, got {}", value);
}

#[test]
fn test_ffi_math_min() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Min(3.0, 5.0) should return 3.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Min(3.0, 5.0)");
    let value = result.unwrap().as_float().unwrap();
    assert!((value - 3.0).abs() < 1e-10, "Expected 3.0, got {}", value);
}

#[test]
fn test_ffi_math_max() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Max(3.0, 5.0) should return 5.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Max(3.0, 5.0)");
    let value = result.unwrap().as_float().unwrap();
    assert!((value - 5.0).abs() < 1e-10, "Expected 5.0, got {}", value);
}

#[test]
fn test_ffi_result_in_arithmetic() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Sqrt(4.0) + 1.0 should return 3.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Sqrt(4.0) + 1.0");
    let value = result.unwrap().as_float().unwrap();
    assert!((value - 3.0).abs() < 1e-10, "Expected 3.0, got {}", value);
}

#[test]
fn test_ffi_in_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: [Math.Sqrt(1.0), Math.Sqrt(4.0), Math.Sqrt(9.0)]
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "[Math.Sqrt(1.0), Math.Sqrt(4.0), Math.Sqrt(9.0)]",
    );
    let array = result.unwrap().as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert!((array.get(0).unwrap().as_float().unwrap() - 1.0).abs() < 1e-10);
    assert!((array.get(1).unwrap().as_float().unwrap() - 2.0).abs() < 1e-10);
    assert!((array.get(2).unwrap().as_float().unwrap() - 3.0).abs() < 1e-10);
}

#[test]
fn test_ffi_in_if_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: if Math.Sqrt(4.0) > 1.5 then 10.0 else 20.0
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "if Math.Sqrt(4.0) > 1.5 then 10.0 else 20.0",
    );
    let value = result.unwrap().as_float().unwrap();
    assert!((value - 10.0).abs() < 1e-10, "Expected 10.0, got {}", value);
}

#[test]
fn test_ffi_chained_method_calls() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Abs(Math.Sin(Math.PI)) should return ~0.0
    let (_code, result) = compile_and_run(&arena, &type_manager, "Math.Abs(Math.Sin(Math.PI))");
    let value = result.unwrap().as_float().unwrap();
    assert!(value.abs() < 1e-10, "Expected ~0.0, got {}", value);
}

#[test]
fn test_ffi_complex_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: Math.Sqrt(a * a + b * b) where { a = 3.0, b = 4.0 } should return 5.0 (Pythagorean triple)
    let (_code, result) = compile_and_run(
        &arena,
        &type_manager,
        "Math.Sqrt(a * a + b * b) where { a = 3.0, b = 4.0 }",
    );
    let value = result.unwrap().as_float().unwrap();
    assert!((value - 5.0).abs() < 1e-10, "Expected 5.0, got {}", value);
}

// === WideArg Tests ===
// These tests verify that the compiler correctly emits WideArg prefixes for large arguments
// and that the VM correctly decodes them.

#[test]
fn test_wide_arg_many_constants() {
    // Test with >255 constants to verify WideArg encoding for ConstLoad
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Build an expression that creates many unique float constants (strings would need >255 unique values)
    // We use a where-clause with many unique float values
    // Formula: sum of first 260 unique floats (0.0 to 259.0), each accessed twice to force constant pool usage
    // Then access constants[256], constants[257], constants[258], constants[259]
    // These will require WideArg since their indices are > 255

    // Generate: c256 + c257 + c258 + c259 where { c0 = 0.0, c1 = 1.0, ..., c259 = 259.0 }
    let mut source = String::new();
    source.push_str("c256 + c257 + c258 + c259 where {\n");
    for i in 0..260 {
        if i > 0 {
            source.push_str(",\n");
        }
        source.push_str(&alloc::format!("    c{} = {}.0", i, i));
    }
    source.push_str("\n}");

    let (code, result) = compile_and_run(&arena, &type_manager, &source);

    // Verify that WideArg instructions are present for the large constant indices
    let has_wide_arg = code
        .instructions
        .iter()
        .any(|i| matches!(i, Instruction::WideArg(_)));
    assert!(
        has_wide_arg,
        "Expected WideArg instructions for constant indices > 255"
    );

    // Verify result: 256.0 + 257.0 + 258.0 + 259.0 = 1030.0
    let value = result.unwrap().as_float().unwrap();
    assert!(
        (value - 1030.0).abs() < 1e-10,
        "Expected 1030.0, got {}",
        value
    );
}

#[test]
fn test_wide_arg_many_locals() {
    // Test with >255 local variables to verify WideArg encoding for LoadLocal/StoreLocal
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Generate: x256 + x257 + x258 + x259 where { x0 = 0, x1 = 1, ..., x259 = 259 }
    let mut source = String::new();
    source.push_str("x256 + x257 + x258 + x259 where {\n");
    for i in 0..260 {
        if i > 0 {
            source.push_str(",\n");
        }
        source.push_str(&alloc::format!("    x{} = {}", i, i));
    }
    source.push_str("\n}");

    let (code, result) = compile_and_run(&arena, &type_manager, &source);

    // Verify we have 260 locals
    assert_eq!(
        code.num_locals, 260,
        "Expected 260 local variables, got {}",
        code.num_locals
    );

    // Verify that WideArg instructions are present for the large local indices
    let has_wide_arg = code
        .instructions
        .iter()
        .any(|i| matches!(i, Instruction::WideArg(_)));
    assert!(
        has_wide_arg,
        "Expected WideArg instructions for local indices > 255"
    );

    // Verify result: 256 + 257 + 258 + 259 = 1030
    assert_eq!(result.unwrap().as_int().unwrap(), 1030);
}

#[test]
fn test_wide_arg_large_array() {
    // Test with a large array (>255 elements) to verify WideArg encoding for MakeArray
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Generate: [0, 1, 2, ..., 299][256]
    // This creates an array with 300 elements (requires WideArg for MakeArray)
    // and accesses element 256 (requires WideArg for ArrayGetConst)
    let mut source = String::new();
    source.push('[');
    for i in 0..300 {
        if i > 0 {
            source.push_str(", ");
        }
        source.push_str(&alloc::format!("{}", i));
    }
    source.push_str("][256]");

    let (code, result) = compile_and_run(&arena, &type_manager, &source);

    // Verify that WideArg instructions are present
    let has_wide_arg = code
        .instructions
        .iter()
        .any(|i| matches!(i, Instruction::WideArg(_)));
    assert!(
        has_wide_arg,
        "Expected WideArg instructions for array with 300 elements"
    );

    // Verify result: element at index 256 is 256
    assert_eq!(result.unwrap().as_int().unwrap(), 256);
}

#[test]
fn test_wide_arg_encoding_bytes() {
    // Test that WideArg encoding works correctly for multi-byte values
    // by verifying the bytecode structure directly
    use crate::vm::VM;

    let arena = Bump::new();

    // Create a code object manually with a large constant index (0x0102 = 258)
    // This should be encoded as: WideArg(0x01), ConstLoad(0x02)
    let mut constants = alloc::vec::Vec::new();
    for i in 0..300i64 {
        constants.push(crate::values::raw::RawValue { int_value: i });
    }

    let code = Code {
        constants,
        adapters: alloc::vec::Vec::new(),
        instructions: alloc::vec![
            Instruction::WideArg(0x01),   // High byte of 258
            Instruction::ConstLoad(0x02), // Low byte of 258
            Instruction::Return,
        ],
        num_locals: 0,
        max_stack_size: 1,
    };

    let result = VM::execute(&arena, &code);
    // The constant at index 258 (0x0102) should be 258
    assert_eq!(unsafe { result.unwrap().int_value }, 258);
}

#[test]
fn test_wide_arg_three_byte_encoding() {
    // Test that three-byte WideArg encoding works (for values >= 65536)
    use crate::vm::VM;

    let arena = Bump::new();

    // Create a large constant pool
    let mut constants = alloc::vec::Vec::new();
    for i in 0..70000i64 {
        constants.push(crate::values::raw::RawValue { int_value: i });
    }

    // Access constant at index 65537 (0x010001)
    // This should be encoded as: WideArg(0x01), WideArg(0x00), ConstLoad(0x01)
    let code = Code {
        constants,
        adapters: alloc::vec::Vec::new(),
        instructions: alloc::vec![
            Instruction::WideArg(0x01),   // High byte
            Instruction::WideArg(0x00),   // Middle byte
            Instruction::ConstLoad(0x01), // Low byte
            Instruction::Return,
        ],
        num_locals: 0,
        max_stack_size: 1,
    };

    let result = VM::execute(&arena, &code);
    // The constant at index 65537 should be 65537
    assert_eq!(unsafe { result.unwrap().int_value }, 65537);
}

// === Wide Jump Tests ===
// These tests verify that jump instructions correctly use WideArg for large offsets

// NOTE: Source-based tests for wide jumps are disabled because deeply nested
// expressions (300+ additions) cause stack overflow in the recursive parser.
// The VM-direct tests below verify the WideArg + Jump functionality works correctly.

#[test]
fn test_wide_jump_if_large_then_branch() {
    // Test if expression with large then and else branches
    // Both branches generate >255 instructions, forcing WideArg for jumps
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Generate: if true then [1, 2, ..., 300][0] else [1, 2, ..., 300][1]
    // Each array element generates ~2 instructions, so 300 elements = ~600 instructions
    let mut source = String::new();
    source.push_str("(if true then [");
    for i in 1..=300 {
        if i > 1 {
            source.push_str(", ");
        }
        source.push_str(&alloc::format!("{}", i));
    }
    source.push_str("][-1] else [");
    for i in 1..=300 {
        if i > 1 {
            source.push_str(", ");
        }
        source.push_str(&alloc::format!("{}", i));
    }
    source.push_str("][-1]) + 10");

    let (code, result) = compile_and_run(&arena, &type_manager, &source);

    // Verify that WideArg instructions are present for the large jumps
    let wide_arg_count = code
        .instructions
        .iter()
        .filter(|i| matches!(i, Instruction::WideArg(_)))
        .count();
    assert!(
        wide_arg_count > 0,
        "Expected WideArg instructions for jump over >255 instructions"
    );

    // Verify result: condition is true, so we get then-branch [-1] = 300, plus 10 = 310
    assert_eq!(result.unwrap().as_int().unwrap(), 310);
}

#[test]
fn test_wide_jump_otherwise_large_primary() {
    // Test otherwise expression with a large primary that requires wide jump
    // PushOtherwise needs to jump over the large primary to the fallback
    // Primary FAILS so PushOtherwise jump is actually taken
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Helper to generate a 300-element array literal
    fn make_array(source: &mut String) {
        source.push('[');
        for i in 1..=300 {
            if i > 1 {
                source.push_str(", ");
            }
            source.push_str(&alloc::format!("{}", i));
        }
        source.push(']');
    }

    // Generate: ([1..300][999] otherwise 50) + 10
    // Primary FAILS (index out of bounds), fallback returns 50, result = 60
    // This exercises the PushOtherwise wide jump
    let mut source = String::new();
    source.push('(');
    make_array(&mut source);
    source.push_str("[999] otherwise 50) + 10");

    let (code, result) = compile_and_run(&arena, &type_manager, &source);

    // Verify that WideArg instructions are present
    let wide_arg_count = code
        .instructions
        .iter()
        .filter(|i| matches!(i, Instruction::WideArg(_)))
        .count();
    assert!(
        wide_arg_count > 0,
        "Expected WideArg instructions for large otherwise primary"
    );

    // Verify result: primary fails, fallback returns 50, plus 10 = 60
    assert_eq!(result.unwrap().as_int().unwrap(), 60);
}

#[test]
fn test_wide_jump_otherwise_large_fallback() {
    // Test otherwise expression with a large fallback that requires wide jump
    // PopOtherwiseAndJump needs to jump over the large fallback to the done label
    // Primary SUCCEEDS so PopOtherwiseAndJump is actually taken
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Helper to generate a 300-element array literal
    fn make_array(source: &mut String) {
        source.push('[');
        for i in 1..=300 {
            if i > 1 {
                source.push_str(", ");
            }
            source.push_str(&alloc::format!("{}", i));
        }
        source.push(']');
    }

    // Generate: (42 otherwise [1..300][-1]) + 10
    // Primary SUCCEEDS with 42, result = 52
    // This exercises the PopOtherwiseAndJump wide jump (skipping over large fallback)
    let mut source = String::new();
    source.push_str("(42 otherwise ");
    make_array(&mut source);
    source.push_str("[-1]) + 10");

    let (code, result) = compile_and_run(&arena, &type_manager, &source);

    // Verify that WideArg instructions are present
    let wide_arg_count = code
        .instructions
        .iter()
        .filter(|i| matches!(i, Instruction::WideArg(_)))
        .count();
    assert!(
        wide_arg_count > 0,
        "Expected WideArg instructions for large otherwise fallback"
    );

    // Verify result: primary succeeds with 42, plus 10 = 52
    assert_eq!(result.unwrap().as_int().unwrap(), 52);
}

#[test]
fn test_wide_jump_otherwise_large_both() {
    // Test otherwise expression with large primary AND large fallback
    // Both PushOtherwise and PopOtherwiseAndJump need WideArg
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Helper to generate a 300-element array literal
    fn make_array(source: &mut String) {
        source.push('[');
        for i in 1..=300 {
            if i > 1 {
                source.push_str(", ");
            }
            source.push_str(&alloc::format!("{}", i));
        }
        source.push(']');
    }

    // Generate: ([1..300][-1] otherwise [1..300][-2]) + 10
    // Primary succeeds with 300, result = 310
    let mut source = String::new();
    source.push('(');
    make_array(&mut source);
    source.push_str("[-1] otherwise ");
    make_array(&mut source);
    source.push_str("[-2]) + 10");

    let (code, result) = compile_and_run(&arena, &type_manager, &source);

    // Verify that WideArg instructions are present for both jumps
    let wide_arg_count = code
        .instructions
        .iter()
        .filter(|i| matches!(i, Instruction::WideArg(_)))
        .count();
    assert!(
        wide_arg_count >= 2,
        "Expected at least 2 WideArg instructions for large otherwise with both branches"
    );

    // Verify result: primary succeeds with 300, plus 10 = 310
    assert_eq!(result.unwrap().as_int().unwrap(), 310);
}

#[test]
fn test_wide_jump_vm_direct() {
    // Test WideArg with JumpForward directly in the VM
    use crate::vm::VM;

    let arena = Bump::new();

    // Create bytecode that jumps over 300 Nop instructions using WideArg
    // Jump offset = 300 (0x012C), encoded as WideArg(0x01), JumpForward(0x2C)
    let mut instructions = alloc::vec::Vec::new();
    instructions.push(Instruction::ConstInt(42)); // Push result first
    instructions.push(Instruction::WideArg(0x01)); // High byte of 300
    instructions.push(Instruction::JumpForward(0x2C)); // Low byte of 300

    // 299 Nop instructions to skip over
    for _ in 0..299 {
        instructions.push(Instruction::Nop);
    }

    instructions.push(Instruction::ConstInt(1)); // Not reached.
    instructions.push(Instruction::Return);

    let code = Code {
        constants: alloc::vec::Vec::new(),
        adapters: alloc::vec::Vec::new(),
        instructions,
        num_locals: 0,
        max_stack_size: 1,
    };

    let result = VM::execute(&arena, &code);
    assert_eq!(unsafe { result.unwrap().int_value }, 42);
}

#[test]
fn test_wide_jump_pop_jump_if_false_vm_direct() {
    // Test WideArg with PopJumpIfFalse directly in the VM
    use crate::vm::VM;

    let arena = Bump::new();

    // Create bytecode that conditionally jumps over 300 Nop instructions
    let mut instructions = alloc::vec::Vec::new();
    instructions.push(Instruction::ConstBool(0)); // Push false - should jump
    instructions.push(Instruction::WideArg(0x01)); // High byte of 300
    instructions.push(Instruction::PopJumpIfFalse(0x2C)); // Low byte of 300
    instructions.push(Instruction::ConstInt(1)); // Not reached (would be result if no jump)

    // 298 Nop instructions (one less because we also have the ConstInt above)
    for _ in 0..298 {
        instructions.push(Instruction::Nop);
    }

    instructions.push(Instruction::ConstInt(1)); // Not reached either.
    instructions.push(Instruction::ConstInt(42)); // This is where we land
    instructions.push(Instruction::Return);

    let code = Code {
        constants: alloc::vec::Vec::new(),
        adapters: alloc::vec::Vec::new(),
        instructions,
        num_locals: 0,
        max_stack_size: 1,
    };

    let result = VM::execute(&arena, &code);
    // Should jump to ConstInt(42) since condition is false
    assert_eq!(unsafe { result.unwrap().int_value }, 42);
}

// ============================================================================
// Bytes Indexing Tests
// ============================================================================

#[test]
fn test_bytes_indexing_first_element() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // b"hello"[0] should return 104 (ASCII 'h')
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"b"hello"[0]"#);
    assert_eq!(result.unwrap().as_int().unwrap(), 104); // 'h' = 104
}

#[test]
fn test_bytes_indexing_last_element() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // b"hello"[4] should return 111 (ASCII 'o')
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"b"hello"[4]"#);
    assert_eq!(result.unwrap().as_int().unwrap(), 111); // 'o' = 111
}

#[test]
fn test_bytes_indexing_negative_index() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // b"hello"[-1] should return 111 (ASCII 'o', last element)
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"b"hello"[-1]"#);
    assert_eq!(result.unwrap().as_int().unwrap(), 111); // 'o' = 111
}

#[test]
fn test_bytes_indexing_with_otherwise() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // b"hi"[10] otherwise 0 should return 0 (index out of bounds)
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"b"hi"[10] otherwise 0"#);
    assert_eq!(result.unwrap().as_int().unwrap(), 0);
}

// ============================================================================
// String Comparison Tests
// ============================================================================

#[test]
fn test_string_less_than() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // "bar" < "foo" should be true (lexicographic comparison)
    let (_code, result) = compile_and_run(&arena, &type_manager, r#""bar" < "foo""#);
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_string_greater_than() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // "foo" > "bar" should be true
    let (_code, result) = compile_and_run(&arena, &type_manager, r#""foo" > "bar""#);
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_string_less_than_or_equal() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // "abc" <= "abc" should be true
    let (_code, result) = compile_and_run(&arena, &type_manager, r#""abc" <= "abc""#);
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_string_greater_than_or_equal() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // "xyz" >= "abc" should be true
    let (_code, result) = compile_and_run(&arena, &type_manager, r#""xyz" >= "abc""#);
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

// ============================================================================
// Bytes Comparison Tests
// ============================================================================

#[test]
fn test_bytes_less_than() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // b"bar" < b"foo" should be true (lexicographic comparison)
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"b"bar" < b"foo""#);
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_bytes_greater_than() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // b"foo" > b"bar" should be true
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"b"foo" > b"bar""#);
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_bytes_less_than_or_equal() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // b"abc" <= b"abc" should be true
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"b"abc" <= b"abc""#);
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}

#[test]
fn test_bytes_greater_than_or_equal() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // b"xyz" >= b"abc" should be true
    let (_code, result) = compile_and_run(&arena, &type_manager, r#"b"xyz" >= b"abc""#);
    assert_eq!(result.unwrap().as_bool().unwrap(), true);
}
