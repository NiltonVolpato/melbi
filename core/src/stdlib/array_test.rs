//! Tests for the Array package

use super::build_array_package;
use crate::{
    api::{CompileOptionsOverride, Engine, EngineOptions},
    stdlib::{build_math_package, build_string_package},
    types::manager::TypeManager,
    values::dynamic::Value,
};
use bumpalo::Bump;

#[test]
fn test_array_package_builds() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let array = build_array_package(&arena, type_mgr).unwrap();
    let record = array.as_record().unwrap();

    // Should have all functions
    assert!(!record.is_empty());
    assert!(record.get("Len").is_some());
    assert!(record.get("IsEmpty").is_some());
    assert!(record.get("Slice").is_some());
    assert!(record.get("Concat").is_some());
    assert!(record.get("Flatten").is_some());
    assert!(record.get("Zip").is_some());
    assert!(record.get("Reverse").is_some());
    assert!(record.get("Map").is_some());
}

// Helper function for integration tests using the Engine to evaluate Melbi code
fn test_array_expr<F>(source: &str, check: F)
where
    F: FnOnce(Value),
{
    let options = EngineOptions::default();
    let arena = Bump::new();

    let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
        let array = build_array_package(arena, type_mgr).unwrap();
        env.register("Array", array).unwrap();
    });

    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, source, &[])
        .unwrap_or_else(|e| panic!("compilation should succeed for: {}\nError: {:?}", source, e));

    let val_arena = Bump::new();
    let result = expr
        .run(Default::default(), &val_arena, &[])
        .unwrap_or_else(|e| panic!("execution should succeed for: {}\nError: {:?}", source, e));

    check(result);
}

// Helper for tests that should fail at compile time with type errors
fn expect_type_error(source: &str, expected_error_substring: &str) {
    let options = EngineOptions::default();
    let arena = Bump::new();

    let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
        let array = build_array_package(arena, type_mgr).unwrap();
        env.register("Array", array).unwrap();

        // Also register other packages for integration tests
        let math = build_math_package(arena, type_mgr).unwrap();
        env.register("Math", math).unwrap();
        let string = build_string_package(arena, type_mgr).unwrap();
        env.register("String", string).unwrap();
    });

    let compile_opts = CompileOptionsOverride::default();
    let result = engine.compile(compile_opts, source, &[]);

    match result {
        Err(err) => {
            let error_message = format!("{:?}", err);
            assert!(
                error_message.contains(expected_error_substring),
                "Expected error containing '{}', but got: {}",
                expected_error_substring,
                error_message
            );
        }
        Ok(_) => panic!(
            "Expected compilation to fail with type error for: {}",
            source
        ),
    }
}

// Helper for integration tests with all packages
fn test_with_all_packages<F>(source: &str, check: F)
where
    F: FnOnce(Value),
{
    let options = EngineOptions::default();
    let arena = Bump::new();

    let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
        let array = build_array_package(arena, type_mgr).unwrap();
        env.register("Array", array).unwrap();
        let math = build_math_package(arena, type_mgr).unwrap();
        env.register("Math", math).unwrap();
        let string = build_string_package(arena, type_mgr).unwrap();
        env.register("String", string).unwrap();
    });

    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, source, &[])
        .unwrap_or_else(|e| panic!("compilation should succeed for: {}\nError: {:?}", source, e));

    let val_arena = Bump::new();
    let result = expr
        .run(Default::default(), &val_arena, &[])
        .unwrap_or_else(|e| panic!("execution should succeed for: {}\nError: {:?}", source, e));

    check(result);
}

// ============================================================================
// Len Tests
// ============================================================================

