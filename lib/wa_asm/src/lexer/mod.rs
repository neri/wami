//! Lexical analyzer

use crate::*;
use core::{cmp, fmt, fmt::Write, ops::Range, str};

mod utf8;
use utf8::*;

#[cfg(test)]
mod tests;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType<KEYWORD> {
    /// End of File
    Eof,
    /// White space
    ///
    /// Currently, this token is removed in the normal tokenization process
    Whitespace,
    /// New Line
    ///
    /// Currently, this token is removed in the normal tokenization process
    NewLine,
    /// Line Comment
    LineComment,
    /// Block Comment
    BlockComment,
    /// Known Keyword
    Keyword(KEYWORD),
    /// Identifier
    Identifier,
    /// Open Parenthesis
    OpenParenthesis,
    /// Close Parenthesis
    CloseParenthesis,
    /// Other Symbolic Characters
    Symbol(char),
    /// Numeric Literal
    NumericLiteral,
    /// Floating Number Literal
    FloatingNumberLiteral,
    /// Broken Numeric Literal
    BrokenNumber,
    /// String Literal
    StringLiteral,
    /// Broken String Literal
    BrokenString,
    /// Uncategorized
    Uncategorized,
}

impl<KEYWORD> TokenType<KEYWORD> {
    // pub const OPEN_BRACE: Self = Self::Symbol('{');
    // pub const CLOSE_BRACE: Self = Self::Symbol('}');
    // pub const OPEN_BRACKET: Self = Self::Symbol('[');
    // pub const CLOSE_BRACKET: Self = Self::Symbol(']');

    #[inline]
    pub fn is_ignorable(&self) -> bool {
        match self {
            TokenType::NewLine
            | TokenType::Whitespace
            | TokenType::LineComment
            | TokenType::BlockComment => true,
            _ => false,
        }
    }

    #[inline]
    pub fn convert<KEYWORD2>(&self) -> TokenType<KEYWORD2> {
        match self {
            TokenType::Keyword(_) | TokenType::Identifier => TokenType::Identifier,
            TokenType::Whitespace => TokenType::Whitespace,
            TokenType::Eof => TokenType::Eof,
            TokenType::LineComment => TokenType::LineComment,
            TokenType::BlockComment => TokenType::BlockComment,
            TokenType::NewLine => TokenType::NewLine,
            TokenType::OpenParenthesis => TokenType::OpenParenthesis,
            TokenType::CloseParenthesis => TokenType::CloseParenthesis,
            TokenType::Symbol(v) => TokenType::Symbol(*v),
            TokenType::NumericLiteral => TokenType::NumericLiteral,
            TokenType::FloatingNumberLiteral => TokenType::FloatingNumberLiteral,
            TokenType::BrokenNumber => TokenType::BrokenNumber,
            TokenType::StringLiteral => TokenType::StringLiteral,
            TokenType::BrokenString => TokenType::BrokenString,
            TokenType::Uncategorized => TokenType::Uncategorized,
        }
    }
}

impl<KEYWORD: core::fmt::Display + core::fmt::Debug> core::fmt::Display for TokenType<KEYWORD>
where
    Self: Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::Eof => f.write_str("EndOfFile"),
            TokenType::StringLiteral => f.write_str("StringLiteral"),
            TokenType::Keyword(keyword) => write!(f, "{}", keyword),
            TokenType::Symbol(symbol) => f.write_char(*symbol),
            TokenType::OpenParenthesis => f.write_char('('),
            TokenType::CloseParenthesis => f.write_char(')'),
            _ => (self as &dyn core::fmt::Debug).fmt(f),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Radix {
    Bin,
    Oct,
    Dec,
    Hex,
}

impl Radix {
    pub const fn is_valid_chars(&self, ch: char) -> bool {
        match self {
            Radix::Bin => matches!(ch, '0'..='1' | '_'),
            Radix::Oct => matches!(ch, '0'..='7' | '_'),
            Radix::Dec => matches!(ch, '0'..='9' | '_'),
            Radix::Hex => matches!(ch, '0'..='9' | 'A'..='F' | 'a'..='f' | '_'),
        }
    }

    #[inline]
    pub const fn value(&self) -> u32 {
        match self {
            Radix::Bin => 2,
            Radix::Oct => 8,
            Radix::Dec => 10,
            Radix::Hex => 16,
        }
    }

    #[inline]
    pub const fn is_invalid_chars(&self, ch: char) -> bool {
        if self.is_valid_chars(ch) {
            false
        } else {
            ch.is_ascii_alphanumeric()
        }
    }
}

pub struct Lexer<KEYWORD>
where
    KEYWORD: Clone + Copy,
{
    fragments: Vec<TokenFragment<KEYWORD>>,
    line_positions: Vec<(usize, usize)>,
    line_number: usize,
    column_number: usize,
    phase: LexerPhase,
    start: usize,
    start_column: usize,
}

impl<KEYWORD: Copy + Clone> Lexer<KEYWORD> {
    pub const MAX_FILE_SIZE: usize = 0x00FF_FFFF;

    #[inline]
    pub fn with_slice<V>(src: &[u8], keyword_resolver: V) -> Result<Tokens<KEYWORD>, LexerError>
    where
        V: Fn(&str) -> Option<KEYWORD>,
    {
        let lexer = Self {
            fragments: Default::default(),
            line_positions: Default::default(),
            line_number: Default::default(),
            column_number: Default::default(),
            phase: Default::default(),
            start: Default::default(),
            start_column: Default::default(),
        };
        lexer._analyze(Arc::new(src.to_vec()), keyword_resolver)
    }

