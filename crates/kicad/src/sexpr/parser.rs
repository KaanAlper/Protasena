//! Tokens → tree, iteratively, with depth and node limits.

use crate::error::{Error, ErrorKind, Pos, Result};

use super::lexer::{Lexer, Token};
use super::{Limits, Sexpr};

/// Parses exactly one top-level list with the default [`Limits`].
pub fn parse(input: &[u8]) -> Result<Sexpr> {
    parse_with(input, &Limits::DEFAULT)
}

/// Parses exactly one top-level list.
pub fn parse_with(input: &[u8], limits: &Limits) -> Result<Sexpr> {
    let mut all = parse_all_with(input, limits)?.into_iter();
    let Some(first) = all.next() else {
        return Err(Error::new(ErrorKind::Empty, Pos { line: 1, col: 1 }));
    };
    if let Some(extra) = all.next() {
        return Err(Error::new(ErrorKind::TrailingData, extra.pos()));
    }
    if first.as_list().is_none() {
        return Err(Error::new(ErrorKind::Invalid("expected a list at the top level".into()), first.pos()));
    }
    Ok(first)
}

/// Parses every top-level expression with the default [`Limits`].
pub fn parse_all(input: &[u8]) -> Result<Vec<Sexpr>> {
    parse_all_with(input, &Limits::DEFAULT)
}

/// Parses every top-level expression.
pub fn parse_all_with(input: &[u8], limits: &Limits) -> Result<Vec<Sexpr>> {
    let start = Pos { line: 1, col: 1 };
    if input.len() > limits.max_input {
        return Err(Error::new(ErrorKind::InputTooLarge { len: input.len(), max: limits.max_input }, start));
    }
    let text = std::str::from_utf8(input).map_err(|e| Error::new(ErrorKind::InvalidUtf8, pos_of(input, e.valid_up_to())))?;
    let mut lexer = Lexer::new(text, limits);
    let mut top: Vec<Sexpr> = Vec::new();
    // Open lists: where each started and the items read so far.
    let mut stack: Vec<(Pos, Vec<Sexpr>)> = Vec::new();
    let mut nodes: usize = 0;
    while let Some((tok, pos)) = lexer.next_token()? {
        let node = match tok {
            Token::Open => {
                if stack.len() >= limits.max_depth {
                    return Err(Error::new(ErrorKind::TooDeep { max: limits.max_depth }, pos));
                }
                count(&mut nodes, limits, pos)?;
                stack.push((pos, Vec::new()));
                continue;
            }
            Token::Close => {
                let Some((open, items)) = stack.pop() else {
                    return Err(Error::new(ErrorKind::UnexpectedClose, pos));
                };
                Sexpr::List { items, pos: open }
            }
            Token::Atom(text) => {
                count(&mut nodes, limits, pos)?;
                Sexpr::Atom { text, pos }
            }
            Token::Str(text) => {
                count(&mut nodes, limits, pos)?;
                Sexpr::Str { text, pos }
            }
        };
        match stack.last_mut() {
            Some((_, items)) => items.push(node),
            None => top.push(node),
        }
    }
    if let Some((open, _)) = stack.first() {
        return Err(Error::new(ErrorKind::UnclosedList, *open));
    }
    Ok(top)
}

fn count(nodes: &mut usize, limits: &Limits, pos: Pos) -> Result<()> {
    *nodes += 1;
    if *nodes > limits.max_nodes {
        return Err(Error::new(ErrorKind::TooManyNodes { max: limits.max_nodes }, pos));
    }
    Ok(())
}

/// Line and column of byte offset `at` (used for UTF-8 errors before lexing).
fn pos_of(input: &[u8], at: usize) -> Pos {
    let mut pos = Pos { line: 1, col: 1 };
    for &b in input.get(..at).unwrap_or(input) {
        if b == b'\n' {
            pos.line = pos.line.saturating_add(1);
            pos.col = 1;
        } else if b & 0xc0 != 0x80 {
            pos.col = pos.col.saturating_add(1);
        }
    }
    pos
}
