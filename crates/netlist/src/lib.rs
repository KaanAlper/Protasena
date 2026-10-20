//! Protasena connectivity (layer L2).
//!
//! [`build`] turns a [`Project`] and a [`DeviceSource`] (any
//! [`Library`](protasena_library::Library)) into a [`Netlist`]: nets with stable names, parts,
//! unconnected pins and a basic electrical rules check. [`placed_pins`] gives a component's pins
//! in sheet space (symbol pin × component transform). [`export`] writes our JSON netlist and a
//! SPICE-style text netlist.
//!
//! # Connection rules
//! Positions are compared with a tolerance of [`TOLERANCE`] thou, sheet by sheet.
//! - **Attachment points** are visible pin ends ([`Pin::at`](protasena_library::Pin)), wire
//!   end points (first and last vertex), junctions, label anchors and terminal anchors.
//! - Attachment points at the same position connect.
//! - An attachment point lying on a wire segment (end or interior) connects to that wire: a
//!   wire ending on another wire makes a T, a junction dot joins every wire through it, a label
//!   names the wire it sits on, a pin end on a wire connects.
//! - All vertices of one wire are one conductor. Two wires that merely cross, or touch only at
//!   interior vertices (bends), do **not** connect.
//! - Labels with the same text (trimmed, case-sensitive) are one net across **all sheets**.
//!   Terminals of kind default, input, output and bidir join by name the same way; power and
//!   ground terminals join the net of their name (`VCC` / `GND` when unnamed). Bus terminals,
//!   buses and bus entries carry no nets yet.
//! - Power-symbol devices (`Symbol::power`) join the net named by the component's value, or by
//!   their pin name when the value is empty; they are not parts and their pins are not nodes.
//! - Hidden power pins (power in / power out) join the net named by the pin name; other hidden
//!   pins connect to nothing.
//! - Gates of one part (same annotated reference, same device, different units) share pins by
//!   number; any other repeated reference is a [`ErcKind::DuplicateReference`] error.
//!
//! Proteus's exact rules (crossing wires, pins on wire interiors, label case, the scope of
//! terminals, unnamed power terminals) still need observation; see `docs/gaps.md`.
//!
//! # Net names
//! A net takes the best of its names: power names (power symbols, power/ground terminals,
//! hidden power pins) first, then labels and other terminals, ties in natural order; the rest
//! are its aliases. Unnamed nets are `#NET0001`, `#NET0002`, … in the natural order of their
//! lowest pin (`R2.1` before `R10.1`), so names do not depend on item order or ids.
//!
//! # Never crash
//! Unknown devices, missing gates, NaN or huge coordinates, duplicate ids and references are
//! reported in [`Netlist::erc`]; the rest of the design is still connected.
//!
//! # Example
//! ```
//! use protasena_doc::{Component, DeviceRef, Item, Op, Project, Wire};
//! use protasena_geom::{Orient, Pt, Transform, Vec2};
//! use protasena_library::Library;
//!
//! let mut p = Project::new("demo");
//! let sheet = p.sheets[0].id;
//! let place = |x| Transform::new(Orient::IDENTITY, Vec2::new(x, 0.0));
//! for (reference, x) in [("R1", 0.0), ("R2", 1000.0)] {
//!     let c = Component::new(p.fresh_id().unwrap(), DeviceRef::new("GENERIC", "RES"), reference, "10k").placed(place(x));
//!     p.apply(Op::Add { sheet, item: Item::Component(c), index: None }).unwrap();
//! }
//! // R1 pin 2 is at x = 200, R2 pin 1 at x = 800.
//! let w = Wire { id: p.fresh_id().unwrap(), points: vec![Pt::new(200.0, 0.0), Pt::new(800.0, 0.0)] };
//! p.apply(Op::Add { sheet, item: Item::Wire(w), index: None }).unwrap();
//! let netlist = protasena_netlist::build(&p, &Library::builtin());
//! let net = netlist.net_of("R1", "2").unwrap();
//! assert_eq!(net.name, "#NET0001");
//! assert_eq!(net.nodes.len(), 2);
//! assert_eq!(netlist.unconnected_pins.len(), 2); // R1.1 and R2.2
//! ```
#![warn(missing_docs)]

mod collect;
mod error;
pub mod export;
mod geometry;
mod model;
mod order;
mod pins;
mod resolve;
mod source;

pub use error::NetlistError;
pub use geometry::{MAX_COORD, TOLERANCE};
pub use model::{ErcItem, ErcKind, Net, Netlist, Node, Part, PartPin, Severity};
pub use order::{natural_cmp, node_cmp};
pub use pins::{PlacedPin, gate_units, placed_pins};
pub use resolve::AUTO_NET_PREFIX;
pub use source::DeviceSource;

use protasena_doc::Project;

/// The connectivity of a project (see the crate docs for the rules).
pub fn build<S: DeviceSource + ?Sized>(project: &Project, devices: &S) -> Netlist {
    collect::Graph::build(project, devices).resolve()
}
