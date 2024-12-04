//! Function section
use super::{identifier::Identifier, keyword::Keyword, ModuleName};
use crate::*;
use ast::{
    types::{ExtVis, FuncTypeLocal, IdAndValtype, TypeUse},
    *,
};
use wasm::code::Code;

/// (`func` _id_? _imports_? _exports_? _typeuse_ (_local_)* (_instr_)*)
#[derive(Debug)]
pub struct Function {
    pub(crate) id: Option<Identifier>,
    pub(crate) vis: Option<ExtVis>,
    pub(crate) typeuse: TypeUse,
    pub(crate) locals: Vec<IdAndValtype>,
    pub(crate) instr: Code,
}

impl Function {
    #[inline]
    pub fn identifier(&self) -> Option<&Identifier> {
        self.id.as_ref()
    }

    #[inline]
    pub fn vis(&self) -> Option<&ExtVis> {
        self.vis.as_ref()
    }

    #[inline]
    pub fn typeuse(&self) -> &TypeUse {
        &self.typeuse
    }

    #[inline]
    pub fn locals(&self) -> &[IdAndValtype] {
        &self.locals
    }

    #[inline]
    pub fn instrs(&self) -> &Code {
        &self.instr
    }
}

impl ModuleName for Function {
    const IDENTIFIER: Keyword = Keyword::Func;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let id = Identifier::try_expect(tokens)?;

        let vis = ExtVis::try_expect(tokens)?;

        let typeuse = TypeUse::expect(tokens)?;

        let mut locals = Vec::new();
        loop {
            if let Some(local) = try_expect_module::<FuncTypeLocal>(tokens)? {
                locals.extend(local.0);
            } else {
                break;
            }
        }

        let instr = Code::expect(tokens)?;

        // expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self {
            id,
            vis,
            typeuse,
            locals,
            instr,
        })
    }
}
