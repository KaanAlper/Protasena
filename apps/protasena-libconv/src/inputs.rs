//! Reading the KiCad library directories: listing (sorted, so the run is deterministic),
//! size-limited reads, and the per-file conversion stages run in parallel.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use protasena_kicad::{Symbol as KSymbol, SymbolLib, parse_footprint, parse_symbol_lib};
use protasena_library::Package;

use crate::device::{self, Draft};
use crate::link::FootprintIndex;
use crate::map::Map;
use crate::report::{Failure, Notes};
use crate::{footprint, par};

/// Largest KiCad file read (the reader's own input limit).
pub const MAX_FILE_BYTES: u64 = 64 << 20;

/// A footprint file to convert.
#[derive(Debug, Clone)]
pub struct FpFile {
    /// Footprint library (directory name without `.pretty`).
    pub lib: String,
    /// Footprint name (file name without `.kicad_mod`).
    pub name: String,
    /// The file.
    pub path: PathBuf,
}

/// A converted footprint.
pub struct FpResult {
    /// The input.
    pub file: FpFile,
    /// The package (named after the footprint; renamed later if not unique).
    pub package: Package,
    /// Approximations.
    pub notes: Notes,
    /// Unknown KiCad tokens.
    pub unknown: BTreeMap<String, usize>,
}

/// A converted symbol library.
pub struct SymResult {
    /// Library name (file name without `.kicad_sym`).
    pub lib: String,
    /// Symbols read.
    pub symbols: usize,
    /// Of which derived (`extends`).
    pub derived: usize,
    /// Devices.
    pub drafts: Vec<Draft>,
    /// Approximations.
    pub notes: Notes,
    /// Unknown KiCad tokens.
    pub unknown: BTreeMap<String, usize>,
    /// Symbols that could not be resolved.
    pub failures: Vec<Failure>,
}

/// Entries of `dir` with extension `ext`, sorted by name, as (stem, path).
pub fn list(dir: &Path, ext: &str, want_dir: bool) -> Result<Vec<(String, PathBuf)>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut out = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.is_dir() != want_dir || path.extension().and_then(|e| e.to_str()) != Some(ext) {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            out.push((stem.to_owned(), path.clone()));
        }
    }
    out.sort();
    Ok(out)
}

/// Every footprint file of every `.pretty` directory.
pub fn footprint_files(dir: &Path) -> Result<(usize, Vec<FpFile>), String> {
    let libs = list(dir, "pretty", true)?;
    let mut files = Vec::new();
    for (lib, path) in &libs {
        for (name, path) in list(path, "kicad_mod", false)? {
            files.push(FpFile { lib: lib.clone(), name, path });
        }
    }
    Ok((libs.len(), files))
}

/// Reads at most [`MAX_FILE_BYTES`].
pub fn read(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(format!("larger than {MAX_FILE_BYTES} bytes"));
    }
    Ok(bytes)
}

/// Converts every footprint; a file that cannot be read or parsed is a failure.
pub fn convert_footprints(files: Vec<FpFile>, jobs: usize) -> Result<Vec<Result<FpResult, Failure>>, String> {
    par::map(&files, jobs, |f| {
        let fail = |error: String| Failure { what: format!("{}.pretty/{}.kicad_mod", f.lib, f.name), error };
        let fp = parse_footprint(&read(&f.path).map_err(fail)?).map_err(|e| fail(e.to_string()))?;
        let mut notes = Notes::default();
        if fp.name != f.name {
            notes.note("footprint name differs from its file name (file name used)");
        }
        let package = footprint::convert(&f.name, &fp, &mut notes);
        Ok(FpResult { file: f.clone(), package, notes, unknown: footprint::unknown_tokens(&fp) })
    })
}

/// Converts every symbol library in `dir`.
pub fn convert_symbols(dir: &Path, map: &Map, fps: &FootprintIndex, jobs: usize) -> Result<Vec<Result<SymResult, Failure>>, String> {
    let files = list(dir, "kicad_sym", false)?;
    par::map(&files, jobs, |(lib, path)| {
        let fail = |error: String| Failure { what: format!("{lib}.kicad_sym"), error };
        let parsed = parse_symbol_lib(&read(path).map_err(fail)?).map_err(|e| fail(e.to_string()))?;
        Ok(convert_lib(lib, &parsed, map, fps))
    })
}

fn convert_lib(lib: &str, parsed: &SymbolLib, map: &Map, fps: &FootprintIndex) -> SymResult {
    let mut r = SymResult {
        lib: lib.to_owned(),
        symbols: parsed.symbols.len(),
        derived: parsed.symbols.iter().filter(|s| s.extends.is_some()).count(),
        drafts: Vec::new(),
        notes: Notes::default(),
        unknown: parsed.report().unknown_tokens,
        failures: Vec::new(),
    };
    let resolved: Vec<Result<KSymbol, (String, protasena_kicad::Error)>> = match parsed.resolve_all() {
        Ok(all) => all.into_iter().map(Ok).collect(),
        // One broken `extends` must not lose the whole library: resolve one by one.
        Err(_) => parsed.symbols.iter().map(|s| parsed.resolve(&s.name).map_err(|e| (s.name.clone(), e))).collect(),
    };
    for item in resolved {
        match item {
            Ok(s) => r.drafts.push(device::build(lib, &s, map, fps, &mut r.notes)),
            Err((name, e)) => r.failures.push(Failure { what: format!("{lib}/{name}"), error: e.to_string() }),
        }
    }
    r
}
