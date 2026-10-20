//! Why a `.pdsprj` container or one of its members could not be read.

use std::fmt;
use std::path::PathBuf;

/// A reading failure. Messages never quote member contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The file could not be read.
    Io {
        /// The file.
        path: PathBuf,
        /// The operating system's message.
        message: String,
    },
    /// An archive or member is larger than the reader accepts.
    TooLarge {
        /// `"archive"` or the member name.
        what: String,
        /// Its size in bytes (at least).
        size: u64,
        /// The limit in bytes.
        limit: u64,
    },
    /// The archive has more members than [`MAX_MEMBERS`](crate::limits::MAX_MEMBERS).
    TooManyMembers {
        /// How many the archive declares.
        count: usize,
        /// The limit.
        limit: usize,
    },
    /// The data is not a ZIP archive, or its directory is damaged.
    NotZip(String),
    /// The archive uses a ZIP feature the reader does not support (ZIP64, several disks,
    /// encryption, a compression method other than stored and deflate).
    Unsupported(String),
    /// A member would expand far more than its compressed size suggests (a "zip bomb").
    SuspiciousRatio {
        /// The member.
        name: String,
        /// Compressed size.
        compressed: u64,
        /// Declared size.
        size: u64,
    },
    /// A member's data does not decompress to what the directory declares.
    Corrupt {
        /// The member.
        name: String,
        /// What is wrong.
        message: String,
    },
    /// Two members have the same name (compared without case).
    DuplicateMember(String),
    /// The archive has no member of that name.
    MissingMember(String),
    /// A member is not what its name says (`ROOT.DSN` without its header, a manifest that is
    /// not the project manifest).
    Format {
        /// The member.
        name: String,
        /// What is wrong.
        message: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
            Self::TooLarge { what, size, limit } => write!(f, "{what} has {size} bytes, more than the limit of {limit}"),
            Self::TooManyMembers { count, limit } => write!(f, "the archive has {count} members, more than the limit of {limit}"),
            Self::NotZip(m) => write!(f, "not a .pdsprj archive: {m}"),
            Self::Unsupported(m) => write!(f, "unsupported archive feature: {m}"),
            Self::SuspiciousRatio { name, compressed, size } => {
                write!(f, "{name} would expand from {compressed} to {size} bytes; refusing a likely zip bomb")
            }
            Self::Corrupt { name, message } => write!(f, "{name} is damaged: {message}"),
            Self::DuplicateMember(name) => write!(f, "the archive holds {name} twice"),
            Self::MissingMember(name) => write!(f, "the archive has no {name}"),
            Self::Format { name, message } => write!(f, "{name}: {message}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_problem() {
        let e = Error::SuspiciousRatio { name: "ROOT.DSN".into(), compressed: 10, size: 1 << 30 };
        assert!(e.to_string().contains("zip bomb"));
        assert_eq!(Error::MissingMember("ROOT.DSN".into()).to_string(), "the archive has no ROOT.DSN");
        let big = Error::TooLarge { what: "archive".into(), size: 3, limit: 2 };
        assert_eq!(big.to_string(), "archive has 3 bytes, more than the limit of 2");
    }
}
