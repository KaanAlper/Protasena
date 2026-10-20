//! Where the session's devices come from: the built-in parts with the parts pack over them
//! (the base), and the user's own KiCad libraries imported at run time (System Settings and
//! the Library Manager keep their paths, `kicad_libraries` in the system settings).
//!
//! The session's library is the base with the imported libraries added
//! ([`Library::with_extra`]): imported devices keep their KiCad library's name, may repeat
//! names of other libraries, and never replace a device of the pack. Problems — a missing
//! path, a file that does not convert, a name that had to change — are kept for the Library
//! Manager ([`Session::library_problems`]) and logged.

use std::path::{Path, PathBuf};

use protasena_libimport::user::{self, Source, SourceReport};
use protasena_library::Library;
use serde_json::{Value, json};

use crate::session::Session;

/// What a background load gives.
#[derive(Debug, Clone, Default)]
pub struct LoadedLibraries {
    /// The pack merged over the built-in parts, when a pack folder was given.
    pub base: Option<Result<Library, String>>,
    /// The user's KiCad libraries, path by path.
    pub imported: Vec<Source>,
}

/// Loads the pack in `pack` (over the built-in parts) and imports the KiCad libraries at
/// `kicad`, cached in `cache`. Meant for a background thread; never panics.
pub fn load_libraries(pack: Option<&Path>, kicad: &[PathBuf], cache: Option<&Path>) -> LoadedLibraries {
    let base = pack.map(|dir| protasena_io::load_library_from(Some(dir)).map_err(|e| e.to_string()));
    LoadedLibraries { base, imported: user::import(kicad, cache, jobs()) }
}

fn jobs() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

impl Session {
    /// What each configured KiCad library path gave, in order.
    pub fn kicad_sources(&self) -> impl Iterator<Item = &SourceReport> {
        self.imported.iter().map(|s| &s.report)
    }

    /// Problems of the libraries added to the pack: names that had to change, devices or
    /// packages left out.
    pub fn library_problems(&self) -> &[String] {
        &self.library_problems
    }

    /// The folder converted KiCad libraries are cached in (none: no cache).
    pub fn cache_dir(&self) -> Option<&Path> {
        self.cache_dir.as_deref()
    }

    /// Sets the cache folder for KiCad libraries imported from now on.
    pub fn set_cache_dir(&mut self, dir: Option<PathBuf>) {
        self.cache_dir = dir;
    }

    /// Takes what a background load gave. Returns the pack's error, if it could not be loaded
    /// (the built-in parts stay in use).
    pub fn take_libraries(&mut self, loaded: LoadedLibraries) -> Option<String> {
        let mut error = None;
        match loaded.base {
            Some(Ok(lib)) => self.library = lib,
            Some(Err(e)) => error = Some(e),
            None => {}
        }
        self.imported = loaded.imported;
        self.remerge();
        error
    }

    /// Imports the KiCad libraries of the settings again (from the cache where nothing
    /// changed) and returns their reports.
    pub(crate) fn import_kicad(&mut self) -> Value {
        let paths: Vec<PathBuf> = self.settings.kicad_libraries.iter().map(PathBuf::from).collect();
        self.imported = user::import(&paths, self.cache_dir.as_deref(), jobs());
        self.remerge();
        for r in self.kicad_sources().cloned().collect::<Vec<_>>() {
            for e in &r.errors {
                self.note(format!("{}: {e}", r.path));
            }
        }
        json!({ "kicad": self.kicad_sources().collect::<Vec<_>>(), "problems": self.library_problems })
    }

    /// Rebuilds the library: the base with the imported libraries added.
    pub(crate) fn remerge(&mut self) {
        let base = std::mem::take(&mut self.library).without_extra();
        let (devices, packages, mut problems) = protasena_libimport::user::fit(&self.imported, &base);
        let (lib, issues) = base.with_extra(devices, packages);
        problems.extend(issues.iter().map(ToString::to_string));
        self.library = lib;
        self.library_problems = problems;
        self.hover = None;
        self.machine = crate::interact::Machine::Idle;
        self.touch();
    }
}
