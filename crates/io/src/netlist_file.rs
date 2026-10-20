//! Netlist export files.

use std::path::Path;

use protasena_doc::Project;
use protasena_netlist::{DeviceSource, Netlist, export};

use crate::atomic::atomic_write;
use crate::error::ExportError;

/// The netlist file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetlistFormat {
    /// Our JSON netlist ([`export::to_json`]); reads back with [`export::from_json`].
    Json,
    /// A SPICE-style text netlist ([`export::to_spice`]).
    Spice,
}

impl NetlistFormat {
    /// The usual file name extension (without the dot).
    pub fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Spice => "cir",
        }
    }
}

/// Builds the project's netlist and writes it to `path` atomically (no backup is kept: a
/// netlist is always generated again from the design). The design name in the file is the
/// project name. Returns the netlist, so the caller can show its ERC results.
///
/// # Errors
/// [`ExportError::Netlist`] if the netlist cannot be serialized, [`ExportError::Io`] when the
/// file cannot be written.
pub fn export_netlist<S: DeviceSource + ?Sized>(project: &Project, devices: &S, path: &Path, format: NetlistFormat) -> Result<Netlist, ExportError> {
    let netlist = protasena_netlist::build(project, devices);
    let text = match format {
        NetlistFormat::Json => export::to_json(&netlist, &project.name).map_err(ExportError::Netlist)?,
        NetlistFormat::Spice => export::to_spice(&netlist, &project.name),
    };
    atomic_write(path, false, |out| out.write_all(text.as_bytes())).map_err(|e| ExportError::Io { path: path.to_owned(), message: e.to_string() })?;
    Ok(netlist)
}
