//! Memory section
use super::{ModuleName, keyword::Keyword};
use crate::*;
use ast::*;
use identifier::IndexToken;

#[derive(Debug)]
pub struct Start {
    index: IndexToken,
    // position: TokenPosition,
}

impl ModuleName for Start {
    const IDENTIFIER: Keyword = Keyword::Start;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let index = IndexToken::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self { index })
    }
}

impl Start {
    #[inline]
    pub fn index(&self) -> &IndexToken {
        &self.index
    }
}
