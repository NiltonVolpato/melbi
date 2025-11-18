//! Implementation of the `#[melbi_fn]` attribute macro

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, Expr, FnArg, GenericArgument, ItemFn, Lit, Meta, Pat, PatType, ReturnType,
    Type,
};

pub fn melbi_fn_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    // Parse the attribute to extract the Melbi function name
    let melbi_name = match parse_attribute(attr) {
        Ok(name) => name,
        Err(err) => return err.to_compile_error().into(),
    };

    // Parse the function signature
    let sig_info = match parse_function_signature(&input_fn) {
        Ok(info) => info,
        Err(err) => return err.to_compile_error().into(),
    };

    // Generate all the code
    match generate_code(&melbi_name, &sig_info, &input_fn) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Information extracted from the function signature
struct SignatureInfo {
    /// Function name
    fn_name: syn::Ident,
    /// Parameter names and types (excluding _arena and _type_mgr)
    params: Vec<(syn::Ident, Box<Type>)>,
    /// Return type
    return_type: Box<Type>,
}

/// Parse the function signature and extract parameter and return types
fn parse_function_signature(func: &ItemFn) -> syn::Result<SignatureInfo> {
    let fn_name = func.sig.ident.clone();
    let mut params = Vec::new();

    // Parse parameters, skipping _arena and _type_mgr
    for input in &func.sig.inputs {
        if let FnArg::Typed(PatType { pat, ty, .. }) = input {
            if let Pat::Ident(pat_ident) = &**pat {
                let param_name = &pat_ident.ident;

                // Skip _arena and _type_mgr parameters
                if param_name == "_arena" || param_name == "_type_mgr" {
                    continue;
                }

                params.push((param_name.clone(), ty.clone()));
            }
        }
    }

    // Extract return type
    let return_type = match &func.sig.output {
        ReturnType::Default => {
            return Err(syn::Error::new_spanned(
                &func.sig,
                "melbi_fn functions must have an explicit return type",
            ));
        }
        ReturnType::Type(_, ty) => ty.clone(),
    };

    Ok(SignatureInfo {
        fn_name,
        params,
        return_type,
    })
}

/// Parse the attribute to extract the name parameter
fn parse_attribute(attr: TokenStream) -> syn::Result<String> {
    // When used as #[melbi_fn(name = "FunctionName")], attr contains just: name = "FunctionName"
    // Parse it as a NameValue meta
    let meta = syn::parse::<Meta>(attr)?;

    if let Meta::NameValue(nv) = meta {
        if nv.path.is_ident("name") {
            if let Expr::Lit(expr_lit) = &nv.value {
                if let Lit::Str(lit) = &expr_lit.lit {
                    return Ok(lit.value());
                }
            }
            return Err(syn::Error::new_spanned(
                &nv.value,
                "name attribute must be a string literal",
            ));
        }
        return Err(syn::Error::new_spanned(
            nv.path,
            "expected 'name' attribute",
        ));
    }

    Err(syn::Error::new_spanned(
        meta,
        "expected attribute format: #[melbi_fn(name = \"FunctionName\")]",
    ))
}

