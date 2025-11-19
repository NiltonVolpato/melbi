//! Tests for the bytecode compiler.

use crate::{
    analyzer,
    compiler::BytecodeCompiler,
    parser,
    types::manager::TypeManager,
    vm::Instruction,
};
use bumpalo::Bump;

#[test]
fn test_compile_simple_integer() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Parse and analyze: "42"
    let parsed = parser::parse(&arena, "42").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Compile
    let code = BytecodeCompiler::compile(typed.expr);

    // Verify bytecode: ConstInt(42), Return
    assert_eq!(code.instructions.len(), 2);
    assert_eq!(code.instructions[0], Instruction::ConstInt(42));
    assert_eq!(code.instructions[1], Instruction::Return);
    assert_eq!(code.max_stack_size, 1);
}

#[test]
fn test_compile_addition() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Parse and analyze: "2 + 3"
    let parsed = parser::parse(&arena, "2 + 3").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Compile
    let code = BytecodeCompiler::compile(typed.expr);

    // Verify bytecode: ConstInt(2), ConstInt(3), IntBinOp('+'), Return
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(2));
    assert_eq!(code.instructions[1], Instruction::ConstInt(3));
    assert_eq!(code.instructions[2], Instruction::IntBinOp(b'+'));
    assert_eq!(code.instructions[3], Instruction::Return);
    assert_eq!(code.max_stack_size, 2, "Stack depth should be 2 (two operands)");
}

#[test]
fn test_compile_subtraction() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Parse and analyze: "10 - 3"
    let parsed = parser::parse(&arena, "10 - 3").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Compile
    let code = BytecodeCompiler::compile(typed.expr);

    // Verify bytecode: ConstInt(10), ConstInt(3), IntBinOp('-'), Return
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(10));
    assert_eq!(code.instructions[1], Instruction::ConstInt(3));
    assert_eq!(code.instructions[2], Instruction::IntBinOp(b'-'));
    assert_eq!(code.instructions[3], Instruction::Return);
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_compile_multiplication() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Parse and analyze: "5 * 7"
    let parsed = parser::parse(&arena, "5 * 7").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Compile
    let code = BytecodeCompiler::compile(typed.expr);

    // Verify bytecode: ConstInt(5), ConstInt(7), IntBinOp('*'), Return
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(5));
    assert_eq!(code.instructions[1], Instruction::ConstInt(7));
    assert_eq!(code.instructions[2], Instruction::IntBinOp(b'*'));
    assert_eq!(code.instructions[3], Instruction::Return);
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_compile_negation() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Parse and analyze: "-(5)"
    // Use parentheses to force unary negation rather than negative literal
    let parsed = parser::parse(&arena, "-(5)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Compile
    let code = BytecodeCompiler::compile(typed.expr);

    // Verify bytecode: ConstInt(5), NegInt, Return
    assert_eq!(code.instructions.len(), 3);
    assert_eq!(code.instructions[0], Instruction::ConstInt(5));
    assert_eq!(code.instructions[1], Instruction::NegInt);
    assert_eq!(code.instructions[2], Instruction::Return);
    assert_eq!(code.max_stack_size, 1);
}

#[test]
fn test_compile_complex_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Parse and analyze: "(2 + 3) * 4"
    let parsed = parser::parse(&arena, "(2 + 3) * 4").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Compile
    let code = BytecodeCompiler::compile(typed.expr);

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
}

#[test]
fn test_stack_depth_tracking() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Parse and analyze: "1 + 2 + 3"
    // This should be parsed as (1 + 2) + 3 due to left-associativity
    let parsed = parser::parse(&arena, "1 + 2 + 3").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Compile
    let code = BytecodeCompiler::compile(typed.expr);

    // Stack never grows beyond 2 because we evaluate left-to-right
    assert_eq!(code.max_stack_size, 2, "Stack should never exceed 2 for left-associative operations");
}