    #[inline]
    pub fn parse<V>(src: Arc<Vec<u8>>, keyword_resolver: V) -> Result<Tokens<KEYWORD>, LexerError>
    where
        V: Fn(&str) -> Option<KEYWORD>,
    {
        let lexer = Self {
            fragments: Default::default(),
            line_positions: Default::default(),
            line_number: Default::default(),
            column_number: Default::default(),
            phase: Default::default(),
            start: Default::default(),
            start_column: Default::default(),
        };
        lexer._analyze(src, keyword_resolver)
    }

    fn _analyze<V>(
        mut self,
        src: Arc<Vec<u8>>,
        keyword_resolver: V,
    ) -> Result<Tokens<KEYWORD>, LexerError>
    where
        V: Fn(&str) -> Option<KEYWORD>,
    {
        if src.len() > Self::MAX_FILE_SIZE {
            return Err(LexerError::new(LexerErrorKind::TooLargeFile, 0, 0));
        }

        self.start = 0;
        self.start_column = 0;
        self.line_number = 1;
        self.column_number = 1;
        self.phase = LexerPhase::default();
        self.fragments.clear();
        self.line_positions.clear();

        let mut line_start = 0;
        let mut next_finalize = false;
        let mut utf = Utf8StateMachine::new();
        let mut prev_index = 0;
        let mut current_index = 0;
        let mut skip_newline = false;
        let mut next_escaped = false;
        for (index, c) in src.iter().enumerate() {
            if utf.len() == 0 {
                current_index = index;
            }
            utf.push(*c).map_err(|_| {
                LexerError::new(
                    LexerErrorKind::InvalidChar,
                    self.line_number,
                    self.column_number,
                )
            })?;
            if utf.needs_trail_bytes() {
                continue;
            }
            let ch = utf.take_valid_char().ok_or(LexerError::new(
                LexerErrorKind::InvalidChar,
                self.line_number,
                self.column_number,
            ))?;

            if next_finalize {
                self._finalize_phase(&src, current_index, false, &keyword_resolver);
                next_finalize = false;
            }

            let sb = str::from_utf8(&src[self.start..current_index]).unwrap();

            match self.phase {
                LexerPhase::WhiteSpace => {
                    self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                }
                LexerPhase::Identifier => {
                    if self.is_id_trail_char(ch) {
                    } else {
                        self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                    }
                }
                LexerPhase::Zero => match ch {
                    'b' | 'B' => self.phase = LexerPhase::Numeric(Radix::Bin),
                    'o' | 'O' => self.phase = LexerPhase::Numeric(Radix::Oct),
                    'x' | 'X' => self.phase = LexerPhase::Numeric(Radix::Hex),
                    '.' => self.phase = LexerPhase::Floating(FloatingPhase::Dot),
                    'e' | 'E' => self.phase = LexerPhase::Floating(FloatingPhase::E),
                    _ => {
                        let radix = Radix::Dec;
                        if radix.is_valid_chars(ch) {
                            self.phase = LexerPhase::Numeric(radix)
                        } else if radix.is_invalid_chars(ch) {
                            self.phase = LexerPhase::BrokenNumber;
                        } else {
                            self._next_phase(
                                &src,
                                current_index,
                                prev_index,
                                ch,
                                &keyword_resolver,
                            );
                        }
                    }
                },
                LexerPhase::Numeric(radix) => {
                    if radix.is_valid_chars(ch) {
                        //
                    } else if radix == Radix::Dec {
                        match ch {
                            '.' => self.phase = LexerPhase::Floating(FloatingPhase::Dot),
                            'e' | 'E' => self.phase = LexerPhase::Floating(FloatingPhase::E),
                            _ => {
                                if radix.is_invalid_chars(ch) {
                                    self.phase = LexerPhase::BrokenNumber;
                                } else {
                                    self._next_phase(
                                        &src,
                                        current_index,
                                        prev_index,
                                        ch,
                                        &keyword_resolver,
                                    );
                                }
                            }
                        }
                    } else {
                        if radix.is_invalid_chars(ch) {
                            self.phase = LexerPhase::BrokenNumber;
                        } else {
                            self._next_phase(
                                &src,
                                current_index,
                                prev_index,
                                ch,
                                &keyword_resolver,
                            );
                        }
                    }
                }
                LexerPhase::Floating(phase) => match phase {
                    FloatingPhase::Dot => match ch {
                        '0'..='9' => self.phase = LexerPhase::Floating(FloatingPhase::Fraction),
                        '.' => self.phase = LexerPhase::NumericAndDoubleDot,
                        _ => {
                            if Radix::Dec.is_invalid_chars(ch) {
                                self.phase = LexerPhase::BrokenNumber;
                            } else {
                                self._next_phase(
                                    &src,
                                    current_index,
                                    prev_index,
                                    ch,
                                    &keyword_resolver,
                                );
                            }
                        }
                    },
                    FloatingPhase::Fraction => match ch {
                        '0'..='9' => {}
                        'e' | 'E' => {
                            self.phase = LexerPhase::Floating(FloatingPhase::E);
                        }
                        _ => {
                            if Radix::Dec.is_invalid_chars(ch) {
                                self.phase = LexerPhase::BrokenNumber;
                            } else {
                                self._next_phase(
                                    &src,
                                    current_index,
                                    prev_index,
                                    ch,
                                    &keyword_resolver,
                                );
                            }
                        }
                    },
                    FloatingPhase::E => match ch {
                        '+' | '-' => {
                            self.phase = LexerPhase::Floating(FloatingPhase::ExpSign);
                        }
                        '0'..='9' => {
                            self.phase = LexerPhase::Floating(FloatingPhase::Exponent);
                        }
                        _ => {
                            if Radix::Dec.is_invalid_chars(ch) {
                                self.phase = LexerPhase::BrokenNumber;
                            } else {
                                self._next_phase(
                                    &src,
                                    current_index,
                                    prev_index,
                                    ch,
                                    &keyword_resolver,
                                );
                            }
                        }
                    },
                    FloatingPhase::ExpSign => match ch {
                        '0'..='9' => {
                            self.phase = LexerPhase::Floating(FloatingPhase::Exponent);
                        }
                        _ => {
                            if Radix::Dec.is_invalid_chars(ch) {
                                self.phase = LexerPhase::BrokenNumber;
                            } else {
                                self._next_phase(
                                    &src,
                                    current_index,
                                    prev_index,
                                    ch,
                                    &keyword_resolver,
                                );
                            }
                        }
                    },
                    FloatingPhase::Exponent => match ch {
                        '0'..='9' => {}
                        _ => {
                            if Radix::Dec.is_invalid_chars(ch) {
                                self.phase = LexerPhase::BrokenNumber;
                            } else {
                                self._next_phase(
                                    &src,
                                    current_index,
                                    prev_index,
                                    ch,
                                    &keyword_resolver,
                                );
                            }
                        }
                    },
                },
                LexerPhase::NumericAndDoubleDot => {
                    self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                }
                LexerPhase::BrokenNumber => {
                    if ch.is_ascii_alphanumeric() {
                    } else {
                        self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                    }
                }
                LexerPhase::Semicolon => match ch {
                    ';' => {
                        // `;;` - line comment
                        self.phase = LexerPhase::LineComment;
                    }
                    _ => {
                        self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                    }
                },
                LexerPhase::OpenParenthesis => match ch {
                    ';' => {
                        // `(;` - block comment
                        self.phase = LexerPhase::BlockComment;
                    }
                    _ => {
                        self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                    }
                },
                LexerPhase::CloseParenthesis => {
                    self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                }
                LexerPhase::BlockComment => {
                    if sb.len() >= 4 && sb.ends_with(";)") {
                        self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                        skip_newline = true;
                    }
                }
                LexerPhase::LineComment => {
                    if ch == '\n' {
                        self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                        skip_newline = true;
                    }
                }
                LexerPhase::Symbol => {
                    self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                }
                LexerPhase::DoubleQuote => {
                    if next_escaped {
                        next_escaped = false
                    } else if ch == '\\' {
                        next_escaped = true
                    } else if ch == '"' {
                        next_finalize = true;
                    }
                }
                LexerPhase::UnicodeEntity => {
                    self._next_phase(&src, current_index, prev_index, ch, &keyword_resolver);
                }
                LexerPhase::UncontinuableChar => {
                    return Err(LexerError::new(
                        LexerErrorKind::InvalidChar,
                        self.line_number,
                        self.column_number,
                    ));
                }
            }

            match ch {
                '\n' => {
                    let line_end = current_index;
                    self.line_positions.push((line_start, line_end));

                    if skip_newline {
                        skip_newline = false;
                    } else if !matches!(self.phase, LexerPhase::BlockComment) {
                        // self.fragments.push(TokenFragment::new(
                        //     TokenType::NewLine,
                        //     self.start,
                        //     line_end,
                        // ));
                    }

                    self.column_number = 1;
                    self.line_number += 1;
                    line_start = line_end + 1;
                }

                _ => {
                    self.column_number += 1;
                }
            }

            prev_index = current_index;
        }
        if utf.needs_trail_bytes() {
            return Err(LexerError::new(
                LexerErrorKind::UnexpectedEof,
                self.line_number,
                self.column_number,
            ));
        }
        self._finalize_phase(&src, src.len(), !next_finalize, &keyword_resolver);

        let last = TokenFragment::new(TokenType::Eof, src.len(), src.len());
        self.fragments.push(last);

        let Self {
            fragments,
            line_positions,
            line_number: _,
            column_number: _,
            phase: _,
            start: _,
            start_column: _,
        } = self;

        Ok(Tokens {
            arc_buffer: src.clone(),
            fragments: Arc::new(fragments),
            lines: Arc::new(line_positions),
            last,
        })
    }

