#![feature(proc_macro_span)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, spanned::Spanned, ItemImpl, ItemTrait, Path};

macro_rules! unexpected_token {
    ($span:expr, $expected:expr ) => {{
        let span = $span.unwrap();
        panic!(
            "Expected {}, found {:?} at {}:{}:{}",
            $expected,
            span.source_text().unwrap_or_default(),
            span.source_file().path().to_str().unwrap(),
            span.line(),
            span.column(),
        );
    }};
}

fn is_primitive(value: &str) -> bool {
    let primitive_types = [
        "bool", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "f32", "f64",
    ];

    primitive_types.contains(&value)
}

fn parse_result_type(
    ty: &syn::Type,
    primitive_only: bool,
    allow_reference: bool,
) -> Option<String> {
    _parse_type(ty, primitive_only, allow_reference, true)
}

fn parse_type(ty: &syn::Type, primitive_only: bool, allow_reference: bool) -> String {
    match _parse_type(ty, primitive_only, allow_reference, false) {
        Some(v) => v,
        None => unexpected_token!(ty.span(), "type"),
    }
}

fn _parse_type(
    ty: &syn::Type,
    primitive_only: bool,
    allow_reference: bool,
    allow_nil: bool,
) -> Option<String> {
    let path = match ty {
        syn::Type::Path(path) => {
            let path = Some(reduce_path(&path.path));

            if primitive_only {
                if !path
                    .as_ref()
                    .map(|v| is_primitive(&v.as_str()))
                    .unwrap_or_default()
                {
                    unexpected_token!(ty.span(), "primitive")
                }
            }

            path
        }
        syn::Type::Reference(reference) => {
            if allow_reference {
                _parse_type(reference.elem.as_ref(), false, allow_reference, allow_nil).map(|v| {
                    if reference.mutability.is_some() {
                        format!("&mut {}", v)
                    } else {
                        format!("&{}", v)
                    }
                })
            } else {
                unexpected_token!(ty.span(), "type")
            }
        }
        syn::Type::Ptr(ptr) => {
            if allow_reference {
                _parse_type(ptr.elem.as_ref(), false, allow_reference, allow_nil).map(|v| {
                    if ptr.mutability.is_some() {
                        format!("WasmPtrMut<{}>", v)
                    } else {
                        format!("WasmPtr<{}>", v)
                    }
                })
            } else {
                unexpected_token!(ty.span(), "type")
            }
        }
        syn::Type::Tuple(tuple) => {
            if allow_nil && tuple.elems.is_empty() {
                None
            } else {
                unexpected_token!(ty.span(), "type")
            }
        }
        _ => {
            todo!("UNEXPECTED {:?}", ty);
            // unexpected_token!(pat.ty.span(), "type")
        }
    };

    path
}

fn reduce_path(path: &Path) -> String {
    let path_sep = "::";
    let output = format!(
        "{}{}",
        if path.leading_colon.is_some() {
            path_sep
        } else {
            ""
        },
        path.segments
            .iter()
            .map(|v| {
                let ident = v.ident.to_string();
                match &v.arguments {
                    syn::PathArguments::None => {}
                    syn::PathArguments::AngleBracketed(v) => {
                        unexpected_token!(v.span(), "simple type")
                    }
                    syn::PathArguments::Parenthesized(v) => {
                        unexpected_token!(v.span(), "simple type")
                    }
                }
                ident
            })
            .collect::<Vec<_>>()
            .join(path_sep),
    );
    output
}

fn type_to_signature(ident: &str) -> &str {
    match ident {
        "void" => "v",
        "bool" | "i8" | "u8" | "i16" | "u16" | "i32" | "u32" => "i",
        "u64" | "i64" => "l",
        "f32" => "f",
        "f64" => "d",
        _ => {
            if ident.starts_with("&") || is_wasm_ptr(ident) {
                "i"
            } else {
                "_"
            }
        }
    }
}

fn is_wasm_ptr(ident: &str) -> bool {
    ident.starts_with("WasmPtr<") || ident.starts_with("WasmPtrMut<")
}

