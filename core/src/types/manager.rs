use crate::{
    Vec,
    types::types::{CompareTypeArgs, Type},
};
use bumpalo::Bump;
use core::cell::{Cell, Ref, RefCell};
use hashbrown::{DefaultHashBuilder, HashMap};

pub struct TypeManager<'a> {
    // Arena holding all types from this TypeManager.
    arena: &'a Bump,
    interned_strs: RefCell<HashMap<&'a str, &'a str, DefaultHashBuilder, &'a Bump>>,
    interned: RefCell<HashMap<CompareTypeArgs<'a>, &'a Type<'a>, DefaultHashBuilder, &'a Bump>>,
    next_type_var: Cell<u16>,
}

impl<'a> TypeManager<'a> {
    pub fn new(arena: &'a Bump) -> &'a Self {
        arena.alloc(Self {
            arena,
            interned_strs: RefCell::new(HashMap::new_in(arena)),
            interned: RefCell::new(HashMap::new_in(arena)),
            next_type_var: Cell::new(0),
        })
    }

    pub(super) fn intern_str(&self, s: &str) -> &'a str {
        if let Some(&interned_str) = self.interned_strs.borrow().get(s) {
            return interned_str;
        }
        let arena_str = self.arena.alloc_str(s);
        self.interned_strs.borrow_mut().insert(arena_str, arena_str);
        arena_str
    }

