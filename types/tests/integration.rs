//! Integration tests for melbi-types.
//!
//! These tests verify that all components work together correctly
//! across both ArenaBuilder and BoxBuilder.

use bumpalo::Bump;
use melbi_types::{
    ArenaBuilder, BoxBuilder, Scalar, Ty, TyDisplay, TypeBuilder, TypeFolder, TypeKind, TypeView,
    TypeVisitor,
};

#[test]
fn test_box_builder_basic_types() {
    let builder = BoxBuilder::new();

    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    let bool_ty = TypeKind::Scalar(Scalar::Bool).intern(builder);
    let float_ty = TypeKind::Scalar(Scalar::Float).intern(builder);

    assert!(int_ty.is_int(builder));
    assert!(!int_ty.is_bool(builder));
    assert!(!int_ty.is_float(builder));

    assert!(bool_ty.is_bool(builder));
    assert!(!bool_ty.is_int(builder));

    assert!(float_ty.is_float(builder));
    assert!(!float_ty.is_bool(builder));
}

#[test]
fn test_box_builder_array_types() {
    let builder = BoxBuilder::new();

    let arr_int = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);

    // Check nested element using view
    match arr_int.view(builder) {
        TypeKind::Array(elem) => {
            assert!(matches!(elem.view(builder), TypeKind::Scalar(Scalar::Int)));
        }
        _ => panic!("Expected Array type"),
    }
}

#[test]
fn test_arena_builder_basic_types() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);

    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    let bool_ty = TypeKind::Scalar(Scalar::Bool).intern(builder);

    assert!(int_ty.is_int(builder));
    assert!(!int_ty.is_bool(builder));

    assert!(bool_ty.is_bool(builder));
    assert!(!bool_ty.is_int(builder));
}

#[test]
fn test_arena_builder_array_types() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);

    let arr_bool = TypeKind::Array(TypeKind::Scalar(Scalar::Bool).intern(builder)).intern(builder);
    assert!(matches!(arr_bool.view(builder), TypeKind::Array(_)));

    match arr_bool.view(builder) {
        TypeKind::Array(elem) => {
            assert!(matches!(elem.view(builder), TypeKind::Scalar(Scalar::Bool)));
        }
        _ => panic!("Expected Array type"),
    }
}

#[test]
fn test_deeply_nested_arrays() {
    let builder = BoxBuilder::new();

    // Create Array[Array[Array[Int]]]
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    let arr1 = TypeKind::Array(int_ty).intern(builder);
    let arr2 = TypeKind::Array(arr1).intern(builder);
    let arr3 = TypeKind::Array(arr2).intern(builder);

    assert!(arr3.is_array(builder));

    // Navigate to the inner Int
    match arr3.view(builder) {
        TypeKind::Array(elem1) => match elem1.view(builder) {
            TypeKind::Array(elem2) => match elem2.view(builder) {
                TypeKind::Array(elem3) => {
                    assert!(elem3.is_int(builder));
                }
                _ => panic!("Expected Array"),
            },
            _ => panic!("Expected Array"),
        },
        _ => panic!("Expected Array"),
    }
}

#[test]
fn test_visitor_counting() {
    struct TypeCounter<B: TypeBuilder> {
        int_count: usize,
        bool_count: usize,
        float_count: usize,
        array_count: usize,
        builder: B,
    }

    impl<B: TypeBuilder> TypeVisitor<B> for TypeCounter<B> {
        fn builder(&self) -> B {
            self.builder
        }

        fn visit(&mut self, ty: B::TypeView) {
            let builder = self.builder;
            match ty.view(builder) {
                TypeKind::Scalar(Scalar::Int) => self.int_count += 1,
                TypeKind::Scalar(Scalar::Bool) => self.bool_count += 1,
                TypeKind::Scalar(Scalar::Float) => self.float_count += 1,
                TypeKind::Array(_) => self.array_count += 1,
                // Ignore new variants for this test
                TypeKind::TypeVar(_)
                | TypeKind::Map(_, _)
                | TypeKind::Record(_)
                | TypeKind::Function { .. }
                | TypeKind::Symbol(_)
                | TypeKind::Scalar(Scalar::Str)
                | TypeKind::Scalar(Scalar::Bytes) => {}
            }
            self.super_visit(ty);
        }
    }

    let builder = BoxBuilder::new();
    // Create Array[Array[Int]]
    let ty = TypeKind::Array(
        TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder),
    )
    .intern(builder);

    let mut counter = TypeCounter {
        int_count: 0,
        bool_count: 0,
        float_count: 0,
        array_count: 0,
        builder,
    };
    counter.visit(ty);

    assert_eq!(counter.int_count, 1);
    assert_eq!(counter.bool_count, 0);
    assert_eq!(counter.float_count, 0);
    assert_eq!(counter.array_count, 2);
}