#[test]
fn test_debug_output() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Parse and analyze: "(2 + 3) * 4"
    let parsed = parser::parse(&arena, "(2 + 3) * 4").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Compile
    let code = BytecodeCompiler::compile(typed.expr);

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

    // Parse and analyze: "10 - 3"
    let parsed = parser::parse(&arena, "10 - 3").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Use convenience method
    let code = BytecodeCompiler::compile(typed.expr);

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

    // Parse and analyze an expression with repeated large constants
    // Large integers (outside -128..255) go into the constant pool
    let parsed = parser::parse(&arena, "1000 + 1000 + 1000").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();

    // Compile
    let code = BytecodeCompiler::compile(typed.expr);

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

    // Test less than: "5 < 10"
    let parsed = parser::parse(&arena, "5 < 10").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(5));
    assert_eq!(code.instructions[1], Instruction::ConstInt(10));
    assert_eq!(code.instructions[2], Instruction::IntCmpOp(b'<'));
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_boolean_not() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "not (5 < 10)"
    let parsed = parser::parse(&arena, "not (5 < 10)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    // Expected: ConstInt(5), ConstInt(10), IntCmpOp('<'), Not
    assert_eq!(code.instructions.len(), 5);
    assert_eq!(code.instructions[0], Instruction::ConstInt(5));
    assert_eq!(code.instructions[1], Instruction::ConstInt(10));
    assert_eq!(code.instructions[2], Instruction::IntCmpOp(b'<'));
    assert_eq!(code.instructions[3], Instruction::Not);
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_boolean_and() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "true and false"
    let parsed = parser::parse(&arena, "true and false").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    // Expected: ConstTrue, ConstFalse, And, Return
    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstTrue);
    assert_eq!(code.instructions[1], Instruction::ConstFalse);
    assert_eq!(code.instructions[2], Instruction::And);
    assert_eq!(code.instructions[3], Instruction::Return);
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_if_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "if true then 42 else 99"
    let parsed = parser::parse(&arena, "if true then 42 else 99").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    // Print debug output to see the generated bytecode
    println!("\n{:?}\n", code);

    // Verify structure (exact offsets depend on jump patching implementation)
    // Should have: ConstTrue, JumpIfFalse, ConstInt(42), Jump, ConstInt(99)
    assert!(code.instructions.len() >= 6);
    assert_eq!(code.instructions[0], Instruction::ConstTrue);
    assert_eq!(code.max_stack_size, 1, "If expressions should have stack depth of 1");
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
        let parsed = parser::parse(&arena, expr).unwrap();
        let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
        let code = BytecodeCompiler::compile(typed.expr);

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

    // Test: "false or true"
    let parsed = parser::parse(&arena, "false or true").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstFalse);
    assert_eq!(code.instructions[1], Instruction::ConstTrue);
    assert_eq!(code.instructions[2], Instruction::Or);
    assert_eq!(code.max_stack_size, 2);
}

#[test]
fn test_complex_boolean_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "(5 < 10) and (3 > 1)"
    let parsed = parser::parse(&arena, "(5 < 10) and (3 > 1)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

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

    // Test: "if true then (if false then 1 else 2) else 3"
    let parsed = parser::parse(&arena, "if true then (if false then 1 else 2) else 3").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nNested if bytecode:\n{:?}\n", code);

    // Should have nested jump structure
    assert!(code.instructions.len() >= 10, "Nested if should have multiple jumps");
    assert_eq!(code.max_stack_size, 1, "Nested if should still have stack depth of 1");
}

#[test]
fn test_if_with_complex_condition() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "if (5 < 10) and (3 > 1) then 100 else 200"
    let parsed = parser::parse(&arena, "if (5 < 10) and (3 > 1) then 100 else 200").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nIf with complex condition:\n{:?}\n", code);

    // Complex condition evaluates two comparisons (depth 3), then jumps based on result
    assert_eq!(code.max_stack_size, 3, "Complex condition with And needs stack depth 3");
}

#[test]
fn test_chained_comparisons() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "1 < 2 and 2 < 3"
    let parsed = parser::parse(&arena, "1 < 2 and 2 < 3").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

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

    // Test: "5 != 10"
    let parsed = parser::parse(&arena, "5 != 10").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    assert_eq!(code.instructions.len(), 4);
    assert_eq!(code.instructions[0], Instruction::ConstInt(5));
    assert_eq!(code.instructions[1], Instruction::ConstInt(10));
    assert_eq!(code.instructions[2], Instruction::IntCmpOp(b'!'));
}

#[test]
fn test_empty_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "[]"
    let parsed = parser::parse(&arena, "[]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    // Should just be MakeArray(0)
    assert_eq!(code.instructions.len(), 2);
    assert_eq!(code.instructions[0], Instruction::MakeArray(0));
    assert_eq!(code.max_stack_size, 1, "Empty array still produces one value");
}

