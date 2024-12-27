//! Exports

use crate::*;
use ast::*;
use identifier::IndexToken;
use keyword::Keyword;
use literal::StringLiteral;

/// (`export` _name_ _desc_)
#[derive(Debug)]
pub struct Export {
    name: StringLiteral,
    desc: ExportDescriptor,
}

#[derive(Debug)]
pub enum ExportDescriptor {
    Func(ExportFunc),
    Table(ExportTable),
    Memory(ExportMemory),
    Global(ExportGlobal),
}

impl Export {
    #[inline]
    pub fn name(&self) -> &StringLiteral {
        &self.name
    }

    #[inline]
    pub fn desc(&self) -> &ExportDescriptor {
        &self.desc
    }
}

impl ModuleName for Export {
    const IDENTIFIER: Keyword = Keyword::Export;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let name = StringLiteral::expect(tokens)?;

        let desc = if let Some(item) = try_expect_module::<ExportFunc>(tokens)? {
            ExportDescriptor::Func(item)
        } else if let Some(item) = try_expect_module::<ExportTable>(tokens)? {
            ExportDescriptor::Table(item)
        } else if let Some(item) = try_expect_module::<ExportMemory>(tokens)? {
            ExportDescriptor::Memory(item)
        } else if let Some(item) = try_expect_module::<ExportGlobal>(tokens)? {
            ExportDescriptor::Global(item)
        } else {
            expect(tokens, &[TokenType::OpenParenthesis])?;
            return Err(AssembleError::unexpected_keyword(
                &[
                    Keyword::Func,
                    Keyword::Table,
                    Keyword::Memory,
                    Keyword::Global,
                ],
                &tokens.next().unwrap(),
            ));
        };

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self { name, desc })
    }
}

/// (`func` _funcidx_)
#[derive(Debug)]
pub struct ExportFunc(pub IndexToken);

impl ModuleName for ExportFunc {
    const IDENTIFIER: Keyword = Keyword::Func;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let index = IndexToken::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self(index))
    }
}

/// (`table` _tableidx_)
#[derive(Debug)]
pub struct ExportTable(pub IndexToken);

impl ModuleName for ExportTable {
    const IDENTIFIER: Keyword = Keyword::Table;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let index = IndexToken::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self(index))
    }
}

/// (`memory` _memidx_)
#[derive(Debug)]
pub struct ExportMemory(pub IndexToken);

impl ModuleName for ExportMemory {
    const IDENTIFIER: Keyword = Keyword::Memory;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let index = IndexToken::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self(index))
    }
}

/// (`global` _globalidx_)
#[derive(Debug)]
pub struct ExportGlobal(pub IndexToken);

impl ModuleName for ExportGlobal {
    const IDENTIFIER: Keyword = Keyword::Global;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let index = IndexToken::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self(index))
    }
}

/// (`export` _name_)
#[derive(Debug)]
pub struct ExportAbbr {
    name: StringLiteral,
}

impl ExportAbbr {
    #[inline]
    pub fn name(&self) -> &StringLiteral {
        &self.name
    }
}

impl ModuleName for ExportAbbr {
    const IDENTIFIER: Keyword = Keyword::Export;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let name = StringLiteral::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self { name })
    }
}
