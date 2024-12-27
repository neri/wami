//! Types
use super::{keyword::Keyword, AssembleError};
use crate::types::ValType;
use crate::*;
use ast::identifier::Identifier;
use ast::*;
use export::ExportAbbr;
use identifier::IndexToken;
use import::ImportAbbr;

/// (`type` _id_? _functype_)
#[derive(Debug)]
pub struct Type {
    id: Option<Identifier>,
    params: Vec<IdAndValtype>,
    results: Vec<ValType>,
}

#[derive(Debug)]
pub struct IdAndValtype {
    id: Option<Identifier>,
    valtype: ValType,
}

impl IdAndValtype {
    #[inline]
    pub const fn anonymous(valtype: ValType) -> Self {
        Self { id: None, valtype }
    }

    #[inline]
    pub fn identifier(&self) -> Option<&Identifier> {
        self.id.as_ref()
    }

    #[inline]
    pub fn valtype(&self) -> ValType {
        self.valtype
    }
}

impl Type {
    #[inline]
    pub fn identifier(&self) -> Option<&Identifier> {
        self.id.as_ref()
    }

    #[inline]
    pub fn params(&self) -> &[IdAndValtype] {
        &self.params
    }

    #[inline]
    pub fn results(&self) -> &[ValType] {
        &self.results
    }
}

impl ModuleName for Type {
    const IDENTIFIER: Keyword = Keyword::Type;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let id = Identifier::try_expect(tokens)?;

        let func = try_expect_module::<FuncType>(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        if let Some(func) = func {
            Ok(Self {
                id,
                params: func.params,
                results: func.results,
            })
        } else {
            Ok(Self {
                id,
                params: Vec::new(),
                results: Vec::new(),
            })
        }
    }
}

/// (`func` _param_* _result_*)
#[derive(Debug)]
pub struct FuncType {
    params: Vec<IdAndValtype>,
    results: Vec<ValType>,
}

impl FuncType {
    #[inline]
    pub fn params(&self) -> &[IdAndValtype] {
        &self.params
    }

    #[inline]
    pub fn results(&self) -> &[ValType] {
        &self.results
    }
}

impl ModuleName for FuncType {
    const IDENTIFIER: Keyword = Keyword::Func;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let mut params = Vec::new();
        let mut results = Vec::new();

        loop {
            if let Some(item) = try_expect_module::<FuncTypeParam>(tokens)? {
                params.extend(item.0);
            } else {
                break;
            }
        }
        loop {
            if let Some(item) = try_expect_module::<FuncTypeResult>(tokens)? {
                results.extend_from_slice(item.0.as_slice());
            } else {
                expect(tokens, &[TokenType::CloseParenthesis])?;
                return Ok(FuncType { params, results });
            }
        }
    }
}

/// (`param` _id_? _valtype_*)
#[derive(Debug)]
pub struct FuncTypeParam(pub Vec<IdAndValtype>);

/// (`result` _valtype_*)
#[derive(Debug)]
pub struct FuncTypeResult(pub(crate) Vec<ValType>);

