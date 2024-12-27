//! Literal
use crate::*;
use ast::expect;
use keyword::Keyword;
use lexer::*;

#[derive(Debug)]
pub struct StringLiteral {
    value: String,
    position: TokenPosition,
}

impl StringLiteral {
    #[inline]
    pub fn get(&self) -> &str {
        &self.value
    }

    #[inline]
    pub fn position(&self) -> TokenPosition {
        self.position
    }

    pub fn expect(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        Self::_expect(tokens, false).map(|v| v.unwrap())
    }

    pub fn try_expect(tokens: &mut TokenStream<Keyword>) -> Result<Option<Self>, AssembleError> {
        Self::_expect(tokens, true)
    }

    fn _expect(
        tokens: &mut TokenStream<Keyword>,
        is_opt: bool,
    ) -> Result<Option<Self>, AssembleError> {
        let token = match expect(tokens, &[TokenType::StringLiteral]) {
            Ok(v) => v,
            Err(e) => {
                if is_opt {
                    return Ok(None);
                } else {
                    return Err(e);
                }
            }
        };
        let value = match token.string_literal() {
            Ok(v) => v.to_string(),
            Err(err) => return Err(AssembleError::invalid_string_literal(err, token.position())),
        };

        Ok(Some(Self {
            value,
            position: token.position(),
        }))
    }
}

impl ToString for StringLiteral {
    #[inline]
    fn to_string(&self) -> String {
        self.get().to_owned()
    }
}

#[derive(Debug)]
pub struct RawBytesLiteral {
    value: Vec<u8>,
    position: TokenPosition,
}

impl RawBytesLiteral {
    #[inline]
    pub fn get(&self) -> &[u8] {
        &self.value
    }

    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        self.value
    }

    #[inline]
    pub fn position(&self) -> TokenPosition {
        self.position
    }

    pub fn expect(tokens: &mut TokenStream<Keyword>) -> Result<Self, AssembleError> {
        Self::_expect(tokens, false).map(|v| v.unwrap())
    }

    pub fn try_expect(tokens: &mut TokenStream<Keyword>) -> Result<Option<Self>, AssembleError> {
        Self::_expect(tokens, true)
    }

    fn _expect(
        tokens: &mut TokenStream<Keyword>,
        is_opt: bool,
    ) -> Result<Option<Self>, AssembleError> {
        let token = match expect(tokens, &[TokenType::StringLiteral]) {
            Ok(v) => v,
            Err(e) => {
                if is_opt {
                    return Ok(None);
                } else {
                    return Err(e);
                }
            }
        };
        let value = match token.raw_bytes_literal(true) {
            Ok(v) => v,
            Err(err) => return Err(AssembleError::invalid_string_literal(err, token.position())),
        };

        Ok(Some(Self {
            value,
            position: token.position(),
        }))
    }
}

#[derive(Debug)]
pub struct NumericRawLiteral {
    source: String,
    is_neg: bool,
    skip: usize,
    radix: Radix,
    position: TokenPosition,
}

impl NumericRawLiteral {
    pub fn from_token<KEYWORD>(token: &Token<KEYWORD>) -> Result<Self, AssembleError>
    where
        KEYWORD: core::fmt::Debug + core::fmt::Display,
    {
        match token.token_type() {
            TokenType::NumericLiteral | TokenType::Uncategorized => {
                let radix = token.radix().ok_or(AssembleError::invalid_number(
                    token.source(),
                    token.position().into(),
                ))?;

                let is_neg = if token.source().starts_with("-") {
                    true
                } else {
                    false
                };

                Ok(Self {
                    source: token.source().to_owned(),
                    is_neg,
                    skip: radix.0,
                    radix: radix.1,
                    position: token.position(),
                })
            }

            _ => Err(AssembleError::missing_token(
                &[TokenType::NumericLiteral],
                token,
            )),
        }
    }

    fn _expect<TYPE, KEYWORD>(
        tokens: &mut TokenStream<KEYWORD>,
        is_opt: bool,
    ) -> Result<Option<Self>, AssembleError>
    where
        TYPE: IsSigned,
        KEYWORD: PartialEq + Copy + core::fmt::Debug + core::fmt::Display,
    {
        let token = match expect(
            tokens,
            if TYPE::is_signed() {
                &[
                    TokenType::Symbol('+'),
                    TokenType::Symbol('-'),
                    TokenType::NumericLiteral,
                ]
            } else {
                &[TokenType::NumericLiteral]
            },
        ) {
            Ok(v) => v,
            Err(e) => {
                if is_opt {
                    return Ok(None);
                } else {
                    return Err(e);
                }
            }
        };
        match token.token_type() {
            TokenType::Symbol('+') => match tokens.next_immed() {
                Some(token) => Self::from_token(&token).map(|v| Some(v)),
                None => {
                    return Err(AssembleError::missing_token(
                        &[TokenType::NumericLiteral],
                        &tokens.next().unwrap(),
                    ));
                }
            },
            TokenType::Symbol('-') => match tokens.next_immed() {
                Some(next) => {
                    let token = tokens.get_raw(TokenPosition((
                        token.position().start() as u32,
                        next.position().end() as u32,
                    )));
                    Self::from_token(&token.as_token::<KEYWORD>()).map(|v| Some(v))
                }
                None => {
                    return Err(AssembleError::missing_token(
                        &[TokenType::NumericLiteral],
                        &tokens.next().unwrap(),
                    ));
                }
            },
            TokenType::NumericLiteral => Self::from_token(&token).map(|v| Some(v)),
            _ => unreachable!(),
        }
    }

