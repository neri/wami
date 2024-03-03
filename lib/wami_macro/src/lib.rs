#![feature(proc_macro_span)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, spanned::Spanned, ItemImpl, Path};

/// A Macro to automatically generate WebAssembly import resolver from `impl`
///
/// - parameter: module name (default is "env")
#[proc_macro_attribute]
pub fn wasm_env(attr: TokenStream, input: TokenStream) -> TokenStream {
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

    fn reduce_path<F>(path: &Path, filter: F) -> String
    where
        F: FnOnce(&str) -> Option<String>,
    {
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
                            unexpected_token!(v.span(), "none")
                        }
                        syn::PathArguments::Parenthesized(v) => unexpected_token!(v.span(), "none"),
                    }
                    ident
                })
                .collect::<Vec<_>>()
                .join(path_sep),
        );

        if let Some(expected) = filter(output.as_str()) {
            unexpected_token!(path.span(), expected)
        }
        output
    }

    fn allow_all(_v: &str) -> Option<String> {
        None
    }

    fn primitive_filter(v: &str) -> Option<String> {
        let safe_types = [
            "bool", "isize", "usize", "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "f32",
            "f64",
        ];

        (!safe_types.contains(&v)).then(|| safe_types.join(", "))
    }

    fn type_identifier(ident: &str) -> char {
        match ident {
            "void" => 'v',
            "bool" | "i8" | "u8" | "i16" | "u16" | "isize" | "usize" | "i32" | "u32" => 'i',
            "u64" | "i64" => 'l',
            "f32" => 'f',
            "f64" => 'd',
            _ => '_',
        }
    }

    let mut mod_name = None;
    if let Some(ident) = attr.into_iter().next() {
        if let proc_macro::TokenTree::Ident(ident) = ident {
            mod_name = Some(ident.to_string())
        }
    }
    let mod_name = mod_name.unwrap_or("env".to_string());

    let impls = parse_macro_input!(input as ItemImpl);

    let class_name = match impls.self_ty.as_ref() {
        syn::Type::Path(path) => reduce_path(&path.path, allow_all),
        _ => unexpected_token!(impls.self_ty.span(), "type path"),
    };

    let mut output = Vec::new();
    let mut resolve_list = Vec::new();

    output.push(format!("impl {class_name} {{"));
    for item in impls.items.iter() {
        let func = match item {
            syn::ImplItem::Fn(v) => v,
            _ => unexpected_token!(item.span(), "function"),
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
                    let param_type = match pat.ty.as_ref() {
                        syn::Type::Path(path) => reduce_path(&path.path, primitive_filter),
                        _ => unexpected_token!(pat.ty.span(), "type path"),
                    };
                    params.push((param_name, param_type));
                }
            }
        }

        let result_type = match &func.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => match ty.as_ref() {
                syn::Type::Path(path) => {
                    path.path.segments.len();

                    let _ = 0;
                    let _ = 1;
                    Some(reduce_path(&path.path, primitive_filter))
                }
                syn::Type::Tuple(tuple) => {
                    if tuple.elems.is_empty() {
                        None
                    } else {
                        unexpected_token!(ty.span(), "type path")
                    }
                }
                _ => {
                    todo!("UNEXPECTED {:?}", ty);
                    // unexpected_token!(ty.span(), "type path")
                }
            },
        };

        let signature = params
            .iter()
            .map(|v| &v.1)
            .chain([result_type.unwrap_or("void".to_string())].iter())
            .map(|v| type_identifier(v))
            .collect::<String>();

        let bridge_fn_name = format!("__env_{func_name}");

        resolve_list.push((func_name.clone(), bridge_fn_name.clone(), signature));

        let mut func_body = Vec::new();
        let mut call_params = Vec::new();
        func_body.push(format!(
            "fn {}(_: &WasmInstance, mut args: WasmArgs) -> WasmResult {{",
            bridge_fn_name
        ));
        for param in params.iter() {
            call_params.push(param.0.clone());
            func_body.push(format!(
                "let Some({}) = args.next::<{}>() else {{
    return WasmResult::Err(Box::new(WasmRuntimeError::from(WasmRuntimeErrorKind::InvalidParameter)))
}};",
                param.0, param.1
            ));
        }
        func_body.push(format!(
            "Self::{}({}).into()",
            func_name,
            call_params.join(", "),
        ));
        func_body.push("}".to_string());

        output.push(func_body.join("\n"));
    }
    output.push("}".to_string());

    {
        let mut func_body = Vec::new();
        func_body.push(format!("impl WasmEnv for {class_name} {{"));
        func_body.push(format!(
            "fn imports_resolver(&self, mod_name: &str, name: &str, type_: &WasmType) -> ImportResult {{",
        ));
        func_body.push(format!(
            "if mod_name != {mod_name:?} {{ return ImportResult::NoModule; }}"
        ));
        func_body.push("match (name, type_.signature().as_str()) {".to_string());
        for item in resolve_list {
            func_body.push(format!(
                "({:?}, {:?}) => ImportResult::Ok(Self::{}),",
                item.0, item.2, item.1
            ));
        }
        func_body.push("_ => ImportResult::NoMethod } } }".to_string());

        output.push(func_body.join("\n"));
    }
    let mut output: TokenStream = output.join("\n").parse().unwrap();
    output.extend(TokenStream::from(impls.into_token_stream()));

    output
}