    fn _next_phase<V>(
        &mut self,
        src: &[u8],
        current_index: usize,
        prev_index: usize,
        ch: char,
        keyword_resolver: V,
    ) where
        V: Fn(&str) -> Option<KEYWORD>,
    {
        if current_index > prev_index {
            self._finalize_phase(src, current_index, false, keyword_resolver);
        }
        let next_phase = self._next_phase_by_char(ch);
        self.start = current_index;
        self.start_column = self.column_number;
        self.phase = next_phase;
    }

    fn _finalize_phase<V>(
        &mut self,
        src: &[u8],
        position_end: usize,
        is_eof: bool,
        keyword_resolver: V,
    ) where
        V: Fn(&str) -> Option<KEYWORD>,
    {
        let sb = str::from_utf8(&src[self.start..position_end]).unwrap();

        match self.phase {
            LexerPhase::WhiteSpace => {}

            LexerPhase::Identifier => {
                if let Some(keyword) = keyword_resolver(sb) {
                    self.fragments.push(TokenFragment::new(
                        TokenType::Keyword(keyword),
                        self.start,
                        position_end,
                    ));
                } else {
                    self.fragments.push(TokenFragment::new(
                        TokenType::Identifier,
                        self.start,
                        position_end,
                    ));
                }
            }
            LexerPhase::Zero => {
                self.fragments.push(TokenFragment::new(
                    TokenType::NumericLiteral,
                    self.start,
                    position_end,
                ));
            }
            LexerPhase::Numeric(_radix) => {
                self.fragments.push(TokenFragment::new(
                    TokenType::NumericLiteral,
                    self.start,
                    position_end,
                ));
            }
            LexerPhase::Floating(phase) => match phase {
                FloatingPhase::Dot | FloatingPhase::E | FloatingPhase::ExpSign => {
                    self.fragments.push(TokenFragment::new(
                        TokenType::BrokenNumber,
                        self.start,
                        position_end,
                    ));
                }
                FloatingPhase::Fraction | FloatingPhase::Exponent => {
                    self.fragments.push(TokenFragment::new(
                        TokenType::FloatingNumberLiteral,
                        self.start,
                        position_end,
                    ));
                }
            },
            LexerPhase::NumericAndDoubleDot => {
                self.fragments.push(TokenFragment::new(
                    TokenType::NumericLiteral,
                    self.start,
                    position_end - 2,
                ));
                for position in position_end - 2..position_end {
                    self.fragments.push(TokenFragment::new(
                        TokenType::Symbol('.'),
                        position,
                        position + 1,
                    ));
                }
            }
            LexerPhase::BrokenNumber => {
                self.fragments.push(TokenFragment::new(
                    TokenType::BrokenNumber,
                    self.start,
                    position_end,
                ));
            }
            LexerPhase::Symbol => {
                let ch = sb.chars().next().unwrap();
                self.fragments.push(TokenFragment::new(
                    TokenType::Symbol(ch),
                    self.start,
                    position_end,
                ));
            }
            LexerPhase::Semicolon => {
                self.fragments.push(TokenFragment::new(
                    TokenType::Symbol(';'),
                    self.start,
                    position_end,
                ));
            }
            LexerPhase::OpenParenthesis => {
                self.fragments.push(TokenFragment::new(
                    TokenType::OpenParenthesis,
                    self.start,
                    position_end,
                ));
            }
            LexerPhase::CloseParenthesis => {
                self.fragments.push(TokenFragment::new(
                    TokenType::CloseParenthesis,
                    self.start,
                    position_end,
                ));
            }
            LexerPhase::LineComment => {
                self.fragments.push(TokenFragment::new(
                    TokenType::LineComment,
                    self.start,
                    position_end,
                ));
            }
            LexerPhase::BlockComment => {
                self.fragments.push(TokenFragment::new(
                    TokenType::BlockComment,
                    self.start,
                    position_end,
                ));
            }
            LexerPhase::DoubleQuote => {
                if is_eof {
                    self.fragments.push(TokenFragment::new(
                        TokenType::BrokenString,
                        self.start,
                        position_end,
                    ));
                } else {
                    assert!(position_end >= self.start + 2);
                    let start = src[self.start];
                    let end = src[position_end - 1];
                    assert_eq!(start, end);
                    self.fragments.push(TokenFragment::new(
                        TokenType::StringLiteral,
                        self.start,
                        position_end,
                    ));
                }
            }
            LexerPhase::UnicodeEntity => {
                self.fragments.push(TokenFragment::new(
                    TokenType::Uncategorized,
                    self.start,
                    position_end,
                ));
            }

            LexerPhase::UncontinuableChar => unreachable!(),
        }
        self.phase = LexerPhase::WhiteSpace;
    }

