//! Protasena design model (layer L1).
//!
//! A [`Project`] holds [`Sheet`]s (name, [`SheetSize`]) with their [`Component`]s, [`Wire`]s,
//! [`Junction`]s, net [`Label`]s, [`Bus`]es, [`BusEntry`]s, [`Terminal`]s and [`Text`]s, plus
//! [`DesignProperties`]. Every sheet and item has a stable typed id ([`ComponentId`], …) from
//! one project-wide counter.
//!
//! Edits are reversible operations: [`Project::apply`] performs an [`Op`] and returns the op that
//! undoes it; [`Op::Batch`] makes several ops one undo step. Annotation ([`Project::annotate`],
//! [`Project::reannotate_all`], [`Project::next_reference`]) returns ops too.
//!
//! # Units and layering
//! Coordinates are sheet coordinates in thou (1/1000 in), y up, as in `protasena-geom` and the
//! library. A component names its device ([`DeviceRef`]) instead of holding it: the library is
//! the same layer, and `protasena-netlist` joins the two (pin positions, connectivity).
//!
//! # Never crash
//! Operations are hostile input: a missing id, a used id, NaN or infinite coordinates, a
//! malformed item or batches nested too deeply give a [`DocError`] and leave the project as it
//! was. [`Project::issues`] checks a project that was built by hand or read from a file.
//!
//! # Example
//! ```
//! use protasena_doc::{Component, ComponentId, DeviceRef, Item, Op, Project};
//!
//! let mut project = Project::new("demo");
//! let sheet = project.sheets[0].id;
//! let id: ComponentId = project.fresh_id().unwrap();
//! let r = Component::new(id, DeviceRef::new("GENERIC", "RES"), "R?", "10k");
//! let undo = project.apply(Op::Add { sheet, item: Item::Component(r), index: None }).unwrap();
//! let undo_annotate = project.apply(project.annotate()).unwrap();
//! assert_eq!(project.component(id).unwrap().reference, "R1");
//! project.apply(undo_annotate).unwrap();
//! project.apply(undo).unwrap();
//! assert!(project.component(id).is_none());
//! ```
#![warn(missing_docs)]

mod annotate;
mod error;
mod ids;
mod item;
mod items;
mod op;
mod project;
mod properties;
#[cfg(feature = "serde")]
mod serde_impl;
mod sheet;

pub use annotate::split_reference;
pub use error::DocError;
pub use ids::{BusEntryId, BusId, ComponentId, Id, ItemId, JunctionId, LabelId, SheetId, TerminalId, TextId, WireId};
pub use item::Item;
pub use items::{Bus, BusEntry, Component, DeviceRef, FieldLabel, Junction, Label, Terminal, TerminalKind, Text, Wire};
pub use op::{MAX_BATCH_DEPTH, Op};
pub use project::{DesignProperties, FIRST_SHEET_NAME, Project};
pub use properties::Properties;
pub use sheet::{Sheet, SheetSize};
