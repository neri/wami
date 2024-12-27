//! Global section

use super::{keyword::Keyword, ModuleName};
use crate::*;
use ast::{types::ExtVis, *};
use identifier::Identifier;
use leb128::{Leb128Writer, WriteError, WriteLeb128};
use wasm::expr::ConstExpr;

/// (`global` _id_? _import_? _export_? _globaltype_ _expr_)`
#[derive(Debug)]
pub struct Global {
    pub(crate) id: Option<Identifier>,
    pub(crate) vis: Option<ExtVis>,
    pub(crate) global_type: GlobalType,
    pub(crate) expr: ConstExpr,
}

impl ModuleName for Global {
    const IDENTIFIER: Keyword = Keyword::Global;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let id = Identifier::try_expect(tokens)?;

        let vis = ExtVis::try_expect(tokens)?;

        let global_type = GlobalType::expect(tokens)?;

        let expr = if matches!(vis, Some(ExtVis::Import(_))) {
            expect(tokens, &[TokenType::CloseParenthesis])?;

            ConstExpr::empty()
        } else {
            ConstExpr::expect(tokens, global_type.valtype)?
        };

        Ok(Self {
            id,
            vis,
            global_type,
            expr,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GlobalType {
    pub valtype: ValType,
    pub is_mut: bool,
}

impl GlobalType {
    pub fn expect(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        if let Some(valtype) = try_expect_module::<GlobalTypeMut>(tokens)? {
            Ok(Self {
                valtype: valtype.0,
                is_mut: true,
            })
        } else {
            Ok(Self {
                valtype: expect_valtype(tokens)?,
                is_mut: false,
            })
        }
    }

    #[inline]
    pub fn write_to_wasm(&self, writer: &mut Leb128Writer) -> Result<(), WriteError> {
        writer.write(self.valtype.as_bytecode())?;
        writer.write_byte(self.is_mut as u8)?;
        Ok(())
    }
}

struct GlobalTypeMut(ValType);

impl ModuleName for GlobalTypeMut {
    const IDENTIFIER: Keyword = Keyword::Mut;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let valtype = expect_valtype(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self(valtype))
    }
}
