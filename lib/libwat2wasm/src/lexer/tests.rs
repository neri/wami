use crate::*;

#[test]
fn token_utf() {
    let str = "ABC \"あ\" \"🍣\"".as_bytes();
    let tokens = Lexer::with_slice(str, Keyword::from_str).unwrap();
    let tokens = tokens.stream();

    assert_eq!(tokens.get(0).unwrap().source(), "ABC");
    assert_eq!(tokens.get(1).unwrap().source(), "\"あ\"");
    assert_eq!(tokens.get(2).unwrap().source(), "\"🍣\"");

    let str = b"A B C '\xE3\x81\x82'\n'\xF0\x9F\x8D\xA3' '\xE3\x81";
    let err = Lexer::with_slice(str, Keyword::from_str).unwrap_err();
    assert_eq!(err.kind(), LexerErrorKind::UnexpectedEof);
    assert_eq!(err.position(), (2, 6));

    let str = b"'\xE3\x81A '";
    let err = Lexer::with_slice(str, Keyword::from_str).unwrap_err();
    assert_eq!(err.kind(), LexerErrorKind::InvalidChar);
    assert_eq!(err.position(), (1, 2));
}

#[test]
fn token_numer() {
    let str = b"0+0x0123456789ABCDEF*0b01 0o01234567 0123456789 1234567890";
    let tokens = Lexer::with_slice(str, Keyword::from_str).unwrap();
    let mut tokens = tokens.stream();

    let number = tokens.expect(&[TokenType::NumericLiteral]).unwrap();
    assert_eq!(number.source(), "0");
    assert_eq!(number.radix().unwrap(), (0, Radix::Dec));

    tokens.expect_symbol('+').unwrap();

    let number = tokens.expect(&[TokenType::NumericLiteral]).unwrap();
    assert_eq!(number.source(), "0x0123456789ABCDEF");
    assert_eq!(number.radix().unwrap(), (2, Radix::Hex));

    tokens.expect_symbol('*').unwrap();

    let number = tokens.expect(&[TokenType::NumericLiteral]).unwrap();
    assert_eq!(number.source(), "0b01");
    assert_eq!(number.radix().unwrap(), (2, Radix::Bin));

    let number = tokens.expect(&[TokenType::NumericLiteral]).unwrap();
    assert_eq!(number.source(), "0o01234567");
    assert_eq!(number.radix().unwrap(), (2, Radix::Oct));

    let number = tokens.expect(&[TokenType::NumericLiteral]).unwrap();
    assert_eq!(number.source(), "0123456789");
    assert_eq!(number.radix().unwrap(), (0, Radix::Dec));

    let number = tokens.expect(&[TokenType::NumericLiteral]).unwrap();
    assert_eq!(number.source(), "1234567890");
    assert_eq!(number.radix().unwrap(), (0, Radix::Dec));

    let str = b"0A+0123456789A*0b2 0o8 123456789a";
    let tokens = Lexer::with_slice(str, Keyword::from_str).unwrap();
    let mut tokens = tokens.stream();

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "0A");

    tokens.expect_symbol('+').unwrap();

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "0123456789A");

    tokens.expect_symbol('*').unwrap();

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "0b2");

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "0o8");

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "123456789a");
}

#[test]
fn token_float() {
    let str = b"0. 1. 123..456 0.e 0.123456789 98765.43210 1.0e 1.0e+ 1.0e- 1.234567e+89 1.234567e-89 1.234567e89";
    let tokens = Lexer::with_slice(str, Keyword::from_str).unwrap();
    let mut tokens = tokens.stream();

    let error_rate = 5;

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "0.");

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "1.");

    let number = tokens.expect(&[TokenType::NumericLiteral]).unwrap();
    assert_eq!(number.source(), "123");
    tokens.expect_immed_symbol('.').unwrap();
    tokens.expect_immed_symbol('.').unwrap();
    let number = tokens.expect_immed(&[TokenType::NumericLiteral]).unwrap();
    assert_eq!(number.source(), "456");

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "0.e");

    let number = tokens.expect(&[TokenType::FloatingNumberLiteral]).unwrap();
    assert_eq!(number.source(), "0.123456789");
    assert_eq!(number.try_parse_float().unwrap(), 0.123456789);

    let number = tokens.expect(&[TokenType::FloatingNumberLiteral]).unwrap();
    assert_eq!(number.source(), "98765.43210");
    assert_eq!(number.try_parse_float().unwrap(), 98765.43210);

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "1.0e");

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "1.0e+");

    let number = tokens.expect(&[TokenType::BrokenNumber]).unwrap();
    assert_eq!(number.source(), "1.0e-");

    let number = tokens.expect(&[TokenType::FloatingNumberLiteral]).unwrap();
    assert_eq!(number.source(), "1.234567e+89");
    assert_almost_eq(number.try_parse_float().unwrap(), 1.234567e+89, error_rate);

    let number = tokens.expect(&[TokenType::FloatingNumberLiteral]).unwrap();
    assert_eq!(number.source(), "1.234567e-89");
    assert_almost_eq(number.try_parse_float().unwrap(), 1.234567e-89, error_rate);

    let number = tokens.expect(&[TokenType::FloatingNumberLiteral]).unwrap();
    assert_eq!(number.source(), "1.234567e89");
    assert_almost_eq(number.try_parse_float().unwrap(), 1.234567e89, error_rate);
}