    fn intern_map(
        &self,
    ) -> Ref<'_, HashMap<CompareTypeArgs<'a>, &'a Type<'a>, DefaultHashBuilder, &'a Bump>> {
        self.interned.borrow()
    }

    fn alloc_and_intern(&self, ty: Type<'a>) -> &'a Type<'a> {
        let arena_ty = self.arena.alloc(ty.clone());
        self.interned
            .borrow_mut()
            .insert(CompareTypeArgs(ty), arena_ty);
        arena_ty
    }

    // Generate fresh type variable
    pub fn fresh_type_var(&self) -> &'a Type<'a> {
        let var_id = self.next_type_var.get();
        self.next_type_var
            .set(var_id.checked_add(1).expect("TypeVar id overflowed"));
        let ty = Type::TypeVar(var_id);
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(ty.clone())) {
            return interned_ty;
        }
        self.alloc_and_intern(ty)
    }

    // Create a type variable with a specific id (for deserialization)
    pub(super) fn type_var(&self, id: u16) -> &'a Type<'a> {
        let ty = Type::TypeVar(id);
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(ty.clone())) {
            return interned_ty;
        }
        self.alloc_and_intern(ty)
    }

    // Factory methods for types.
    pub fn int(&self) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(Type::Int)) {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Int)
    }
    pub fn float(&self) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(Type::Float)) {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Float)
    }
    pub fn bool(&self) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(Type::Bool)) {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Bool)
    }
    pub fn str(&self) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(Type::Str)) {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Str)
    }
    pub fn bytes(&self) -> &'a Type<'a> {
        if let Some(&interned_ty) = self.intern_map().get(&CompareTypeArgs(Type::Bytes)) {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Bytes)
    }
    pub fn array(&self, elem_ty: &'a Type<'a>) -> &'a Type<'a> {
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Array(elem_ty)))
        {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Array(elem_ty))
    }

    pub fn map(&self, key_ty: &'a Type<'a>, val_ty: &'a Type<'a>) -> &'a Type<'a> {
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Map(key_ty, val_ty)))
        {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Map(key_ty, val_ty))
    }

    pub fn record(&self, fields: Vec<(&str, &'a Type<'a>)>) -> &'a Type<'a> {
        // SAFETY: We own the data in the Vec, which was moved. Also, we immediately change
        // the lifetime of the &str field to 'a.
        let mut fields: Vec<(&str, &'a Type<'a>)> = unsafe { core::mem::transmute(fields) };
        // Intern all field names in-place to ensure pointer equality works
        for (name, _) in fields.iter_mut() {
            *name = self.intern_str(name);
        }

        // Sort by interned field names in-place
        fields.sort_by_key(|(name, _)| *name);

        // Lookup using the Vec as a slice
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Record(&fields)))
        {
            return interned_ty;
        }

        // Not found - allocate directly from Vec into arena (zero-copy move)
        let arena_fields = self.arena.alloc_slice_fill_iter(fields.into_iter());
        self.alloc_and_intern(Type::Record(arena_fields))
    }

    pub fn function(&self, params: &[&'a Type<'a>], ret: &'a Type<'a>) -> &'a Type<'a> {
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Function { params, ret }))
        {
            return interned_ty;
        }
        self.alloc_and_intern(Type::Function {
            params: self.arena.alloc_slice_copy(params),
            ret,
        })
    }

    pub fn symbol(&self, parts: Vec<&str>) -> &'a Type<'a> {
        // SAFETY: We own the data in the Vec, which was moved. Also, we immediately change
        // the lifetime of the &str field to 'a.
        let mut parts: Vec<&str> = unsafe { core::mem::transmute(parts) };

        // Intern all symbol parts in-place to ensure pointer equality works
        for part in parts.iter_mut() {
            *part = self.intern_str(part);
        }

        // Sort by interned parts in-place
        parts.sort();

        // Lookup using the Vec as a slice
        if let Some(&interned_ty) = self
            .intern_map()
            .get(&CompareTypeArgs(Type::Symbol(&parts)))
        {
            return interned_ty;
        }

        // Not found - allocate directly from Vec into arena (zero-copy move)
        let arena_parts = self.arena.alloc_slice_fill_iter(parts.into_iter());
        self.alloc_and_intern(Type::Symbol(arena_parts))
    }

    // TODO: Implement custom types and their capabilities.
    // pub fn custom(&mut self, name: String) -> &'a Type<'a> {
    //     self.intern(Type::Custom { name })
    // }

    // // Register a custom type's capabilities
    // pub fn register_custom_type<T: TypeCapabilities + 'static>(&mut self, capabilities: T) {
    //     self.type_registry.register_type(capabilities);
    // }

    // // Check if a custom type supports an operation
    // pub fn custom_type_supports(&self, type_name: &str, operation: &str) -> bool {
    //     self.type_registry.supports_operation(type_name, operation)
    // }

    // // Get capabilities for a custom type
    // pub fn get_custom_capabilities(&self, type_name: &str) -> Option<&dyn TypeCapabilities> {
    //     self.type_registry.get_capabilities(type_name)
    // }

    /// Recursively copies a type from another TypeManager into this TypeManager's arena,
    /// returning the interned equivalent in this manager.
    pub fn adopt<'b>(&self, other: &TypeManager<'b>, ty: &'b Type<'b>) -> &'a Type<'a> {
        fn inner<'a, 'b>(
            this: &TypeManager<'a>,
            _other: &TypeManager<'b>,
            ty: &'b Type<'b>,
            var_map: &mut HashMap<*const Type<'b>, &'a Type<'a>>,
        ) -> &'a Type<'a> {
            match ty {
                Type::Int => this.int(),
                Type::Float => this.float(),
                Type::Bool => this.bool(),
                Type::Str => this.str(),
                Type::Bytes => this.bytes(),
                Type::TypeVar(_id) => {
                    let ptr = ty as *const Type<'b>;
                    if let Some(&mapped) = var_map.get(&ptr) {
                        mapped
                    } else {
                        // Use fresh_type_var to generate a new variable in this manager
                        let fresh = this.fresh_type_var();
                        var_map.insert(ptr, fresh);
                        fresh
                    }
                }
                Type::Array(elem_ty) => {
                    let elem = inner(this, _other, elem_ty, var_map);
                    this.array(elem)
                }
                Type::Map(key_ty, val_ty) => {
                    let key = inner(this, _other, key_ty, var_map);
                    let val = inner(this, _other, val_ty, var_map);
                    this.map(key, val)
                }
                Type::Record(fields) => {
                    let adopted_fields: Vec<(&str, &'a Type<'a>)> = fields
                        .iter()
                        .map(|(name, t)| {
                            let t = inner(this, _other, t, var_map);
                            (*name, t)
                        })
                        .collect();
                    this.record(adopted_fields)
                }
                Type::Function { params, ret } => {
                    let adopted_params: Vec<&'a Type<'a>> = params
                        .iter()
                        .map(|p| inner(this, _other, p, var_map))
                        .collect();
                    let adopted_ret = inner(this, _other, ret, var_map);
                    this.function(&adopted_params, adopted_ret)
                }
                Type::Symbol(parts) => {
                    let adopted_parts: Vec<&str> = (*parts).iter().copied().collect();
                    this.symbol(adopted_parts)
                }
            }
        }
        let mut var_map = HashMap::new();
        inner(self, other, ty, &mut var_map)
    }

    /// Performs alpha conversion (renaming) of type variables in a type.
    /// Takes a mapping from old variable names to new variable names.
    pub fn alpha_convert(&self, ty: &'a Type<'a>) -> &'a Type<'a> {
        pub fn inner<'a>(
            this: &TypeManager<'a>,
            ty: &'a Type<'a>,
            var_map: &mut HashMap<*const Type<'a>, &'a Type<'a>>,
        ) -> &'a Type<'a> {
            match ty {
                Type::Int | Type::Float | Type::Bool | Type::Str | Type::Bytes => ty,
                Type::TypeVar(_) => {
                    let ptr = ty as *const Type<'a>;
                    if let Some(&mapped) = var_map.get(&ptr) {
                        mapped
                    } else {
                        let fresh = this.fresh_type_var();
                        var_map.insert(ptr, fresh);
                        fresh
                    }
                }
                Type::Array(elem_ty) => {
                    let elem = inner(this, elem_ty, var_map);
                    this.array(elem)
                }
                Type::Map(key_ty, val_ty) => {
                    let key = inner(this, key_ty, var_map);
                    let val = inner(this, val_ty, var_map);
                    this.map(key, val)
                }
                Type::Record(fields) => {
                    let converted_fields: Vec<(&'a str, &'a Type<'a>)> = fields
                        .iter()
                        .map(|(name, t)| {
                            let t = inner(this, t, var_map);
                            (*name, t)
                        })
                        .collect();
                    this.record(converted_fields)
                }
                Type::Function { params, ret } => {
                    let converted_params: Vec<&'a Type<'a>> =
                        params.iter().map(|p| inner(this, p, var_map)).collect();
                    let converted_ret = inner(this, ret, var_map);
                    this.function(&converted_params, converted_ret)
                }
                Type::Symbol(_parts) => ty, // Symbols don't contain type variables, return as-is
            }
        }
        let mut var_map = HashMap::new();
        inner(&self, ty, &mut var_map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interning() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        let int_type = manager.int();
        let float_type = manager.float();

        // Verify that calling the factory methods again returns the same pointer
        assert!(core::ptr::eq(int_type, manager.int()));
        assert!(core::ptr::eq(float_type, manager.float()));
    }

    #[test]
    fn test_adopt_preserves_typevar_identity() {
        let bump1 = Bump::new();
        let bump2 = Bump::new();
        let mgr1 = TypeManager::new(&bump1);
        let mgr2 = TypeManager::new(&bump2);

        // Create a type with repeated typevars: (Map[k, v], k, v) -> v
        let k = mgr1.fresh_type_var();
        let v = mgr1.fresh_type_var();
        let map = mgr1.map(k, v);
        let fields = vec![("map", map), ("k", k), ("v", v)];
        let tuple = mgr1.record(fields);
        let fun = mgr1.function(&[tuple], v);

        // Adopt into mgr2
        let adopted = mgr2.adopt(&mgr1, fun);

        // Extract adopted typevars
        if let Type::Function { params, ret } = adopted {
            if let Type::Record(fields) = params[0] {
                let adopted_map = fields.iter().find(|(n, _)| *n == "map").unwrap().1;
                let adopted_k = fields.iter().find(|(n, _)| *n == "k").unwrap().1;
                let adopted_v = fields.iter().find(|(n, _)| *n == "v").unwrap().1;

                // In the adopted type, the k in map and the k field must be the same pointer
                if let Type::Map(map_k, map_v) = adopted_map {
                    assert!(
                        core::ptr::eq(*map_k, adopted_k),
                        "k typevar identity not preserved"
                    );
                    assert!(
                        core::ptr::eq(*map_v, adopted_v),
                        "v typevar identity not preserved"
                    );
                } else {
                    panic!("Expected Map type in adopted record");
                }

                // The return type must be the same as the v field
                assert!(
                    core::ptr::eq(*ret, adopted_v),
                    "return typevar identity not preserved"
                );
            } else {
                panic!("Expected Record type in adopted function parameter");
            }
        } else {
            panic!("Expected Function type at top level");
        }
    }

    #[test]
    fn test_alpha_convert_simple_typevar() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create a type variable
        let var_a = manager.type_var(42);

        // Alpha convert
        let converted = manager.alpha_convert(var_a);

        // Should be a fresh type variable (not the same as input)
        assert!(!core::ptr::eq(converted, var_a));
        // Should still be a TypeVar
        if let Type::TypeVar(_) = converted {
            // ok
        } else {
            panic!("Expected TypeVar after alpha_convert");
        }
    }

    #[test]
    fn test_alpha_convert_function_type_same_var() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create function type: a -> a
        let var_a = manager.type_var(42);
        let func = manager.function(&[var_a], var_a);

        // Alpha convert
        let converted = manager.alpha_convert(func);

        // Should be a function type
        if let Type::Function { params, ret } = converted {
            assert_eq!(params.len(), 1);
            // Both param and ret should be the same pointer (same fresh var)
            assert!(
                core::ptr::eq(params[0], *ret),
                "Param and ret should be the same fresh typevar"
            );
            // Should not be the same as the original var_a
            assert!(!core::ptr::eq(params[0], var_a));
        } else {
            panic!("Expected Function type after alpha_convert");
        }
    }

    #[test]
    fn test_alpha_convert_function_type_different_vars() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create function type: a -> b
        let var_a = manager.type_var(42);
        let var_b = manager.type_var(43);
        let func = manager.function(&[var_a], var_b);

        // Alpha convert
        let converted = manager.alpha_convert(func);

        // Should be a function type
        if let Type::Function { params, ret } = converted {
            assert_eq!(params.len(), 1);
            // Param and ret should be different pointers (different fresh vars)
            assert!(
                !core::ptr::eq(params[0], *ret),
                "Param and ret should be different fresh typevars"
            );
            // Should not be the same as the original var_a or var_b
            assert!(!core::ptr::eq(params[0], var_a));
            assert!(!core::ptr::eq(*ret, var_b));
        } else {
            panic!("Expected Function type after alpha_convert");
        }
    }

    #[test]
    fn test_alpha_convert_complex_type() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create complex type: Map[a, Array[a]] -> a
        let var_a = manager.type_var(42);
        let array_a = manager.array(var_a);
        let map_a_array_a = manager.map(var_a, array_a);
        let func = manager.function(&[map_a_array_a], var_a);

        // Alpha convert
        let converted = manager.alpha_convert(func);

        // Should be a function type
        if let Type::Function { params, ret } = converted {
            assert_eq!(params.len(), 1);
            // ret and the typevar inside param[0] should be the same pointer
            if let Type::Map(key, val) = params[0] {
                if let Type::Array(elem) = val {
                    // All three should be the same pointer
                    assert!(core::ptr::eq(*key, *elem));
                    assert!(core::ptr::eq(*key, *ret));
                } else {
                    panic!("Expected Array type in Map value");
                }
            } else {
                panic!("Expected Map type in function param");
            }
        } else {
            panic!("Expected Function type after alpha_convert");
        }
    }

    #[test]
    fn test_alpha_convert_no_typevars() {
        let bump = Bump::new();
        let manager = TypeManager::new(&bump);

        // Create function type: Int -> Float
        let int_ty = manager.int();
        let float_ty = manager.float();
        let func = manager.function(&[int_ty], float_ty);

        // Alpha convert
        let converted = manager.alpha_convert(func);

        // Should be unchanged
        assert!(core::ptr::eq(converted, func));
    }
}
