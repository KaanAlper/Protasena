//! Error types of project files and netlist files.

use std::fmt;
use std::path::PathBuf;

use protasena_doc::DocError;
use protasena_netlist::NetlistError;

/// Why a project file could not be read.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadError {
    /// The file could not be read.
    Io {
        /// The file.
        path: PathBuf,
        /// The operating system's message.
        message: String,
    },
    /// The file is larger than [`MAX_FILE_BYTES`](crate::limits::MAX_FILE_BYTES).
    TooLarge {
        /// Its size in bytes (at least).
        size: u64,
        /// The limit in bytes.
        limit: u64,
    },
    /// Arrays and objects are nested deeper than [`MAX_DEPTH`](crate::limits::MAX_DEPTH).
    TooDeep {
        /// The limit.
        limit: usize,
    },
    /// The file is not valid JSON, or the project in it has the wrong shape (with line and
    /// column).
    Json(String),
    /// The file is JSON but not a Protasena project (no or another format tag, no version).
    NotAProject(String),
    /// The file was written in a version this build does not read.
    UnsupportedVersion {
        /// The file's version.
        found: u64,
        /// The newest version this build reads.
        supported: u32,
    },
    /// The project has more sheets or items than the loader accepts.
    TooMany {
        /// `"sheets"` or `"items"`.
        what: &'static str,
        /// How many there are (at least).
        count: usize,
        /// The limit.
        limit: usize,
    },
    /// The project parsed but is unusable (duplicate ids): every such problem.
    Invalid(Vec<DocError>),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
            Self::TooLarge { size, limit } => write!(f, "the file has {size} bytes, more than the limit of {limit}"),
            Self::TooDeep { limit } => write!(f, "the file is nested deeper than {limit} levels"),
            Self::Json(m) => write!(f, "invalid project file: {m}"),
            Self::NotAProject(m) => write!(f, "not a Protasena project: {m}"),
            Self::UnsupportedVersion { found: 0, .. } => f.write_str("project file version 0 does not exist"),
            Self::UnsupportedVersion { found, supported } => {
                write!(f, "the project was saved by a newer Protasena (file version {found}, this one reads up to {supported})")
            }
            Self::TooMany { what, count, limit } => write!(f, "the project has {count} {what}, more than the limit of {limit}"),
            Self::Invalid(errors) => {
                write!(f, "{} problem(s) make the project unusable", errors.len())?;
                errors.iter().try_for_each(|e| write!(f, "\n  {e}"))
            }
        }
    }
}

impl std::error::Error for LoadError {}

/// Why a project could not be saved. A failed save leaves the previous file as it was.
#[derive(Debug, Clone, PartialEq)]
pub enum SaveError {
    /// A number is NaN or infinite (`what` names it): the file would not load back.
    NonFinite(&'static str),
    /// Serialization failed.
    Json(String),
    /// The file (or its backup) could not be written.
    Io {
        /// The file.
        path: PathBuf,
        /// The operating system's message.
        message: String,
    },
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite(what) => write!(f, "cannot save: {what} is not a finite number"),
            Self::Json(m) => write!(f, "cannot save: {m}"),
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
        }
    }
}

impl std::error::Error for SaveError {}

/// Why a netlist file could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// The netlist could not be serialized.
    Netlist(NetlistError),
    /// The file could not be written.
    Io {
        /// The file.
        path: PathBuf,
        /// The operating system's message.
        message: String,
    },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Netlist(e) => e.fmt(f),
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
        }
    }
}

impl std::error::Error for ExportError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_explain_the_problem() {
        let newer = LoadError::UnsupportedVersion { found: 3, supported: 1 };
        assert!(newer.to_string().contains("newer Protasena"));
        assert!(LoadError::UnsupportedVersion { found: 0, supported: 1 }.to_string().contains("does not exist"));
        let invalid = LoadError::Invalid(vec![DocError::IdsExhausted, DocError::BatchTooDeep]);
        assert_eq!(invalid.to_string().lines().count(), 3);
        let many = LoadError::TooMany { what: "sheets", count: 5, limit: 4 };
        assert_eq!(many.to_string(), "the project has 5 sheets, more than the limit of 4");
        assert!(SaveError::NonFinite("wire point").to_string().contains("wire point"));
        let io = ExportError::Io { path: "a/b.cir".into(), message: "denied".into() };
        assert!(io.to_string().ends_with("denied"));
    }
}
