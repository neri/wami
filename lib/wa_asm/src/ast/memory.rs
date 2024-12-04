//! Memory section
use super::{identifier::Identifier, keyword::Keyword, ModuleName};
use crate::*;
use ast::*;
use core::num::{NonZero, NonZeroU32};
use literal::NumericLiteral;

#[derive(Debug)]
pub struct Memory {
    id: Option<Identifier>,
    min: NumericLiteral<u32>,
    max: Option<NumericLiteral<u32>>,
}

impl Memory {
    #[inline]
    pub fn id(&self) -> Option<&Identifier> {
        self.id.as_ref()
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

impl ModuleName for Memory {
    const IDENTIFIER: Keyword = Keyword::Memory;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let id = Identifier::try_expect(tokens)?;

        let min = NumericLiteral::<u32>::expect(tokens)?;

        let max = NumericLiteral::<u32>::try_expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self { id, min, max })
    }
}

#[derive(Debug)]
pub struct MemUse {
    index: NumericLiteral<u32>,
}

impl MemUse {
    #[inline]
    pub fn index(&self) -> &NumericLiteral<u32> {
        &self.index
    }
}

impl ModuleName for MemUse {
    const IDENTIFIER: Keyword = Keyword::Memory;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let index = NumericLiteral::<u32>::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self { index })
    }
}
