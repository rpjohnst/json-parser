use std::{char, str};

/// A JSON lexer over a UTF-8 string.
///
/// The lexer produces JSON tokens according to RFC 7159.
/// When it encounters invalid tokens, it returns an error token that includes
/// the invalid bytes in its span. The parser can use this for error recovery.
pub(crate) struct Lex<'source> {
    source: &'source [u8],
}

/// A single JSON token.
#[derive(PartialEq, Debug)]
pub(crate) struct Token<'source> {
    pub(crate) span: &'source str,
    pub(crate) kind: TokenKind,
}

/// A kind of token, including its payload.
#[derive(PartialEq, Debug)]
pub(crate) enum TokenKind {
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Colon,
    Comma,

    String(String),
    Number(f64),
    Bool(bool),
    Null,

    Error,
    End,
}

impl<'source> Lex<'source> {
    /// Create a new lexer for a JSON string.
    pub(crate) fn new(source: &'source str) -> Lex<'source> {
        let source = source.as_bytes();
        Lex { source }
    }

    /// Read the next token from the lexer.
    pub(crate) fn token(&mut self) -> Token<'source> {
        // Skip any whitespace before a token.
        loop {
            match *self.source {
                [b, ref rest..] if [b' ', b'\t', b'\r', b'\n'].contains(&b) => self.source = rest,
                _ => break,
            }
        }

        // Determine the token kind by its first byte.
        let (kind, rest) = match *self.source {
            [b'{', ref rest..] => (TokenKind::LeftBrace, rest),
            [b'}', ref rest..] => (TokenKind::RightBrace, rest),
            [b'[', ref rest..] => (TokenKind::LeftBracket, rest),
            [b']', ref rest..] => (TokenKind::RightBracket, rest),
            [b':', ref rest..] => (TokenKind::Colon, rest),
            [b',', ref rest..] => (TokenKind::Comma, rest),

            [b'"', ref rest..] => Self::string(rest),
            ref rest @ [b'-', ..] | ref rest @ [b'0'..=b'9', ..] => Self::number(rest),
            [b't', b'r', b'u', b'e', ref rest..] => (TokenKind::Bool(true), rest),
            [b'f', b'a', b'l', b's', b'e', ref rest..] => (TokenKind::Bool(false), rest),
            [b'n', b'u', b'l', b'l', ref rest..] => (TokenKind::Null, rest),

            [_, ref rest..] => (TokenKind::Error, rest),
            [ref rest..] => (TokenKind::End, rest),
        };

        // Build the token's span from the post-whitespace position and the current position.
        let len = rest.as_ptr() as usize - self.source.as_ptr() as usize;
        let span = unsafe { str::from_utf8_unchecked(self.source.get_unchecked(..len)) };

        self.source = rest;
        Token { span, kind }
    }

