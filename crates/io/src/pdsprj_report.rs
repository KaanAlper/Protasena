//! What a `.pdsprj` import did and did not carry over, and why one can fail.

use std::fmt;

/// Records of one unknown kind that the import skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownRecords {
    /// The records' tag byte.
    pub tag: u8,
    /// How many.
    pub count: usize,
    /// Their bytes in all.
    pub bytes: usize,
}

/// A device definition embedded in the project. It is reported, not converted: components
/// reference their device by name in the [`PDSPRJ_LIBRARY`](crate::PDSPRJ_LIBRARY) library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedDevice {
    /// Device name.
    pub name: String,
    /// Its category, if the definition names one.
    pub category: Option<String>,
    /// Number of property definitions.
    pub property_defs: usize,
}

/// The outcome of [`import_pdsprj`](crate::import_pdsprj).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportReport {
    /// Archive members.
    pub members: Vec<String>,
    /// Program release that saved the project (the release number × 100).
    pub release: Option<u32>,
    /// Sheets created.
    pub sheets: usize,
    /// Components imported.
    pub components: usize,
    /// Wires imported.
    pub wires: usize,
    /// Buses imported.
    pub buses: usize,
    /// Junction dots imported (nodes joining three or more records).
    pub junctions: usize,
    /// Terminals imported.
    pub terminals: usize,
    /// Generators and probes skipped (simulation objects are not supported yet).
    pub skipped_generators: usize,
    /// Nodes joining fewer than three records (no junction dot) skipped.
    pub skipped_nodes: usize,
    /// Records of unknown kinds, by tag.
    pub unknown_records: Vec<UnknownRecords>,
    /// Embedded device definitions (not converted).
    pub embedded_devices: Vec<EmbeddedDevice>,
    /// Embedded symbols (not converted).
    pub embedded_symbols: usize,
    /// What the import could not carry over or had to guess.
    pub warnings: Vec<String>,
}

impl ImportReport {
    pub(crate) fn count_unknown(&mut self, tag: u8, bytes: usize) {
        match self.unknown_records.iter_mut().find(|u| u.tag == tag) {
            Some(u) => {
                u.count = u.count.saturating_add(1);
                u.bytes = u.bytes.saturating_add(bytes);
            }
            None => self.unknown_records.push(UnknownRecords { tag, count: 1, bytes }),
        }
    }
}

/// Why a `.pdsprj` could not be imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportError(pub protasena_pdsprj::Error);

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot import the Proteus project: {}", self.0)
    }
}

impl std::error::Error for ImportError {}

impl From<protasena_pdsprj::Error> for ImportError {
    fn from(e: protasena_pdsprj::Error) -> Self {
        Self(e)
    }
}
