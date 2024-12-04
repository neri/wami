//! Import section
use crate::*;
use ast::{types::TypeUse, *};
use global::GlobalType;
use identifier::Identifier;
use keyword::Keyword;
use literal::StringLiteral;

/// (`import` _mod_name_ _name_ _desc_)
#[derive(Debug)]
pub struct Import {
    mod_name: StringLiteral,
    name: StringLiteral,
    desc: ImportDescriptor,
}

#[derive(Debug)]
pub enum ImportDescriptor {
    Func(ImportFunc),
    Global(ImportGlobal),
}

impl Import {
    #[inline]
    pub fn mod_name(&self) -> &StringLiteral {
        &self.mod_name
    }

    #[inline]
    pub fn name(&self) -> &StringLiteral {
        &self.name
    }

    #[inline]
    pub fn desc(&self) -> &ImportDescriptor {
        &self.desc
    }
}

impl ModuleName for Import {
    const IDENTIFIER: Keyword = Keyword::Import;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let mod_name = StringLiteral::expect(tokens)?;

        let name = StringLiteral::expect(tokens)?;

        let desc = if let Some(func) = try_expect_module::<ImportFunc>(tokens)? {
            ImportDescriptor::Func(func)
        } else if let Some(global) = try_expect_module::<ImportGlobal>(tokens)? {
            ImportDescriptor::Global(global)
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

        Ok(Self {
            mod_name,
            name,
            desc,
        })
    }
}

#[derive(Debug)]
pub struct ImportFunc {
    id: Option<Identifier>,
    typeuse: TypeUse,
}

impl ImportFunc {
    #[inline]
    pub fn identifier(&self) -> Option<&Identifier> {
        self.id.as_ref()
    }

    #[inline]
    pub fn typeuse(&self) -> &TypeUse {
        &self.typeuse
    }
}

impl ModuleName for ImportFunc {
    const IDENTIFIER: Keyword = Keyword::Func;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let id = Identifier::try_expect(tokens)?;

        let typeuse = TypeUse::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self { id, typeuse })
    }
}

#[derive(Debug)]
pub struct ImportGlobal {
    id: Option<Identifier>,
    global_type: GlobalType,
}

impl ImportGlobal {
    #[inline]
    pub fn identifier(&self) -> Option<&Identifier> {
        self.id.as_ref()
    }

    #[inline]
    pub fn global_type(&self) -> &GlobalType {
        &self.global_type
    }
}

impl ModuleName for ImportGlobal {
    const IDENTIFIER: Keyword = Keyword::Global;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let id = Identifier::try_expect(tokens)?;

        let global_type = GlobalType::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self { id, global_type })
    }
}

/// (`import` _mod_name_ _name_)
#[derive(Debug)]
pub struct ImportAbbr {
    mod_name: StringLiteral,
    name: StringLiteral,
}

impl ImportAbbr {
    #[inline]
    pub fn mod_name(&self) -> &StringLiteral {
        &self.mod_name
    }

    #[inline]
    pub fn name(&self) -> &StringLiteral {
        &self.name
    }
}

impl ModuleName for ImportAbbr {
    const IDENTIFIER: Keyword = Keyword::Import;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let mod_name = StringLiteral::expect(tokens)?;

        let name = StringLiteral::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self { mod_name, name })
    }
}
