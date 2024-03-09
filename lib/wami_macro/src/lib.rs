#![feature(proc_macro_span)]

extern crate proc_macro;

use proc_macro::TokenStream;
use std::borrow::Cow;
use syn::{parse_macro_input, spanned::Spanned, ItemImpl, ItemTrait};

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

/// A Macro to automatically generate WebAssembly import resolver from `impl`
///
/// - parameter: module name (default name is "env")
#[proc_macro_attribute]
pub fn wasm_env(attr: TokenStream, input: TokenStream) -> TokenStream {
    // println!("INPUT: {:?}", input.to_string());

    let mut input_ = input.clone();

    let mut mod_name = None;
    if let Some(ident) = attr.into_iter().next() {
        if let proc_macro::TokenTree::Ident(ident) = ident {
            mod_name = Some(ident.to_string())
        }
    }
    let mod_name = mod_name.unwrap_or("env".to_string());

    let impls = parse_macro_input!(input as ItemImpl);

    let class_name = ParsedType::new(impls.self_ty.as_ref(), ParseOption::CLASS_NAME).unwrap();

    let mut output = Vec::new();
    let mut resolve_list = Vec::new();

    output.push(format!("impl {} {{", class_name.to_string()));
    for item in impls.items.iter() {
        // I want `pub fn`
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
                    let param_type =
                        ParsedType::new(pat.ty.as_ref(), ParseOption::IMPORTS_PARAM_TYPE).unwrap();
                    params.push((param_name, param_type));
                }
            }
        }

        let result_type = match &func.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => ParsedType::new(ty, ParseOption::RESULT_TYPE),
        };

        let var_instance = "instance";
        let var_memory = "memory";

        let signature = ParsedType::function_signature(
            &result_type,
            params
                .iter()
                .map(|v| &v.1)
                .filter(|v| !v.is_wasm_instance()),
        );

        let bridge_fn_name = format!("__env_{func_name}");

        resolve_list.push((func_name.clone(), bridge_fn_name.clone(), signature));

        let mut func_body = Vec::new();
        let mut call_params = Vec::new();
        func_body.push(format!(
            "fn {bridge_fn_name}({var_instance}: {}, mut args: WasmArgs) -> WasmDynResult {{",
            IntrinsicType::WasmInstance.to_string()
        ));

        for param in params.iter() {
            match &param.1 {
                ParsedType::IntrinsicType(intrinsics) => match intrinsics {
                    IntrinsicType::WasmInstance => {
                        call_params.push(var_instance.to_string());
                    }
                    IntrinsicType::Str => {
                        call_params.push(param.0.clone());
                        func_body.push(format!("let {var_memory} = {var_instance}.memory(0).unwrap().try_borrow()?;
let {} = {{
    let base = args.next::<WasmPtr<u8>>()?;
    let len = args.next::<u32>().map(|v| v as usize)?;
    {var_memory}.slice(base, len).and_then(|v| core::str::from_utf8(v).map_err(|_| WasmRuntimeErrorKind::InvalidParameter.into()))?
}};",
                        param.0))
                    }
                },
                _ => {
                    call_params.push(param.0.clone());
                    func_body.push(format!(
                        "let {} = args.next::<{}>()?;",
                        param.0,
                        param.1.to_string()
                    ));
                }
            }
        }

        let call_method = format!("Self::{}({})", func_name, call_params.join(", "),);
        match result_type.as_ref() {
            None => {
                func_body.push(format!("{call_method}; Ok(None)",));
            }
            Some(v) => match v {
                ParsedType::Result(v) => match v {
                    Some(_) => {
                        func_body.push(format!("{call_method}.map(|v| Some(v.into()))"));
                    }
                    None => func_body.push(format!("{call_method}.map(|_| None)")),
                },
                _ => {
                    func_body.push(format!("let r = {call_method}; Ok(Some(r.into()))",));
                }
            },
        }
        func_body.push("}".to_string());

        output.push(func_body.join("\n"));
    }
    output.push("}".to_string());

    {
        let mut func_body = Vec::new();
        func_body.push(format!("impl WasmEnv for {} {{", class_name.to_string()));
        func_body.push(format!(
            "fn resolve_imports(&self, mod_name: &str, name: &str, type_: &WasmType) -> WasmImportResult {{",
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

    input_.extend(output.join("\n").parse::<TokenStream>().unwrap());
    input_
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
        // I want only `fn`
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
                    let param_type =
                        ParsedType::new(pat.ty.as_ref(), ParseOption::EXPORTS_PARAM_TYPE).unwrap();
                    params.push((param_name, param_type));
                }
            }
        }

        let result_type = match &func.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => ParsedType::new(ty, ParseOption::RESULT_TYPE),
        };

        let mut push_args = Vec::new();
        let mut func_sig = Vec::new();
        for param in params.iter() {
            push_args.push(format!("WasmValue::from({})", param.0));
            func_sig.push(format!("{}: {}", param.0, param.1.to_string()));
        }

        let output_sig = format!(
            "fn {func_name}(&self, {}) -> WasmResult<{}>",
            func_sig.join(", "),
            result_type
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or(Cow::Borrowed("()")),
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
                    Some(v) => v.get_{}().map_err(|e| e.into()),
                    None => Err(WasmRuntimeErrorKind::TypeMismatch.into()),
                }})",
                result_type.to_string(),
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