#[test]
fn test_array_with_constants() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "[1, 2, 3]"
    let parsed = parser::parse(&arena, "[1, 2, 3]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    // Should be: ConstInt(1), ConstInt(2), ConstInt(3), MakeArray(3)
    assert_eq!(code.instructions.len(), 5);
    assert_eq!(code.instructions[0], Instruction::ConstInt(1));
    assert_eq!(code.instructions[1], Instruction::ConstInt(2));
    assert_eq!(code.instructions[2], Instruction::ConstInt(3));
    assert_eq!(code.instructions[3], Instruction::MakeArray(3));
    assert_eq!(code.max_stack_size, 3, "Need to hold all elements before array creation");
}

#[test]
fn test_array_with_expressions() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "[1 + 2, 3 * 4]"
    let parsed = parser::parse(&arena, "[1 + 2, 3 * 4]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

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

    // Test: "[[1, 2], [3, 4]]"
    let parsed = parser::parse(&arena, "[[1, 2], [3, 4]]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nNested arrays:\n{:?}\n", code);

    // Should have two MakeArray(2) for inner arrays, then MakeArray(2) for outer
    let make_array_count = code
        .instructions
        .iter()
        .filter(|inst| matches!(inst, Instruction::MakeArray(_)))
        .count();
    assert_eq!(make_array_count, 3, "Should have 3 MakeArray instructions");

    // Second-to-last instruction should be MakeArray(2) for outer array, then Return
    assert_eq!(code.instructions[code.instructions.len() - 2], Instruction::MakeArray(2));
    assert_eq!(code.instructions[code.instructions.len() - 1], Instruction::Return);
}

#[test]
fn test_array_of_booleans() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "[true, false, 5 < 10]"
    let parsed = parser::parse(&arena, "[true, false, 5 < 10]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nArray of booleans:\n{:?}\n", code);

    // Should compile each element, create array, then return
    assert_eq!(code.instructions[0], Instruction::ConstTrue);
    assert_eq!(code.instructions[1], Instruction::ConstFalse);
    assert_eq!(code.instructions[4], Instruction::IntCmpOp(b'<'));
    assert_eq!(code.instructions[code.instructions.len() - 2], Instruction::MakeArray(3));
    assert_eq!(code.instructions[code.instructions.len() - 1], Instruction::Return);
}

#[test]
fn test_single_element_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "[42]"
    let parsed = parser::parse(&arena, "[42]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    assert_eq!(code.instructions.len(), 3);
    assert_eq!(code.instructions[0], Instruction::ConstInt(42));
    assert_eq!(code.instructions[1], Instruction::MakeArray(1));
    assert_eq!(code.max_stack_size, 1);
}

#[test]
fn test_float_addition() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "1.5 + 2.5"
    let parsed = parser::parse(&arena, "1.5 + 2.5").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

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
        let parsed = parser::parse(&arena, expr).unwrap();
        let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
        let code = BytecodeCompiler::compile(typed.expr);

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

    // Test: "-(3.14)"
    let parsed = parser::parse(&arena, "-(3.14)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

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
        let parsed = parser::parse(&arena, expr).unwrap();
        let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
        let code = BytecodeCompiler::compile(typed.expr);

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

    // Test: "(1.5 + 2.5) * 3.0"
    let parsed = parser::parse(&arena, "(1.5 + 2.5) * 3.0").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

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

    // Test: "[1.0, 2.0, 3.0]"
    let parsed = parser::parse(&arena, "[1.0, 2.0, 3.0]").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

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

    // Test: "x + 1 where { x = 5 }"
    let parsed = parser::parse(&arena, "x + 1 where { x = 5 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nSimple where binding:\n{:?}\n", code);

    // Should: ConstInt(5), StoreLocal(0), LoadLocal(0), ConstInt(1), IntBinOp(+)
    assert!(code.instructions.iter().any(|i| matches!(i, Instruction::StoreLocal(0))));
    assert!(code.instructions.iter().any(|i| matches!(i, Instruction::LoadLocal(0))));
    assert_eq!(code.num_locals, 1, "Should have 1 local variable");
}

#[test]
fn test_multiple_where_bindings() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "x + y where { x = 10, y = 20 }"
    let parsed = parser::parse(&arena, "x + y where { x = 10, y = 20 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nMultiple where bindings:\n{:?}\n", code);

    // Should have 2 local variables
    assert_eq!(code.num_locals, 2, "Should have 2 local variables");

    // Should have StoreLocal for both variables
    let store_count = code.instructions.iter()
        .filter(|i| matches!(i, Instruction::StoreLocal(_)))
        .count();
    assert_eq!(store_count, 2, "Should have 2 StoreLocal instructions");

    // Should load both variables and add them
    assert!(code.instructions.iter().any(|i| matches!(i, Instruction::LoadLocal(0))));
    assert!(code.instructions.iter().any(|i| matches!(i, Instruction::LoadLocal(1))));
}

