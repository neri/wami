//! Elem section

use super::data::Offset;
use super::identifier::{Identifier, IndexToken};
use super::{keyword::Keyword, ModuleName};
use crate::*;
use wasm::expr::ConstExpr;

/// (`elem` _id_? _offset_ _elemlist_)`
#[derive(Debug)]
pub struct Elem {
    // pub(crate) id: Option<Identifier>,
    pub(crate) offset: ConstExpr,
    pub(crate) elemlist: Vec<IndexToken>,
}

impl ModuleName for Elem {
    const IDENTIFIER: Keyword = Keyword::Elem;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let _id = Identifier::try_expect(tokens)?;

        let offset = Offset::expect(tokens)?;

        let _ = tokens.expect_keyword(Keyword::Func);

        let mut elemlist = Vec::new();
        while let Some(funcidx) = IndexToken::try_expect(tokens)? {
            elemlist.push(funcidx)
        }

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self { offset, elemlist })
    }
}