#[derive(Debug)]
enum ParsedType {
    Primitive(Primitive),
    IntrinsicType(IntrinsicType),
    NonPrimitive(String),
    Ref(Box<Self>),
    RefMut(Box<Self>),
    Ptr(Box<Self>),
    PtrMut(Box<Self>),
    Result(Option<Box<Self>>),
}

#[derive(Debug, Clone, Copy, Default)]
struct ParseOption {
    primitive_only: bool,
    allow_intrinsics: bool,
    allow_reference: bool,
    allow_nil: bool,
}

impl ParseOption {
    const CLASS_NAME: Self = Self {
        primitive_only: false,
        allow_intrinsics: false,
        allow_reference: false,
        allow_nil: false,
    };

    const RESULT_TYPE: Self = Self {
        primitive_only: true,
        allow_intrinsics: false,
        allow_reference: false,
        allow_nil: true,
    };

    const EXPORTS_PARAM_TYPE: Self = Self {
        primitive_only: true,
        allow_intrinsics: false,
        allow_reference: true,
        allow_nil: false,
    };

    const IMPORTS_PARAM_TYPE: Self = Self {
        primitive_only: true,
        allow_intrinsics: true,
        allow_reference: true,
        allow_nil: false,
    };

    fn extend<F>(&self, f: F) -> Self
    where
        F: FnOnce(&mut Self),
    {
        let mut config = *self;
        f(&mut config);
        config
    }
}

impl ParsedType {
    fn new(ty: &syn::Type, options: ParseOption) -> Option<ParsedType> {
        if options.allow_intrinsics {
            if let Some(intrinsics) = IntrinsicType::new(ty, options) {
                return Some(ParsedType::IntrinsicType(intrinsics));
            }
        }
        match ty {
            syn::Type::Path(path) => {
                let Some(path) = ParsedType::parse_path(&path.path, options) else {
                    unexpected_token!(path.span(), "simple type")
                };
                Some(path)
            }
            syn::Type::Reference(reference) => {
                if options.allow_reference {
                    Self::new(
                        reference.elem.as_ref(),
                        options.extend(|v| v.allow_nil = false),
                    )
                    .map(|v| {
                        if reference.mutability.is_some() {
                            ParsedType::RefMut(Box::new(v))
                        } else {
                            ParsedType::Ref(Box::new(v))
                        }
                    })
                } else {
                    unexpected_token!(ty.span(), "type")
                }
            }
            syn::Type::Ptr(ptr) => {
                if options.allow_reference {
                    Self::new(ptr.elem.as_ref(), options.extend(|v| v.allow_nil = false)).map(|v| {
                        if ptr.mutability.is_some() {
                            ParsedType::PtrMut(Box::new(v))
                        } else {
                            ParsedType::Ptr(Box::new(v))
                        }
                    })
                } else {
                    unexpected_token!(ty.span(), "type")
                }
            }
            syn::Type::Tuple(tuple) => {
                if options.allow_nil && tuple.elems.is_empty() {
                    None
                } else {
                    unexpected_token!(ty.span(), "type")
                }
            }
            _ => {
                todo!("UNEXPECTED {:?}", ty);
                // unexpected_token!(pat.ty.span(), "type")
            }
        }
    }

