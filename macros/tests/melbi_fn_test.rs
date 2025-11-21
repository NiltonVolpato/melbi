//! Test the #[melbi_fn] macro

use bumpalo::Bump;
use melbi_core::{
    types::manager::TypeManager,
    values::{
        dynamic::Value,
        function::{AnnotatedFunction, Function},
        typed::Str,
    },
};
use melbi_macros::melbi_fn;

/// Simple integer addition function
#[melbi_fn(name = "Add")]
fn add_function(_arena: &Bump, _type_mgr: &TypeManager, a: i64, b: i64) -> i64 {
    a + b
}

/// String length function
#[melbi_fn(name = "Len")]
fn len_function(_arena: &Bump, _type_mgr: &TypeManager, s: Str) -> i64 {
    s.chars().count() as i64
}

/// String uppercase function with explicit lifetimes
#[melbi_fn(name = "Upper")]
fn string_upper<'a>(arena: &'a Bump, _type_mgr: &'a TypeManager, s: Str<'a>) -> Str<'a> {
    let upper = s.to_ascii_uppercase();
    Str::from_str(arena, &upper)
}

#[test]
fn test_macro_generates_struct() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Should be able to create instances
    let add_fn = Add::new(type_mgr);
    let len_fn = Len::new(type_mgr);

    // Check metadata
    assert_eq!(add_fn.name(), "Add");
    assert_eq!(len_fn.name(), "Len");

    // Check locations are set
    let (crate_name, version, file, line, col) = add_fn.location();
    // The file path will be from the macro expansion location
    assert!(
        crate_name.contains("melbi") || crate_name.contains("macro_test"),
        "{}",
        crate_name
    );
    assert!(!version.is_empty());
    assert!(file.contains("macro_test.rs") || file.contains("melbi_fn.rs"));
    assert!(line > 0);
    assert!(col > 0);
}

#[test]
fn test_function_trait_impl() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = Add::new(type_mgr);

    // Check type is correct - just verify we can get the type
    let _fn_ty = add_fn.ty();

    // Create Value and call the function
    let value = Value::function(&arena, add_fn).unwrap();

    // Create arguments
    let a = Value::int(type_mgr, 5);
    let b = Value::int(type_mgr, 3);
    let args = [a, b];

    // Call the function
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&arena, type_mgr, &args)
            .unwrap()
    };

    // Check result
    assert_eq!(result.as_int().unwrap(), 8);
}

#[test]
fn test_annotated_function_register() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Register the function using RecordBuilder
    let add_fn = Add::new(type_mgr);
    let builder = Value::record_builder(type_mgr);
    let builder = add_fn.register(&arena, builder).unwrap();

    // Build the record and verify it has the function
    let record = builder.build(&arena).unwrap();
    assert!(record.as_record().is_ok());
}

#[test]
fn test_string_function() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let len_fn = Len::new(type_mgr);
    let value = Value::function(&arena, len_fn).unwrap();

    // Create a string argument
    let str_ty = type_mgr.str();
    let s = Value::str(&arena, str_ty, "hello");
    let args = [s];

    // Call the function
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&arena, type_mgr, &args)
            .unwrap()
    };

    // Check result
    assert_eq!(result.as_int().unwrap(), 5);
}