    #[inline]
    fn _next_phase_by_char(&self, ch: char) -> LexerPhase {
        if Self::is_id_leading_char(ch) {
            LexerPhase::Identifier
        } else if Self::is_whitespace(ch) {
            LexerPhase::WhiteSpace
        } else {
            match ch {
                '0' => LexerPhase::Zero,
                '1'..='9' => LexerPhase::Numeric(Radix::Dec),
                '\x22' => LexerPhase::DoubleQuote,
                ';' => LexerPhase::Semicolon,
                '(' => LexerPhase::OpenParenthesis,
                ')' => LexerPhase::CloseParenthesis,
                '\u{FEFF}' | '\u{FFFE}' => LexerPhase::UncontinuableChar,
                _ => {
                    if ch.is_ascii_graphic() {
                        LexerPhase::Symbol
                    } else {
                        LexerPhase::UnicodeEntity
                    }
                }
            }
        }
    }

    pub fn is_whitespace(ch: char) -> bool {
        match ch {
            '\x09'..='\x0D' | '\x20' => true,

            '\u{0085}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}' => true,

            _ => false,
        }
    }

    #[inline]
    pub fn is_numeric(ch: char) -> bool {
        matches!(ch, '0'..='9')
    }

    #[inline]
    pub fn is_id_leading_char(ch: char) -> bool {
        matches!(ch, 'A'..='Z' | 'a'..='z' | '_' | '$')
    }