/// Generate all the code: impl function, struct, and trait implementations
fn generate_code(
    melbi_name: &str,
    sig_info: &SignatureInfo,
    input_fn: &ItemFn,
) -> syn::Result<TokenStream2> {
    let struct_name = syn::Ident::new(melbi_name, proc_macro2::Span::call_site());

    // Extract components
    let generics = input_fn.sig.generics.clone();
    let param_names: Vec<_> = sig_info.params.iter().map(|(name, _)| name).collect();
    let param_types: Vec<_> = sig_info.params.iter().map(|(_, ty)| ty).collect();
    let return_type = &sig_info.return_type;

    // Copy input function as is.
    let impl_function = quote! {
        #input_fn
    };

    // Generate struct definition (only store the function type)
    let struct_def = quote! {
        pub struct #struct_name #generics {
            fn_type: &'types ::melbi_core::types::Type<'types>,
        }
    };

    // Generate constructor
    let constructor = generate_constructor(&struct_name, &param_types, return_type)?;

    // Generate Function trait impl
    let function_impl = generate_function_impl(
        &struct_name,
        &sig_info.fn_name,
        melbi_name,
        &param_names,
        &param_types,
        return_type,
    )?;

    // Generate AnnotatedFunction trait impl with inlined metadata
    // file!(), line!(), column!() will expand at the call site
    let annotated_impl = quote! {
        impl<'types> ::melbi_core::values::function::AnnotatedFunction<'types> for #struct_name<'types> {
            fn name(&self) -> &str {
                #melbi_name
            }

            fn location(&self) -> (&str, &str, &str, u32, u32) {
                (env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"), file!(), line!(), column!())
            }

            fn doc(&self) -> Option<&str> {
                None
            }
        }
    };

    Ok(quote! {
        #impl_function

        #struct_def

        #constructor

        #function_impl

        #annotated_impl
    })
}

/// Generate the constructor method
fn generate_constructor(
    struct_name: &syn::Ident,
    param_types: &[&Box<Type>],
    return_type: &Type,
) -> syn::Result<TokenStream2> {
    Ok(quote! {
        impl<'types> #struct_name<'types> {
            pub fn new(type_mgr: &'types ::melbi_core::types::manager::TypeManager<'types>) -> Self {
                use ::melbi_core::values::typed::Bridge;

                let fn_type = type_mgr.function(
                    &[#( <#param_types as Bridge>::type_from(type_mgr) ),*],
                    <#return_type as Bridge>::type_from(type_mgr),
                );

                Self {
                    fn_type,
                }
            }
        }
    })
}

/// Generate the Function trait implementation
fn generate_function_impl(
    struct_name: &syn::Ident,
    impl_fn_name: &syn::Ident,
    melbi_name: &str,
    param_names: &[&syn::Ident],
    param_types: &[&Box<Type>],
    return_type: &Type,
) -> syn::Result<TokenStream2> {
    let arity = param_names.len();

    // Generate parameter extraction code
    let param_extractions: Vec<_> = param_names.iter().zip(param_types.iter()).enumerate().map(|(i, (name, ty))| {
        quote! {
            let #name = unsafe { <#ty as ::melbi_core::values::typed::RawConvertible>::from_raw_value(args[#i].raw()) };
        }
    }).collect();

    Ok(quote! {
        impl<'types, 'arena> ::melbi_core::values::function::Function<'types, 'arena> for #struct_name<'types> {
            fn ty(&self) -> &'types ::melbi_core::types::Type<'types> {
                self.fn_type
            }

            unsafe fn call_unchecked(
                &self,
                arena: &'arena ::bumpalo::Bump,
                type_mgr: &'types ::melbi_core::types::manager::TypeManager<'types>,
                args: &[::melbi_core::values::dynamic::Value<'types, 'arena>],
            ) -> Result<::melbi_core::values::dynamic::Value<'types, 'arena>, ::melbi_core::evaluator::ExecutionError> {
                use ::melbi_core::values::typed::Bridge;

                debug_assert_eq!(
                    args.len(),
                    #arity,
                    "{} expects {} argument(s), got {}",
                    #melbi_name,
                    #arity,
                    args.len()
                );

                #( #param_extractions )*

                let result = #impl_fn_name(arena, type_mgr, #( #param_names ),*);

                let raw = <#return_type as ::melbi_core::values::typed::RawConvertible>::to_raw_value(arena, result);
                let ty = <#return_type as ::melbi_core::values::typed::Bridge>::type_from(type_mgr);

                // SAFETY: We just created the raw value from the correct type, so it matches
                Ok(unsafe {
                    ::melbi_core::values::dynamic::Value::from_raw(ty, raw)
                })
            }
        }
    })
}
