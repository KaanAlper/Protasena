//! The user's own KiCad libraries, imported at run time (Library Manager and System Settings
//! keep the list of paths).
//!
//! Each configured path is a `.kicad_sym` file, a `.pretty` footprint folder, or a folder
//! holding either (not searched further down). It is converted with the same code as the pack
//! ([`crate::convert::convert_files`], with an empty map: categories come from the library
//! names), and the result is cached under the user's cache directory ([`cache`]), keyed by the
//! modification time and size of every file it was made from, so a path is converted again
//! only after one of its files changed.
//!
//! Symbols link only to the footprints imported with them (the same path); devices keep their
//! KiCad library's name, and [`fit`] renames what would clash with the libraries already loaded.
//! Every problem is reported in the path's [`SourceReport`]; nothing here panics.

mod cache;
mod fit;
mod scan;

use std::path::{Path, PathBuf};

use protasena_library::{Device, Library, Package};
use serde::Serialize;

use crate::convert::convert_files;
use crate::map::Map;
use crate::report::Report;

pub use cache::{CACHE_FORMAT, CACHE_VERSION};
pub use fit::{LIBRARY_SUFFIX, fit};
pub use scan::{Inputs, scan};

/// Most paths imported.
pub const MAX_SOURCES: usize = 256;
/// Most problems kept per path (a count of the rest follows).
pub const MAX_ERRORS: usize = 50;

/// What one configured path gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceReport {
    /// The path as configured.
    pub path: String,
    /// Libraries it gave, in order (symbol libraries, then `NAME.pretty` footprint libraries).
    pub libraries: Vec<String>,
    /// Devices it gave.
    pub devices: usize,
    /// Packages it gave.
    pub packages: usize,
    /// Read from the cache (no file changed since the last conversion).
    pub cached: bool,
    /// Problems: files or symbols that could not be converted, a missing path…
    pub errors: Vec<String>,
}

/// What one path gave: its report and its devices and packages.
#[derive(Debug, Clone)]
pub struct Source {
    /// The report.
    pub report: SourceReport,
    /// Its devices.
    pub devices: Vec<Device>,
    /// Its packages (the devices' packages are among them).
    pub packages: Vec<Package>,
}

/// Imports every path (at most [`MAX_SOURCES`]), using and refreshing the cache in `cache`
/// when given, on up to `jobs` threads. One [`Source`] per path, in the order given.
pub fn import(paths: &[PathBuf], cache: Option<&Path>, jobs: usize) -> Vec<Source> {
    let mut out: Vec<Source> = paths
        .iter()
        .take(MAX_SOURCES)
        .map(|path| {
            let (lib, mut report) = import_one(path, cache, jobs);
            report.devices = lib.len();
            report.packages = lib.package_count();
            report.libraries = lib.libraries().iter().map(|l| l.name.clone()).collect();
            Source { report, devices: lib.devices().to_vec(), packages: lib.packages().cloned().collect() }
        })
        .collect();
    if let Some(last) = out.last_mut()
        && paths.len() > MAX_SOURCES
    {
        last.report.errors.push(format!("only the first {MAX_SOURCES} library paths are imported"));
    }
    out
}

fn import_one(path: &Path, cache: Option<&Path>, jobs: usize) -> (Library, SourceReport) {
    let mut report =
        SourceReport { path: path.display().to_string(), libraries: Vec::new(), devices: 0, packages: 0, cached: false, errors: Vec::new() };
    let inputs = match scan(path) {
        Ok(i) => i,
        Err(e) => {
            report.errors.push(e);
            return (Library::default(), report);
        }
    };
    let stamp = cache::stamp(&inputs);
    if let Some(dir) = cache
        && let Some((lib, errors)) = cache::load(dir, path, &stamp)
    {
        report.cached = true;
        report.errors = errors;
        return (lib, report);
    }
    let mut conversion = Report::default();
    let lib = match convert_files(&inputs.symbols, inputs.footprints, &Map::default(), jobs, &mut conversion) {
        Ok(lib) => lib,
        Err(e) => {
            report.errors.push(e);
            return (Library::default(), report);
        }
    };
    report.errors = capped(conversion.failures.iter().map(|f| format!("{}: {}", f.what, f.error)).collect());
    if let Some(dir) = cache
        && let Err(e) = cache::store(dir, path, &stamp, &lib, &report.errors)
    {
        report.errors.push(format!("the converted library could not be cached: {e}"));
    }
    (lib, report)
}

/// At most [`MAX_ERRORS`] lines, then a count of the rest.
fn capped(mut errors: Vec<String>) -> Vec<String> {
    if errors.len() > MAX_ERRORS {
        let more = errors.len() - MAX_ERRORS;
        errors.truncate(MAX_ERRORS);
        errors.push(format!("… and {more} more problems"));
    }
    errors
}