    #[inline]
    pub fn is_id_trail_char(&self, ch: char) -> bool {
        match ch {
            ' ' | '"' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' => false,
            _ => ch.is_ascii_graphic(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum LexerPhase {
    #[default]
    WhiteSpace,

    Identifier,
    Symbol,

    Zero,
    Numeric(Radix),
    Floating(FloatingPhase),
    /// numeric and ".."
    NumericAndDoubleDot,
    BrokenNumber,

    /// `;` - maybe comment
    Semicolon,

    /// `(` - maybe comment
    OpenParenthesis,
    CloseParenthesis,

    LineComment,
    BlockComment,

    DoubleQuote,

    UnicodeEntity,

    /// Non-tokenizable characters
    UncontinuableChar,
}

#[derive(Debug, Clone, Copy)]
enum FloatingPhase {
    Dot,
    Fraction,
    E,
    ExpSign,
    Exponent,
}

#[derive(Debug, Clone, Copy)]
pub struct TokenFragment<KEYWORD>
where
    KEYWORD: Copy + Clone,
{
    token_type: TokenType<KEYWORD>,
    position: TokenPosition,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TokenPosition(pub (u32, u32));

impl TokenPosition {
    #[inline]
    pub fn start(&self) -> usize {
        (self.0).0 as usize
    }

    #[inline]
    pub fn end(&self) -> usize {
        (self.0).1 as usize
    }

    #[inline]
    pub const fn new_at(position: usize) -> Self {
        Self((position as u32, position as u32))
    }

    #[inline]
    pub const fn empty() -> Self {
        Self((0, 0))
    }

    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start()..self.end()
    }

    #[inline]
    pub fn merged(&self, other: &Self) -> Self {
        Self(((self.0).0.min((other.0).0), (self.0).1.max((other.0).1)))
    }

    #[inline]
    pub fn is_continuous(&self, next: &Self) -> bool {
        self.end() == next.start()
    }
}

impl core::fmt::Debug for TokenPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(start: {}, end: {})", self.0.0, self.0.1)
    }
}

impl<KEYWORD: Copy + Clone> TokenFragment<KEYWORD> {
    #[inline]
    pub fn new(token_type: TokenType<KEYWORD>, file_start: usize, file_end: usize) -> Self {
        Self {
            token_type,
            position: TokenPosition((file_start as u32, file_end as u32)),
        }
    }
}

#[derive(Debug)]
pub struct LexerError {
    kind: LexerErrorKind,
    line: usize,
    column: usize,
}

impl LexerError {
    #[inline]
    pub fn new(kind: LexerErrorKind, line: usize, column: usize) -> Self {
        Self { kind, line, column }
    }

    #[inline]
    pub const fn kind(&self) -> LexerErrorKind {
        self.kind
    }

    #[inline]
    pub const fn line(&self) -> usize {
        self.line
    }

    #[inline]
    pub const fn column(&self) -> usize {
        self.column
    }

    #[inline]
    pub const fn position(&self) -> (usize, usize) {
        (self.line, self.column)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexerErrorKind {
    InvalidChar,
    UnexpectedEof,
    TooLargeFile,
}

pub struct Token<KEYWORD> {
    arc_buffer: Arc<Vec<u8>>,
    token_type: TokenType<KEYWORD>,
    position: TokenPosition,
}

impl<KEYWORD> Token<KEYWORD> {
    #[inline]
    pub fn eof() -> Self {
        Self {
            arc_buffer: Arc::new(Vec::new()),
            token_type: TokenType::Eof,
            position: TokenPosition::empty(),
        }
    }

    #[inline]
    #[track_caller]
    fn _source(arc_buffer: &Arc<Vec<u8>>, range: Range<usize>) -> &str {
        arc_buffer
            .get(range.clone())
            .map(|v| unsafe { core::str::from_utf8_unchecked(v) })
            .unwrap_or_default()
    }

    #[inline]
    #[track_caller]
    pub fn source(&self) -> &str {
        Self::_source(&self.arc_buffer, self.position.range())
    }

    #[inline]
    pub fn token_type(&self) -> &TokenType<KEYWORD> {
        &self.token_type
    }

    #[inline]
    pub fn position(&self) -> TokenPosition {
        self.position
    }

    #[inline]
    pub fn is_continuous(&self, next: &Self) -> bool {
        self.position().is_continuous(&next.position())
    }

    pub fn convert<F, KEYWORD2>(&self, keyword_resolver: F) -> Token<KEYWORD2>
    where
        F: Fn(&str) -> Option<KEYWORD2>,
        KEYWORD: Copy + Clone + PartialEq,
        KEYWORD2: PartialEq,
    {
        match self.token_type() {
            TokenType::Keyword(_) | TokenType::Identifier => {
                if let Some(key2) = keyword_resolver(self.source()) {
                    Token::<KEYWORD2> {
                        arc_buffer: self.arc_buffer.clone(),
                        token_type: TokenType::Keyword(key2),
                        position: self.position,
                    }
                } else {
                    Token::<KEYWORD2> {
                        arc_buffer: self.arc_buffer.clone(),
                        token_type: TokenType::Identifier,
                        position: self.position,
                    }
                }
            }
            _ => Token::<KEYWORD2> {
                arc_buffer: self.arc_buffer.clone(),
                token_type: self.token_type.convert(),
                position: self.position,
            },
        }
    }

    pub fn radix(&self) -> Option<(usize, Radix)> {
        let mut chars = self.source().chars();
        match chars.next()? {
            '+' | '-' => match chars.next()? {
                '0' => match chars.next() {
                    Some(next) => match next {
                        'b' | 'B' => Some((3, Radix::Bin)),
                        'o' | 'O' => Some((3, Radix::Oct)),
                        'x' | 'X' => Some((3, Radix::Hex)),
                        '0'..='9' => Some((1, Radix::Dec)),
                        _ => None,
                    },
                    None => Some((1, Radix::Dec)),
                },
                '1'..='9' => Some((1, Radix::Dec)),
                _ => None,
            },
            '0' => match chars.next() {
                Some(next) => match next {
                    'b' | 'B' => Some((2, Radix::Bin)),
                    'o' | 'O' => Some((2, Radix::Oct)),
                    'x' | 'X' => Some((2, Radix::Hex)),
                    '0'..='9' => Some((0, Radix::Dec)),
                    _ => None,
                },
                None => Some((0, Radix::Dec)),
            },
            '1'..='9' => Some((0, Radix::Dec)),
            _ => None,
        }
    }

    pub fn raw_bytes_literal(&self, allow_binary: bool) -> Result<Vec<u8>, StringLiteralError> {
        #[derive(Debug, PartialEq)]
        enum Phase {
            Default,
            /// escaped by back slash (`\`)
            Escaped,
            /// '/'_h_: raw byte value, next is one of `0-9A-F`
            RawByte,
            /// '\u': unicode literal, next is `{`
            UnicodeStart,
            /// '\u{': unicode literal, next is one of `0-9A-F` or '}'
            Unicode,
        }
        let TokenType::StringLiteral = *self.token_type() else {
            return Err(StringLiteralError::NaT);
        };
        let mut phase = Phase::Default;
        let source = &self.source()[1..self.source().len() - 1];
        if self.source().find('\\').is_none() {
            Ok(source.as_bytes().to_vec())
        } else {
            let position_start = 1;
            let mut code_buf = Vec::with_capacity(5);
            let mut code_start_position = 0;
            let mut sb = Vec::new();
            for (index, ch) in source.bytes().enumerate() {
                match phase {
                    Phase::Default => match ch {
                        b'\\' => phase = Phase::Escaped,
                        _ => sb.push(ch),
                    },
                    Phase::Escaped => {
                        match ch {
                            b't' => {
                                sb.push(b'\t');
                            }
                            b'n' => {
                                sb.push(b'\n');
                            }
                            b'r' => {
                                sb.push(b'\r');
                            }
                            b'u' => {
                                phase = Phase::UnicodeStart;
                                continue;
                            }
                            b'\x21'..=b'\x2f'
                            | b'\x3a'..=b'\x3f'
                            | b'\x5b'..=b'\x5f'
                            | b'\x7b'..=b'\x7e' => {
                                // All symbol characters following the backslash pass through as is.
                                sb.push(ch);
                            }
                            b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f' => {
                                code_start_position = index;
                                phase = Phase::RawByte;
                                code_buf.push(ch);
                                continue;
                            }
                            _ => {
                                return Err(StringLiteralError::InvalidCharSequence(
                                    position_start + index,
                                ));
                            }
                        }
                        phase = Phase::Default
                    }
                    Phase::RawByte => match ch {
                        b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f' => {
                            code_buf.push(ch);
                            let hex = str::from_utf8(&code_buf).unwrap();
                            let raw_byte = u32::from_str_radix(&hex, 16).unwrap();

                            if raw_byte < 0x80 || allow_binary {
                                sb.push(raw_byte as u8);
                            } else {
                                return Err(StringLiteralError::InvalidCharSequence(
                                    position_start + code_start_position,
                                ));
                            }

                            code_buf.clear();
                            phase = Phase::Default;
                        }
                        _ => {
                            return Err(StringLiteralError::InvalidCharSequence(
                                position_start + index,
                            ));
                        }
                    },
                    Phase::UnicodeStart => {
                        if ch == b'{' {
                            // `\u{`
                            code_start_position = index - 2;
                            phase = Phase::Unicode;
                        } else {
                            return Err(StringLiteralError::InvalidCharSequence(
                                position_start + index,
                            ));
                        }
                    }
                    Phase::Unicode => match ch {
                        b'}' => {
                            if code_buf.len() == 0 || code_buf.len() > 5 {
                                return Err(StringLiteralError::InvalidCharSequence(
                                    position_start + index,
                                ));
                            }
                            let hex = str::from_utf8(&code_buf).unwrap();
                            let unicode =
                                match char::from_u32(u32::from_str_radix(&hex, 16).unwrap()) {
                                    Some(v) => v,
                                    None => {
                                        return Err(StringLiteralError::InvalidUnicodeChar(
                                            position_start + code_start_position,
                                        ));
                                    }
                                };
                            sb.extend(unicode.to_string().as_bytes());

                            code_buf.clear();
                            phase = Phase::Default;
                        }
                        b'_' => {}
                        b'0'..=b'9' | b'A'..=b'F' | b'a'..=b'f' => {
                            code_buf.push(ch);
                        }
                        _ => {
                            return Err(StringLiteralError::InvalidCharSequence(
                                position_start + index,
                            ));
                        }
                    },
                }
            }
            if phase != Phase::Default {
                return Err(StringLiteralError::InvalidCharSequence(
                    self.source().len() - 1,
                ));
            }

            Ok(sb)
        }
    }

    #[inline]
    pub fn into_keyword(self) -> Result<KeywordToken<KEYWORD>, Token<KEYWORD>> {
        KeywordToken::from_token(self)
    }

    pub fn string_literal<'a>(&'a self) -> Result<Cow<'a, str>, StringLiteralError> {
        let TokenType::StringLiteral = *self.token_type() else {
            return Err(StringLiteralError::NaT);
        };
        let source = &self.source()[1..self.source().len() - 1];
        if self.source().find('\\').is_none() {
            Ok(Cow::Borrowed(source))
        } else {
            self.raw_bytes_literal(false)
                .map(|v| Cow::Owned(String::from_utf8(v).unwrap()))
        }
    }
}