#[test]
fn test_folder_transformation() {
    struct IntToBoolFolder {
        builder: BoxBuilder,
    }

    impl TypeFolder<BoxBuilder> for IntToBoolFolder {
        fn builder(&self) -> BoxBuilder {
            self.builder
        }

        fn fold_ty(&mut self, ty: Ty<BoxBuilder>) -> Ty<BoxBuilder> {
            if ty.is_int(self.builder) {
                TypeKind::Scalar(Scalar::Bool).intern(self.builder)
            } else {
                self.super_fold_ty(ty)
            }
        }
    }

    let builder = BoxBuilder::new();
    let original = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);

    let mut folder = IntToBoolFolder { builder };
    let transformed = folder.fold_ty(original);

    // Check that Array[Int] became Array[Bool]
    assert!(transformed.is_array(builder));
    match transformed.view(builder) {
        TypeKind::Array(elem) => {
            assert!(matches!(elem.view(builder), TypeKind::Scalar(Scalar::Bool)));
        }
        _ => panic!("Expected Array"),
    }
}

#[test]
fn test_display_formatting() {
    let builder = BoxBuilder::new();

    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    assert_eq!(int_ty.display(builder), "Int");

    let bool_ty = TypeKind::Scalar(Scalar::Bool).intern(builder);
    assert_eq!(bool_ty.display(builder), "Bool");

    let float_ty = TypeKind::Scalar(Scalar::Float).intern(builder);
    assert_eq!(float_ty.display(builder), "Float");

    let arr_int = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
    assert_eq!(arr_int.display(builder), "Array[Int]");

    let nested = TypeKind::Array(
        TypeKind::Array(TypeKind::Scalar(Scalar::Bool).intern(builder)).intern(builder),
    )
    .intern(builder);
    assert_eq!(nested.display(builder), "Array[Array[Bool]]");
}

#[test]
fn test_type_equality() {
    let builder = BoxBuilder::new();

    let int1 = TypeKind::Scalar(Scalar::Int).intern(builder);
    let int2 = TypeKind::Scalar(Scalar::Int).intern(builder);

    // Note: Without proper interning, these will be different Rc instances
    // but they should still represent the same logical type
    assert!(int1.is_int(builder));
    assert!(int2.is_int(builder));

    let arr1 = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);
    let arr2 = TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder);

    assert!(arr1.is_array(builder));
    assert!(arr2.is_array(builder));
}

#[test]
fn test_arena_multiple_types() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);

    // Create multiple types in the same arena
    let types = vec![
        TypeKind::Scalar(Scalar::Int).intern(builder),
        TypeKind::Scalar(Scalar::Bool).intern(builder),
        TypeKind::Scalar(Scalar::Float).intern(builder),
        TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder),
        TypeKind::Array(TypeKind::Scalar(Scalar::Bool).intern(builder)).intern(builder),
    ];

    assert!(types[0].is_int(builder));
    assert!(types[1].is_bool(builder));
    assert!(types[2].is_float(builder));
    assert!(types[3].is_array(builder));
    assert!(types[4].is_array(builder));

    // All should still be accessible
    for ty in types {
        match ty.view(builder) {
            TypeKind::Scalar(_) | TypeKind::Array(_) => {}
            // Ignore new variants for this test
            TypeKind::TypeVar(_)
            | TypeKind::Map(_, _)
            | TypeKind::Record(_)
            | TypeKind::Function { .. }
            | TypeKind::Symbol(_) => {}
        }
    }
}

