//! Errors of the library loader and validator.

use std::fmt;
use std::path::PathBuf;

/// Why a pack could not be loaded or a library could not be built.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryError {
    /// A file could not be read.
    Io {
        /// The file.
        path: PathBuf,
        /// The operating system's message.
        message: String,
    },
    /// A file, or the pack as a whole, is larger than the loader accepts.
    TooLarge {
        /// The file (the pack directory for the total).
        path: PathBuf,
        /// Its size in bytes (at least).
        size: u64,
        /// The limit in bytes.
        limit: u64,
    },
    /// A file is not valid JSON of the expected shape.
    Json {
        /// The file.
        path: PathBuf,
        /// What the parser reported (with line and column).
        message: String,
    },
    /// A file has the wrong format tag or an unsupported version.
    Format {
        /// The file.
        path: PathBuf,
        /// What is wrong.
        message: String,
    },
    /// The data parsed, but is inconsistent: every problem found, in file order.
    Invalid(Vec<Issue>),
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
            Self::TooLarge { path, size, limit } => write!(f, "{}: {size} bytes is more than the limit of {limit}", path.display()),
            Self::Json { path, message } => write!(f, "{}: invalid JSON: {message}", path.display()),
            Self::Format { path, message } => write!(f, "{}: {message}", path.display()),
            Self::Invalid(issues) => {
                write!(f, "{} problem(s) in the library", issues.len())?;
                for issue in issues {
                    write!(f, "\n  {issue}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for LibraryError {}

/// One validation problem, located by library and device (or package).
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    /// The pack library.
    pub library: String,
    /// The device or package concerned, if any.
    pub item: Option<String>,
    /// What is wrong.
    pub kind: IssueKind,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.item {
            Some(item) => write!(f, "{}/{item}: {}", self.library, self.kind),
            None => write!(f, "{}: {}", self.library, self.kind),
        }
    }
}

/// The kinds of validation problem.
#[derive(Debug, Clone, PartialEq)]
pub enum IssueKind {
    /// A required name is empty.
    EmptyName(&'static str),
    /// A name is longer than the limit or contains control characters.
    BadName(&'static str),
    /// Two libraries in the index have the same name.
    DuplicateLibrary,
    /// An added library ([`crate::Library::with_extra`]) has the name of a library already
    /// loaded.
    LibraryTaken,
    /// Two devices share a name (compared without case); `first_library` holds the first one.
    DuplicateDevice {
        /// Library of the device seen first.
        first_library: String,
    },
    /// Two packages share a name.
    DuplicatePackage {
        /// Library of the package seen first.
        first_library: String,
    },
    /// A device refers to a package the pack does not have.
    DanglingPackage(String),
    /// The default package is not one of the device's packages.
    BadDefaultPackage(String),
    /// A pin map entry names a pin the symbol does not have.
    PinMapMissingPin {
        /// The package.
        package: String,
        /// The pin number.
        pin: String,
    },
    /// A pin map entry names a pad the package does not have.
    PinMapMissingPad {
        /// The package.
        package: String,
        /// The pad name.
        pad: String,
    },
    /// A pin is mapped twice in one package.
    PinMappedTwice {
        /// The package.
        package: String,
        /// The pin number.
        pin: String,
    },
    /// A device lists the same package twice.
    PackageListedTwice(String),
    /// Two pins of one unit share a number.
    DuplicatePin(String),
    /// Two pads of a package share a name.
    DuplicatePad(String),
    /// Two property definitions share a name.
    DuplicatePropDef(String),
    /// A keyword property's default is not one of its choices.
    BadKeywordDefault {
        /// The property.
        prop: String,
        /// The default value.
        value: String,
    },
    /// The symbol has no units.
    NoUnits,
    /// A coordinate or length is not finite (or a size is negative).
    NonFinite(&'static str),
    /// A count exceeds a loader limit.
    TooMany {
        /// What was counted.
        what: &'static str,
        /// How many there are.
        count: usize,
        /// The limit.
        limit: usize,
    },
    /// The index's device count disagrees with the library file.
    CountMismatch {
        /// Count in `index.json`.
        index: u64,
        /// Devices in the file.
        file: usize,
    },
    /// A device or package in a library file names a different library.
    LibraryMismatch(String),
    /// A file reference in the index is not a plain relative path inside the pack.
    BadPath(String),
}

impl fmt::Display for IssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName(what) => write!(f, "empty {what} name"),
            Self::BadName(what) => write!(f, "{what} name is too long or contains control characters"),
            Self::DuplicateLibrary => write!(f, "library listed twice in the index"),
            Self::LibraryTaken => write!(f, "a library of this name is already loaded"),
            Self::DuplicateDevice { first_library } => write!(f, "device name already used in library {first_library}"),
            Self::DuplicatePackage { first_library } => write!(f, "package name already used in library {first_library}"),
            Self::DanglingPackage(p) => write!(f, "package {p} does not exist"),
            Self::BadDefaultPackage(p) => write!(f, "default package {p} is not one of the device's packages"),
            Self::PinMapMissingPin { package, pin } => write!(f, "{package}: pin map names pin {pin}, which the symbol does not have"),
            Self::PinMapMissingPad { package, pad } => write!(f, "{package}: pin map names pad {pad}, which the package does not have"),
            Self::PinMappedTwice { package, pin } => write!(f, "{package}: pin {pin} is mapped twice"),
            Self::PackageListedTwice(p) => write!(f, "package {p} is listed twice"),
            Self::DuplicatePin(n) => write!(f, "pin number {n} is used twice in one unit"),
            Self::DuplicatePad(n) => write!(f, "pad {n} is defined twice"),
            Self::DuplicatePropDef(n) => write!(f, "property {n} is defined twice"),
            Self::BadKeywordDefault { prop, value } => write!(f, "property {prop}: default {value} is not one of its choices"),
            Self::NoUnits => write!(f, "symbol has no units"),
            Self::NonFinite(what) => write!(f, "{what} has a coordinate or size that is not a finite number"),
            Self::TooMany { what, count, limit } => write!(f, "{count} {what} (limit {limit})"),
            Self::CountMismatch { index, file } => write!(f, "index says {index} devices, the file has {file}"),
            Self::LibraryMismatch(other) => write!(f, "entry says it belongs to library {other}"),
            Self::BadPath(p) => write!(f, "file {p} is not a relative path inside the pack"),
        }
    }
}
