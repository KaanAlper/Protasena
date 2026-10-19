//! Protasena device library (layer L1).
//!
//! The device model ([`Device`]: Pick Devices index fields, Edit Component property
//! definitions, [`Symbol`], [`PackageRef`]s with pin maps, simulation model link, source), PCB
//! [`Package`]s, the on-disk pack format and its validating loader ([`Library::load_dir`]),
//! keyword/category search for Pick Devices ([`Library::search`], [`Library::categories`]) and a
//! small built-in set of generic parts drawn in code ([`Library::builtin`]).
//!
//! # Units
//! All library geometry is in **thou** (1/1000 in), y up; see [`units`].
//!
//! # Never crash
//! Packs are hostile input: files are size-limited, every list is length-capped while it is
//! read ([`limits`]), and inconsistencies are reported as [`Issue`]s, never panics.
//!
//! # Example
//! ```
//! use protasena_library::{Library, Query};
//!
//! let lib = Library::builtin();
//! let res = lib.get("res").unwrap();
//! assert_eq!(res.prefix, "R");
//! let hits = lib.search(&Query::keywords("capacitor"));
//! assert!(hits.iter().any(|d| d.name == "CAP-ELEC"));
//! ```
#![warn(missing_docs)]

mod bounded;
mod builtin;
mod device;
mod error;
mod library;
pub mod limits;
mod pack;
mod package;
mod symbol;
pub mod units;
mod validate;

pub use builtin::BUILTIN_LIBRARY;
pub use device::{Device, PackageRef, PropDef, PropKind, SimModelRef, Source};
pub use error::{Issue, IssueKind, LibraryError};
pub use library::{Category, Library, LibraryInfo, Query};
pub use pack::{
    DEVICES_FORMAT, INDEX_FILE, INDEX_FORMAT, IndexEntry, PACKAGES_FORMAT, PackIndex, VERSION, parse_devices, parse_index, parse_packages,
};
pub use package::{LayerGraphic, Package, Pad, PadShape, layer};
pub use symbol::{Fill, Graphic, Pin, PinDir, PinKind, Shape, Symbol, Unit};