#[test]
fn test_complex_visitor_with_state() {
    struct DepthCounter<B: TypeBuilder> {
        max_depth: usize,
        current_depth: usize,
        builder: B,
    }

    impl<B: TypeBuilder> TypeVisitor<B> for DepthCounter<B> {
        fn builder(&self) -> B {
            self.builder
        }

        fn visit(&mut self, ty: B::TypeView) {
            let builder = self.builder;
            let is_array = matches!(ty.view(builder), TypeKind::Array(_));
            if is_array {
                self.current_depth += 1;
                if self.current_depth > self.max_depth {
                    self.max_depth = self.current_depth;
                }
            }
            self.super_visit(ty);
            if is_array {
                self.current_depth -= 1;
            }
        }
    }

    let builder = BoxBuilder::new();

    // Create Array[Array[Array[Int]]] - depth 3
    let ty = TypeKind::Array(
        TypeKind::Array(
            TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder),
        )
        .intern(builder),
    )
    .intern(builder);

    let mut counter = DepthCounter {
        max_depth: 0,
        current_depth: 0,
        builder,
    };
    counter.visit(ty);

    assert_eq!(counter.max_depth, 3);
}

#[test]
fn test_folder_preserves_structure() {
    struct NoOpFolder {
        builder: BoxBuilder,
    }

    impl TypeFolder<BoxBuilder> for NoOpFolder {
        fn builder(&self) -> BoxBuilder {
            self.builder
        }
    }

    let builder = BoxBuilder::new();
    let original = TypeKind::Array(
        TypeKind::Array(TypeKind::Scalar(Scalar::Int).intern(builder)).intern(builder),
    )
    .intern(builder);

    let mut folder = NoOpFolder { builder };
    let result = folder.fold_ty(original.clone());

    // Structure should be preserved
    assert_eq!(original.display(builder), result.display(builder));
}

#[test]
fn test_type_var() {
    let builder = BoxBuilder::new();

    // Create type variables
    let tv0 = TypeKind::TypeVar(0).intern(builder);
    let tv1 = TypeKind::TypeVar(1).intern(builder);
    let tv42 = TypeKind::TypeVar(42).intern(builder);

    // Test display
    assert_eq!(tv0.display(builder), "_0");
    assert_eq!(tv1.display(builder), "_1");
    assert_eq!(tv42.display(builder), "_42");

    // Test kind matching
    match tv0.view(builder) {
        TypeKind::TypeVar(id) => assert_eq!(*id, 0),
        _ => panic!("Expected TypeVar"),
    }
}

#[test]
fn test_map_type() {
    let builder = BoxBuilder::new();

    // Create Map[Int, Bool]
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    let bool_ty = TypeKind::Scalar(Scalar::Bool).intern(builder);
    let map_ty = TypeKind::Map(int_ty.clone(), bool_ty.clone()).intern(builder);

    // Test display
    assert_eq!(map_ty.display(builder), "Map[Int, Bool]");

    // Test kind matching
    match map_ty.view(builder) {
        TypeKind::Map(key, val) => {
            assert!(key.is_int(builder));
            assert!(val.is_bool(builder));
        }
        _ => panic!("Expected Map"),
    }

    // Test nested maps: Map[Int, Map[Bool, Float]]
    let float_ty = TypeKind::Scalar(Scalar::Float).intern(builder);
    let inner_map = TypeKind::Map(bool_ty, float_ty).intern(builder);
    let outer_map = TypeKind::Map(int_ty, inner_map).intern(builder);

    assert_eq!(outer_map.display(builder), "Map[Int, Map[Bool, Float]]");
}

#[test]
fn test_record_type() {
    let builder = BoxBuilder::new();

    // Create Record[name: Str, age: Int]
    let str_ty = TypeKind::Scalar(Scalar::Str).intern(builder);
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);

    let fields = vec![("name", str_ty), ("age", int_ty)];
    let record_ty = TypeKind::Record(builder.intern_field_types(fields)).intern(builder);

    // Test display (fields should be sorted)
    assert_eq!(record_ty.display(builder), "Record[age: Int, name: Str]");

    // Test kind matching
    match record_ty.view(builder) {
        TypeKind::Record(fields) => {
            let field_data = builder.field_types_data(fields);
            assert_eq!(field_data.len(), 2);
            // Fields should be sorted by name
            assert_eq!(field_data[0].0.as_ref(), "age");
            assert_eq!(field_data[1].0.as_ref(), "name");
        }
        _ => panic!("Expected Record"),
    }
}

