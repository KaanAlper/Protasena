//! Protasena project files (layer L3).
//!
//! - **Project files** (`.psproj`): [`save`] / [`load`], or [`encode`] / [`decode`] in memory.
//! - **Netlist files**: [`export_netlist`] in [`NetlistFormat::Json`] or
//!   [`NetlistFormat::Spice`].
//! - **The device library**: [`load_library`] merges the built-in parts with the pack in
//!   [`library_dir`] (`PROTASENA_LIBRARY_DIR`).
//! - [`atomic_write`] replaces a file in one step, optionally keeping a backup.
//! - **System settings** (per user, the System menu): [`load_settings`] / [`save_settings`] at
//!   [`settings_path`] (in [`config_dir`], or `PROTASENA_CONFIG_DIR`); reading never fails.
//!
//! - **Proteus projects** (`.pdsprj`, read-only): [`import_pdsprj`] maps what the
//!   `protasena-pdsprj` reader decodes (wires, buses, junctions, components, terminals) into a
//!   project and returns an [`ImportReport`] of everything it skipped or guessed.
//!
//! # The `.psproj` format
//! ```text
//! {
//!   "format": "protasena-project",
//!   "version": 2,
//!   "project": { "name": …, "design": {…}, "sheets": [ { "id": 1, "name": …, "size": …,
//!                "components": […], "wires": […], …, "subcircuits": […] } ],
//!                "template": {…}, "next_id": … }
//! }
//! ```
//! - The `project` object is the design model's serde form (`protasena-doc`, feature `serde`):
//!   coordinates in thou as `[x, y]`, ids as plain numbers, orientations as
//!   `{"angle": 90, "mirror": true}`.
//! - **Deterministic**: fields in a fixed order, sheets and items in the model's list order
//!   (which is part of the design: insertion positions and undo depend on it, and new items
//!   are appended, so it is id order unless an edit inserted elsewhere), pretty-printed with a
//!   two-space indent, arrays of plain values (points `[x, y]`, property pairs) on one line,
//!   and a final newline. Saving the same project twice gives the same bytes.
//! - **Exact numbers**: floats are written in their shortest form that reads back bit for bit.
//!   NaN and ∞ cannot be written ([`SaveError::NonFinite`]).
//! - **Versions**: unknown fields are ignored (not preserved when saved again); a file of a
//!   newer [`VERSION`] is refused with [`LoadError::UnsupportedVersion`]; older versions are
//!   read and saved as the current one. Version 2 added the template settings and
//!   subcircuits; a version 1 file reads with Protasena's default template. A template whose
//!   values are out of range is replaced by the defaults with a warning.
//! - **Saving** is atomic and keeps the previous file as `<name>.psproj.bak`
//!   ([`backup_path`]).
//! - **Loading** treats the file as hostile: at most [`limits::MAX_FILE_BYTES`], nesting at
//!   most [`limits::MAX_DEPTH`], bounded sheet and item counts; duplicate ids are errors
//!   ([`LoadError::Invalid`]), other [`Project::issues`](protasena_doc::Project::issues) are
//!   returned as [`LoadReport::warnings`].
//!
//! # Example
//! ```
//! use protasena_doc::Project;
//!
//! let project = Project::new("demo");
//! let text = protasena_io::encode(&project).unwrap();
//! let (back, report) = protasena_io::decode(text.as_bytes()).unwrap();
//! assert_eq!(back, project);
//! assert!(report.warnings.is_empty());
//! ```
#![warn(missing_docs)]

mod atomic;
mod codec;
mod error;
mod file;
mod library_pack;
pub mod limits;
mod netlist_file;
mod pdsprj_import;
mod pdsprj_report;
mod pretty;
mod settings;
mod settings_file;

pub use atomic::{atomic_write, backup_path};
pub use codec::{EXTENSION, FORMAT, LoadReport, VERSION, decode, encode};
pub use error::{ExportError, LoadError, SaveError};
pub use file::{load, save};
pub use library_pack::{LIBRARY_DIR_VAR, library_dir, library_dir_from, load_library, load_library_from, merge_libraries};
pub use netlist_file::{NetlistFormat, export_netlist};
pub use pdsprj_import::{PDSPRJ_LIBRARY, convert as convert_pdsprj, import_pdsprj, import_pdsprj_bytes};
pub use pdsprj_report::{EmbeddedDevice, ImportError, ImportReport, UnknownRecords};
pub use settings::{
    DisplayOptions, MAX_ENTRIES, MAX_SETTINGS_BYTES, MAX_STRING, NamedSheetSize, PropertyDefinition, SETTINGS_FORMAT, SETTINGS_VERSION, SHEET_SIDE,
    SystemSettings, TextEditorOptions, decode_settings, default_sheet_sizes, encode_settings,
};
pub use settings_file::{CONFIG_DIR_VAR, SETTINGS_FILE, config_dir, config_dir_from, load_settings, save_settings, settings_path};
