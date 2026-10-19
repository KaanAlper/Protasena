//! Tokenizer: bytes → `(`, `)`, atoms and quoted strings, each with its position.

use crate::error::{Error, ErrorKind, Pos, Result};

use super::Limits;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Open,
    Close,
    Atom(String),
    Str(String),
}

pub(crate) struct Lexer<'a> {
    bytes: &'a [u8],
    i: usize,
    line: u32,
    col: u32,
    max_atom: usize,
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn is_delim(b: u8) -> bool {
    is_space(b) || matches!(b, b'(' | b')' | b'"')
}

impl<'a> Lexer<'a> {
    /// `text` is already known to be UTF-8; a leading byte-order mark is skipped.
    pub(crate) fn new(text: &'a str, limits: &Limits) -> Self {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        Self { bytes: text.as_bytes(), i: 0, line: 1, col: 1, max_atom: limits.max_atom }
    }

    fn pos(&self) -> Pos {
        Pos { line: self.line, col: self.col }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.i += 1;
        if b == b'\n' {
            self.line = self.line.saturating_add(1);
            self.col = 1;
        } else if b & 0xc0 != 0x80 {
            // Continuation bytes of a multi-byte character do not move the column.
            self.col = self.col.saturating_add(1);
        }
        Some(b)
    }

    /// The next token and its position, `None` at the end of the input.
    pub(crate) fn next_token(&mut self) -> Result<Option<(Token, Pos)>> {
        while self.peek().is_some_and(is_space) {
            self.bump();
        }
        let pos = self.pos();
        let Some(b) = self.peek() else { return Ok(None) };
        let tok = match b {
            b'(' => {
                self.bump();
                Token::Open
            }
            b')' => {
                self.bump();
                Token::Close
            }
            b'"' => Token::Str(self.string(pos)?),
            _ => Token::Atom(self.atom(pos)?),
        };
        Ok(Some((tok, pos)))
    }

    fn atom(&mut self, pos: Pos) -> Result<String> {
        let start = self.i;
        while self.peek().is_some_and(|b| !is_delim(b)) {
            if self.i - start >= self.max_atom {
                return Err(Error::new(ErrorKind::AtomTooLong { max: self.max_atom }, pos));
            }
            self.bump();
        }
        let slice = self.bytes.get(start..self.i).unwrap_or_default();
        std::str::from_utf8(slice).map(str::to_owned).map_err(|_| Error::new(ErrorKind::InvalidUtf8, pos))
    }

    fn string(&mut self, pos: Pos) -> Result<String> {
        self.bump(); // opening quote
        let mut out: Vec<u8> = Vec::new();
        loop {
            if out.len() > self.max_atom {
                return Err(Error::new(ErrorKind::AtomTooLong { max: self.max_atom }, pos));
            }
            let Some(b) = self.bump() else {
                return Err(Error::new(ErrorKind::UnterminatedString, pos));
            };
            match b {
                b'"' => break,
                b'\\' => self.escape(&mut out),
                _ => out.push(b),
            }
        }
        if out.len() > self.max_atom {
            return Err(Error::new(ErrorKind::AtomTooLong { max: self.max_atom }, pos));
        }
        String::from_utf8(out).map_err(|_| Error::new(ErrorKind::InvalidUtf8, pos))
    }

    /// After a backslash. Known escapes are decoded; anything else keeps the backslash, so no
    /// input is lost.
    fn escape(&mut self, out: &mut Vec<u8>) {
        let simple = match self.peek() {
            Some(b'"') => Some(b'"'),
            Some(b'\\') => Some(b'\\'),
            Some(b'\'') => Some(b'\''),
            Some(b'n') => Some(b'\n'),
            Some(b'r') => Some(b'\r'),
            Some(b't') => Some(b'\t'),
            Some(b'a') => Some(0x07),
            Some(b'b') => Some(0x08),
            Some(b'f') => Some(0x0c),
            Some(b'v') => Some(0x0b),
            _ => None,
        };
        if let Some(c) = simple {
            self.bump();
            out.push(c);
            return;
        }
        let code = match self.peek() {
            Some(b'x') => {
                self.bump();
                self.digits(16, 2)
            }
            Some(b'0'..=b'7') => self.digits(8, 3),
            _ => None,
        };
        match code.and_then(char::from_u32) {
            Some(c) => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
            None => out.push(b'\\'),
        }
    }

    /// Up to `max` digits in `radix`; `None` (nothing consumed) when there is none.
    fn digits(&mut self, radix: u32, max: usize) -> Option<u32> {
        let mut value: u32 = 0;
        let mut n = 0;
        while n < max {
            let Some(d) = self.peek().and_then(|b| char::from(b).to_digit(radix)) else { break };
            self.bump();
            value = value.saturating_mul(radix).saturating_add(d);
            n += 1;
        }
        (n > 0).then_some(value)
    }
}
