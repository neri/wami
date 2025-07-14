//! WebAssembly assembler
#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[cfg(test)]
pub mod tests;

pub mod ast;
pub mod error;
pub mod ir;
pub mod lexer;
pub mod types;
pub mod wasm;

#[path = "_generated/keyword.rs"]
pub mod keyword;

#[allow(unused)]
pub(crate) use alloc::{
    borrow::Cow,
    borrow::ToOwned,
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
#[allow(unused)]
pub(crate) use core::{convert::Infallible, ops::ControlFlow};
pub(crate) use error::*;

use keyword::Keyword;
use lexer::*;

pub struct WatAssembler;

impl WatAssembler {
    fn _from_src<F, R>(file_name: &str, src: Vec<u8>, kernel: F) -> Result<R, String>
    where
        F: FnOnce(&mut TokenStream<Keyword>) -> Result<R, AssembleError>,
    {
        let src = Arc::new(src);
        let tokens = Lexer::parse(src.clone(), Keyword::from_str)
            .map_err(|e| AssembleError::from(e).to_detail_string(file_name, &src, &[]))?;
        kernel(&mut tokens.stream())
            .map_err(|e| e.to_detail_string(file_name, &src, tokens.line_positions()))
    }

    pub fn assemble(file_name: &str, src: Vec<u8>) -> Result<Vec<u8>, String> {
        Self::_from_src(file_name, src, |tokens: &mut TokenStream<Keyword>| {
            let ir_module = ir::Module::from_ast(ast::AstModule::parse(tokens)?)?;

            ir_module.write_to_wasm().map_err(|e| {
                AssembleError::internal_inconsistency(
                    &format!("{:?}", e),
                    ErrorPosition::Unspecified,
                )
            })
        })
    }
}

pub trait WasmBinding<T, E> {
    fn wasm_binding(&self) -> Result<T, E>;
}

pub struct DumpHex<'a, T: core::fmt::Debug + core::fmt::LowerHex>(&'a [T]);

impl<T: core::fmt::Debug + core::fmt::LowerHex> core::fmt::Debug for DumpHex<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[")?;
        let mut needs_newline = false;
        for line in self.0.chunks(8) {
            write!(f, "\n    ")?;
            for item in line {
                write!(f, "0x{:x}, ", item)?;
            }
            needs_newline = true;
        }
        if needs_newline {
            writeln!(f, "")?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

pub(crate) fn expect<KEYWORD>(
    tokens: &mut TokenStream<KEYWORD>,
    expected: &[TokenType<KEYWORD>],
) -> Result<Token<KEYWORD>, AssembleError>
where
    KEYWORD: core::fmt::Display + core::fmt::Debug + Copy + PartialEq,
{
    tokens
        .expect(expected)
        .map_err(|e| AssembleError::missing_token(expected, &e))
}

pub(crate) fn expect_immed<KEYWORD>(
    tokens: &mut TokenStream<KEYWORD>,
    expected: &[TokenType<KEYWORD>],
) -> Result<Token<KEYWORD>, AssembleError>
where
    KEYWORD: core::fmt::Display + core::fmt::Debug + Copy + PartialEq,
{
    tokens
        .expect_immed(expected)
        .map_err(|e| AssembleError::missing_token(expected, &e))
}

pub(crate) fn expect_keywords<KEYWORD>(
    tokens: &mut TokenStream<KEYWORD>,
    expected: &[KEYWORD],
) -> Result<KeywordToken<KEYWORD>, AssembleError>
where
    KEYWORD: core::fmt::Display + core::fmt::Debug + Copy + PartialEq,
{
    tokens
        .expect_keywords(expected)
        .map_err(|e| AssembleError::unexpected_keyword(expected, &e))
}

pub(crate) fn expect_valtype<KEYWORD>(
    tokens: &mut TokenStream<KEYWORD>,
) -> Result<types::ValType, AssembleError>
where
    KEYWORD: core::fmt::Display + core::fmt::Debug + Copy + PartialEq,
{
    let token = expect(tokens, &[TokenType::Identifier])?;
    types::ValType::from_str(token.source()).ok_or(AssembleError::invalid_identifier(
        token.source(),
        token.position().into(),
    ))
}

pub(crate) fn try_expect_free_keyword<KEYWORD>(
    tokens: &mut TokenStream<KEYWORD>,
    keyword: &str,
) -> Result<Option<Token<KEYWORD>>, AssembleError>
where
    KEYWORD: core::fmt::Display + core::fmt::Debug + Copy + PartialEq,
{
    match tokens.transaction(|tokens| {
        let token = tokens.next_non_blank();
        if token.source() == keyword {
            ControlFlow::Continue(token)
        } else {
            ControlFlow::Break(Option::<Infallible>::None)
        }
    }) {
        Ok(v) => Ok(Some(v)),
        Err(e) => match e {
            #[allow(unreachable_patterns)]
            Some(_) => todo!(),
            None => Ok(None),
        },
    }
}