impl<KEYWORD> core::fmt::Debug for Token<KEYWORD>
where
    KEYWORD: core::fmt::Debug + core::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("source", &self.source())
            .field("token_type", &self.token_type)
            .field("position", &self.position)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringLiteralError {
    /// Not a Thing
    NaT,
    /// Invalid char sequence
    InvalidCharSequence(usize),
    /// Invalid unicode sequence
    InvalidUnicodeChar(usize),
}

#[derive(Debug)]
pub struct KeywordToken<KEYWORD> {
    arc_buffer: Arc<Vec<u8>>,
    keyword: KEYWORD,
    position: TokenPosition,
}

impl<KEYWORD> KeywordToken<KEYWORD> {
    #[inline]
    pub fn from_token(token: Token<KEYWORD>) -> Result<KeywordToken<KEYWORD>, Token<KEYWORD>> {
        match token.token_type {
            TokenType::Keyword(keyword) => Ok(KeywordToken {
                arc_buffer: token.arc_buffer.clone(),
                keyword,
                position: token.position,
            }),
            _ => Err(token),
        }
    }

    #[inline]
    pub fn into_token(self) -> Token<KEYWORD> {
        Token {
            arc_buffer: self.arc_buffer,
            token_type: TokenType::Keyword(self.keyword),
            position: self.position,
        }
    }

