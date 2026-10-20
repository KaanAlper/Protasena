//! `ROOT.DSN`, the schematic: a binary, little-endian file of sections, each introduced by an
//! ASCII banner ending in 0x1A: the header block, the design configuration (styles and the
//! embedded library) and one or more circuits (object records).

mod circuit;
mod config;
mod field;
mod header;
mod records;
mod text;

pub use circuit::{Circuit, record_offset};
pub use config::{Config, DeviceDef, LibraryEntry};
pub use field::{Coord, TextField, UNITS_PER_THOU};
pub use header::Header;
pub use records::{Component, Generator, Node, Record, Terminal, Wire};
pub use text::{TextRecord, TextSection, parse_property_block, parse_sections};

use crate::error::Error;
use crate::limits::{MAX_CIRCUITS, MAX_MEMBER_BYTES};

/// Name of the schematic member.
pub const DESIGN: &str = "ROOT.DSN";

/// A decoded `ROOT.DSN`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Design {
    /// The header block.
    pub header: Header,
    /// The design configuration, when its layout was recognised.
    pub config: Option<Config>,
    /// The circuit sections in file order.
    pub circuits: Vec<Circuit>,
}

impl Design {
    /// Every record of every circuit.
    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.circuits.iter().flat_map(|c| &c.records)
    }

    /// Number of [`Record::Unknown`] records in all circuits.
    pub fn unknown_count(&self) -> usize {
        self.circuits.iter().map(Circuit::unknown_count).sum()
    }
}

/// Decodes `ROOT.DSN`.
///
/// # Errors
/// [`Error::TooLarge`] above [`MAX_MEMBER_BYTES`]; [`Error::Format`] when the data does not
/// start with the schematic header. Anything after the header that cannot be decoded is
/// reported in the result (no configuration, unknown records), not as an error.
pub fn parse_design(data: &[u8]) -> Result<Design, Error> {
    let size = u64::try_from(data.len()).unwrap_or(u64::MAX);
    if size > MAX_MEMBER_BYTES {
        return Err(Error::TooLarge { what: DESIGN.into(), size, limit: MAX_MEMBER_BYTES });
    }
    let header = header::parse_header(data).ok_or_else(|| Error::Format { name: DESIGN.into(), message: "no schematic header".into() })?;
    let parsed = config::parse_config(data, header::HEADER_LEN);
    let after = parsed.as_ref().map_or(header::HEADER_LEN, |(_, end)| *end);
    let config = parsed.map(|(config, _)| config);
    let styles = config.as_ref().map(|c| c.text_styles.as_slice()).unwrap_or_default();
    let starts = banners(data, after);
    let circuits = starts
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let end = starts.get(i.saturating_add(1)).copied().unwrap_or(data.len());
            circuit::parse_circuit(data, start, end, styles)
        })
        .collect();
    Ok(Design { header, config, circuits })
}

/// Offsets of the circuit banners at or after `from` (at most [`MAX_CIRCUITS`]).
fn banners(data: &[u8], from: usize) -> Vec<usize> {
    let banner = circuit::CIRCUIT_BANNER;
    let Some(rest) = data.get(from..) else { return Vec::new() };
    rest.windows(banner.len()).enumerate().filter(|(_, w)| *w == banner).map(|(i, _)| i.saturating_add(from)).take(MAX_CIRCUITS).collect()
}