    /// Read the rest of a string, after the open quote.
    ///
    /// Replaces invalid unicode escape sequences with U+FFFD.
    /// Returns TokenKind::Error for unterminated strings.
    fn string(mut source: &'source [u8]) -> (TokenKind, &'source [u8]) {
        let mut string = String::new();
        loop {
            match *source {
                // Closing quote.
                [b'"', ref rest..] => { source = rest; break; }

                // Escape sequences.
                [b'\\', b'"', ref rest..] => { source = rest; string.push_str("\""); }
                [b'\\', b'\\', ref rest..] => { source = rest; string.push_str("\\"); }
                [b'\\', b'/', ref rest..] => { source = rest; string.push_str("/"); }
                [b'\\', b'b', ref rest..] => { source = rest; string.push_str("\x08"); }
                [b'\\', b'f', ref rest..] => { source = rest; string.push_str("\x0C"); }
                [b'\\', b'n', ref rest..] => { source = rest; string.push_str("\n"); }
                [b'\\', b'r', ref rest..] => { source = rest; string.push_str("\r"); }
                [b'\\', b't', ref rest..] => { source = rest; string.push_str("\t"); }
                [b'\\', b'u', ref rest..] => {
                    let (c, rest) = Self::unicode_escape(rest);
                    source = rest;
                    string.push(c);
                }

                // UTF-8 codepoints.
                // TODO: replace this with library code somehow?
                [0x00..=0x7F, ref rest..] => {
                    let s = unsafe { str::from_utf8_unchecked(source.get_unchecked(..1)) };
                    source = rest;
                    string.push_str(s);
                }
                [0xC0..=0xDF, 0x80..=0xBF, ref rest..] => {
                    let s = unsafe { str::from_utf8_unchecked(source.get_unchecked(..2)) };
                    source = rest; 
                    string.push_str(s);
                }
                [0xE0..=0xEF, 0x80..=0xBF, 0x80..=0xBF, ref rest..] => {
                    let s = unsafe { str::from_utf8_unchecked(source.get_unchecked(..3)) };
                    source = rest;
                    string.push_str(s);
                }
                [0xF0..=0xFF, 0x80..=0xBF, 0x80..=0xBF, 0x80..=0xBF, ref rest..] => {
                    let s = unsafe { str::from_utf8_unchecked(source.get_unchecked(..4)) };
                    source = rest;
                    string.push_str(s);
                }

                // The input is valid UTF-8, so this should never happen.
                [_, _..] => unreachable!(),

                // Unterminated string.
                [ref rest..] => return (TokenKind::Error, rest),
            }
        }

        (TokenKind::String(string), source)
    }

    /// Read the rest of a Unicode escape sequence, after the \u.
    ///
    /// Reads two escape sequences if the first is a leading surrogate.
    /// Replaces invalid codepoints, including incomplete escape sequences and
    /// unpaired surrogates, with U+FFFD.
    fn unicode_escape(mut source: &'source [u8]) -> (char, &'source [u8]) {
        let code_point = match Self::code_unit(source) {
            (Some(s1 @ 0xD800..=0xDBFF), rest) => {
                let (s2, rest) = match *rest {
                    [b'\\', b'u', ref rest..] => Self::code_unit(rest),
                    _ => (None, rest),
                };
                source = rest;

                if let Some(s2 @ 0xDC00..=0xDFFF) = s2 {
                    Some(0x1_0000 + (((s1 - 0xD800) << 10) | (s2 - 0xDC00)))
                } else {
                    None
                }
            }
            (code_unit, rest) => { source = rest; code_unit }
        };

        let c = code_point.and_then(char::from_u32).unwrap_or('\u{FFFD}');
        (c, source)
    }

    /// Read the body of a JSON unicode escape sequence.
    ///
    /// Returns None if there is not a complete escape sequence.
    /// Returns a u32 even though a u16 would suffice, to simplify `unicode_escape`.
    fn code_unit(mut source: &'source [u8]) -> (Option<u32>, &'source [u8]) {
        let mut code_unit: u16 = 0;
        for _ in 0..4 {
            let digit = match *source {
                [b @ b'0'..=b'9', ref rest..] => {
                    source = rest;
                    (b - b'0') as u16
                }
                [b @ b'A'..=b'Z', ref rest..] => {
                    source = rest;
                    (b - b'A') as u16 + 10
                }
                [b @ b'a'..=b'z', ref rest..] => {
                    source = rest;
                    (b - b'a') as u16 + 10
                }
                _ => return (None, source),
            };
            code_unit = 16 * code_unit + digit;
        }
        (Some(code_unit as u32), source)
    }

