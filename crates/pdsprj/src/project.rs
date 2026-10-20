//! A whole `.pdsprj`: the container, its manifest and its schematic.

use std::path::Path;

use crate::container::{Container, Member};
use crate::dsn::{DESIGN, Design, parse_design};
use crate::error::Error;
use crate::manifest::{MANIFEST, Manifest, parse_manifest};

/// A read project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// The archive's members.
    pub members: Vec<Member>,
    /// The manifest, if the archive has a readable one.
    pub manifest: Option<Manifest>,
    /// The decoded schematic.
    pub design: Design,
}

impl Project {
    /// Reads a `.pdsprj` (or a `.pdsbak` backup, which has the same format) from disk.
    ///
    /// # Errors
    /// Container errors, [`Error::MissingMember`] without `ROOT.DSN`, and the errors of
    /// [`parse_design`]. A missing or unreadable manifest is not an error.
    pub fn open(path: &Path) -> Result<Self, Error> {
        Self::from_container(&Container::open(path)?)
    }

    /// Reads a project held in memory.
    ///
    /// # Errors
    /// As [`Project::open`], without I/O errors.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Self::from_container(&Container::from_bytes(bytes)?)
    }

    /// Reads a project from an opened container.
    ///
    /// # Errors
    /// As [`Project::open`], without I/O errors.
    pub fn from_container(container: &Container) -> Result<Self, Error> {
        let design = parse_design(&container.read(DESIGN)?)?;
        let manifest = container.read(MANIFEST).ok().and_then(|bytes| parse_manifest(&bytes).ok());
        Ok(Self { members: container.members().to_vec(), manifest, design })
    }
}
