//! WebAssembly Abstract Syntax Tree
use crate::keyword::Keyword;
use crate::types::ValType;
use crate::*;
use core::convert::Infallible;
use core::ops::ControlFlow;
use data::Data;
use elem::Elem;
use export::Export;
use function::Function;
use global::Global;
use import::Import;
use memory::Memory;
use start::Start;
use table::Table;
use types::Type;

pub mod data;
pub mod elem;
pub mod export;
pub mod function;
pub mod global;
pub mod identifier;
pub mod import;
pub mod literal;
pub mod memory;
pub mod start;
pub mod table;
pub mod types;

#[cfg(test)]
mod tests;

/// WebAssemby AST module
#[derive(Debug, Default)]
pub struct AstModule {
    pub types: Vec<Type>,
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
    pub tables: Vec<Table>,
    pub memories: Vec<Memory>,
    pub globals: Vec<Global>,
    pub exports: Vec<Export>,
    pub start: Option<Start>,
    pub elems: Vec<Elem>,
    pub data_segments: Vec<Data>,
}

pub trait ModuleName: Sized {
    const IDENTIFIER: Keyword;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError>;
}

impl AstModule {
    pub fn parse(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        if tokens.expect(&[TokenType::Eof]).is_ok() {
            return Ok(AstModule::default());
        }

        expect_sexpr(tokens, &[Keyword::Module], false)?;

        let mut module = AstModule::default();

        loop {
            if let Some(type_) = try_expect_module::<Type>(tokens)? {
                module.types.push(type_);
            } else if let Some(import) = try_expect_module::<Import>(tokens)? {
                module.imports.push(import);
            } else if let Some(func) = try_expect_module::<Function>(tokens)? {
                module.functions.push(func);
            } else if let Some(table) = try_expect_module::<Table>(tokens)? {
                module.tables.push(table);
            } else if let Some(memory) = try_expect_module::<Memory>(tokens)? {
                module.memories.push(memory);
            } else if let Some(global) = try_expect_module::<Global>(tokens)? {
                module.globals.push(global);
            } else if let Some(export) = try_expect_module::<Export>(tokens)? {
                module.exports.push(export);
            } else if let Some(start) = try_expect_module::<Start>(tokens)? {
                if module.start.is_some() {
                    // return Err(AssembleError::duplicated_module::<Start>(start));
                    todo!()
                } else {
                    module.start = Some(start);
                }
            } else if let Some(elem) = try_expect_module::<Elem>(tokens)? {
                module.elems.push(elem);
            } else if let Some(data) = try_expect_module::<Data>(tokens)? {
                module.data_segments.push(data);
            } else if expect(tokens, &[TokenType::CloseParenthesis]).is_ok() {
                expect(tokens, &[TokenType::Eof])?;
                return Ok(module);
            } else {
                expect(tokens, &[TokenType::OpenParenthesis])?;
                return Err(AssembleError::unexpected_keyword(
                    &[
                        Keyword::Data,
                        Keyword::Elem,
                        Keyword::Export,
                        Keyword::Func,
                        Keyword::Global,
                        Keyword::Import,
                        Keyword::Memory,
                        Keyword::Start,
                        Keyword::Table,
                        Keyword::Type,
                    ],
                    &tokens.next().unwrap(),
                ));
            }
        }
    }

    #[inline]
    pub fn types(&self) -> &[Type] {
        &self.types
    }

    #[inline]
    pub fn imports(&self) -> &[Import] {
        &self.imports
    }

    #[inline]
    pub fn exports(&self) -> &[Export] {
        &self.exports
    }

    #[inline]
    pub fn memories(&self) -> &[Memory] {
        &self.memories
    }

    #[inline]
    pub fn globals(&self) -> &[Global] {
        &self.globals
    }

    #[inline]
    pub fn data_segments(&self) -> &[Data] {
        &self.data_segments
    }

    #[inline]
    pub fn functions(&self) -> &[Function] {
        &self.functions
    }
}

pub(crate) fn try_expect_module<MODULE: ModuleName>(
    tokens: &mut TokenStream<Keyword>,
) -> Result<Option<MODULE>, AssembleError> {
    expect_sexpr(tokens, &[MODULE::IDENTIFIER], true).and_then(|v| match v {
        Some(_v) => MODULE::from_tokens(tokens).map(|v| Some(v)),
        None => Ok(None),
    })
}

fn expect_sexpr(
    tokens: &mut TokenStream<Keyword>,
    expected: &[Keyword],
    is_opt: bool,
) -> Result<Option<KeywordToken<Keyword>>, AssembleError> {
    match tokens.transaction(|tokens| {
        let start = tokens.next_non_blank();
        if start.token_type() != &TokenType::OpenParenthesis {
            if is_opt {
                return ControlFlow::Break(None);
            } else {
                return ControlFlow::Break(Some(AssembleError::missing_token(
                    &[TokenType::OpenParenthesis],
                    &start,
                )));
            }
        }

        let keyword = match expect_keywords(tokens, expected) {
            Ok(v) => v,
            Err(e) => {
                if is_opt {
                    return ControlFlow::Break(None);
                } else {
                    return ControlFlow::Break(Some(e));
                }
            }
        };

        match tokens
            .transaction(|tokens| {
                let next = tokens.peek_immed().unwrap();
                let expected = [
                    TokenType::Whitespace,
                    TokenType::NewLine,
                    TokenType::CloseParenthesis,
                ];
                if next.token_type().is_ignorable() || expected.contains(next.token_type()) {
                    ControlFlow::<Result<(), AssembleError>, Infallible>::Break(Ok(()))
                } else {
                    ControlFlow::Break(Err(AssembleError::missing_token(&expected, &next)))
                }
            })
            .unwrap_err()
        {
            Ok(_) => {}
            Err(e) => return ControlFlow::Break(Some(e)),
        }

        ControlFlow::Continue(keyword)
    }) {
        Ok(v) => Ok(Some(v)),
        Err(e) => match e {
            Some(e) => Err(e),
            None => Ok(None),
        },
    }
}
