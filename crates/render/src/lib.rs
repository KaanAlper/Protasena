//! Protasena drawing (layer L2).
//!
//! Turns the design and the library into toolkit-independent drawing primitives
//! ([`Prim`]: lines, polylines, circles, arcs, Bézier curves, filled outlines and text runs with
//! an anchor, alignment and direction) in sheet coordinates: thou, y up. Each primitive carries
//! a semantic [`Ink`] (body, pin, wire, label…) instead of a colour, so the user interface maps
//! inks to its theme, and the design item it belongs to, for selection highlighting.
//!
//! - [`symbol_prims`]: a device's gate placed by a transform (body graphics, pins with names
//!   and numbers); used by the editing window and the Pick Devices preview.
//! - [`component_prims`], [`wire_prims`], [`terminal_prims`], [`subcircuit_prims`], …: one
//!   design item each.
//! - [`sheet_display_list`]: a whole sheet in drawing order, with its border.
//! - [`package_prims`]: a PCB package (pads and silkscreen) for the package preview.
//! - [`geometry`]: arcs through three points, flattening, readable text directions.
//! - [`export`]: display lists as SVG, PDF, EPS, DXF, PNG and BMP files (Export Graphics,
//!   Print Design).
//!
//! Hit testing stays in the engine (`protasena_engine::hit`).
//!
//! # Never crash
//! Library data is hostile: non-finite shapes are skipped, the number of points and graphics
//! drawn per item is capped, and nothing here indexes or unwraps.
//!
//! # Example
//! ```
//! use protasena_geom::Transform;
//! use protasena_library::Library;
//! use protasena_render::{Ink, symbol_prims};
//!
//! let lib = Library::builtin();
//! let prims = symbol_prims(lib.get("RES").unwrap(), 0, Transform::IDENTITY);
//! assert_eq!(prims.iter().filter(|p| p.ink == Ink::Pin).count(), 2);
//! ```
#![warn(missing_docs)]

pub mod export;
pub mod geometry;
mod graphics;
mod items;
mod package;
mod prim;
pub mod style;
mod subcircuit;
mod symbol;

pub use graphics::{GRAPHIC_WIDTH, graphic_prims, graphic_shape};
pub use items::{
    BUS_WIDTH, JUNCTION_RADIUS, LABEL_SIZE, MISSING_HALF, REFERENCE_SIZE, TEXT_SIZE, VALUE_SIZE, WIRE_WIDTH, bus_entry_prims, bus_prims,
    component_prims, field_align, find_device, item_prims, junction_prims, label_prims, sheet_display_list, sheet_rect, styled_sheet_display_list,
    terminal_prims, text_prims, wire_prims,
};
pub use package::package_prims;
pub use prim::{DisplayList, HAlign, Ink, Prim, Shape, VAlign};
pub use style::{ink_colour, styled};
pub use subcircuit::{PORT_LENGTH, subcircuit_prims};
pub use symbol::{BODY_WIDTH, PIN_TEXT, PIN_WIDTH, body_prims, graphic, pin_prims, symbol_prims};
