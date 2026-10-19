//! Errors of the reader: every error carries the line and column where it was found.

use std::fmt;

/// A position in the input: 1-based line and column (columns count characters, not bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.col)
    }
}

/// What went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// The input is larger than [`crate::sexpr::Limits::max_input`].
    InputTooLarge { len: usize, max: usize },
    /// Lists are nested deeper than [`crate::sexpr::Limits::max_depth`].
    TooDeep { max: usize },
    /// More nodes than [`crate::sexpr::Limits::max_nodes`].
    TooManyNodes { max: usize },
    /// One atom or string is longer than [`crate::sexpr::Limits::max_atom`] bytes.
    AtomTooLong { max: usize },
    /// The input is not UTF-8.
    InvalidUtf8,
    /// A `"` string is not closed before the end of the input.
    UnterminatedString,
    /// A `)` without a matching `(`.
    UnexpectedClose,
    /// A `(` without a matching `)` before the end of the input.
    UnclosedList,
    /// The input holds no expression.
    Empty,
    /// More than one top-level expression where exactly one is expected.
    TrailingData,
    /// The top-level expression is not the expected kind of file.
    WrongRoot { expected: &'static str, found: String },
    /// A known token has the wrong shape (missing or malformed attribute).
    Invalid(String),
    /// A symbol `extends` a parent that is not in the library.
    MissingParent { symbol: String, parent: String },
    /// A chain of `extends` loops back on itself.
    ExtendsCycle { symbol: String },
    /// No symbol of this name in the library.
    UnknownSymbol(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge { len, max } => write!(f, "input is {len} bytes, more than the limit of {max}"),
            Self::TooDeep { max } => write!(f, "lists nested deeper than {max} levels"),
            Self::TooManyNodes { max } => write!(f, "more than {max} nodes"),
            Self::AtomTooLong { max } => write!(f, "atom or string longer than {max} bytes"),
            Self::InvalidUtf8 => write!(f, "input is not valid UTF-8"),
            Self::UnterminatedString => write!(f, "string is not closed"),
            Self::UnexpectedClose => write!(f, "')' without a matching '('"),
            Self::UnclosedList => write!(f, "'(' is not closed"),
            Self::Empty => write!(f, "no expression in the input"),
            Self::TrailingData => write!(f, "unexpected data after the top-level expression"),
            Self::WrongRoot { expected, found } => write!(f, "expected a ({expected} …) file, found '{found}'"),
            Self::Invalid(msg) => write!(f, "{msg}"),
            Self::MissingParent { symbol, parent } => write!(f, "symbol '{symbol}' extends '{parent}', which is not in the library"),
            Self::ExtendsCycle { symbol } => write!(f, "symbol '{symbol}' extends itself through a cycle"),
            Self::UnknownSymbol(name) => write!(f, "no symbol named '{name}'"),
        }
    }
}

/// A reader error with its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub pos: Pos,
}

impl Error {
    pub fn new(kind: ErrorKind, pos: Pos) -> Self {
        Self { kind, pos }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.pos, self.kind)
    }
}

impl std::error::Error for Error {}

/// Result of the reader.
pub type Result<T, E = Error> = std::result::Result<T, E>;
