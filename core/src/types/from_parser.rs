//! Conversion from parser TypeExpr to type system Type.

use crate::parser;
use crate::types::{Type, manager::TypeManager};

/// Error returned when converting a TypeExpr to a Type.
#[derive(Debug)]
pub enum TypeConversionError {
    UnknownType {
        name: String,
    },
    WrongParameterCount {
        type_name: String,
        expected: usize,
        got: usize,
    },
}

impl std::fmt::Display for TypeConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeConversionError::UnknownType { name } => {
                write!(f, "Unknown type: {}", name)
            }
            TypeConversionError::WrongParameterCount {
                type_name,
                expected,
                got,
            } => {
                write!(
                    f,
                    "{} type expects {} type parameter{}, got {}",
                    type_name,
                    expected,
                    if *expected == 1 { "" } else { "s" },
                    got
                )
            }
        }
    }
}

impl std::error::Error for TypeConversionError {}

/// Converts a parser TypeExpr into the type system's Type representation.
///
/// Returns a `TypeConversionError` without span information. The caller should
/// annotate this error with the appropriate source span using the Error type.
pub fn type_expr_to_type<'types>(
    type_manager: &'types TypeManager<'types>,
    type_expr: &parser::TypeExpr<'_>,
) -> Result<&'types Type<'types>, TypeConversionError> {
    match type_expr {
        parser::TypeExpr::Path(path) => {
            // Map type path to built-in types
            match *path {
                "Int" => Ok(type_manager.int()),
                "Float" => Ok(type_manager.float()),
                "Bool" => Ok(type_manager.bool()),
                "String" => Ok(type_manager.str()),
                "Bytes" => Ok(type_manager.bytes()),
                _ => Err(TypeConversionError::UnknownType {
                    name: path.to_string(),
                }),
            }
        }
        parser::TypeExpr::Parametrized { path, params } => match *path {
            "Array" => {
                if params.len() != 1 {
                    return Err(TypeConversionError::WrongParameterCount {
                        type_name: "Array".to_string(),
                        expected: 1,
                        got: params.len(),
                    });
                }
                let element_ty = type_expr_to_type(type_manager, &params[0])?;
                Ok(type_manager.array(element_ty))
            }
            "Map" => {
                if params.len() != 2 {
                    return Err(TypeConversionError::WrongParameterCount {
                        type_name: "Map".to_string(),
                        expected: 2,
                        got: params.len(),
                    });
                }
                let key_ty = type_expr_to_type(type_manager, &params[0])?;
                let value_ty = type_expr_to_type(type_manager, &params[1])?;
                Ok(type_manager.map(key_ty, value_ty))
            }
            _ => Err(TypeConversionError::UnknownType {
                name: path.to_string(),
            }),
        },
        parser::TypeExpr::Record(fields) => {
            let field_types: Result<Vec<_>, TypeConversionError> = fields
                .iter()
                .map(|(name, type_expr)| {
                    let field_ty = type_expr_to_type(type_manager, type_expr)?;
                    Ok::<_, TypeConversionError>((*name, field_ty))
                })
                .collect();
            let field_types = field_types?;
            Ok(type_manager.record(&field_types))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::TypeExpr;
    use bumpalo::Bump;

    #[test]
    fn test_simple_types() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        // Test all built-in types
        let test_cases = [
            (TypeExpr::Path("Int"), type_manager.int()),
            (TypeExpr::Path("Float"), type_manager.float()),
            (TypeExpr::Path("Bool"), type_manager.bool()),
            (TypeExpr::Path("String"), type_manager.str()),
            (TypeExpr::Path("Bytes"), type_manager.bytes()),
        ];

        for (type_expr, expected) in test_cases {
            let result = type_expr_to_type(type_manager, &type_expr).unwrap();
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_unknown_type() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        let type_expr = TypeExpr::Path("UnknownType");
        let result = type_expr_to_type(type_manager, &type_expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_array_type() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        let type_expr = TypeExpr::Parametrized {
            path: "Array",
            params: &[TypeExpr::Path("Int")],
        };

        let result = type_expr_to_type(type_manager, &type_expr).unwrap();
        assert_eq!(result, type_manager.array(type_manager.int()));
    }

    #[test]
    fn test_map_type() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        let type_expr = TypeExpr::Parametrized {
            path: "Map",
            params: &[TypeExpr::Path("String"), TypeExpr::Path("Int")],
        };

        let result = type_expr_to_type(type_manager, &type_expr).unwrap();
        assert_eq!(
            result,
            type_manager.map(type_manager.str(), type_manager.int())
        );
    }

    #[test]
    fn test_nested_parametrized_type() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        let type_expr = TypeExpr::Parametrized {
            path: "Array",
            params: &[TypeExpr::Parametrized {
                path: "Array",
                params: &[TypeExpr::Path("Int")],
            }],
        };

        let result = type_expr_to_type(type_manager, &type_expr).unwrap();
        assert_eq!(
            result,
            type_manager.array(type_manager.array(type_manager.int()))
        );
    }

    #[test]
    fn test_record_type() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        let type_expr = TypeExpr::Record(&[
            ("name", TypeExpr::Path("String")),
            ("age", TypeExpr::Path("Int")),
        ]);

        let result = type_expr_to_type(type_manager, &type_expr).unwrap();
        assert_eq!(
            result,
            type_manager.record(&[("name", type_manager.str()), ("age", type_manager.int())])
        );
    }

    #[test]
    fn test_array_wrong_param_count() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        let type_expr = TypeExpr::Parametrized {
            path: "Array",
            params: &[TypeExpr::Path("Int"), TypeExpr::Path("String")],
        };

        let result = type_expr_to_type(type_manager, &type_expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_map_wrong_param_count() {
        let bump = Bump::new();
        let type_manager = TypeManager::new(&bump);

        let type_expr = TypeExpr::Parametrized {
            path: "Map",
            params: &[TypeExpr::Path("Int")],
        };

        let result = type_expr_to_type(type_manager, &type_expr);
        assert!(result.is_err());
    }
}
