use alloc::string::ToString;
use core::marker::PhantomData;

use hashbrown::HashMap;

use crate::{
    String, Vec,
    types::type_traits::{TypeBuilder, TypeKind, TypeView, display_type},
};

/// Types of unification errors.
#[derive(Debug)]
pub enum Error {
    OccursCheckFailed { type_var: String, ty: String },
    FieldCountMismatch { expected: usize, found: usize },
    FieldNameMismatch { expected: String, found: String },
    FunctionParamCountMismatch { expected: usize, found: usize },
    TypeMismatch { left: String, right: String },
}

/// Generic unification for types.
///
/// Performs Hindley-Milner style unification over any `TypeView` representation,
/// building unified types using the provided `TypeBuilder`.
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
pub struct Unification<'a, B: TypeBuilder<'a>> {
    builder: B,
    subst: HashMap<u16, B::Repr>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, B: TypeBuilder<'a>> Unification<'a, B>
where
    B::Repr: TypeView<'a>,
{
    /// Create a new unification instance with the given type constructor.
    pub fn new(builder: B) -> Self {
        Self {
            builder,
            subst: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Resolve a type by following the substitution chain.
    ///
    /// Iteratively resolves type variables until a non-variable type is found
    /// or a variable with no substitution is reached.
    fn resolve(&self, mut ty: B::Repr) -> B::Repr {
        // TODO: add path compression
        loop {
            if let TypeKind::TypeVar(id) = ty.view() {
                if let Some(&t) = self.subst.get(&id) {
                    ty = t;
                    continue;
                }
            }
            break;
        }
        ty
    }

    /// Check if type variable tv occurs in type t.
    ///
    /// Prevents creating infinite types like `a = Array[a]`.
    fn occurs_in(&self, id: u16, t: B::Repr) -> bool {
        use TypeKind::*;

        let resolved = self.resolve(t).view();

        if let TypeVar(resolved_id) = resolved {
            if resolved_id == id {
                return true;
            }
        }

        // Recursively check for occurrence in composite types
        match resolved {
            Array(e) => self.occurs_in(id, e),
            Map(k, v) => self.occurs_in(id, k) || self.occurs_in(id, v),
            Record(mut fields) => fields.any(|(_, field_ty)| self.occurs_in(id, field_ty)),
            Function { mut params, ret } => {
                params.any(|p| self.occurs_in(id, p)) || self.occurs_in(id, ret)
            }
            Symbol(_) | Int | Float | Bool | Str | Bytes | TypeVar(_) => false,
        }
    }

    /// Unify two types, returning the unified type or an error.
    ///
    /// This implements Hindley-Milner unification:
    /// - Type variables unify with any type (occurs check prevents infinite types)
    /// - Primitives unify only with identical primitives
    /// - Composite types unify recursively
    ///
    /// The substitution map is updated with any new type variable bindings.
    pub fn unifies_to(&mut self, t1: B::Repr, t2: B::Repr) -> Result<B::Repr, Error> {
        let t1 = self.resolve(t1);
        let t2 = self.resolve(t2);

        // Fast path: equality (works via TypeView: Eq bound)
        if t1 == t2 {
            return Ok(t1);
        }

        use Error::*;
        use TypeKind::*;

        match (t1.view(), t2.view()) {
            // Type variable cases - bind variable to the other type
            (TypeVar(id), _) => {
                if self.occurs_in(id, t2) {
                    return Err(OccursCheckFailed {
                        type_var: display_type(t1),
                        ty: display_type(t2),
                    });
                }
                self.subst.insert(id, t2);
                Ok(t2)
            }
            (_, TypeVar(id)) => {
                if self.occurs_in(id, t1) {
                    return Err(OccursCheckFailed {
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
                let elem = self.unifies_to(e1, e2)?;
                Ok(self.builder.array(elem))
            }

            // Map - unify key and value types
            (Map(k1, v1), Map(k2, v2)) => {
                let k = self.unifies_to(k1, k2)?;
                let v = self.unifies_to(v1, v2)?;
                Ok(self.builder.map(k, v))
            }

            // Record - unify field by field
            (Record(fields1), Record(fields2)) => {
                // Collect fields into vectors to check length
                let f1: Vec<_> = fields1.collect();
                let f2: Vec<_> = fields2.collect();

                if f1.len() != f2.len() {
                    return Err(FieldCountMismatch {
                        expected: f1.len(),
                        found: f2.len(),
                    });
                }

                let mut unified_fields = Vec::with_capacity(f1.len());
                for ((n1, t1), (n2, t2)) in f1.iter().zip(f2.iter()) {
                    if n1 != n2 {
                        return Err(FieldNameMismatch {
                            expected: n1.to_string(),
                            found: n2.to_string(),
                        });
                    }
                    let u = self.unifies_to(*t1, *t2)?;
                    unified_fields.push((*n1, u));
                }
                Ok(self.builder.record(unified_fields.iter().copied()))
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
                    return Err(FunctionParamCountMismatch {
                        expected: params1.len(),
                        found: params2.len(),
                    });
                }

                let mut unified_params = Vec::with_capacity(params1.len());
                for (a, b) in params1.iter().zip(params2.iter()) {
                    let u = self.unifies_to(*a, *b)?;
                    unified_params.push(u);
                }

                let r = self.unifies_to(r1, r2)?;
                Ok(self.builder.function(unified_params.iter().copied(), r))
            }

            // Symbol - must have identical parts
            (Symbol(parts1), Symbol(parts2)) => {
                let p1: Vec<_> = parts1.collect();
                let p2: Vec<_> = parts2.collect();
                if p1 == p2 {
                    Ok(t1)
                } else {
                    Err(TypeMismatch {
                        left: display_type(t1),
                        right: display_type(t2),
                    })
                }
            }

            // Mismatch - types don't unify
            _ => Err(TypeMismatch {
                left: display_type(t1),
                right: display_type(t2),
            }),
        }
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
