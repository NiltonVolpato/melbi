use super::manager::TypeManager;
use bumpalo::Bump;

#[test]
fn test_interning() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    let int_type = manager.int();
    let float_type = manager.float();

    assert_eq!(int_type, manager.int());
    assert_eq!(float_type, manager.float());
}

#[test]
fn test_interning_record() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    let fields = [("x", manager.int()), ("y", manager.float())];
    let record_type = manager.record(&fields);

    let same_record_type = manager.record(&fields);
    assert_eq!(record_type, same_record_type);

    let fields_unordered = [("y", manager.float()), ("x", manager.int())];
    let record_type_unordered = manager.record(&fields_unordered);
    assert_eq!(record_type, record_type_unordered);
}

#[test]
fn test_interning_primitives() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    // Test Bool
    let bool_type = manager.bool();
    assert_eq!(bool_type, manager.bool());

    // Test Str
    let str_type = manager.str();
    assert_eq!(str_type, manager.str());

    // Test Bytes
    let bytes_type = manager.bytes();
    assert_eq!(bytes_type, manager.bytes());
}

#[test]
fn test_interning_array() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    let int_type = manager.int();
    let int_array = manager.array(int_type);
    let same_int_array = manager.array(int_type);
    assert_eq!(int_array, same_int_array);

    let float_type = manager.float();
    let float_array = manager.array(float_type);
    assert_ne!(int_array, float_array);

    // Nested arrays
    let nested_array = manager.array(int_array);
    let int_array_again = manager.array(int_type);
    let same_nested_array = manager.array(int_array_again);
    assert_eq!(nested_array, same_nested_array);
}

#[test]
fn test_interning_map() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    let str_type = manager.str();
    let int_type = manager.int();
    let str_to_int = manager.map(str_type, int_type);
    let same_str_to_int = manager.map(str_type, int_type);
    assert_eq!(str_to_int, same_str_to_int);

    let float_type = manager.float();
    let str_to_float = manager.map(str_type, float_type);
    assert_ne!(str_to_int, str_to_float);

    // Different key types
    let int_to_str = manager.map(int_type, str_type);
    assert_ne!(str_to_int, int_to_str);
}

#[test]
fn test_interning_function() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    // Simple function: (Int) => Bool
    let int_type = manager.int();
    let bool_type = manager.bool();
    let func1 = manager.function(&[int_type], bool_type);
    let same_func1 = manager.function(&[int_type], bool_type);
    assert_eq!(func1, same_func1);

    // Different return type
    let float_type = manager.float();
    let func2 = manager.function(&[int_type], float_type);
    assert_ne!(func1, func2);

    // Multiple parameters: (Int, Float) => Str
    let str_type = manager.str();
    let func3 = manager.function(&[int_type, float_type], str_type);
    let same_func3 = manager.function(&[int_type, float_type], str_type);
    assert_eq!(func3, same_func3);

    // Different parameter order
    let func4 = manager.function(&[float_type, int_type], str_type);
    assert_ne!(func3, func4);

    // No parameters: () => Bool
    let func5 = manager.function(&[], bool_type);
    let same_func5 = manager.function(&[], bool_type);
    assert_eq!(func5, same_func5);
}

#[test]
fn test_interning_symbol() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    let sym1 = manager.symbol(&["red", "green", "blue"]);
    let same_sym1 = manager.symbol(&["red", "green", "blue"]);
    assert_eq!(sym1, same_sym1);

    // Different order should still be equal (symbols are sorted)
    let sym1_unordered = manager.symbol(&["blue", "red", "green"]);
    assert_eq!(sym1, sym1_unordered);

    // Different symbols
    let sym2 = manager.symbol(&["red", "green"]);
    assert_ne!(sym1, sym2);

    // Single element symbol
    let sym3 = manager.symbol(&["single"]);
    let same_sym3 = manager.symbol(&["single"]);
    assert_eq!(sym3, same_sym3);
}

#[test]
fn test_interning_complex_types() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    // Array of Records
    let str_type = manager.str();
    let int_type = manager.int();
    let person_record = manager.record(&[("name", str_type), ("age", int_type)]);
    let people_array = manager.array(person_record);
    let same_person_record = manager.record(&[("name", str_type), ("age", int_type)]);
    let same_people_array = manager.array(same_person_record);
    assert_eq!(people_array, same_people_array);

    // Function returning Map
    let float_type = manager.float();
    let str_int_map = manager.map(str_type, int_type);
    let func = manager.function(&[str_type], str_int_map);
    let same_str_int_map = manager.map(str_type, int_type);
    let same_func = manager.function(&[str_type], same_str_int_map);
    assert_eq!(func, same_func);

    // Record with complex field types
    let int_array = manager.array(int_type);
    let str_float_map = manager.map(str_type, float_type);
    let complex_record = manager.record(&[("data", int_array), ("lookup", str_float_map)]);
    let same_int_array = manager.array(int_type);
    let same_str_float_map = manager.map(str_type, float_type);
    let same_complex_record =
        manager.record(&[("lookup", same_str_float_map), ("data", same_int_array)]);
    assert_eq!(complex_record, same_complex_record);
}

