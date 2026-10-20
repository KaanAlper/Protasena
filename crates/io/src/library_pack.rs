//! Where the device library comes from: the built-in parts plus an optional pack.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use protasena_library::{Library, LibraryError};

/// Environment variable naming the library pack directory.
pub const LIBRARY_DIR_VAR: &str = "PROTASENA_LIBRARY_DIR";

/// The library pack directory: [`LIBRARY_DIR_VAR`] when it is set and not empty, else `None`
/// (only the built-in parts are available).
pub fn library_dir() -> Option<PathBuf> {
    library_dir_from(std::env::var_os(LIBRARY_DIR_VAR))
}

/// [`library_dir`] for a given value of the variable.
pub fn library_dir_from(value: Option<OsString>) -> Option<PathBuf> {
    value.filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// The library to work with: the built-in parts merged with the pack in [`library_dir`].
///
/// # Errors
/// The pack's [`LibraryError`] when it cannot be loaded (the caller may fall back to
/// [`Library::builtin`]).
pub fn load_library() -> Result<Library, LibraryError> {
    load_library_from(library_dir().as_deref())
}

/// [`load_library`] with the pack directory given (`None`: built-in parts only).
///
/// # Errors
/// As [`load_library`].
pub fn load_library_from(dir: Option<&Path>) -> Result<Library, LibraryError> {
    let builtin = Library::builtin();
    match dir {
        None => Ok(builtin),
        Some(dir) => Ok(builtin.overlay(Library::load_dir(dir)?)),
    }
}

/// The built-in parts and a pack in one library. The pack wins name clashes: a built-in
/// device is left out when the pack has a device of the same name (without case), and a
/// built-in package when the pack has one of the same name, together with the built-in devices
/// that use it (their pin maps were made for the built-in package). The built-in library comes
/// first in the library list; a pack library that is also called `GENERIC` simply adds to it.
/// Same as [`Library::overlay`] on copies.
///
/// # Errors
/// [`LibraryError::Invalid`] if the merged set is inconsistent (it is not when both inputs are
/// valid libraries).
pub fn merge_libraries(builtin: &Library, pack: &Library) -> Result<Library, LibraryError> {
    let merged = builtin.clone().overlay(pack.clone());
    let issues = merged.issues();
    if issues.is_empty() { Ok(merged) } else { Err(LibraryError::Invalid(issues)) }
}