    fn parse_int_u(&self) -> Option<u64> {
        let mut acc = 0u64;
        let radix = self.radix.value() as u64;

        for ch in self.source.bytes().skip(self.skip) {
            if ch == b'_' {
                continue;
            }
            let delta = match ch {
                b'0'..=b'9' => (ch - b'0') as u64,
                b'A'..=b'F' => (ch - b'A' + 10) as u64,
                b'a'..=b'f' => (ch - b'a' + 10) as u64,
                _ => return None,
            };
            if delta > radix {
                return None;
            }
            acc = acc.checked_mul(radix)?.checked_add(delta)?;
        }

        Some(acc)
    }
}

pub trait Eval<TYPE, ERR> {
    fn eval(&self) -> Result<TYPE, ERR>;
}

impl Eval<u64, ()> for NumericRawLiteral {
    #[inline]
    fn eval(&self) -> Result<u64, ()> {
        self.parse_int_u().ok_or(())
    }
}

impl Eval<i64, ()> for NumericRawLiteral {
    #[inline]
    fn eval(&self) -> Result<i64, ()> {
        let abs = self
            .parse_int_u()
            .and_then(|v| v.try_into().ok())
            .map(|v: u64| v as i64)
            .ok_or(())?;
        if self.is_neg {
            Ok(0i64.wrapping_sub(abs))
        } else {
            Ok(abs)
        }
    }
}

impl Eval<u32, ()> for NumericRawLiteral {
    #[inline]
    fn eval(&self) -> Result<u32, ()> {
        self.parse_int_u().and_then(|v| v.try_into().ok()).ok_or(())
    }
}

impl Eval<i32, ()> for NumericRawLiteral {
    #[inline]
    fn eval(&self) -> Result<i32, ()> {
        let abs = self
            .parse_int_u()
            .and_then(|v| v.try_into().ok())
            .map(|v: u32| v as i32)
            .ok_or(())?;
        if self.is_neg {
            Ok(0i32.wrapping_sub(abs))
        } else {
            Ok(abs)
        }
    }
}

trait IsSigned {
    fn is_signed() -> bool;
}

impl IsSigned for u32 {
    #[inline]
    fn is_signed() -> bool {
        false
    }
}

impl IsSigned for u64 {
    #[inline]
    fn is_signed() -> bool {
        false
    }
}

impl IsSigned for i32 {
    #[inline]
    fn is_signed() -> bool {
        true
    }
}

impl IsSigned for i64 {
    #[inline]
    fn is_signed() -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(private_bounds)]
pub struct NumericLiteral<TYPE: IsSigned> {
    value: TYPE,
    position: TokenPosition,
}

#[allow(private_bounds)]
impl<TYPE: IsSigned + Copy> NumericLiteral<TYPE> {
    pub fn expect<KEYWORD>(tokens: &mut TokenStream<KEYWORD>) -> Result<Self, AssembleError>
    where
        NumericRawLiteral: Eval<TYPE, ()>,
        KEYWORD: PartialEq + Copy + core::fmt::Debug + core::fmt::Display,
    {
        Self::_expect(tokens, false).map(|v| v.unwrap())
    }

    pub fn try_expect<KEYWORD>(
        tokens: &mut TokenStream<KEYWORD>,
    ) -> Result<Option<Self>, AssembleError>
    where
        NumericRawLiteral: Eval<TYPE, ()>,
        KEYWORD: PartialEq + Copy + core::fmt::Debug + core::fmt::Display,
    {
        Self::_expect(tokens, true)
    }

    pub fn from_token<KEYWORD>(token: &Token<KEYWORD>) -> Result<Self, AssembleError>
    where
        NumericRawLiteral: Eval<TYPE, ()>,
        KEYWORD: PartialEq + Copy + core::fmt::Debug + core::fmt::Display,
    {
        let token = NumericRawLiteral::from_token(token)?;
        let position = token.position;
        let value = Eval::<TYPE, ()>::eval(&token)
            .map_err(|_| AssembleError::invalid_number(&token.source, position.into()))?;
        Ok(Self { value, position })
    }

    fn _expect<KEYWORD>(
        tokens: &mut TokenStream<KEYWORD>,
        is_opt: bool,
    ) -> Result<Option<Self>, AssembleError>
    where
        NumericRawLiteral: Eval<TYPE, ()>,
        KEYWORD: PartialEq + Copy + core::fmt::Debug + core::fmt::Display,
    {
        let token = match NumericRawLiteral::_expect::<TYPE, KEYWORD>(tokens, is_opt) {
            Ok(v) => match v {
                Some(v) => v,
                None => return Ok(None),
            },
            Err(e) => return Err(e),
        };

        let position = token.position;
        let value = Eval::<TYPE, ()>::eval(&token)
            .map_err(|_| AssembleError::invalid_number(&token.source, position.into()))?;
        Ok(Some(Self { value, position }))
    }

    #[inline]
    pub fn get(&self) -> TYPE {
        self.value
    }

    #[inline]
    pub fn position(&self) -> TokenPosition {
        self.position
    }
}