#[test]
fn test_display_primitives() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    assert_eq!(manager.int().to_string(), "Int");
    assert_eq!(manager.float().to_string(), "Float");
    assert_eq!(manager.bool().to_string(), "Bool");
    assert_eq!(manager.str().to_string(), "Str");
    assert_eq!(manager.bytes().to_string(), "Bytes");
}

#[test]
fn test_display_array() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    let int_type = manager.int();
    let int_array = manager.array(int_type);
    assert_eq!(int_array.to_string(), "Array[Int]");

    let str_type = manager.str();
    let str_array = manager.array(str_type);
    assert_eq!(str_array.to_string(), "Array[Str]");

    // Nested array
    let nested_array = manager.array(int_array);
    assert_eq!(nested_array.to_string(), "Array[Array[Int]]");
}

#[test]
fn test_display_map() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    let str_type = manager.str();
    let int_type = manager.int();
    let str_to_int = manager.map(str_type, int_type);
    assert_eq!(str_to_int.to_string(), "Map[Str, Int]");

    let float_type = manager.float();
    let int_to_float = manager.map(int_type, float_type);
    assert_eq!(int_to_float.to_string(), "Map[Int, Float]");

    // Map with complex value type
    let int_array = manager.array(int_type);
    let str_to_int_array = manager.map(str_type, int_array);
    assert_eq!(str_to_int_array.to_string(), "Map[Str, Array[Int]]");
}

#[test]
fn test_display_record() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    let str_type = manager.str();
    let int_type = manager.int();

    // Simple record
    let person = manager.record(&[("name", str_type), ("age", int_type)]);
    assert_eq!(person.to_string(), "Record[age: Int, name: Str]");

    // Single field record
    let single = manager.record(&[("id", int_type)]);
    assert_eq!(single.to_string(), "Record[id: Int]");

    // Record with complex fields
    let int_array = manager.array(int_type);
    let float_type = manager.float();
    let str_float_map = manager.map(str_type, float_type);
    let complex = manager.record(&[("data", int_array), ("lookup", str_float_map)]);
    assert_eq!(
        complex.to_string(),
        "Record[data: Array[Int], lookup: Map[Str, Float]]"
    );
}

#[test]
fn test_display_function() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    let int_type = manager.int();
    let bool_type = manager.bool();
    let str_type = manager.str();
    let float_type = manager.float();

    // Single parameter function
    let func1 = manager.function(&[int_type], bool_type);
    assert_eq!(func1.to_string(), "(Int) => Bool");

    // Multiple parameters function
    let func2 = manager.function(&[int_type, str_type, float_type], bool_type);
    assert_eq!(func2.to_string(), "(Int, Str, Float) => Bool");

    // No parameters function
    let func3 = manager.function(&[], int_type);
    assert_eq!(func3.to_string(), "() => Int");

    // Function returning function
    let func4 = manager.function(&[int_type], func1);
    assert_eq!(func4.to_string(), "(Int) => (Int) => Bool");

    // Function with complex return type
    let int_array = manager.array(int_type);
    let func5 = manager.function(&[str_type], int_array);
    assert_eq!(func5.to_string(), "(Str) => Array[Int]");
}

#[test]
fn test_display_symbol() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    // Multiple symbol parts
    let sym1 = manager.symbol(&["red", "green", "blue"]);
    assert_eq!(sym1.to_string(), "Symbol[blue|green|red]");

    // Single symbol part
    let sym2 = manager.symbol(&["single"]);
    assert_eq!(sym2.to_string(), "Symbol[single]");

    // Verify symbols are sorted
    let sym3 = manager.symbol(&["zebra", "apple", "monkey"]);
    assert_eq!(sym3.to_string(), "Symbol[apple|monkey|zebra]");
}

#[test]
fn test_display_complex_types() {
    let bump = Bump::new();
    let mut manager = TypeManager::new(&bump);

    let str_type = manager.str();
    let int_type = manager.int();
    let float_type = manager.float();

    // Array of Records
    let person_record = manager.record(&[("name", str_type), ("age", int_type)]);
    let people_array = manager.array(person_record);
    assert_eq!(
        people_array.to_string(),
        "Array[Record[age: Int, name: Str]]"
    );

    // Function returning Map
    let str_int_map = manager.map(str_type, int_type);
    let func = manager.function(&[str_type], str_int_map);
    assert_eq!(func.to_string(), "(Str) => Map[Str, Int]");

    // Record with nested complex types
    let int_array = manager.array(int_type);
    let str_float_map = manager.map(str_type, float_type);
    let func_type = manager.function(&[int_type], str_type);
    let complex_record = manager.record(&[
        ("data", int_array),
        ("lookup", str_float_map),
        ("transform", func_type),
    ]);
    assert_eq!(
        complex_record.to_string(),
        "Record[data: Array[Int], lookup: Map[Str, Float], transform: (Int) => Str]"
    );

    // Map of Functions
    let bool_type = manager.bool();
    let int_to_bool = manager.function(&[int_type], bool_type);
    let str_to_func = manager.map(str_type, int_to_bool);
    assert_eq!(str_to_func.to_string(), "Map[Str, (Int) => Bool]");
}
