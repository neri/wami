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
    position: TokenPosition,

    is_neg: bool,
    skip: usize,
    radix: Radix,

    float_value: Option<f64>,
}

impl NumericRawLiteral {
    #[allow(private_bounds)]
    pub fn from_token<KEYWORD, TYPE>(token: &Token<KEYWORD>) -> Result<Self, AssembleError>
    where
        KEYWORD: core::fmt::Debug + core::fmt::Display,
        TYPE: IsSigned + IsFloat,
    {
        if TYPE::is_float() {
            let float_value = token.try_parse_float().map_err(|v| {
                let mut position = token.position();
                position.0.0 += v as u32;
                AssembleError::invalid_number(token.source(), position.into())
            })?;

            Ok(Self {
                source: token.source().to_owned(),
                position: token.position(),
                float_value: Some(float_value),
                is_neg: false,
                skip: 0,
                radix: Radix::Dec,
            })
        } else {
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
                position: token.position(),
                is_neg,
                skip: radix.0,
                radix: radix.1,
                float_value: None,
            })
        }
    }

    fn _expect<TYPE, KEYWORD>(
        tokens: &mut TokenStream<KEYWORD>,
        is_opt: bool,
    ) -> Result<Option<Self>, AssembleError>
    where
        TYPE: IsSigned + IsFloat,
        KEYWORD: PartialEq + Copy + core::fmt::Debug + core::fmt::Display,
    {
        let canonical_token = if TYPE::is_float() {
            TokenType::FloatingNumberLiteral
        } else {
            TokenType::NumericLiteral
        };
        let expect_tokens: &[TokenType<KEYWORD>] = if TYPE::is_signed() {
            &[
                TokenType::Symbol('+'),
                TokenType::Symbol('-'),
                canonical_token,
            ]
        } else {
            &[canonical_token]
        };
        let token = match expect(tokens, expect_tokens) {
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
                Some(token) => Self::from_token::<KEYWORD, TYPE>(&token).map(|v| Some(v)),
                None => {
                    return Err(AssembleError::missing_token(
                        &[canonical_token],
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
                    Self::from_token::<KEYWORD, TYPE>(&token.as_token::<KEYWORD>()).map(|v| Some(v))
                }
                None => {
                    return Err(AssembleError::missing_token(
                        &[canonical_token],
                        &tokens.next().unwrap(),
                    ));
                }
            },
            TokenType::NumericLiteral => Self::from_token::<KEYWORD, TYPE>(&token).map(|v| Some(v)),
            TokenType::FloatingNumberLiteral => {
                Self::from_token::<KEYWORD, TYPE>(&token).map(|v| Some(v))
            }
            _ => unreachable!(),
        }
    }

    fn parse_int_u(&self) -> Option<u64> {
        if self.float_value.is_some() {
            return None;
        }
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

    #[inline]
    pub fn try_parse_float(&self) -> Option<f64> {
        self.float_value
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

impl Eval<f32, ()> for NumericRawLiteral {
    #[inline]
    fn eval(&self) -> Result<f32, ()> {
        self.try_parse_float().map(|v| v as f32).ok_or(())
    }
}

impl Eval<f64, ()> for NumericRawLiteral {
    #[inline]
    fn eval(&self) -> Result<f64, ()> {
        self.try_parse_float().ok_or(())
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

impl IsSigned for f32 {
    #[inline]
    fn is_signed() -> bool {
        true
    }
}

impl IsSigned for f64 {
    #[inline]
    fn is_signed() -> bool {
        true
    }
}

trait IsFloat {
    fn is_float() -> bool;
}

impl IsFloat for f32 {
    #[inline]
    fn is_float() -> bool {
        true
    }
}

impl IsFloat for f64 {
    #[inline]
    fn is_float() -> bool {
        true
    }
}

impl IsFloat for u32 {
    #[inline]
    fn is_float() -> bool {
        false
    }
}
impl IsFloat for u64 {
    #[inline]
    fn is_float() -> bool {
        false
    }
}
impl IsFloat for i32 {
    #[inline]
    fn is_float() -> bool {
        false
    }
}
impl IsFloat for i64 {
    #[inline]
    fn is_float() -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(private_bounds)]
pub struct NumericLiteral<TYPE: IsSigned> {
    value: TYPE,
    position: TokenPosition,
}

#[allow(private_bounds)]
impl<TYPE: IsSigned + IsFloat + Copy> NumericLiteral<TYPE> {
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
        let token = NumericRawLiteral::from_token::<KEYWORD, TYPE>(token)?;
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
