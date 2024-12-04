//! Table section
use super::{identifier::Identifier, keyword::Keyword, ModuleName};
use crate::*;
use ast::*;
use core::num::{NonZero, NonZeroU32};
use leb128::{Leb128Writer, WriteError};
use literal::NumericLiteral;

#[derive(Debug)]
pub struct Table {
    id: Option<Identifier>,
    reftype: RefType,
    min: NumericLiteral<u32>,
    max: Option<NumericLiteral<u32>>,
}

impl Table {
    #[inline]
    pub fn id(&self) -> Option<&Identifier> {
        self.id.as_ref()
    }

    #[inline]
    pub fn reftype(&self) -> RefType {
        self.reftype
    }

    #[inline]
    pub fn min(&self) -> u32 {
        self.min.get()
    }

    #[inline]
    pub fn max(&self) -> Option<NonZero<u32>> {
        self.max.as_ref().and_then(|v| NonZeroU32::new(v.get()))
    }
}

impl ModuleName for Table {
    const IDENTIFIER: Keyword = Keyword::Table;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let id = Identifier::try_expect(tokens)?;

        let min = NumericLiteral::<u32>::expect(tokens)?;

        let max = NumericLiteral::<u32>::try_expect(tokens)?;

        let reftype = expect_keywords(
            tokens,
            &[Keyword::Anyfunc, Keyword::Funcref, Keyword::Externref],
        )?;
        let reftype = match reftype.keyword() {
            Keyword::Anyfunc | Keyword::Funcref => RefType::FuncRef,
            Keyword::Externref => RefType::ExternRef,
            _ => unreachable!(),
        };

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self {
            id,
            reftype,
            min,
            max,
        })
    }
}

#[derive(Debug)]
pub struct TableUse {
    index: NumericLiteral<u32>,
}

impl TableUse {
    #[inline]
    pub fn index(&self) -> &NumericLiteral<u32> {
        &self.index
    }
}

impl ModuleName for TableUse {
    const IDENTIFIER: Keyword = Keyword::Table;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let index = NumericLiteral::<u32>::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self { index })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefType {
    FuncRef,
    ExternRef,
}

impl RefType {
    #[inline]
    pub fn write_to_wasm(&self, writer: &mut Leb128Writer) -> Result<(), WriteError> {
        match self {
            RefType::FuncRef => writer.write_byte(0x70),
            RefType::ExternRef => writer.write_byte(0x6F),
        }
    }
}