#[test]
fn test_nested_where_bindings() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "y + 1 where { y = (x * 2 where { x = 5 }) }"
    let parsed = parser::parse(&arena, "y + 1 where { y = (x * 2 where { x = 5 }) }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nNested where bindings:\n{:?}\n", code);

    // Should have 2 local variables (x and y)
    assert_eq!(code.num_locals, 2, "Should have 2 local variables (x and y)");
}

#[test]
fn test_where_with_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "result where { result = 2 + 3 }"
    let parsed = parser::parse(&arena, "result where { result = 2 + 3 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nWhere with expression:\n{:?}\n", code);

    // The binding should evaluate the expression first
    // Expected: ConstInt(2), ConstInt(3), IntBinOp(+), StoreLocal(0), LoadLocal(0)
    let add_pos = code.instructions.iter().position(|i| matches!(i, Instruction::IntBinOp(b'+')));
    let store_pos = code.instructions.iter().position(|i| matches!(i, Instruction::StoreLocal(0)));

    assert!(add_pos.is_some() && store_pos.is_some());
    assert!(add_pos.unwrap() < store_pos.unwrap(), "Addition should happen before store");
}

#[test]
fn test_where_in_array() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "[x, x + 1, x + 2] where { x = 10 }"
    let parsed = parser::parse(&arena, "[x, x + 1, x + 2] where { x = 10 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nWhere in array:\n{:?}\n", code);

    // Should load x multiple times
    let load_count = code.instructions.iter()
        .filter(|i| matches!(i, Instruction::LoadLocal(0)))
        .count();
    assert_eq!(load_count, 3, "Should load x three times for array elements");

    // Should end with MakeArray(3), Return
    assert_eq!(code.instructions[code.instructions.len() - 2], Instruction::MakeArray(3));
    assert_eq!(code.instructions[code.instructions.len() - 1], Instruction::Return);
}

#[test]
fn test_where_with_shadowing() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "x where { x = (x where { x = 5 }) }"
    // Inner x shadows outer x
    let parsed = parser::parse(&arena, "x where { x = (x where { x = 5 }) }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nWhere with shadowing:\n{:?}\n", code);

    // Both bindings use the same variable name 'x'
    // The compiler will allocate SEPARATE slots for proper shadowing
    // Inner x gets slot 0, outer x gets slot 1
    assert_eq!(code.num_locals, 2, "Should allocate separate slots for shadowed variables");
}

#[test]
fn test_where_scope_unshadowing() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "x + (x where { x = 10 }) where { x = 5 }"
    // This tests that:
    // 1. Outer scope sets x = 5 (slot 0)
    // 2. Inner scope shadows x = 10 (slot 1)
    // 3. After inner scope exits, outer x is still accessible
    // Expected: outer x (5) + inner x (10) = 15
    let parsed = parser::parse(&arena, "x + (x where { x = 10 }) where { x = 5 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nWhere scope unshadowing:\n{:?}\n", code);

    // Should allocate 2 slots: one for outer x, one for inner x
    assert_eq!(code.num_locals, 2, "Should allocate 2 slots for outer and inner x");

    // Verify we're loading from both slots
    assert!(code.instructions.iter().any(|i| matches!(i, Instruction::StoreLocal(0))),
            "Should store to slot 0 (outer x)");
    assert!(code.instructions.iter().any(|i| matches!(i, Instruction::StoreLocal(1))),
            "Should store to slot 1 (inner x)");
    assert!(code.instructions.iter().any(|i| matches!(i, Instruction::LoadLocal(0))),
            "Should load from slot 0 (outer x for addition)");
    assert!(code.instructions.iter().any(|i| matches!(i, Instruction::LoadLocal(1))),
            "Should load from slot 1 (inner x)");

    // Should end with an addition
    assert!(code.instructions.iter().any(|i| matches!(i, Instruction::IntBinOp(b'+'))),
            "Should add the two x values");
}

