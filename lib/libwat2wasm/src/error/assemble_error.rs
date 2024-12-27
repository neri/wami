use crate::*;
use ast::ModuleName;
use ast::identifier::Identifier;
use core::cmp;
use core::ops::Range;
use lexer::*;
use types::ValType;

#[derive(Debug)]
pub struct AssembleError {
    pub(crate) kind: AssembleErrorKind,
    pub(crate) explanation: Option<String>,
    pub(crate) position: ErrorPosition,
}

#[derive(Debug)]
pub enum AssembleErrorKind {
    LexerError(LexerErrorKind),

    SyntaxError,
    NameError,
    TypeMismatch,
    OutOfBounds,

    InternalError,
}

#[derive(Debug, PartialEq)]
pub enum ErrorPosition {
    CharAt(usize, usize),
    Range(TokenPosition),
    Unspecified,
}

impl AssembleError {
    #[inline]
    pub fn with_kind(kind: AssembleErrorKind, position: ErrorPosition) -> Self {
        Self {
            kind,
            explanation: None,
            position,
        }
    }

    #[inline]
    pub fn missing_token<T: core::fmt::Debug + core::fmt::Display>(
        expected: &[TokenType<T>],
        token: &Token<T>,
    ) -> Self {
        let explanation = Self::explanation_token_type_strings(expected)
            .map(|v| format!("Unexpected token {}. Expected {}.", token.token_type(), v,));
        AssembleError {
            kind: AssembleErrorKind::SyntaxError,
            explanation,
            position: token.position().into(),
        }
    }

    pub fn unexpected_keyword<KEYWORD>(expected: &[KEYWORD], token: &Token<KEYWORD>) -> Self
    where
        KEYWORD: core::fmt::Display + core::fmt::Debug,
    {
        let explanation = Self::explanation_keyword_strings(expected)
            .map(|v| format!("Unexpected keyword {}. Expected {}.", token.token_type(), v,));
        AssembleError {
            kind: AssembleErrorKind::SyntaxError,
            explanation,
            position: token.position().into(),
        }
    }

    pub fn invalid_mnemonic<KEYWORD>(token: &Token<KEYWORD>) -> Self
    where
        KEYWORD: core::fmt::Debug + core::fmt::Display,
    {
        let position = token.position().into();
        let explanation = Some(format!("Invalid mnemonic: {:?}", token.source(),));
        AssembleError {
            kind: AssembleErrorKind::SyntaxError,
            explanation,
            position,
        }
    }

    pub fn invalid_number(source: &str, position: ErrorPosition) -> Self {
        let explanation = Some(format!("Invalid number: {:?}", source));
        AssembleError {
            kind: AssembleErrorKind::SyntaxError,
            explanation,
            position,
        }
    }

    pub fn invalid_string_literal(err: StringLiteralError, position: TokenPosition) -> Self {
        let explanation: &str;
        let mut position = position;
        match err {
            StringLiteralError::NaT => explanation = "Not a string",
            StringLiteralError::InvalidCharSequence(index) => {
                explanation = "Invalid character sequence";
                position = TokenPosition::new_at(position.start() + index);
            }
            StringLiteralError::InvalidUnicodeChar(index) => {
                explanation = "Invalid unicode sequence";
                position = TokenPosition::new_at(position.start() + index);
            }
        }
        AssembleError {
            kind: AssembleErrorKind::SyntaxError,
            explanation: Some(explanation.to_owned()),
            position: position.into(),
        }
    }

    pub fn invalid_identifier(source: &str, position: ErrorPosition) -> Self {
        let explanation = Some(format!("Invalid identifier: {:?}", source));
        AssembleError {
            kind: AssembleErrorKind::SyntaxError,
            explanation,
            position,
        }
    }

    pub fn undefined_identifier(source: &str, position: ErrorPosition) -> Self {
        let explanation = Some(format!("Not found in this scope: {:?}", source));
        AssembleError {
            kind: AssembleErrorKind::NameError,
            explanation,
            position,
        }
    }

    #[inline]
    pub fn duplicated_identifier(identifier: &Identifier) -> Self {
        let explanation = Some(format!(
            "The identifier {:?} has already been defined",
            identifier.name()
        ));
        AssembleError {
            kind: AssembleErrorKind::NameError,
            explanation,
            position: identifier.position().into(),
        }
    }

    #[inline]
    pub fn duplicated_module<MODULE: ModuleName>(position: ErrorPosition) -> Self {
        let explanation = Some(format!(
            "The module {:?} has already been taken",
            MODULE::IDENTIFIER.as_str(),
        ));
        AssembleError {
            kind: AssembleErrorKind::SyntaxError,
            explanation,
            position: position,
        }
    }

    #[inline]
    pub fn out_of_bounds(explain: &str, position: ErrorPosition) -> Self {
        AssembleError {
            kind: AssembleErrorKind::OutOfBounds,
            explanation: Some(explain.to_owned()),
            position,
        }
    }

    pub fn check_types(
        expected: &[ValType],
        actual: &[ValType],
        position: ErrorPosition,
    ) -> Result<(), AssembleError> {
        (expected == actual)
            .then(|| ())
            .ok_or_else(|| AssembleError::type_mismatch(expected, actual, position))
    }

