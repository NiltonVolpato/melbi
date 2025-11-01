//! Benchmarks for the TypeManager.
//!
//! This file contains benchmarks to measure type interning and serialization performance.
//! Run with: `cargo bench --bench type_manager` in the core/ directory.
//!
//! Benchmark groups:
//! 1. record_creation: Creating new record types (not yet interned)
//! 2. record_interning: Getting already-interned record types
//! 3. serialization: Serializing types to bytes
//! 4. deserialization: Deserializing types from bytes

use bumpalo::Bump;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use melbi_core::{Type, types::manager::TypeManager};
use pprof::criterion::{Output, PProfProfiler};

/// Helper function to create various complex types by name
fn create_complex_type<'a>(manager: &'a TypeManager<'a>, name: &str) -> &'a Type<'a> {
    match name {
        "int" => manager.int(),
        "float" => manager.float(),
        "bool" => manager.bool(),
        "string" => manager.str(),
        "bytes" => manager.bytes(),
        "array_int" => manager.array(manager.int()),
        "map_string_int" => manager.map(manager.str(), manager.int()),
        "simple_record" => {
            let fields = vec![
                ("name", manager.str()),
                ("age", manager.int()),
                ("active", manager.bool()),
            ];
            manager.record(fields)
        }
        "nested_record" => {
            let inner_fields = vec![("street", manager.str()), ("city", manager.str())];
            let inner = manager.record(inner_fields);

            let outer_fields = vec![("name", manager.str()), ("address", inner)];
            manager.record(outer_fields)
        }
        "function" => {
            let params = vec![manager.int(), manager.str(), manager.bool()];
            manager.function(&params, manager.int())
        }
        _ => manager.int(),
    }
}