#[test]
fn test_where_scope_restoration() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "[ x, x where { x = 10 }, x ] where { x = 1 }"
    // This tests that:
    // 1. Outer scope sets x = 1 (slot 0)
    // 2. First array element: x = 1
    // 3. Inner scope shadows x = 10 (slot 1), returns 10
    // 4. After inner scope exits, x is restored to 1
    // 5. Third array element: x = 1 again
    // Expected array: [1, 10, 1]
    let parsed = parser::parse(&arena, "[ x, x where { x = 10 }, x ] where { x = 1 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    println!("\nWhere scope restoration:\n{:?}\n", code);

    // Should allocate 2 slots: one for outer x, one for inner x
    assert_eq!(code.num_locals, 2, "Should allocate 2 slots for outer and inner x");

    // Should load from slot 0 twice (first and third array elements)
    let load_local_0_count = code.instructions.iter()
        .filter(|i| matches!(i, Instruction::LoadLocal(0)))
        .count();
    assert_eq!(load_local_0_count, 2,
               "Should load from slot 0 twice (first and third array elements)");

    // Should load from slot 1 once (for the inner x reference)
    let load_local_1_count = code.instructions.iter()
        .filter(|i| matches!(i, Instruction::LoadLocal(1)))
        .count();
    assert_eq!(load_local_1_count, 1,
               "Should load from slot 1 once (inner x)");

    // Should create an array with 3 elements
    assert!(code.instructions.iter().any(|i| matches!(i, Instruction::MakeArray(3))),
            "Should create array with 3 elements");
}

// ============================================================================
// VM Execution Tests
// ============================================================================

#[test]
fn test_vm_simple_integer() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "42").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    let mut vm = crate::vm::VM::new(&arena, &code);
    let result = vm.run().expect("VM execution failed");

    // Result should be 42
    assert_eq!(unsafe { result.int_value }, 42);
}

#[test]
fn test_vm_arithmetic() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "10 + 5 * 2").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    let mut vm = crate::vm::VM::new(&arena, &code);
    let result = vm.run().expect("VM execution failed");

    // Result should be 20 (10 + (5 * 2))
    assert_eq!(unsafe { result.int_value }, 20);
}

#[test]
fn test_vm_boolean_operations() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let parsed = parser::parse(&arena, "(5 < 10) and (3 > 1)").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    let mut vm = crate::vm::VM::new(&arena, &code);
    let result = vm.run().expect("VM execution failed");

    // Result should be true
    assert_eq!(unsafe { result.bool_value }, true);
}

#[test]
fn test_vm_if_expression() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: if true then 42 else 99
    let parsed = parser::parse(&arena, "if true then 42 else 99").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    let mut vm = crate::vm::VM::new(&arena, &code);
    let result = vm.run().expect("VM execution failed");

    // Result should be 42
    assert_eq!(unsafe { result.int_value }, 42);
}

#[test]
fn test_vm_where_binding() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "x + 1 where { x = 5 }"
    let parsed = parser::parse(&arena, "x + 1 where { x = 5 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    let mut vm = crate::vm::VM::new(&arena, &code);
    let result = vm.run().expect("VM execution failed");

    // Result should be 6 (5 + 1)
    assert_eq!(unsafe { result.int_value }, 6);
}

#[test]
fn test_vm_scope_restoration() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "[ x, x where { x = 10 }, x ] where { x = 1 }"
    // Should produce [1, 10, 1]
    let parsed = parser::parse(&arena, "[ x, x where { x = 10 }, x ] where { x = 1 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    let mut vm = crate::vm::VM::new(&arena, &code);
    let result = vm.run().expect("VM execution failed");

    // Result should be an array [1, 10, 1]
    let array_data = crate::values::ArrayData::from_raw_value(result);
    assert_eq!(array_data.length(), 3);
    assert_eq!(unsafe { array_data.get(0).int_value }, 1);
    assert_eq!(unsafe { array_data.get(1).int_value }, 10);
    assert_eq!(unsafe { array_data.get(2).int_value }, 1);
}

#[test]
fn test_vm_shadowing_unshadowing() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Test: "x + (x where { x = 10 }) where { x = 5 }"
    // Should be 5 + 10 = 15
    let parsed = parser::parse(&arena, "x + (x where { x = 10 }) where { x = 5 }").unwrap();
    let typed = analyzer::analyze(type_manager, &arena, &parsed, &[], &[]).unwrap();
    let code = BytecodeCompiler::compile(typed.expr);

    let mut vm = crate::vm::VM::new(&arena, &code);
    let result = vm.run().expect("VM execution failed");

    // Result should be 15
    assert_eq!(unsafe { result.int_value }, 15);
}
