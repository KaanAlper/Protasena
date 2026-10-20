//! Error type of the engine.

use std::fmt;

use protasena_doc::DocError;
use protasena_io::{ExportError, LoadError, SaveError};

/// Why a command or an interactive event was refused. A refused command leaves the session as
/// it was.
#[derive(Debug, Clone, PartialEq)]
pub enum EngineError {
    /// No command has this id.
    UnknownCommand(String),
    /// The command cannot run now (`why` says what is missing, e.g. "nothing selected").
    Disabled {
        /// The command id.
        cmd: String,
        /// The reason.
        why: String,
    },
    /// The JSON parameters are missing, malformed or out of range.
    BadParams {
        /// The command id.
        cmd: String,
        /// What is wrong.
        msg: String,
    },
    /// The design model refused the edit.
    Doc(DocError),
    /// A project file could not be read.
    Load(LoadError),
    /// A project file could not be written.
    Save(SaveError),
    /// A netlist file could not be written.
    Export(ExportError),
    /// A command panicked; the design was restored to its state before the command.
    Internal {
        /// The command id.
        cmd: String,
        /// The panic message.
        msg: String,
    },
    /// Any other refusal, in plain words.
    Other(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand(id) => write!(f, "unknown command `{id}`"),
            Self::Disabled { cmd, why } => write!(f, "{cmd}: {why}"),
            Self::BadParams { cmd, msg } => write!(f, "{cmd}: {msg}"),
            Self::Doc(e) => e.fmt(f),
            Self::Load(e) => e.fmt(f),
            Self::Save(e) => e.fmt(f),
            Self::Export(e) => e.fmt(f),
            Self::Internal { cmd, msg } => write!(f, "{cmd} failed unexpectedly ({msg}); the design was restored"),
            Self::Other(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<DocError> for EngineError {
    fn from(e: DocError) -> Self {
        Self::Doc(e)
    }
}

impl From<LoadError> for EngineError {
    fn from(e: LoadError) -> Self {
        Self::Load(e)
    }
}

impl From<SaveError> for EngineError {
    fn from(e: SaveError) -> Self {
        Self::Save(e)
    }
}

impl From<ExportError> for EngineError {
    fn from(e: ExportError) -> Self {
        Self::Export(e)
    }
}

/// Result of engine calls.
pub type Result<T, E = EngineError> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_command() {
        let e = EngineError::BadParams { cmd: "wire".into(), msg: "`points` is required".into() };
        assert_eq!(e.to_string(), "wire: `points` is required");
        assert!(EngineError::UnknownCommand("x".into()).to_string().contains("`x`"));
        assert!(EngineError::Internal { cmd: "a".into(), msg: "boom".into() }.to_string().contains("restored"));
    }
}
