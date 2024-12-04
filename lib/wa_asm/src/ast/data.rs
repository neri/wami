//! Data section

use super::{keyword::Keyword, ModuleName};
use crate::*;
use ast::*;
use literal::RawBytesLiteral;
use memory::MemUse;
use wasm::expr::ConstExpr;

/// (`data` _memuse?_ (`offset`? _instr_) _datastring_* )`
pub struct Data {
    pub(crate) mem_use: Option<MemUse>,
    pub(crate) offset: ConstExpr,
    pub(crate) bytes: Vec<u8>,
}

impl ModuleName for Data {
    const IDENTIFIER: Keyword = Keyword::Data;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let mem_use = try_expect_module::<MemUse>(tokens)?;

        let offset = Offset::expect(tokens)?;

        let mut bytes = RawBytesLiteral::expect(tokens)?.into_vec();
        while let Some(raw_bytes) = RawBytesLiteral::try_expect(tokens)? {
            bytes.extend(raw_bytes.get());
        }

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self {
            mem_use,
            offset,
            bytes,
        })
    }
}

impl core::fmt::Debug for Data {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Data")
            .field("mem_use", &self.mem_use)
            .field("offset", &self.offset)
            .field("bytes", &DumpHex(&self.bytes))
            .finish()
    }
}

/// (`offset`? _instr_)
#[derive(Debug)]
pub struct Offset(pub ConstExpr);

impl Offset {
    pub fn expect(tokens: &mut TokenStream<Keyword>) -> Result<ConstExpr, AssembleError> {
        if let Some(offset) = try_expect_module::<Offset>(tokens)? {
            Ok(offset.0)
        } else {
            expect(tokens, &[TokenType::OpenParenthesis])?;
            ConstExpr::expect(tokens, ValType::I32)
        }
    }
}

impl ModuleName for Offset {
    const IDENTIFIER: Keyword = Keyword::Offset;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        ConstExpr::expect(tokens, ValType::I32).map(|v| Offset(v))
    }
}
