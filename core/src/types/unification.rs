use crate::{Type, types::manager::TypeManager};
use miette::{Report, WrapErr};
use std::collections::HashMap;

pub struct UnificationContext<'a> {
    pub constraints: Vec<(String, String)>,
    pub subst: HashMap<&'a Type<'a>, &'a Type<'a>>,
}

impl<'a> UnificationContext<'a> {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            subst: HashMap::new(),
        }
    }

    pub fn push(&mut self, msg: impl Into<String>, prov: String) {
        self.constraints.push((msg.into(), prov));
    }

    /// Iteratively resolve type variables to their representative type.
    pub fn resolve<'b>(&'b self, mut ty: &'a Type<'a>) -> &'a Type<'a> {
        while let Some(t) = self.subst.get(ty) {
            ty = t;
        }
        ty
    }
}

impl<'a> TypeManager<'a> {
    pub fn unifies_to(
        &self,
        t1: &'a Type<'a>,
        t2: &'a Type<'a>,
        ctx: &mut UnificationContext<'a>,
    ) -> Result<&'a Type<'a>, Report> {
        use crate::types::types::Type;

        let t1 = ctx.resolve(t1);
        let t2 = ctx.resolve(t2);

        if std::ptr::eq(t1, t2) {
            return Ok(t1);
        }

        match (t1, t2) {
            (Type::TypeVar(_), _) => {
                if occurs_in_typevar(t1, t2, ctx) {
                    return Err(Report::msg(format!(
                        "Occurs check failed: {:?} occurs in {:?}",
                        t1, t2
                    )));
                }
                ctx.subst.insert(t1, t2);
                Ok(t2)
            }
            (_, Type::TypeVar(_)) => {
                if occurs_in_typevar(t2, t1, ctx) {
                    return Err(Report::msg(format!(
                        "Occurs check failed: {:?} occurs in {:?}",
                        t2, t1
                    )));
                }
                ctx.subst.insert(t2, t1);
                Ok(t1)
            }
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
            | (Type::Str, Type::Str)
            | (Type::Bytes, Type::Bytes) => Ok(t1),

            (Type::Array(e1), Type::Array(e2)) => {
                let elem = self
                    .unifies_to(e1, e2, ctx)
                    .wrap_err("Array element types must unify")?;
                Ok(self.array(elem))
            }

            (Type::Map(k1, v1), Type::Map(k2, v2)) => {
                let k = self
                    .unifies_to(k1, k2, ctx)
                    .wrap_err("Map key types must unify")?;
                let v = self
                    .unifies_to(v1, v2, ctx)
                    .wrap_err("Map value types must unify")?;
                Ok(self.map(k, v))
            }

            (Type::Record(f1), Type::Record(f2)) => {
                if f1.len() != f2.len() {
                    return Err(Report::msg("Record field count mismatch"));
                }
                let mut fields = Vec::with_capacity(f1.len());
                for ((n1, t1), (n2, t2)) in f1.iter().zip(f2.iter()) {
                    if n1 != n2 {
                        return Err(Report::msg(format!(
                            "Record field name mismatch: {} vs {}",
                            n1, n2
                        )));
                    }
                    let u = self
                        .unifies_to(t1, t2, ctx)
                        .wrap_err(format!("Record field '{}' types must unify", n1))?;
                    fields.push((*n1, u));
                }
                Ok(self.record(fields.as_slice()))
            }

            (
                Type::Function {
                    params: p1,
                    ret: r1,
                },
                Type::Function {
                    params: p2,
                    ret: r2,
                },
            ) => {
                if p1.len() != p2.len() {
                    return Err(Report::msg("Function parameter count mismatch"));
                }
                let mut arg_types = Vec::with_capacity(p1.len());
                for (i, (a, b)) in p1.iter().zip(p2.iter()).enumerate() {
                    let u = self
                        .unifies_to(a, b, ctx)
                        .wrap_err(format!("Function parameter {} types must unify", i))?;
                    arg_types.push(u);
                }
                let r = self
                    .unifies_to(r1, r2, ctx)
                    .wrap_err("Function return types must unify")?;
                Ok(self.function(arg_types.as_slice(), r))
            }

            (Type::Symbol(parts1), Type::Symbol(parts2)) => {
                // TODO: Implement union of symbols
                if parts1 == parts2 {
                    Ok(t1)
                } else {
                    Err(Report::msg(format!(
                        "Symbol mismatch: {:?} vs {:?}",
                        parts1, parts2
                    )))
                }
            }

            _ => Err(Report::msg(format!("Type mismatch: {:?} vs {:?}", t1, t2))),
        }
    }
}

// Helper: occurs check (does type variable tv occur in type t?)
fn occurs_in_typevar<'a>(tv: &'a Type<'a>, t: &'a Type<'a>, ctx: &UnificationContext<'a>) -> bool {
    use crate::types::types::Type;
    let t = ctx.resolve(t);
    if std::ptr::eq(tv, t) {
        return true;
    }
    match t {
        Type::Array(e) => occurs_in_typevar(tv, e, ctx),
        Type::Map(k, v) => occurs_in_typevar(tv, k, ctx) || occurs_in_typevar(tv, v, ctx),
        Type::Record(fields) => fields.iter().any(|(_, t)| occurs_in_typevar(tv, t, ctx)),
        Type::Function { params, ret } => {
            params.iter().any(|p| occurs_in_typevar(tv, p, ctx)) || occurs_in_typevar(tv, ret, ctx)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::manager::TypeManager;

    #[test]
    fn test_unifies_to_with_provenance_chain() {
        let arena = bumpalo::Bump::new();
        let type_manager = TypeManager::new(&arena);

        // Example: unify Map[Int, String] with Map[Int, String] (should succeed)
        let int_ty = type_manager.int();
        let str_ty = type_manager.str();
        let map_ty1 = type_manager.map(int_ty, str_ty);
        let map_ty2 = type_manager.map(int_ty, str_ty);

        let mut ctx = UnificationContext::new();
        let result = type_manager.unifies_to(map_ty1, map_ty2, &mut ctx);
        assert!(result.is_ok(), "Expected types to unify");

        // Example: unify Map[Int, Int] with Map[Int, String] (should fail)
        let map_ty3 = type_manager.map(int_ty, int_ty);

        let mut ctx = UnificationContext::new();

        let result = type_manager.unifies_to(map_ty3, map_ty2, &mut ctx);
        assert!(result.is_err(), "Expected types not to unify");

        if let Err(err) = result {
            // Print error for debugging
            println!("Type error: {err}");
        }
    }
}
