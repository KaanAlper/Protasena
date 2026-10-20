//! Protasena `.pdsprj` reader (layer L0, standalone).
//!
//! Reads the project files of Proteus 8. The structure is worked out only from data files the
//! program saved (black-box analysis); the program itself is never inspected. All input is
//! hostile: every read is bounds-checked, every size is capped ([`limits`]) and nothing panics.
//! The format is described in `docs/pdsprj.md`.
//!
//! - [`Container`]: the ZIP archive (members, extraction with size, ratio and CRC checks).
//! - [`parse_manifest`]: `PROJECT.XML` (release, file version, times, scripts).
//! - [`parse_design`]: `ROOT.DSN` (header, style names, embedded library, circuits with wires,
//!   nodes, components, terminals, generators and probes; unknown records kept as
//!   [`Record::Unknown`] with their tag and size only).
//! - [`Project::open`] / [`Project::from_bytes`]: all of the above in one step.
//!
//! Coordinates are in file units of 10 nm ([`UNITS_PER_THOU`] per thou).
//!
//! # Example
//! ```
//! let err = protasena_pdsprj::Project::from_bytes(b"not a zip".to_vec()).unwrap_err();
//! assert!(matches!(err, protasena_pdsprj::Error::NotZip(_)));
//! ```
#![warn(missing_docs)]

mod container;
mod crc32;
mod cursor;
mod dsn;
mod error;
pub mod limits;
mod manifest;
mod project;

pub use container::{Container, Member, Method};
pub use crc32::crc32;
pub use dsn::{
    Circuit, Component, Config, Coord, DESIGN, Design, DeviceDef, Generator, Header, LibraryEntry, Node, Record, Terminal, TextField, TextRecord,
    TextSection, UNITS_PER_THOU, Wire, parse_design, parse_property_block, parse_sections, record_offset,
};
pub use error::Error;
pub use manifest::{MANIFEST, Manifest, parse_manifest};
pub use project::Project;