/// A Macro to automatically generate WebAssembly import resolver from `impl`
///
/// - parameter: module name (default name is "env")
#[proc_macro_attribute]
pub fn wasm_env(attr: TokenStream, input: TokenStream) -> TokenStream {
    // println!("INPUT: {:?}", input.to_string());

    let mut mod_name = None;
    if let Some(ident) = attr.into_iter().next() {
        if let proc_macro::TokenTree::Ident(ident) = ident {
            mod_name = Some(ident.to_string())
        }
    }
    let mod_name = mod_name.unwrap_or("env".to_string());

    let impls = parse_macro_input!(input as ItemImpl);

    let class_name = parse_type(impls.self_ty.as_ref(), false, false);

    let mut output = Vec::new();
    let mut resolve_list = Vec::new();

    output.push(format!("impl {class_name} {{"));
    for item in impls.items.iter() {
        let func = match item {
            syn::ImplItem::Fn(v) => v,
            _ => continue,
        };
        match func.vis {
            syn::Visibility::Public(_) => {}
            _ => continue,
        }

        let func_name = func.sig.ident.to_string();

        let mut params = Vec::new();
        for input in &func.sig.inputs {
            match input {
                syn::FnArg::Receiver(recv) => unexpected_token!(recv.span(), "ident"),
                syn::FnArg::Typed(pat) => {
                    let param_name = match pat.pat.as_ref() {
                        syn::Pat::Ident(ident) => ident.ident.to_string(),
                        _ => unexpected_token!(pat.pat.span(), "ident"),
                    };
                    let param_type = parse_type(pat.ty.as_ref(), true, true);
                    params.push((param_name, param_type));
                }
            }
        }

        let result_type = match &func.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => parse_result_type(ty.as_ref(), false, false),
        };

        let var_instance = "instance";
        let type_instance = "&WasmInstance";
        let signature = [result_type.clone().unwrap_or("void".to_string())]
            .iter()
            .chain(params.iter().map(|v| &v.1).filter(|v| *v != type_instance))
            .map(|v| type_to_signature(v))
            .collect::<String>();

        let bridge_fn_name = format!("__env_{func_name}");

        resolve_list.push((func_name.clone(), bridge_fn_name.clone(), signature));

        let mut func_body = Vec::new();
        let mut call_params = Vec::new();
        func_body.push(format!(
            "fn {bridge_fn_name}({var_instance}: {type_instance}, mut args: WasmArgs) -> WasmDynResult {{"
        ));

        for param in params.iter() {
            if param.1 == type_instance {
                call_params.push(var_instance.to_string());
            } else if is_wasm_ptr(param.1.as_str()) {
                call_params.push(param.0.clone());
                func_body.push(format!(
                    "let Some({}) = args.next::<{}>() else {{
    return WasmDynResult::Err(Box::new(WasmRuntimeError::from(WasmRuntimeErrorKind::InvalidParameter)))
}};",
                    param.0, param.1
                ));
            } else {
                call_params.push(param.0.clone());
                func_body.push(format!(
                    "let Some({}) = args.next::<{}>() else {{
    return WasmDynResult::Err(Box::new(WasmRuntimeError::from(WasmRuntimeErrorKind::InvalidParameter)))
}};",
                    param.0, param.1
                ));
            }
        }

        match result_type.as_ref() {
            None => {
                func_body.push(format!(
                    "Self::{}({}); WasmDynResult::Val(None)",
                    func_name,
                    call_params.join(", "),
                ));
            }
            Some(_) => {
                func_body.push(format!(
                    "let r = Self::{}({}); WasmDynResult::Val(Some(r.into()))",
                    func_name,
                    call_params.join(", "),
                ));
            }
        }
        func_body.push("}".to_string());

        output.push(func_body.join("\n"));
    }
    output.push("}".to_string());

    {
        let mut func_body = Vec::new();
        func_body.push(format!("impl WasmEnv for {class_name} {{"));
        func_body.push(format!(
            "fn imports_resolver(&self, mod_name: &str, name: &str, type_: &WasmType) -> WasmImportResult {{",
        ));
        func_body.push(format!(
            "if mod_name != {mod_name:?} {{ return WasmImportResult::NoModule; }}"
        ));
        func_body.push("match (name, type_.signature().as_str()) {".to_string());
        for item in resolve_list {
            func_body.push(format!(
                "({:?}, {:?}) => WasmImportResult::Ok(Self::{}),",
                item.0, item.2, item.1
            ));
        }
        func_body.push("_ => WasmImportResult::NoMethod } } }".to_string());

        output.push(func_body.join("\n"));
    }

    // println!("{}", output.join("\n"));

    let mut output: TokenStream = output.join("\n").parse().unwrap();
    output.extend(TokenStream::from(impls.into_token_stream()));

    output
}