#[test]
fn test_function_type() {
    let builder = BoxBuilder::new();

    // Create (Int, Bool) => Float
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    let bool_ty = TypeKind::Scalar(Scalar::Bool).intern(builder);
    let float_ty = TypeKind::Scalar(Scalar::Float).intern(builder);

    let params = vec![int_ty.clone(), bool_ty];
    let func_ty = TypeKind::Function {
        params: builder.intern_types(params),
        ret: float_ty,
    }
    .intern(builder);

    // Test display
    assert_eq!(func_ty.display(builder), "(Int, Bool) => Float");

    // Test kind matching
    match func_ty.view(builder) {
        TypeKind::Function { params, ret } => {
            let param_data = builder.types_data(params);
            assert_eq!(param_data.len(), 2);
            assert!(param_data[0].is_int(builder));
            assert!(param_data[1].is_bool(builder));
            assert!(ret.is_float(builder));
        }
        _ => panic!("Expected Function"),
    }

    // Test zero-parameter function: () => Int
    let no_params_func = TypeKind::Function {
        params: builder.intern_types(Vec::<Ty<BoxBuilder>>::new()),
        ret: int_ty,
    }
    .intern(builder);

    assert_eq!(no_params_func.display(builder), "() => Int");
}

#[test]
fn test_symbol_type() {
    let builder = BoxBuilder::new();

    // Create Symbol[foo|bar|baz]
    let parts = vec!["foo", "bar", "baz"];
    let symbol_ty = TypeKind::Symbol(builder.intern_symbol_parts(parts)).intern(builder);

    // Test display (parts should be sorted)
    assert_eq!(symbol_ty.display(builder), "Symbol[bar|baz|foo]");

    // Test kind matching
    match symbol_ty.view(builder) {
        TypeKind::Symbol(parts) => {
            let part_data = builder.symbol_parts_data(parts);
            assert_eq!(part_data.len(), 3);
            // Parts should be sorted
            assert_eq!(part_data[0].as_ref(), "bar");
            assert_eq!(part_data[1].as_ref(), "baz");
            assert_eq!(part_data[2].as_ref(), "foo");
        }
        _ => panic!("Expected Symbol"),
    }
}

#[test]
fn test_str_and_bytes_scalars() {
    let builder = BoxBuilder::new();

    let str_ty = TypeKind::Scalar(Scalar::Str).intern(builder);
    let bytes_ty = TypeKind::Scalar(Scalar::Bytes).intern(builder);

    // Test display
    assert_eq!(str_ty.display(builder), "Str");
    assert_eq!(bytes_ty.display(builder), "Bytes");

    // Test scalar predicates
    assert!(Scalar::Str.is_string_like());
    assert!(Scalar::Bytes.is_string_like());
    assert!(!Scalar::Int.is_string_like());

    assert!(!Scalar::Str.is_numeric());
    assert!(!Scalar::Bytes.is_numeric());

    assert!(Scalar::Str.is_comparable());
    assert!(!Scalar::Bytes.is_comparable());
}

#[test]
fn test_complex_nested_types() {
    let builder = BoxBuilder::new();

    // Create a complex type: Map[Str, Record[id: Int, tags: Symbol[a|b]]]
    let str_ty = TypeKind::Scalar(Scalar::Str).intern(builder);
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);

    let symbol_ty = TypeKind::Symbol(builder.intern_symbol_parts(vec!["a", "b"])).intern(builder);

    let record_fields = vec![("id", int_ty), ("tags", symbol_ty)];
    let record_ty = TypeKind::Record(builder.intern_field_types(record_fields)).intern(builder);

    let map_ty = TypeKind::Map(str_ty, record_ty).intern(builder);

    // Test display
    let display_str = map_ty.display(builder);
    assert!(display_str.contains("Map[Str, Record["));
    assert!(display_str.contains("id: Int"));
    assert!(display_str.contains("tags: Symbol["));
}