    #[inline]
    pub fn as_token(&self) -> Token<KEYWORD>
    where
        KEYWORD: Copy,
    {
        Token {
            arc_buffer: self.arc_buffer.clone(),
            token_type: TokenType::Keyword(self.keyword),
            position: self.position,
        }
    }

    #[inline]
    pub fn source(&self) -> &str
    where
        KEYWORD: Copy,
    {
        Token::<KEYWORD>::_source(&self.arc_buffer, self.position.range())
    }

    #[inline]
    pub fn keyword(&self) -> &KEYWORD {
        &self.keyword
    }

    #[inline]
    pub fn position(&self) -> TokenPosition {
        self.position
    }
}

impl<KEYWORD> From<KeywordToken<KEYWORD>> for Token<KEYWORD> {
    #[inline]
    fn from(value: KeywordToken<KEYWORD>) -> Self {
        value.into_token()
    }
}

pub struct Tokens<KEYWORD>
where
    KEYWORD: Copy + Clone,
{
    arc_buffer: Arc<Vec<u8>>,
    fragments: Arc<Vec<TokenFragment<KEYWORD>>>,
    last: TokenFragment<KEYWORD>,
    lines: Arc<Vec<(usize, usize)>>,
}

pub struct TokenStream<KEYWORD>
where
    KEYWORD: Copy + Clone,
{
    arc_buffer: Arc<Vec<u8>>,
    fragments: Arc<Vec<TokenFragment<KEYWORD>>>,
    last: TokenFragment<KEYWORD>,
    lines: Arc<Vec<(usize, usize)>>,
    index: usize,
    range: Range<usize>,
}

impl<KEYWORD: Copy + Clone> Tokens<KEYWORD> {
    #[inline]
    pub fn line_positions(&self) -> &[(usize, usize)] {
        &self.lines
    }

    pub fn line_index(&self, position: usize) -> Option<usize> {
        self.lines
            .binary_search_by(|(line_start, line_end)| {
                if position < *line_start {
                    cmp::Ordering::Greater
                } else if position > *line_end {
                    cmp::Ordering::Less
                } else {
                    cmp::Ordering::Equal
                }
            })
            .ok()
    }

    #[inline]
    pub fn stream(&self) -> TokenStream<KEYWORD> {
        TokenStream {
            arc_buffer: self.arc_buffer.clone(),
            fragments: self.fragments.clone(),
            last: self.last,
            lines: self.lines.clone(),
            index: 0,
            range: 0..self.fragments.len(),
        }
    }
}

impl<KEYWORD: Copy + Clone> fmt::Debug for Tokens<KEYWORD> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenStream").finish()
    }
}

impl<KEYWORD> fmt::Debug for TokenStream<KEYWORD>
where
    KEYWORD: Copy + Clone + PartialEq + core::fmt::Debug + core::fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let snapshot = self.snapshot();
        f.debug_list().entries(snapshot.into_iter()).finish()
    }
}

impl<KEYWORD: Copy + Clone + PartialEq> TokenStream<KEYWORD> {
    #[inline]
    pub fn snapshot(&self) -> Self {
        Self {
            arc_buffer: self.arc_buffer.clone(),
            fragments: self.fragments.clone(),
            last: self.last,
            lines: self.lines.clone(),
            index: self.index,
            range: self.index..self.range.end,
        }
    }

    #[inline]
    pub fn make_replay(&self, snapshot: TokenStream<KEYWORD>) -> TokenStream<KEYWORD> {
        let range_end = if self.index > 0 { self.index - 1 } else { 0 };
        let last_position = TokenPosition::new_at(self.fragments[self.index].position.start());
        TokenStream {
            arc_buffer: self.arc_buffer.clone(),
            fragments: self.fragments.clone(),
            last: TokenFragment {
                token_type: TokenType::Eof,
                position: last_position,
            },
            lines: self.lines.clone(),
            index: snapshot.index,
            range: snapshot.index..range_end,
        }
    }

    pub fn get_raw(&self, range: TokenPosition) -> RawToken {
        RawToken {
            arc_buffer: self.arc_buffer.clone(),
            position: range,
        }
    }

    pub(crate) fn get(&self, index: usize) -> Option<Token<KEYWORD>> {
        let fragment = self
            .range
            .contains(&index)
            .then(|| self.fragments.get(index))
            .flatten()?;

        Some(Token {
            arc_buffer: self.arc_buffer.clone(),
            token_type: fragment.token_type,
            position: fragment.position,
        })
    }

    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    pub fn eof(&self) -> Token<KEYWORD> {
        Token {
            arc_buffer: self.arc_buffer.clone(),
            token_type: self.last.token_type,
            position: self.last.position,
        }
    }

    #[inline]
    pub fn peek(&self) -> Option<Token<KEYWORD>> {
        self.get(self.index)
    }