/// A Macro to automatically generate WebAssembly exports from `trait`
#[proc_macro_attribute]
pub fn wasm_exports(_attr: TokenStream, input: TokenStream) -> TokenStream {
    // println!("INPUT: {:?}", input.to_string());

    let traits = parse_macro_input!(input as ItemTrait);

    let class_name = traits.ident.to_string();

    let mut output_trait = Vec::new();
    let mut output_impl = Vec::new();
    output_trait.push(format!("trait {class_name} {{"));
    output_impl.push(format!("impl {class_name} for WasmExports<'_> {{"));

    for item in &traits.items {
        let func = match item {
            syn::TraitItem::Fn(func_item) => func_item,
            _ => unexpected_token!(item.span(), "fn"),
        };

        let func_name = func.sig.ident.to_string();

        let mut params = Vec::new();
        for input in &func.sig.inputs {
            match input {
                syn::FnArg::Receiver(recv) => unexpected_token!(recv.span(), "ident"),
                syn::FnArg::Typed(pat) => {
                    let param_name = match pat.pat.as_ref() {
                        syn::Pat::Ident(ident) => ident.ident.to_string(),
                        _ => unexpected_token!(pat.pat.span(), "ident"),
                    };
                    let param_type = parse_type(pat.ty.as_ref(), true, true);
                    params.push((param_name, param_type));
                }
            }
        }

        let result_type = match &func.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => parse_result_type(ty.as_ref(), false, false),
        };

        let mut push_args = Vec::new();
        let mut func_sig = Vec::new();
        for param in params.iter() {
            push_args.push(format!("WasmValue::from({})", param.0));
            func_sig.push(format!("{}: {}", param.0, param.1));
        }

        let output_sig = format!(
            "fn {func_name}(&self, {}) -> WasmResult<{}>",
            func_sig.join(", "),
            result_type.clone().unwrap_or("()".to_string()),
        );
        output_trait.push(format!("{output_sig};"));

        output_impl.push(format!("{output_sig} {{"));
        output_impl.push(format!("let args = [{}];", push_args.join(",")));
        output_impl.push(format!(
            "self.instance().exports().get({:?})
                .ok_or(WasmRuntimeErrorKind::NoMethod.into())
                .and_then(|v| v.invoke(&args))",
            func_name
        ));
        match result_type {
            Some(ref result_type) => output_impl.push(format!(
                ".and_then(|v| match v {{
                    Some(v) => v.get_{result_type}().map_err(|e| e.into()),
                    None => Err(WasmRuntimeErrorKind::TypeMismatch.into()),
                }})"
            )),
            None => output_impl.push(
                ".and_then(|v| match v {
                    Some(_) => Err(WasmRuntimeErrorKind::TypeMismatch.into()),
                    None => Ok(())
                })"
                .to_string(),
            ),
        }
        output_impl.push("}".to_string());
    }
    output_trait.push("}".to_string());
    output_impl.push("}".to_string());

    let output = format!("{}\n{}", output_trait.join("\n"), output_impl.join("\n"),);

    // println!("{}", output);

    output.parse().unwrap()
}
