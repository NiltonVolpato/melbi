use alloc::{string::ToString, sync::Arc};
use core::marker::PhantomData;

use hashbrown::HashMap;

use crate::{
    String, Vec, format,
    types::type_traits::{TypeConstructor, TypeKind, TypeView, display_type},
};

/// Unification error with context for provenance tracking.
#[derive(Debug)]
pub struct Error {
    pub kind: Arc<ErrorKind>,
    pub context: Vec<String>,
}

/// Types of unification errors.
#[derive(Debug)]
pub enum ErrorKind {
    OccursCheckFailed { type_var: String, ty: String },
    FieldCountMismatch { expected: usize, found: usize },
    FieldNameMismatch { expected: String, found: String },
    FunctionParamCountMismatch { expected: usize, found: usize },
    TypeMismatch { left: String, right: String },
}

/// Generic unification for types.
///
/// Performs Hindley-Milner style unification over any `TypeView` representation,
/// building unified types using the provided `TypeConstructor`.
///
/// # Example
///
/// ```ignore
/// use crate::types::{TypeManager, unification::Unification};
/// use bumpalo::Bump;
///
/// let bump = Bump::new();
/// let manager = TypeManager::new(&bump);
/// let mut unify = Unification::new(manager);
///
/// let t1 = manager.array(manager.type_var(0));
/// let t2 = manager.array(manager.int());
///
/// let result = unify.unifies_to(t1, t2)?;
/// // Result: Array[Int], with substitution {0 -> Int}
/// ```
pub struct Unification<'a, C: TypeConstructor<'a>> {
    constructor: C,
    constraints: Vec<String>,
    subst: HashMap<u16, C::Repr>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, C: TypeConstructor<'a>> Unification<'a, C>
where
    C::Repr: TypeView<'a>,
{
    /// Create a new unification instance with the given type constructor.
    pub fn new(constructor: C) -> Self {
        Self {
            constructor,
            constraints: Vec::new(),
            subst: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Add a constraint for error reporting.
    pub fn push_constraint(&mut self, msg: impl Into<String>) {
        self.constraints.push(msg.into());
    }

    /// Get the current substitution map.
    pub fn substitutions(&self) -> &HashMap<u16, C::Repr> {
        &self.subst
    }

    /// Resolve a type by following the substitution chain.
    ///
    /// Iteratively resolves type variables until a non-variable type is found
    /// or a variable with no substitution is reached.
    fn resolve(&self, mut ty: C::Repr) -> C::Repr {
        loop {
            match ty.view() {
                TypeKind::TypeVar(id) => {
                    if let Some(&t) = self.subst.get(&id) {
                        ty = t;
                        continue;
                    }
                }
                _ => break,
            }
        }
        ty
    }

    /// Create an error with the current context.
    fn error(&self, kind: ErrorKind) -> Result<C::Repr, Error> {
        Err(Error {
            kind: Arc::new(kind),
            context: self.constraints.clone(),
        })
    }

    /// Unify two types, returning the unified type or an error.
    ///
    /// This implements Hindley-Milner unification:
    /// - Type variables unify with any type (occurs check prevents infinite types)
    /// - Primitives unify only with identical primitives
    /// - Composite types unify recursively
    ///
    /// The substitution map is updated with any new type variable bindings.
    pub fn unifies_to(&mut self, t1: C::Repr, t2: C::Repr) -> Result<C::Repr, Error> {
        let t1 = self.resolve(t1);
        let t2 = self.resolve(t2);

        // Fast path: equality (works via TypeView: Eq bound)
        if t1 == t2 {
            return Ok(t1);
        }

        use TypeKind::*;

        match (t1.view(), t2.view()) {
            // Type variable cases - bind variable to the other type
            (TypeVar(id), _) => {
                if occurs_in(t1, t2, &self.subst) {
                    return self.error(ErrorKind::OccursCheckFailed {
                        type_var: display_type(t1),
                        ty: display_type(t2),
                    });
                }
                self.subst.insert(id, t2);
                Ok(t2)
            }
            (_, TypeVar(id)) => {
                if occurs_in(t2, t1, &self.subst) {
                    return self.error(ErrorKind::OccursCheckFailed {
                        type_var: display_type(t2),
                        ty: display_type(t1),
                    });
                }
                self.subst.insert(id, t1);
                Ok(t1)
            }

            // Primitives - must match exactly
            (Int, Int) | (Float, Float) | (Bool, Bool) | (Str, Str) | (Bytes, Bytes) => Ok(t1),

            // Array - unify element types
            (Array(e1), Array(e2)) => {
                let elem = self.unifies_to(e1, e2).map_err(|mut e| {
                    e.context.push("Array element types must unify".into());
                    e
                })?;
                Ok(self.constructor.array(elem))
            }

            // Map - unify key and value types
            (Map(k1, v1), Map(k2, v2)) => {
                let k = self.unifies_to(k1, k2).map_err(|mut e| {
                    e.context.push("Map key types must unify".into());
                    e
                })?;
                let v = self.unifies_to(v1, v2).map_err(|mut e| {
                    e.context.push("Map value types must unify".into());
                    e
                })?;
                Ok(self.constructor.map(k, v))
            }

            // Record - unify field by field
            (Record(fields1), Record(fields2)) => {
                // Collect fields into vectors to check length
                let f1: Vec<_> = fields1.collect();
                let f2: Vec<_> = fields2.collect();

                if f1.len() != f2.len() {
                    return self.error(ErrorKind::FieldCountMismatch {
                        expected: f1.len(),
                        found: f2.len(),
                    });
                }

                let mut unified_fields = Vec::with_capacity(f1.len());
                for ((n1, t1), (n2, t2)) in f1.iter().zip(f2.iter()) {
                    if n1 != n2 {
                        return self.error(ErrorKind::FieldNameMismatch {
                            expected: n1.to_string(),
                            found: n2.to_string(),
                        });
                    }
                    let u = self.unifies_to(*t1, *t2).map_err(|mut e| {
                        e.context
                            .push(format!("Record field '{}' types must unify", n1));
                        e
                    })?;
                    unified_fields.push((*n1, u));
                }
                Ok(self.constructor.record(unified_fields.iter().copied()))
            }

            // Function - unify parameters and return type
            (
                Function {
                    params: p1,
                    ret: r1,
                },
                Function {
                    params: p2,
                    ret: r2,
                },
            ) => {
                // Collect params to check length
                let params1: Vec<_> = p1.collect();
                let params2: Vec<_> = p2.collect();

                if params1.len() != params2.len() {
                    return self.error(ErrorKind::FunctionParamCountMismatch {
                        expected: params1.len(),
                        found: params2.len(),
                    });
                }

                let mut unified_params = Vec::with_capacity(params1.len());
                for (i, (a, b)) in params1.iter().zip(params2.iter()).enumerate() {
                    let u = self.unifies_to(*a, *b).map_err(|mut e| {
                        e.context
                            .push(format!("Function parameter {} types must unify", i));
                        e
                    })?;
                    unified_params.push(u);
                }

                let r = self.unifies_to(r1, r2).map_err(|mut e| {
                    e.context.push("Function return types must unify".into());
                    e
                })?;
                Ok(self.constructor.function(unified_params.iter().copied(), r))
            }

            // Symbol - must have identical parts
            (Symbol(parts1), Symbol(parts2)) => {
                let p1: Vec<_> = parts1.collect();
                let p2: Vec<_> = parts2.collect();
                if p1 == p2 {
                    Ok(t1)
                } else {
                    self.error(ErrorKind::TypeMismatch {
                        left: display_type(t1),
                        right: display_type(t2),
                    })
                }
            }

            // Mismatch - types don't unify
            _ => self.error(ErrorKind::TypeMismatch {
                left: display_type(t1),
                right: display_type(t2),
            }),
        }
    }
}

/// Helper: occurs check (does type variable tv occur in type t?)
///
/// Prevents creating infinite types like `a = Array[a]`.
fn occurs_in<'a, T>(tv: T, t: T, subst: &HashMap<u16, T>) -> bool
where
    T: TypeView<'a> + Copy,
{
    // Resolve t through the substitution chain
    let mut resolved = t;
    loop {
        match resolved.view() {
            TypeKind::TypeVar(id) => {
                if let Some(&sub) = subst.get(&id) {
                    resolved = sub;
                    continue;
                }
                break;
            }
            _ => break,
        }
    }

    // Fast path: equality (works via TypeView: Eq)
    if tv == resolved {
        return true;
    }

    use TypeKind::*;

    // Recursively check for occurrence in composite types
    match resolved.view() {
        Array(e) => occurs_in(tv, e, subst),
        Map(k, v) => occurs_in(tv, k, subst) || occurs_in(tv, v, subst),
        Record(mut fields) => fields.any(|(_, field_ty)| occurs_in(tv, field_ty, subst)),
        Function { mut params, ret } => {
            params.any(|p| occurs_in(tv, p, subst)) || occurs_in(tv, ret, subst)
        }
        Symbol(_) | Int | Float | Bool | Str | Bytes | TypeVar(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manager::TypeManager;

    #[test]
    fn test_unifies_to_success() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let mut unify = Unification::new(type_manager);

        // Example: unify Map[Int, String] with Map[Int, String] (should succeed)
        let int_ty = type_manager.int();
        let str_ty = type_manager.str();
        let map_ty1 = type_manager.map(int_ty, str_ty);
        let map_ty2 = type_manager.map(int_ty, str_ty);

        let result = unify.unifies_to(map_ty1, map_ty2);
        assert!(result.is_ok(), "Expected types to unify");
    }

    #[test]
    fn test_unifies_to_failure() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);
        let mut unify = Unification::new(type_manager);

        // Example: unify Map[Int, Int] with Map[Int, String] (should fail)
        let int_ty = type_manager.int();
        let str_ty = type_manager.str();
        let map_ty1 = type_manager.map(int_ty, int_ty);
        let map_ty2 = type_manager.map(int_ty, str_ty);

        let result = unify.unifies_to(map_ty1, map_ty2);
        assert!(result.is_err(), "Expected types not to unify");

        if let Err(err) = result {
            // Print error for debugging
            println!("Type error: {err:#?}");
        }
    }
}