    fn parse_path(path: &syn::Path, options: ParseOption) -> Option<Self> {
        let mut segments = path.segments.iter();
        let Some(first_elem) = segments.next() else {
            return None;
        };
        let first_type = first_elem.ident.to_string();
        let first_type = first_type.as_str();
        match first_type {
            "WasmPtr" => match &first_elem.arguments {
                syn::PathArguments::AngleBracketed(v) => match v.args.first().unwrap() {
                    syn::GenericArgument::Type(ty) => {
                        return ParsedType::new(ty, options).map(|v| ParsedType::Ptr(Box::new(v)))
                    }
                    _ => unexpected_token!(path.span(), "simple type"),
                },
                _ => unexpected_token!(path.span(), "simple type"),
            },
            "WasmPtrMut" => match &first_elem.arguments {
                syn::PathArguments::AngleBracketed(v) => match v.args.first().unwrap() {
                    syn::GenericArgument::Type(ty) => {
                        return ParsedType::new(ty, options)
                            .map(|v| ParsedType::PtrMut(Box::new(v)))
                    }
                    _ => unexpected_token!(path.span(), "simple type"),
                },
                _ => unexpected_token!(path.span(), "simple type"),
            },
            "WasmResult" => match &first_elem.arguments {
                syn::PathArguments::AngleBracketed(v) => match v.args.first().unwrap() {
                    syn::GenericArgument::Type(ty) => {
                        match ParsedType::new(ty, options)
                            .map(|v| ParsedType::Result(Some(Box::new(v))))
                        {
                            Some(v) => return Some(v),
                            None => {
                                if options.allow_nil {
                                    return Some(ParsedType::Result(None));
                                }
                                unexpected_token!(path.span(), "simple type")
                            }
                        }
                    }
                    _ => unexpected_token!(path.span(), "simple type"),
                },
                _ => unexpected_token!(path.span(), "simple type"),
            },
            _ => {
                if let Some(primitive) = Primitive::from_str(first_type) {
                    return Some(ParsedType::Primitive(primitive));
                }

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
                if !options.primitive_only {
                    return Some(ParsedType::NonPrimitive(output));
                }
            }
        }

        unexpected_token!(path.span(), "simple type")
    }

    fn to_string(&self) -> Cow<'static, str> {
        match self {
            Self::Primitive(v) => Cow::Borrowed(v.as_str()),
            Self::IntrinsicType(v) => v.to_string(),
            Self::NonPrimitive(v) => Cow::Owned(v.clone()),
            Self::Ref(v) => Cow::Owned(format!("&{}", v.to_string())),
            Self::RefMut(v) => Cow::Owned(format!("&mut{}", v.to_string())),
            Self::Ptr(v) => Cow::Owned(format!("WasmPtr<{}>", v.to_string())),
            Self::PtrMut(v) => Cow::Owned(format!("WasmPtrMut<{}>", v.to_string())),
            Self::Result(v) => match v {
                Some(v) => Cow::Owned(format!("WasmResult<{}>", v.to_string())),
                None => Cow::Borrowed("WasmResult<()>"),
            },
        }
    }

    fn signature<T: AsRef<Self>>(_self: Option<&T>) -> &'static str {
        let Some(_self) = _self else { return "v" };
        match _self.as_ref() {
            Self::Primitive(v) => v.signature(),
            Self::IntrinsicType(v) => v.signature(),
            Self::NonPrimitive(_) => "_",
            Self::Result(v) => Self::signature(v.as_ref()),
            _ => Primitive::POINTER_TYPE.signature(),
        }
    }

    fn function_signature<'a>(
        result_type: &Option<Self>,
        param_types: impl Iterator<Item = &'a Self>,
    ) -> String {
        let result_type = Self::signature(result_type.as_ref());
        let param = param_types
            .map(|v| Self::signature(Some(v)))
            .collect::<String>();
        let param = if param.is_empty() {
            Self::signature::<Self>(None).to_string()
        } else {
            param
        };

        format!("{}{}", result_type, param)
    }

    fn is_wasm_instance(&self) -> bool {
        matches!(self, ParsedType::IntrinsicType(IntrinsicType::WasmInstance))
    }
}

impl AsRef<Self> for ParsedType {
    fn as_ref(&self) -> &Self {
        self
    }
}

#[allow(unused)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Primitive {
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

impl Primitive {
    const POINTER_TYPE: Self = Self::U32;

    pub fn from_str(v: &str) -> Option<Self> {
        match v {
            "bool" => Some(Self::Bool),
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            "i16" => Some(Self::I16),
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            "i8" => Some(Self::I8),
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            "u64" => Some(Self::U64),
            "u8" => Some(Self::U8),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I8 => "i8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U8 => "u8",
        }
    }

    pub fn signature(&self) -> &'static str {
        match self {
            Self::Bool | Self::I8 | Self::U8 | Self::I16 | Self::U16 | Self::I32 | Self::U32 => "i",
            Self::I64 | Self::U64 => "l",
            Self::F32 => "f",
            Self::F64 => "d",
        }
    }
}

#[derive(Debug)]
enum IntrinsicType {
    WasmInstance,
    Str,
}

impl IntrinsicType {
    pub fn new(ty: &syn::Type, options: ParseOption) -> Option<Self> {
        let Some(v) = ty.span().source_text() else {
            return None;
        };
        let _ = options;
        match v.as_str() {
            "&WasmInstance" => Some(Self::WasmInstance),
            "&str" => Some(Self::Str),
            _ => None,
        }
    }

    pub fn to_string(&self) -> Cow<'static, str> {
        match self {
            Self::WasmInstance => Cow::Borrowed("&WasmInstance"),
            Self::Str => Cow::Borrowed("&str"),
        }
    }

    pub fn signature(&self) -> &'static str {
        match self {
            Self::Str => "ii",
            _ => "_",
        }
    }
}
