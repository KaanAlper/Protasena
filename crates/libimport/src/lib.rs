//! Protasena library import (layer L2): KiCad's symbol and footprint libraries converted into
//! Protasena [`Device`](protasena_library::Device)s and [`Package`](protasena_library::Package)s.
//!
//! Two users share this code: the `protasena-libconv` app, which converts KiCad's official
//! libraries into the published pack ([`run`]), and the application, which imports the user's
//! own KiCad libraries at run time ([`user`]).
//!
//! KiCad libraries are only ever read as data here; the converted pack is published separately
//! (`KaanAlper/Protasena-library`, CC-BY-SA 4.0) and never vendored into this repository.
//!
//! Stages ([`convert::run`]): footprints → [`protasena_library::Package`]s ([`footprint`],
//! [`pad`], [`layers`]); symbols (with `extends` applied) → [`protasena_library::Device`]s
//! ([`device`], [`symbol`], [`pin`]) linked to packages ([`link`]); categories and
//! Proteus-style names from `map.toml` ([`map`]); pack-wide unique names ([`names`]);
//! validation; writing ([`pack`]); and a final load of the written pack. Output is
//! deterministic: inputs are read in sorted order, results are sorted, nothing depends on time
//! or hash order. Units and axes: [`coords`].
//!
//! # Never crash
//! KiCad files are hostile input: reads are size-limited, and a file or symbol that cannot be
//! converted is a [`report::Failure`], never a panic.

pub mod convert;
pub mod coords;
pub mod device;
pub mod footprint;
pub mod glob;
pub mod inputs;
pub mod layers;
pub mod link;
pub mod map;
pub mod names;
pub mod pack;
pub mod pad;
pub mod par;
pub mod pin;
pub mod report;
pub mod symbol;
pub mod user;

pub use convert::{Options, run};
pub use map::Map;
pub use report::Report;
