//! Protasena KiCad reader (layer L0, standalone).
//!
//! Reads KiCad's S-expression files (`.kicad_sym` symbol libraries, `.kicad_mod` footprints)
//! into plain models. Written from KiCad's published file format documentation only, never from
//! KiCad's code. Input is hostile: depth and size limits ([`sexpr::Limits`]), no panics, every
//! error with line and column.
//!
//! Reading is lossless: tokens the models do not know are kept as [`Unknown`] trees next to
//! the typed fields, and `report()` counts them.
//!
//! ```
//! let lib = protasena_kicad::parse_symbol_lib(br#"(kicad_symbol_lib (version 20241209)
//!   (symbol "R" (property "Reference" "R" (at 0 0 0)) (symbol "R_0_1" (rectangle (start -1 -2) (end 1 2)))))"#)?;
//! assert_eq!(lib.symbols[0].value_of("Reference"), Some("R"));
//! assert!(lib.report().is_clean());
//! # Ok::<(), protasena_kicad::Error>(())
//! ```

pub mod common;
mod decode;
pub mod error;
pub mod footprint;
pub mod report;
pub mod sexpr;
pub mod symbol;

pub use error::{Error, ErrorKind, Pos, Result};
pub use footprint::{Footprint, parse_footprint};
pub use report::{Inspect, Report, Unknown};
pub use sexpr::{Limits, Sexpr};
pub use symbol::{Symbol, SymbolLib, parse_symbol_lib};