impl ModuleName for FuncTypeParam {
    const IDENTIFIER: Keyword = Keyword::Param;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        // The optional identifier names for parameters in a function type only have documentation purpose.
        // They cannot be referenced from anywhere.
        match Identifier::try_expect(tokens)? {
            Some(id) => {
                let valtype = expect_valtype(tokens)?;

                expect(tokens, &[TokenType::CloseParenthesis])?;

                Ok(Self(
                    [IdAndValtype {
                        id: Some(id),
                        valtype,
                    }]
                    .into(),
                ))
            }

            None => {
                let mut valtypes = Vec::new();
                valtypes.push(IdAndValtype::anonymous(expect_valtype(tokens)?));

                loop {
                    let token = expect(
                        tokens,
                        &[TokenType::CloseParenthesis, TokenType::Identifier],
                    )?;
                    match token.token_type() {
                        TokenType::CloseParenthesis => return Ok(Self(valtypes)),

                        TokenType::Identifier => {
                            let item = ValType::from_str(token.source()).ok_or(
                                AssembleError::invalid_identifier(
                                    token.source(),
                                    token.position().into(),
                                ),
                            )?;
                            valtypes.push(IdAndValtype::anonymous(item));
                        }

                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}

impl ModuleName for FuncTypeResult {
    const IDENTIFIER: Keyword = Keyword::Result;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let mut result = Vec::new();

        result.push(expect_valtype(tokens)?);

        loop {
            let token = expect(
                tokens,
                &[TokenType::CloseParenthesis, TokenType::Identifier],
            )?;
            match token.token_type() {
                TokenType::CloseParenthesis => return Ok(Self(result)),

                TokenType::Identifier => {
                    let item = ValType::from_str(token.source()).ok_or(
                        AssembleError::invalid_identifier(token.source(), token.position().into()),
                    )?;
                    result.push(item);
                }

                _ => unreachable!(),
            }
        }
    }
}

#[derive(Debug)]
pub struct TypeUse {
    kind: TypeUseKind,
    token: RawToken,
}

#[derive(Debug)]
pub enum TypeUseKind {
    Index(TypeIndex),
    FuncType(FuncType),
    Both(TypeIndex, FuncType),
}

impl TypeUse {
    #[inline]
    pub fn kind(&self) -> &TypeUseKind {
        &self.kind
    }

    #[inline]
    pub fn token(&self) -> &RawToken {
        &self.token
    }

    pub fn expect(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let start = match tokens.peek() {
            Some(v) => v.position().start(),
            _ => {
                return Err(AssembleError::missing_token(
                    &[TokenType::OpenParenthesis],
                    &tokens.eof(),
                ))
            }
        };
        let mut params = Vec::new();
        let mut results = Vec::new();

        if let Some(typeidx) = try_expect_module::<TypeIndex>(tokens)? {
            loop {
                if let Some(param) = try_expect_module::<FuncTypeParam>(tokens)? {
                    params.extend(param.0)
                } else {
                    break;
                }
            }
            loop {
                if let Some(result) = try_expect_module::<FuncTypeResult>(tokens)? {
                    results.extend(result.0.as_slice());
                } else {
                    let token = tokens.get_raw(TokenPosition((
                        start as u32,
                        tokens.peek_last().unwrap().position().end() as u32,
                    )));
                    if params.len() > 0 || results.len() > 0 {
                        return Ok(TypeUse {
                            kind: TypeUseKind::Both(typeidx, FuncType { params, results }),
                            token,
                        });
                    } else {
                        return Ok(TypeUse {
                            kind: TypeUseKind::Index(typeidx),
                            token,
                        });
                    }
                }
            }
        }

        loop {
            if let Some(param) = try_expect_module::<FuncTypeParam>(tokens)? {
                params.extend(param.0)
            } else {
                break;
            }
        }
        loop {
            if let Some(result) = try_expect_module::<FuncTypeResult>(tokens)? {
                results.extend(result.0.as_slice());
            } else {
                let token = tokens.get_raw(TokenPosition((
                    start as u32,
                    tokens.peek_last().unwrap().position().end() as u32,
                )));
                return Ok(TypeUse {
                    kind: TypeUseKind::FuncType(FuncType { params, results }),
                    token,
                });
            }
        }
    }
}

/// (`local` _id_? _valtype_*)
#[derive(Debug)]
pub struct FuncTypeLocal(pub Vec<IdAndValtype>);

impl ModuleName for FuncTypeLocal {
    const IDENTIFIER: Keyword = Keyword::Local;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        match Identifier::try_expect(tokens)? {
            Some(id) => {
                let valtype = expect_valtype(tokens)?;

                expect(tokens, &[TokenType::CloseParenthesis])?;

                Ok(Self(
                    [IdAndValtype {
                        id: Some(id),
                        valtype,
                    }]
                    .into(),
                ))
            }
            None => {
                let mut valtypes = Vec::new();

                valtypes.push(IdAndValtype {
                    id: None,
                    valtype: expect_valtype(tokens)?,
                });

                loop {
                    let token = expect(
                        tokens,
                        &[TokenType::CloseParenthesis, TokenType::Identifier],
                    )?;
                    match token.token_type() {
                        TokenType::CloseParenthesis => return Ok(Self(valtypes)),

                        TokenType::Identifier => {
                            let valtype = ValType::from_str(token.source()).ok_or(
                                AssembleError::invalid_identifier(
                                    token.source(),
                                    token.position().into(),
                                ),
                            )?;
                            valtypes.push(IdAndValtype { id: None, valtype });
                        }

                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct TypeIndex(pub IndexToken);

impl ModuleName for TypeIndex {
    const IDENTIFIER: Keyword = Keyword::Type;

    fn from_tokens(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        let index = IndexToken::expect(tokens)?;

        expect(tokens, &[TokenType::CloseParenthesis])?;

        Ok(Self(index))
    }
}

///
#[derive(Debug)]
pub enum ExtVis {
    Import(ImportAbbr),
    Export(ExportAbbr),
}

impl ExtVis {
    pub fn try_expect(tokens: &mut TokenStream<Keyword>) -> Result<Option<Self>, AssembleError> {
        let mut vis = None;
        if let Some(import) = try_expect_module::<ImportAbbr>(tokens)? {
            vis = Some(ExtVis::Import(import));
        } else if let Some(export) = try_expect_module::<ExportAbbr>(tokens)? {
            vis = Some(ExtVis::Export(export));
        }
        Ok(vis)
    }
}
