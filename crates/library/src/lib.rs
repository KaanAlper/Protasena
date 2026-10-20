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
//! # Memory
//! A loaded pack keeps its devices in memory, with equal symbols and pin maps shared between
//! devices, and reads each package file again the first time one of its packages is asked for
//! ([`Library::package`]). The full KiCad pack (22 583 devices, 15 415 packages) takes about
//! 200 MB this way (`examples/pack_stats.rs` measures it).
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
mod intern;
mod library;
pub mod limits;
mod pack;
mod package;
mod store;
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
