//! Protasena engine (layer L4): commands, the editing session, undo, and Proteus's editing
//! modes.
//!
//! # Everything is a command
//! Every user-visible action is a [`CommandSpec`] in the registry ([`command_specs`]): a stable
//! lower-case id, Proteus's label, menu path and shortcut, a params doc, an enabled rule and a
//! JSON form. Menus, toolbars, keys, the CLI and the control channel all reach the same
//! commands:
//! - [`Session::execute`] runs the JSON form. It never opens a dialog: what is not given takes
//!   its default. A command that changes the design is exactly one undo step.
//! - [`Session::invoke`] is a menu pick or key: a command with a dialog returns
//!   [`Outcome::Dialog`] with a [`Request`] for the UI, which shows the dialog and then calls
//!   `execute` with the values entered.
//!
//! # Interactive modes
//! [`Session::pointer`] and [`Session::key`] feed editing-window events to the active
//! [`Mode`]'s state machine ([`Machine`]); [`Session::preview`] gives the rubber-band geometry
//! (the wire being routed, the component about to be placed, dragged items, the selection
//! box). The machines change the design only through commands. See [`interact`].
//!
//! # Units
//! Sheet coordinates in thou (1/1000 in), y up, as in the design model. The View menu's snap
//! settings are 10 th, 50 th, 100 th (0.1 in, the default) and 500 th.
//!
//! # Never crash
//! Parameters are hostile input ([`args`]): types, finiteness, sizes and counts are checked.
//! Commands run under `catch_unwind`; a panic restores the design and is reported as
//! [`EngineError::Internal`].
//!
//! # Example
//! ```
//! use protasena_engine::Session;
//! use serde_json::json;
//!
//! let mut s = Session::new();
//! s.execute("pick-device", &json!({ "pick": "RES" })).unwrap();
//! s.execute("place-component", &json!({ "at": [0, 0] })).unwrap();
//! s.execute("place-component", &json!({ "at": [1000, 0] })).unwrap();
//! let w = s.execute("wire", &json!({ "from": { "component": "R1", "pin": "2" }, "to": { "component": "R2", "pin": "1" } })).unwrap();
//! assert_eq!(w["points"], json!([[200.0, 0.0], [800.0, 0.0]]));
//! s.execute("undo", &json!({})).unwrap();
//! assert!(s.sheet().unwrap().wires.is_empty());
//! ```
#![warn(missing_docs)]

pub mod args;
mod bodies;
mod cmd;
mod error;
mod follow;
pub mod hit;
pub mod interact;
mod mode;
mod registry;
mod request;
pub mod route;
mod scene;
mod session;
mod view;
mod wiring;

pub use error::{EngineError, Result};
pub use hit::Hit;
pub use interact::{Button, Feedback, Machine, Modifiers, PointerEvent, PointerKind, Preview, WireDraft};
pub use mode::Mode;
pub use registry::{CommandInfo, CommandSpec, Outcome, command_for_key, command_specs, find_command};
pub use request::Request;
pub use session::{Clipboard, MAX_UNDO, Session, Step};
pub use view::{Snap, View};
pub use wiring::Anchor;