#[test]
fn token_string() {
    let str = "\"ABC\" \"あいうえお\" \"🍣\"".as_bytes();
    let tokens = Lexer::with_slice(str, Keyword::from_str).unwrap();
    let mut tokens = tokens.stream();

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"ABC\"");
    assert_eq!(literal.string_literal().unwrap(), "ABC");

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"あいうえお\"");
    assert_eq!(literal.string_literal().unwrap(), "あいうえお");

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"🍣\"");
    assert_eq!(literal.string_literal().unwrap(), "🍣");

    let str = b"\"ABC\" \"A\\\"B\\\"C\" \"\\t\" \"\\z\" \"\\\\\" \"A\\\" ";
    let tokens = Lexer::with_slice(str, Keyword::from_str).unwrap();
    let mut tokens = tokens.stream();

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"ABC\"");
    assert_eq!(literal.string_literal().unwrap(), "ABC");

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"A\\\"B\\\"C\"");
    assert_eq!(literal.string_literal().unwrap(), "A\"B\"C");

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\t\"");
    assert_eq!(literal.string_literal().unwrap(), "\t");

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\z\"");
    assert_eq!(
        literal.string_literal().unwrap_err(),
        StringLiteralError::InvalidCharSequence(2)
    );

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\\\\"");
    assert_eq!(literal.string_literal().unwrap(), "\\");

    tokens.expect(&[TokenType::BrokenString]).unwrap();

    let str = b"\"\\u\" \"\\u{\" \"\\u{}\" \"\\u{0}\" \"\\u{22}\" \"\\u{3042}\" \"\\u{1F363}\" \"\\21\" \"\\2\"";
    let tokens = Lexer::with_slice(str, Keyword::from_str).unwrap();
    let mut tokens = tokens.stream();

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\u\"");
    assert_eq!(
        literal.string_literal().unwrap_err(),
        StringLiteralError::InvalidCharSequence(3)
    );

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\u{\"");
    assert_eq!(
        literal.string_literal().unwrap_err(),
        StringLiteralError::InvalidCharSequence(4)
    );

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\u{}\"");
    assert_eq!(
        literal.string_literal().unwrap_err(),
        StringLiteralError::InvalidCharSequence(4)
    );

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\u{0}\"");
    assert_eq!(literal.string_literal().unwrap(), "\0");

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\u{22}\"");
    assert_eq!(literal.string_literal().unwrap(), "\"");

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\u{3042}\"");
    assert_eq!(literal.string_literal().unwrap(), "あ");

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\u{1F363}\"");
    assert_eq!(literal.string_literal().unwrap(), "🍣");

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\21\"");
    assert_eq!(literal.string_literal().unwrap(), "!");

    let literal = tokens.expect(&[TokenType::StringLiteral]).unwrap();
    assert_eq!(literal.source(), "\"\\2\"");
    assert_eq!(
        literal.string_literal().unwrap_err(),
        StringLiteralError::InvalidCharSequence(3)
    );
}

#[test]
fn token_comment() {
    let str = b"
abc(;def
ghi
;)jkl(;mno;)
pqr;;stu
vwx
";
    let tokens = Lexer::with_slice(str, Keyword::from_str).unwrap();
    let mut tokens = tokens.stream();

    let token = tokens.next().unwrap();
    assert_eq!(token.token_type(), &TokenType::Identifier);
    assert_eq!(token.source(), "abc");

    let token = tokens.next().unwrap();
    assert_eq!(token.token_type(), &TokenType::BlockComment);
    assert_eq!(token.source(), "(;def\nghi\n;)");

    let token = tokens.next().unwrap();
    assert_eq!(token.token_type(), &TokenType::Identifier);
    assert_eq!(token.source(), "jkl");

    let token = tokens.next().unwrap();
    assert_eq!(token.token_type(), &TokenType::BlockComment);
    assert_eq!(token.source(), "(;mno;)");

    let token = tokens.next().unwrap();
    assert_eq!(token.token_type(), &TokenType::Identifier);
    assert_eq!(token.source(), "pqr");

    let token = tokens.next().unwrap();
    assert_eq!(token.token_type(), &TokenType::LineComment);
    assert_eq!(token.source(), ";;stu");

    let token = tokens.next().unwrap();
    assert_eq!(token.token_type(), &TokenType::Identifier);
    assert_eq!(token.source(), "vwx");

    let token = tokens.next().unwrap();
    assert_eq!(token.token_type(), &TokenType::Eof);
}

#[track_caller]
fn assert_almost_eq(lhs: f64, rhs: f64, error_rate: u64) {
    match (lhs.is_nan(), rhs.is_nan()) {
        // If both are NaN, ok
        (true, true) => return,
        // Error if only one of them is NaN
        (true, false) | (false, true) => {
            panic!(
                "assertion `left == right` failed
  left: {lhs}
 right: {rhs}
"
            );
        }
        (false, false) => {
            let mask = 0xFFFF_FFFF_FFFF_FF00;
            if (lhs.to_bits() & mask) != (rhs.to_bits() & mask)
                || lhs.to_bits().abs_diff(rhs.to_bits()) > error_rate
            {
                panic!(
                    "assertion `left == right` failed
  left: {lhs}
 right: {rhs}
"
                );
            }
        }
    }
}