    #[inline]
    pub fn type_mismatch(
        expected: &[ValType],
        actual: &[ValType],
        position: ErrorPosition,
    ) -> Self {
        let explanation = Some(format!(
            "Type mismatch {}. expected {}",
            Self::explanation_valtype_strings(actual),
            Self::explanation_valtype_strings(expected),
        ));
        AssembleError {
            kind: AssembleErrorKind::TypeMismatch,
            explanation,
            position,
        }
    }

    #[inline]
    pub fn internal_inconsistency(explain: &str, position: ErrorPosition) -> Self {
        AssembleError {
            kind: AssembleErrorKind::InternalError,
            explanation: (explain.len() > 0).then(|| explain.to_owned()),
            position,
        }
    }

    pub fn explanation_valtype_strings(types: &[ValType]) -> String {
        if types.len() > 0 {
            let mut buf = Vec::new();
            for valtype in types {
                buf.push(valtype.as_str());
            }
            format!("({})", buf.join(", "))
        } else {
            "empty".to_owned()
        }
    }

    pub fn explanation_token_type_strings<T: core::fmt::Display + core::fmt::Debug>(
        types: &[TokenType<T>],
    ) -> Option<String> {
        match types.len() {
            0 => None,
            1 => Some(format!("{}", types[0])),
            len => {
                let mut tokens = types.iter().map(|v| format!("{}", v)).collect::<Vec<_>>();
                tokens.sort();
                let tokens = if len > 2 {
                    let (last, tokens) = tokens.split_last().unwrap();
                    [tokens.join(", "), last.clone()].join(" and ")
                } else {
                    tokens.join(" and ")
                };
                Some(tokens)
            }
        }
    }

    pub fn explanation_keyword_strings<KEYWORD>(types: &[KEYWORD]) -> Option<String>
    where
        KEYWORD: core::fmt::Display + core::fmt::Debug,
    {
        match types.len() {
            0 => None,
            1 => Some(format!("{}", types[0])),
            len => {
                let mut tokens = types.iter().map(|v| format!("{}", v)).collect::<Vec<_>>();
                tokens.sort();
                let tokens = if len > 2 {
                    let (last, tokens) = tokens.split_last().unwrap();
                    [tokens.join(", "), last.clone()].join(" and ")
                } else {
                    tokens.join(" and ")
                };
                Some(tokens)
            }
        }
    }

    #[inline]
    pub fn explanation(&self) -> Option<&str> {
        self.explanation.as_ref().map(|v| v.as_str())
    }

    #[inline]
    pub fn position(&self) -> &ErrorPosition {
        &self.position
    }

    pub fn to_detail_string(
        &self,
        file_name: &str,
        blob: &[u8],
        line_positions: &[(usize, usize)],
    ) -> String {
        let mut lines = Vec::new();
        lines.push(format!("error: {:?}", self.kind));

        match self.position() {
            ErrorPosition::Unspecified => {
                lines.push(format!("{}:", file_name));
            }
            ErrorPosition::CharAt(line, column) => {
                if *line > 0 && *column > 0 {
                    lines.push(format!("{}:{}:{}", file_name, line, column));
                }
            }
            ErrorPosition::Range(position) => {
                let range_start = position.start();
                let range_end = position.end();

                if let Ok(line_index) = line_positions.binary_search_by(|(line_start, line_end)| {
                    if range_start < *line_start {
                        cmp::Ordering::Greater
                    } else if range_start > *line_end {
                        cmp::Ordering::Less
                    } else {
                        cmp::Ordering::Equal
                    }
                }) {
                    let position = line_positions[line_index];
                    let line = line_index + 1;
                    let column = range_start - position.0 + 1;

                    lines.push(format!("{}:{}:{}", file_name, line, column));

                    if let Some(line) = blob
                        .get(position.0..position.1)
                        .and_then(|v| core::str::from_utf8(v).ok())
                    {
                        lines.push(format!("\n{}", line));

                        let leading = if column > 1 {
                            " ".repeat(column - 1)
                        } else {
                            "".to_string()
                        };
                        let cursor_end = range_end.min(position.1);
                        let cursor_len = cursor_end.saturating_sub(range_start);
                        let cursor = if cursor_len > 1 {
                            "^".repeat(cursor_len)
                        } else {
                            "^".to_string()
                        };

                        lines.push(format!("{}{}", leading, cursor));
                    }
                }
            }
        }

        if let Some(explanation) = self.explanation() {
            lines.push(format!("\n{}", explanation));
        }

        lines.join("\n")
    }

    #[inline]
    pub fn check_index<T>(
        index: T,
        bounds: Range<T>,
        position: TokenPosition,
    ) -> Result<T, AssembleError>
    where
        T: PartialOrd + core::fmt::Debug,
    {
        if bounds.contains(&index) {
            Ok(index)
        } else {
            let explanation = Some(format!(
                "'{:?}' is an invalid value. The valid range is between {:?} and {:?}",
                index, bounds.start, bounds.end,
            ));
            Err(AssembleError {
                kind: AssembleErrorKind::OutOfBounds,
                explanation,
                position: position.into(),
            })
        }
    }
}

impl From<TokenPosition> for ErrorPosition {
    #[inline]
    fn from(value: TokenPosition) -> Self {
        ErrorPosition::Range(value)
    }
}

impl From<LexerError> for AssembleError {
    #[inline]
    fn from(e: LexerError) -> Self {
        Self::with_kind(
            AssembleErrorKind::LexerError(e.kind()),
            ErrorPosition::CharAt(e.position().0, e.position().1),
        )
    }
}