#[test]
fn test_len() {
    // Different types - polymorphism
    test_array_expr("Array.Len([1, 2, 3, 4, 5]) == 5", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    test_array_expr("Array.Len([\"hello\", \"world\"]) == 2", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    test_array_expr("Array.Len([1.5, 2.5, 3.5]) == 3", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    test_array_expr("Array.Len([true, false, true]) == 3", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Empty array
    test_array_expr("Array.Len([]) == 0", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Nested arrays
    test_array_expr("Array.Len([[1,2], [3,4]]) == 2", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });
}

// ============================================================================
// IsEmpty Tests
// ============================================================================

#[test]
fn test_is_empty() {
    test_array_expr("Array.IsEmpty([]) == true", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    test_array_expr("Array.IsEmpty([1]) == false", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    test_array_expr("Array.IsEmpty([1, 2, 3]) == false", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });
}

// ============================================================================
// Reverse Tests
// ============================================================================

#[test]
fn test_reverse() {
    // Basic reverse with integers
    test_array_expr("Array.Reverse([1, 2, 3]) == [3, 2, 1]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Reverse with strings
    test_array_expr(
        "Array.Reverse([\"a\", \"b\", \"c\"]) == [\"c\", \"b\", \"a\"]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Empty array
    test_array_expr("Array.Reverse([]) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Single element
    test_array_expr("Array.Reverse([42]) == [42]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });
}

// ============================================================================
// Map Tests
// ============================================================================

#[test]
fn test_map() {
    // Basic map with integers - double each element
    test_with_all_packages("Array.Map([1, 2, 3], (x) => x * 2) == [2, 4, 6]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Map to different type - Int to Bool
    test_with_all_packages(
        "Array.Map([1, 2, 3], (x) => x > 1) == [false, true, true]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Single element
    test_with_all_packages("Array.Map([42], (x) => x + 1) == [43]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Complex expression in mapper
    test_with_all_packages(
        "Array.Map([1, 2, 3], (x) => x * x + 1) == [2, 5, 10]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Nested operations
    test_with_all_packages("Array.Map([1, 2, 3], (x) => x * 2 + x) == [3, 6, 9]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });
}

#[test]
fn test_map_with_string_package() {
    // Map strings to their lengths
    test_with_all_packages(
        "Array.Map([\"a\", \"bb\", \"ccc\"], (s) => String.Len(s)) == [1, 2, 3]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Map strings to uppercase
    test_with_all_packages(
        "Array.Map([\"hello\", \"world\"], (s) => String.Upper(s)) == [\"HELLO\", \"WORLD\"]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );
}

#[test]
fn test_map_composition() {
    // Map then Reverse
    test_with_all_packages(
        "Array.Reverse(Array.Map([1, 2, 3], (x) => x * 2)) == [6, 4, 2]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Map then Len
    test_with_all_packages(
        "Array.Len(Array.Map([1, 2, 3, 4], (x) => x * 2)) == 4",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Map then Slice
    test_with_all_packages(
        "Array.Slice(Array.Map([1, 2, 3, 4, 5], (x) => x * 10), 1, 4) == [20, 30, 40]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Flatten then Map
    test_with_all_packages(
        "Array.Map(Array.Flatten([[1, 2], [3]]), (x) => x * 2) == [2, 4, 6]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );
}

#[test]
#[ignore = "TODO: Bug - empty arrays with different type variables don't compare equal"]
fn test_map_empty_array() {
    // Empty array
    test_with_all_packages("Array.Map([], (x) => x * 2) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });
}

#[test]
fn test_map_type_errors() {
    // Map expects function as second argument
    expect_type_error("Array.Map([1, 2, 3], 42)", "Type mismatch");

    // Map expects array as first argument
    expect_type_error("Array.Map(\"not array\", (x) => x)", "Type mismatch");

    // Function parameter type must match array element type
    expect_type_error(
        "Array.Map([1, 2, 3], (s) => String.Len(s))",
        "Type mismatch",
    );
}

// ============================================================================
// Slice Tests
// ============================================================================

#[test]
fn test_slice() {
    // Basic slice
    test_array_expr("Array.Slice([1, 2, 3, 4, 5], 1, 4) == [2, 3, 4]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Full array
    test_array_expr("Array.Slice([1, 2, 3], 0, 3) == [1, 2, 3]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Empty range (start == end)
    test_array_expr("Array.Slice([1, 2, 3], 2, 2) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Start > end
    test_array_expr("Array.Slice([1, 2, 3], 3, 1) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Start beyond length
    test_array_expr("Array.Slice([1, 2, 3], 10, 20) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // End beyond length (clamped)
    test_array_expr("Array.Slice([1, 2, 3], 1, 100) == [2, 3]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Empty array
    test_array_expr("Array.Slice([], 0, 5) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Slice of strings
    test_array_expr(
        "Array.Slice([\"a\", \"b\", \"c\", \"d\"], 1, 3) == [\"b\", \"c\"]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );
}

// ============================================================================
// Concat Tests
// ============================================================================

#[test]
fn test_concat() {
    // Basic concat
    test_array_expr("Array.Concat([1, 2], [3, 4]) == [1, 2, 3, 4]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Empty first array
    test_array_expr("Array.Concat([], [1, 2]) == [1, 2]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Empty second array
    test_array_expr("Array.Concat([1, 2], []) == [1, 2]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Both empty
    test_array_expr("Array.Concat([], []) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Strings (polymorphism)
    test_array_expr(
        "Array.Concat([\"a\", \"b\"], [\"c\", \"d\"]) == [\"a\", \"b\", \"c\", \"d\"]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );
}

// ============================================================================
// Flatten Tests
// ============================================================================

#[test]
fn test_flatten() {
    // Basic flatten
    test_array_expr(
        "Array.Flatten([[1, 2], [3, 4], [5]]) == [1, 2, 3, 4, 5]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // With empty inner arrays
    test_array_expr("Array.Flatten([[1], [], [2, 3]]) == [1, 2, 3]", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // All empty inner
    test_array_expr("Array.Flatten([[], [], []]) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });
}

#[test]
#[ignore = "TODO: Bug - empty arrays with different type variables don't compare equal (Array.Flatten([]) returns Array[_N], [] is Array[_M])"]
fn test_flatten_empty_outer() {
    test_array_expr("Array.Flatten([]) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Strings
    test_array_expr(
        "Array.Flatten([[\"a\", \"b\"], [\"c\"]]) == [\"a\", \"b\", \"c\"]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );
}

// ============================================================================
// Zip Tests
// ============================================================================

#[test]
fn test_zip() {
    // Basic zip
    test_array_expr(
        "Array.Zip([1, 2, 3], [4, 5, 6]) == [{first = 1, second = 4}, {first = 2, second = 5}, {first = 3, second = 6}]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Different lengths - first shorter
    test_array_expr(
        "Array.Zip([1, 2], [3, 4, 5, 6]) == [{first = 1, second = 3}, {first = 2, second = 4}]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Different lengths - second shorter
    test_array_expr(
        "Array.Zip([1, 2, 3, 4], [5, 6]) == [{first = 1, second = 5}, {first = 2, second = 6}]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Different types
    test_array_expr(
        "Array.Zip([1, 2], [\"a\", \"b\"]) == [{first = 1, second = \"a\"}, {first = 2, second = \"b\"}]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Accessing tuple fields
    test_array_expr("Array.Zip([1, 2], [3, 4])[0].first == 1", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    test_array_expr("Array.Zip([1, 2], [3, 4])[1].second == 4", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });
}

#[test]
#[ignore = "TODO: Bug - empty arrays with different type variables don't compare equal"]
fn test_zip_both_empty() {
    test_array_expr("Array.Zip([], []) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });
}

#[test]
#[ignore = "TODO: Bug - empty arrays with different type variables don't compare equal"]
fn test_zip_first_empty() {
    test_array_expr("Array.Zip([], [1, 2, 3]) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });
}

#[test]
#[ignore = "TODO: Bug - empty arrays with different type variables don't compare equal"]
fn test_zip_second_empty() {
    test_array_expr("Array.Zip([1, 2, 3], []) == []", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });
}

// ============================================================================
// Composition and Chaining Tests
// ============================================================================

#[test]
fn test_composition() {
    // Reverse after Concat
    test_array_expr(
        "Array.Reverse(Array.Concat([1, 2], [3, 4])) == [4, 3, 2, 1]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Len of Slice
    test_array_expr("Array.Len(Array.Slice([1, 2, 3, 4, 5], 1, 4)) == 3", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Reverse after Flatten
    test_array_expr(
        "Array.Reverse(Array.Flatten([[1, 2], [3]])) == [3, 2, 1]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Slice after Concat
    test_array_expr(
        "Array.Slice(Array.Concat([1, 2, 3], [4, 5, 6]), 2, 5) == [3, 4, 5]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Complex chain: Flatten, Reverse, Slice
    test_array_expr(
        "Array.Slice(Array.Reverse(Array.Flatten([[1, 2], [3, 4]])), 1, 3) == [3, 2]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );
}

// ============================================================================
// Type Safety Tests - Should fail at compile time
// ============================================================================

#[test]
fn test_type_errors() {
    // Len expects array, not string
    expect_type_error("Array.Len(\"not an array\")", "Type mismatch");

    // Concat expects same element types
    expect_type_error("Array.Concat([1, 2], [\"a\", \"b\"])", "Type mismatch");

    // Slice expects Int indices, not strings
    expect_type_error("Array.Slice([1, 2, 3], \"start\", 2)", "Type mismatch");

    // Slice expects Int indices, not floats
    expect_type_error("Array.Slice([1, 2, 3], 1.5, 2)", "Type mismatch");

    // Flatten expects array of arrays
    expect_type_error("Array.Flatten([1, 2, 3])", "Type mismatch");

    // IsEmpty expects array
    expect_type_error("Array.IsEmpty(5)", "Type mismatch");

    // Reverse expects array
    expect_type_error("Array.Reverse(\"not an array\")", "Type mismatch");
}

// ============================================================================
// Integration Tests with String and Math packages
// ============================================================================

#[test]
fn test_integration_with_string() {
    // Len of Split result
    test_with_all_packages("Array.Len(String.Split(\"a,b,c\", \",\")) == 3", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Reverse Split result
    test_with_all_packages(
        "Array.Reverse(String.Split(\"a,b,c\", \",\")) == [\"c\", \"b\", \"a\"]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Concat two Split results
    test_with_all_packages(
        "Array.Concat(String.Split(\"a,b\", \",\"), String.Split(\"c,d\", \",\")) == [\"a\", \"b\", \"c\", \"d\"]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Slice of Split
    test_with_all_packages(
        "Array.Slice(String.Split(\"a,b,c,d\", \",\"), 1, 3) == [\"b\", \"c\"]",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );
}

#[test]
fn test_integration_with_math() {
    // Array of Math results
    test_with_all_packages("Array.Len([Math.Floor(3.7), Math.Ceil(2.1)]) == 2", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Reverse array of floats with Math.PI
    test_with_all_packages("Array.Reverse([Math.PI, Math.E])[0] == Math.E", |r| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    // Zip with Math results
    test_with_all_packages(
        "Array.Zip([1, 2], [Math.Floor(Math.PI), Math.Ceil(Math.E)])[0].second == 3",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );
}

#[test]
fn test_integration_combined() {
    // Complex: Split, Len, with Math
    test_with_all_packages(
        "Array.Len(String.Split(\"hello,world,test\", \",\")) == Math.Floor(Math.PI)",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );

    // Zip String results with Math results
    test_with_all_packages(
        "Array.Len(Array.Zip(String.Split(\"a,b\", \",\"), [Math.Floor(1.5), Math.Ceil(2.5)])) == 2",
        |r| {
            assert_eq!(r.as_bool().unwrap(), true);
        },
    );
}
