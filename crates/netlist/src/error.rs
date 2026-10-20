//! Error type of the netlist exporters.

use std::fmt;

/// Why a netlist could not be written or read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetlistError {
    /// JSON (de)serialization failed.
    Json(String),
    /// The JSON is not a netlist of a version we read.
    Format(String),
}

impl fmt::Display for NetlistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(m) => write!(f, "invalid JSON netlist: {m}"),
            Self::Format(m) => write!(f, "not a Protasena netlist: {m}"),
        }
    }
}

impl std::error::Error for NetlistError {}