    #[inline]
    pub fn peek_last(&self) -> Option<Token<KEYWORD>> {
        if self.index > 0 {
            let fragment = self.fragments.get(self.index - 1)?;

            Some(Token {
                arc_buffer: self.arc_buffer.clone(),
                token_type: fragment.token_type,
                position: fragment.position,
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn shift(&mut self) -> Option<Token<KEYWORD>> {
        self.get(self.index).map(|v| {
            self.index += 1;
            v
        })
    }

    #[inline]
    pub fn unshift(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }

    /// Returns the next non-blank token.
    #[inline]
    pub fn next_non_blank(&mut self) -> Token<KEYWORD> {
        self.skip_ignorable();
        self.shift().unwrap_or(self.eof())
    }

    /// Returns the token immediately following the last token, if one exists.
    pub fn peek_immed(&self) -> Option<Token<KEYWORD>> {
        let Some(current) = self.peek_last() else {
            return None;
        };
        self.peek().map(|token| {
            if current.position().end() == token.position().start() {
                token
            } else {
                Token {
                    arc_buffer: Arc::new(Vec::new()),
                    token_type: TokenType::Whitespace,
                    position: TokenPosition::new_at(current.position().end()),
                }
            }
        })
    }

    /// Returns the token immediately following the last token, if one exists.
    pub fn next_immed(&mut self) -> Option<Token<KEYWORD>> {
        let Some(current) = self.peek_last() else {
            return None;
        };
        self.peek().map(|token| {
            if current.position().end() == token.position().start() {
                self.index += 1;
                token
            } else {
                Token {
                    arc_buffer: Arc::new(Vec::new()),
                    token_type: TokenType::Whitespace,
                    position: TokenPosition::new_at(current.position().end()),
                }
            }
        })
    }

    #[inline]
    pub fn skip_ignorable(&mut self) {
        while let Some(token) = self.peek() {
            if token.token_type().is_ignorable() {
                self.index += 1;
            } else {
                break;
            }
        }
    }

    #[inline]
    pub fn transaction<F, B, C>(&mut self, kernel: F) -> Result<C, B>
    where
        F: FnOnce(&mut Self) -> ControlFlow<B, C>,
    {
        let mut snapshot = self.snapshot();
        match kernel(&mut snapshot) {
            ControlFlow::Continue(result) => {
                self.index = snapshot.index;
                Ok(result)
            }
            ControlFlow::Break(err) => Err(err),
        }
    }

    pub fn expect_fn<F>(&mut self, kernel: F) -> Result<Token<KEYWORD>, Token<KEYWORD>>
    where
        F: FnOnce(&Token<KEYWORD>) -> bool,
    {
        self.transaction(|tokens| {
            let token = tokens.next_non_blank();
            if kernel(&token) {
                ControlFlow::Continue(token)
            } else {
                ControlFlow::Break(token)
            }
        })
    }

    pub fn expect(
        &mut self,
        expected: &[TokenType<KEYWORD>],
    ) -> Result<Token<KEYWORD>, Token<KEYWORD>> {
        self.expect_fn(|token| expected.contains(token.token_type()))
    }

    pub fn expect_symbol(&mut self, expected: char) -> Result<Token<KEYWORD>, Token<KEYWORD>> {
        self.expect(&[TokenType::Symbol(expected)])
    }

    pub fn expect_keyword(
        &mut self,
        expected: KEYWORD,
    ) -> Result<KeywordToken<KEYWORD>, Token<KEYWORD>> {
        self.expect_keywords(&[expected])
    }

    pub fn expect_keywords(
        &mut self,
        expected: &[KEYWORD],
    ) -> Result<KeywordToken<KEYWORD>, Token<KEYWORD>> {
        self.transaction(|tokens| {
            let token = tokens.next_non_blank();
            match KeywordToken::from_token(token) {
                Ok(c) => {
                    if expected.contains(c.keyword()) {
                        ControlFlow::Continue(c)
                    } else {
                        ControlFlow::Break(c.into_token())
                    }
                }
                Err(b) => ControlFlow::Break(b),
            }
        })
    }

    pub fn expect_any_keyword(&mut self) -> Result<KeywordToken<KEYWORD>, Token<KEYWORD>> {
        self.transaction(|tokens| {
            let token = tokens.next_non_blank();
            match KeywordToken::from_token(token) {
                Ok(c) => ControlFlow::Continue(c),
                Err(b) => ControlFlow::Break(b),
            }
        })
    }

    pub fn expect_immed(
        &mut self,
        expected: &[TokenType<KEYWORD>],
    ) -> Result<Token<KEYWORD>, Token<KEYWORD>> {
        self.transaction(|tokens| match tokens.next_immed() {
            Some(token) => {
                if expected.contains(token.token_type()) {
                    ControlFlow::Continue(token)
                } else {
                    ControlFlow::Break(token)
                }
            }
            None => ControlFlow::Break(Token::eof()),
        })
    }

    pub fn expect_immed_symbol(
        &mut self,
        expected: char,
    ) -> Result<Token<KEYWORD>, Token<KEYWORD>> {
        self.expect_immed(&[TokenType::Symbol(expected)])
    }
}

impl<KEYWORD: Copy + Clone + PartialEq> Iterator for TokenStream<KEYWORD> {
    type Item = Token<KEYWORD>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.shift()
    }
}

pub struct RawToken {
    arc_buffer: Arc<Vec<u8>>,
    position: TokenPosition,
}

impl RawToken {
    #[inline]
    pub fn source<KEYWORD>(&self) -> &str {
        Token::<KEYWORD>::_source(&self.arc_buffer, self.position.range())
    }

    #[inline]
    pub fn position(&self) -> TokenPosition {
        self.position
    }

    #[inline]
    pub fn as_token<T>(&self) -> Token<T> {
        Token {
            arc_buffer: self.arc_buffer.clone(),
            token_type: TokenType::Uncategorized,
            position: self.position(),
        }
    }
}

impl core::fmt::Debug for RawToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("source", &self.source::<()>())
            .field("position", &self.position)
            .finish()
    }
}