    /// Read a number.
    ///
    /// Returns TokenKind::Error on invalid numbers.
    fn number(mut source: &'source [u8]) -> (TokenKind, &'source [u8]) {
        let positive = match *source {
            [b'-', ref rest..] => { source = rest; false }
            _ => true,
        };

        let mut significand: u64;
        match *source {
            [b'0', ref rest..] => {
                source = rest;
                significand = 0;
            }
            [b @ b'1'..=b'9', ref rest..] => {
                source = rest;
                significand = (b - b'0') as u64;
                while let [b @ b'0'..=b'9', ref rest..] = *source {
                    source = rest;

                    let digit = (b - b'0') as u64;
                    significand = 10 * significand + digit;
                }
            }
            _ => return (TokenKind::Error, source),
        };

        let mut exponent: i32 = 0;
        if let [b'.', ref rest..] = *source {
            source = rest;
            let mut any_digits = false;
            while let [b @ b'0'..=b'9', ref rest..] = *source {
                source = rest;
                any_digits = true;

                let digit = (b - b'0') as u64;
                significand = 10 * significand + digit;
                exponent -= 1;
            }
            if !any_digits {
                return (TokenKind::Error, source);
            }
        }

        // TODO: simplify this with if_while_or_patterns (rust-lang/rust#48215)
        let (has_exponent, rest) = match *source {
            [b'e', ref rest..] | [b'E', ref rest..] => (true, rest),
            _ => (false, source),
        };
        if has_exponent {
            source = rest;

            let positive = match *source {
                [b'+', ref rest..] => { source = rest; true }
                [b'-', ref rest..] => { source = rest; false }
                _ => true,
            };

            let mut explicit_exponent: i32 = 0;
            let mut any_digits = false;
            while let [b @ b'0'..=b'9', ref rest..] = *source {
                source = rest;
                any_digits = true;

                let digit = (b - b'0') as i32;
                explicit_exponent = 10 * explicit_exponent + digit;
            }
            if !any_digits {
                return (TokenKind::Error, source);
            }

            if positive {
                exponent += explicit_exponent;
            } else {
                exponent -= explicit_exponent;
            }
        }

        let mut magnitude = significand as f64;
        for _ in 0..i32::abs(exponent) {
            if exponent > 0 {
                magnitude *= 10.0;
            } else {
                magnitude /= 10.0;
            }
        }
        let value = if positive { magnitude } else { -magnitude };

        (TokenKind::Number(value), source)
    }
}

#[cfg(test)]
mod tests {
    use lex::{Lex, Token, TokenKind};

    #[test]
    fn simple() {
        let s = r#"{ "foo": 3, "bar": ["baz", -5.8], "qux": 13e5 }"#;
        let mut lex = Lex::new(s);

        assert_eq!(lex.token(), Token { span: &s[0..1], kind: TokenKind::LeftBrace });

        let foo = String::from("foo");
        assert_eq!(lex.token(), Token { span: &s[2..7], kind: TokenKind::String(foo) });
        assert_eq!(lex.token(), Token { span: &s[7..8], kind: TokenKind::Colon });
        assert_eq!(lex.token(), Token { span: &s[9..10], kind: TokenKind::Number(3.0) });
        assert_eq!(lex.token(), Token { span: &s[10..11], kind: TokenKind::Comma });

        let bar = String::from("bar");
        assert_eq!(lex.token(), Token { span: &s[12..17], kind: TokenKind::String(bar) });
        assert_eq!(lex.token(), Token { span: &s[17..18], kind: TokenKind::Colon });

        assert_eq!(lex.token(), Token { span: &s[19..20], kind: TokenKind::LeftBracket });
        let baz = String::from("baz");
        assert_eq!(lex.token(), Token { span: &s[20..25], kind: TokenKind::String(baz) });
        assert_eq!(lex.token(), Token { span: &s[25..26], kind: TokenKind::Comma });
        assert_eq!(lex.token(), Token { span: &s[27..31], kind: TokenKind::Number(-5.8) });
        assert_eq!(lex.token(), Token { span: &s[31..32], kind: TokenKind::RightBracket });
        assert_eq!(lex.token(), Token { span: &s[32..33], kind: TokenKind::Comma });

        let qux = String::from("qux");
        assert_eq!(lex.token(), Token { span: &s[34..39], kind: TokenKind::String(qux) });
        assert_eq!(lex.token(), Token { span: &s[39..40], kind: TokenKind::Colon });
        assert_eq!(lex.token(), Token { span: &s[41..45], kind: TokenKind::Number(13.0e5) });

        assert_eq!(lex.token(), Token { span: &s[46..47], kind: TokenKind::RightBrace });
    }
}