#[test]
fn test_visitor_with_new_variants() {
    use melbi_types::TypeVisitor;

    struct NewTypeCounter<B: TypeBuilder> {
        map_count: usize,
        record_count: usize,
        function_count: usize,
        symbol_count: usize,
        typevar_count: usize,
        builder: B,
    }

    impl<B: TypeBuilder> TypeVisitor<B> for NewTypeCounter<B> {
        fn builder(&self) -> B {
            self.builder
        }

        fn visit(&mut self, ty: B::TypeView) {
            let builder = self.builder;
            match ty.view(builder) {
                TypeKind::Map(_, _) => self.map_count += 1,
                TypeKind::Record(_) => self.record_count += 1,
                TypeKind::Function { .. } => self.function_count += 1,
                TypeKind::Symbol(_) => self.symbol_count += 1,
                TypeKind::TypeVar(_) => self.typevar_count += 1,
                _ => {}
            }
            self.super_visit(ty);
        }
    }

    let builder = BoxBuilder::new();

    // Create a type with multiple new variants
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    let tv = TypeKind::TypeVar(0).intern(builder);
    let symbol = TypeKind::Symbol(builder.intern_symbol_parts(vec!["x"])).intern(builder);

    let map_ty = TypeKind::Map(tv, symbol).intern(builder);

    let record_fields = vec![("data", map_ty)];
    let record_ty = TypeKind::Record(builder.intern_field_types(record_fields)).intern(builder);

    let func_ty = TypeKind::Function {
        params: builder.intern_types(vec![int_ty]),
        ret: record_ty,
    }
    .intern(builder);

    let mut counter = NewTypeCounter {
        map_count: 0,
        record_count: 0,
        function_count: 0,
        symbol_count: 0,
        typevar_count: 0,
        builder,
    };

    counter.visit(func_ty);

    assert_eq!(counter.function_count, 1);
    assert_eq!(counter.record_count, 1);
    assert_eq!(counter.map_count, 1);
    assert_eq!(counter.symbol_count, 1);
    assert_eq!(counter.typevar_count, 1);
}

#[test]
fn test_folder_with_new_variants() {
    use melbi_types::TypeFolder;

    // Folder that replaces all TypeVars with Int
    struct TypeVarToIntFolder {
        builder: BoxBuilder,
    }

    impl TypeFolder<BoxBuilder> for TypeVarToIntFolder {
        fn builder(&self) -> BoxBuilder {
            self.builder
        }

        fn fold_ty(&mut self, ty: Ty<BoxBuilder>) -> Ty<BoxBuilder> {
            match ty.kind(self.builder) {
                TypeKind::TypeVar(_) => TypeKind::Scalar(Scalar::Int).intern(self.builder),
                _ => self.super_fold_ty(ty),
            }
        }
    }

    let builder = BoxBuilder::new();

    // Create Map[_0, _1]
    let tv0 = TypeKind::TypeVar(0).intern(builder);
    let tv1 = TypeKind::TypeVar(1).intern(builder);
    let map_ty = TypeKind::Map(tv0, tv1).intern(builder);

    let mut folder = TypeVarToIntFolder { builder };
    let result = folder.fold_ty(map_ty);

    // Should become Map[Int, Int]
    assert_eq!(result.display(builder), "Map[Int, Int]");
}

#[test]
fn test_arena_builder_with_new_variants() {
    use bumpalo::Bump;
    use melbi_types::ArenaBuilder;

    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);

    // Test TypeVar
    let tv = TypeKind::TypeVar(5).intern(builder);
    assert_eq!(tv.display(builder), "_5");

    // Test Map
    let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
    let str_ty = TypeKind::Scalar(Scalar::Str).intern(builder);
    let map_ty = TypeKind::Map(int_ty, str_ty).intern(builder);
    assert_eq!(map_ty.display(builder), "Map[Int, Str]");

    // Test Record
    let fields = vec![("x", int_ty), ("y", str_ty)];
    let record_ty = TypeKind::Record(builder.intern_field_types(fields)).intern(builder);
    assert!(record_ty.display(builder).contains("Record["));

    // Test Function
    let func_ty = TypeKind::Function {
        params: builder.intern_types(vec![int_ty, str_ty]),
        ret: int_ty,
    }
    .intern(builder);
    assert_eq!(func_ty.display(builder), "(Int, Str) => Int");

    // Test Symbol
    let symbol_ty =
        TypeKind::Symbol(builder.intern_symbol_parts(vec!["alpha", "beta"])).intern(builder);
    assert!(symbol_ty.display(builder).contains("Symbol["));
}