/// Benchmark: Creating new record types (first time, not yet interned).
///
/// Measures the cost of creating a record type with N fields, including:
/// - String interning for field names
/// - Sorting fields
/// - Type interning
/// - Arena allocation
fn bench_record_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("record_creation");

    let type_names = [
        "int",
        "float",
        "bool",
        "string",
        "array_int",
        "map_string_int",
    ];

    for num_fields in [5, 10, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_fields),
            &num_fields,
            |b, &num_fields| {
                // Setup: Create arena and manager OUTSIDE the benchmark
                let arena = Bump::new();
                let manager = TypeManager::new(&arena);
                let mut counter = 0u64;

                b.iter(|| {
                    // Prepare field data with varied types and unique names per iteration
                    let fields: Vec<(&str, _)> = (0..num_fields)
                        .map(|i| {
                            // Make field names unique per iteration to avoid cache hits
                            let name = arena.alloc_str(&format!("field_{}_{}", i, counter));
                            let type_name = type_names[i % type_names.len()];
                            let ty = create_complex_type(manager, type_name);
                            (name as &str, ty)
                        })
                        .collect();

                    counter += 1;

                    // Benchmark: Create the record type (fresh each time)
                    let record = manager.record(black_box(fields));
                    black_box(record);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Getting already-interned record types.
///
/// Measures the cost of getting a record type that's already been interned.
/// This should be fast - just a HashMap lookup.
fn bench_record_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("record_interning");

    let type_names = [
        "int",
        "float",
        "bool",
        "string",
        "array_int",
        "map_string_int",
    ];

    for num_fields in [5, 10, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_fields),
            &num_fields,
            |b, &num_fields| {
                // Setup: Create arena, manager, and pre-intern the record
                let arena = Bump::new();
                let manager = TypeManager::new(&arena);

                let fields: Vec<(&str, _)> = (0..num_fields)
                    .map(|i| {
                        let name = arena.alloc_str(&format!("field_{}", i));
                        let type_name = type_names[i % type_names.len()];
                        let ty = create_complex_type(manager, type_name);
                        (name as &str, ty)
                    })
                    .collect();

                // Pre-intern the record
                let _first = manager.record(fields.clone());

                // Benchmark: Get the already-interned record
                b.iter(|| {
                    let fields: Vec<(&str, _)> = (0..num_fields)
                        .map(|i| {
                            let name = arena.alloc_str(&format!("field_{}", i));
                            let type_name = type_names[i % type_names.len()];
                            let ty = create_complex_type(manager, type_name);
                            (name as &str, ty)
                        })
                        .collect();

                    let record = manager.record(black_box(fields.clone()));
                    black_box(record)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Serializing types to bytes.
///
/// Measures the cost of serializing different types to postcard format.
/// Uses the same complex types and records as the creation benchmarks.
fn bench_type_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_serialization");

    let type_names = [
        "int",
        "float",
        "bool",
        "string",
        "bytes",
        "array_int",
        "map_string_int",
        "simple_record",
        "nested_record",
        "function",
    ];
    let type_names_for_records = [
        "int",
        "float",
        "bool",
        "string",
        "array_int",
        "map_string_int",
    ];

    // Benchmark individual complex types
    for type_name in type_names {
        group.bench_function(type_name, |b| {
            let arena = Bump::new();
            let manager = TypeManager::new(&arena);
            let ty = create_complex_type(manager, type_name);

            // Benchmark: Serialize the type
            b.iter(|| {
                let bytes = manager
                    .serialize_type(black_box(ty))
                    .expect("Serialization failed");
                black_box(bytes)
            });
        });
    }

    // Benchmark records with varying number of fields (same as creation benchmarks)
    for num_fields in [5, 10, 50] {
        group.bench_function(format!("record_{}_fields", num_fields), |b| {
            let arena = Bump::new();
            let manager = TypeManager::new(&arena);

            let fields: Vec<(&str, _)> = (0..num_fields)
                .map(|i| {
                    let name = arena.alloc_str(&format!("field_{}", i));
                    let type_name = type_names_for_records[i % type_names_for_records.len()];
                    let ty = create_complex_type(manager, type_name);
                    (name as &str, ty)
                })
                .collect();

            let record = manager.record(fields);

            // Benchmark: Serialize the record
            b.iter(|| {
                let bytes = manager
                    .serialize_type(black_box(record))
                    .expect("Serialization failed");
                black_box(bytes)
            });
        });
    }

    group.finish();
}

/// Benchmark: Deserializing types from bytes.
///
/// Measures the cost of deserializing types from postcard format.
/// Uses the same complex types and records as the creation benchmarks.
fn bench_type_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_deserialization");

    let type_names = [
        "int",
        "float",
        "bool",
        "string",
        "bytes",
        "array_int",
        "map_string_int",
        "simple_record",
        "nested_record",
        "function",
    ];
    let type_names_for_records = [
        "int",
        "float",
        "bool",
        "string",
        "array_int",
        "map_string_int",
    ];

    // Benchmark individual complex types
    for type_name in type_names {
        group.bench_function(type_name, |b| {
            let arena = Bump::new();
            let manager = TypeManager::new(&arena);
            let ty = create_complex_type(manager, type_name);
            let bytes = manager.serialize_type(ty).expect("Serialization failed");

            // Benchmark: Deserialize the type
            b.iter(|| {
                let deserialized = manager
                    .deserialize_type(black_box(&bytes))
                    .expect("Deserialization failed");
                black_box(deserialized)
            });
        });
    }

    // Benchmark records with varying number of fields (same as creation benchmarks)
    for num_fields in [5, 10, 50] {
        group.bench_function(format!("record_{}_fields", num_fields), |b| {
            let arena = Bump::new();
            let manager = TypeManager::new(&arena);

            let fields: Vec<(&str, _)> = (0..num_fields)
                .map(|i| {
                    let name = arena.alloc_str(&format!("field_{}", i));
                    let type_name = type_names_for_records[i % type_names_for_records.len()];
                    let ty = create_complex_type(manager, type_name);
                    (name as &str, ty)
                })
                .collect();

            let record = manager.record(fields);
            let bytes = manager
                .serialize_type(record)
                .expect("Serialization failed");

            // Benchmark: Deserialize the record
            b.iter(|| {
                let deserialized = manager
                    .deserialize_type(black_box(&bytes))
                    .expect("Deserialization failed");
                black_box(deserialized)
            });
        });
    }

    group.finish();
}

/// Benchmark: Comparing types using serialized byte arrays.
///
/// Compares the cost of checking type equality using byte array comparison
/// vs the current approach (pointer equality after interning).
fn bench_type_equality_bytes(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_equality_bytes");

    let type_names = [
        "int",
        "float",
        "bool",
        "string",
        "bytes",
        "array_int",
        "map_string_int",
        "simple_record",
        "nested_record",
        "function",
    ];
    let type_names_for_records = [
        "int",
        "float",
        "bool",
        "string",
        "array_int",
        "map_string_int",
    ];

    // Benchmark comparing individual complex types
    for type_name in type_names {
        group.bench_function(type_name, |b| {
            let arena = Bump::new();
            let manager = TypeManager::new(&arena);
            let ty = create_complex_type(manager, type_name);

            // Serialize once
            let bytes1 = manager.serialize_type(ty).expect("Serialization failed");
            let bytes2 = bytes1.clone();

            // Benchmark: Compare byte arrays
            b.iter(|| {
                let result = black_box(&bytes1) == black_box(&bytes2);
                black_box(result)
            });
        });
    }

    // Benchmark comparing records with varying number of fields
    for num_fields in [5, 10, 50] {
        group.bench_function(format!("record_{}_fields", num_fields), |b| {
            let arena = Bump::new();
            let manager = TypeManager::new(&arena);

            let fields: Vec<(&str, _)> = (0..num_fields)
                .map(|i| {
                    let name = arena.alloc_str(&format!("field_{}", i));
                    let type_name = type_names_for_records[i % type_names_for_records.len()];
                    let ty = create_complex_type(manager, type_name);
                    (name as &str, ty)
                })
                .collect();

            let record = manager.record(fields);

            // Serialize once
            let bytes1 = manager
                .serialize_type(record)
                .expect("Serialization failed");
            let bytes2 = bytes1.clone();

            // Benchmark: Compare byte arrays
            b.iter(|| {
                let result = black_box(&bytes1) == black_box(&bytes2);
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark: Comparing types using pointer equality (current approach).
///
/// This is what happens after types are interned - just pointer comparison.
fn bench_type_equality_pointers(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_equality_pointers");

    let type_names = [
        "int",
        "float",
        "bool",
        "string",
        "bytes",
        "array_int",
        "map_string_int",
        "simple_record",
        "nested_record",
        "function",
    ];
    let type_names_for_records = [
        "int",
        "float",
        "bool",
        "string",
        "array_int",
        "map_string_int",
    ];

    // Benchmark comparing individual complex types
    for type_name in type_names {
        group.bench_function(type_name, |b| {
            let arena = Bump::new();
            let manager = TypeManager::new(&arena);
            let ty1 = create_complex_type(manager, type_name);
            let ty2 = ty1; // Same pointer

            // Benchmark: Compare pointers
            b.iter(|| {
                let result = core::ptr::eq(black_box(ty1), black_box(ty2));
                black_box(result)
            });
        });
    }

    // Benchmark comparing records with varying number of fields
    for num_fields in [5, 10, 50] {
        group.bench_function(format!("record_{}_fields", num_fields), |b| {
            let arena = Bump::new();
            let manager = TypeManager::new(&arena);

            let fields: Vec<(&str, _)> = (0..num_fields)
                .map(|i| {
                    let name = arena.alloc_str(&format!("field_{}", i));
                    let type_name = type_names_for_records[i % type_names_for_records.len()];
                    let ty = create_complex_type(manager, type_name);
                    (name as &str, ty)
                })
                .collect();

            let record = manager.record(fields);
            let same_record = record; // Same pointer

            // Benchmark: Compare pointers
            b.iter(|| {
                let result = core::ptr::eq(black_box(record), black_box(same_record));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark: Reading type discriminant from serialized bytes.
///
/// Measures the cost of zero-copy inspection: just reading the first byte
/// to determine what kind of type this is (Int, Record, etc.).
fn bench_read_discriminant(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_discriminant");

    let type_names = [
        "int",
        "float",
        "bool",
        "string",
        "bytes",
        "array_int",
        "map_string_int",
        "simple_record",
        "nested_record",
        "function",
    ];
    let type_names_for_records = [
        "int",
        "float",
        "bool",
        "string",
        "array_int",
        "map_string_int",
    ];

    // Benchmark reading discriminant from various types
    for type_name in type_names {
        group.bench_function(type_name, |b| {
            let arena = Bump::new();
            let manager = TypeManager::new(&arena);
            let ty = create_complex_type(manager, type_name);
            let bytes = manager.serialize_type(ty).expect("Serialization failed");

            // Benchmark: Read first byte
            b.iter(|| {
                let discriminant = black_box(&bytes)[0];
                black_box(discriminant)
            });
        });
    }

    // Benchmark reading discriminant from records
    for num_fields in [5, 10, 50] {
        group.bench_function(format!("record_{}_fields", num_fields), |b| {
            let arena = Bump::new();
            let manager = TypeManager::new(&arena);

            let fields: Vec<(&str, _)> = (0..num_fields)
                .map(|i| {
                    let name = arena.alloc_str(&format!("field_{}", i));
                    let type_name = type_names_for_records[i % type_names_for_records.len()];
                    let ty = create_complex_type(manager, type_name);
                    (name as &str, ty)
                })
                .collect();

            let record = manager.record(fields);
            let bytes = manager
                .serialize_type(record)
                .expect("Serialization failed");

            // Benchmark: Read first byte
            b.iter(|| {
                let discriminant = black_box(&bytes)[0];
                black_box(discriminant)
            });
        });
    }

    group.finish();
}

// Configure Criterion with profiling support
criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_record_creation, bench_record_interning, bench_type_serialization,
              bench_type_deserialization, bench_type_equality_bytes, bench_type_equality_pointers,
              bench_read_discriminant
}
criterion_main!(benches);
