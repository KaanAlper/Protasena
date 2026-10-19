//! KiCad's S-expression syntax: a tree of lists, bare atoms and quoted strings.
//!
//! The parser is iterative (no recursion), treats its input as hostile and enforces
//! [`Limits`]. [`Sexpr`]'s `Display` writes the tree back in a form that parses to the same
//! tree.

mod lexer;
mod parser;
mod write;

use crate::error::Pos;

pub use parser::{parse, parse_all, parse_all_with, parse_with};

/// Resource limits for parsing hostile input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Largest accepted input in bytes.
    pub max_input: usize,
    /// Deepest accepted list nesting.
    pub max_depth: usize,
    /// Longest accepted atom or (decoded) string in bytes.
    pub max_atom: usize,
    /// Most nodes (lists, atoms and strings) in one input.
    pub max_nodes: usize,
}

impl Limits {
    pub const DEFAULT: Self = Self { max_input: 64 << 20, max_depth: 256, max_atom: 1 << 20, max_nodes: 16 << 20 };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One node of the tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Sexpr {
    /// `( … )`
    List { items: Vec<Sexpr>, pos: Pos },
    /// A bare token such as `hide`, `1.27` or `F.Cu`.
    Atom { text: String, pos: Pos },
    /// A `"quoted"` string, escapes decoded.
    Str { text: String, pos: Pos },
}

impl Sexpr {
    /// Where the node starts in the input.
    pub fn pos(&self) -> Pos {
        match self {
            Self::List { pos, .. } | Self::Atom { pos, .. } | Self::Str { pos, .. } => *pos,
        }
    }

    /// The items of a list.
    pub fn as_list(&self) -> Option<&[Sexpr]> {
        match self {
            Self::List { items, .. } => Some(items),
            _ => None,
        }
    }

    /// The text of a bare atom.
    pub fn as_atom(&self) -> Option<&str> {
        match self {
            Self::Atom { text, .. } => Some(text),
            _ => None,
        }
    }

    /// The text of a bare atom or a quoted string.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Atom { text, .. } | Self::Str { text, .. } => Some(text),
            Self::List { .. } => None,
        }
    }

    /// The token name of a list: its first item when that is a bare atom.
    pub fn head(&self) -> Option<&str> {
        self.as_list()?.first()?.as_atom()
    }

    /// The items of a list after its token name (empty for atoms and empty lists).
    pub fn args(&self) -> &[Sexpr] {
        self.as_list().and_then(|items| items.get(1..)).unwrap_or_default()
    }

    /// The first child list named `name`.
    pub fn child(&self, name: &str) -> Option<&Sexpr> {
        self.args().iter().find(|c| c.head() == Some(name))
    }

    /// A short name for reports: the token name of a list, the text of an atom, `"<string>"`
    /// for a quoted string, `"<list>"` for a list that does not start with an atom.
    pub fn name(&self) -> &str {
        match self {
            Self::List { .. } => self.head().unwrap_or("<list>"),
            Self::Atom { text, .. } => text,
            Self::Str { .. } => "<string>",
        }
    }
}

#[cfg(test)]
mod tests;
