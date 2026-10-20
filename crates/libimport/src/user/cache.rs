//! The conversion cache: one folder per imported path under `<cache>/kicad/`, holding the
//! converted library as a pack and `meta.json` with what it was made from.
//!
//! ```text
//! <cache>/kicad/<hash of the path>/
//!   meta.json   {"format":"protasena-kicad-cache","version":1,"pack_version":2,"path":…,
//!                "stamp":[{"file":…,"secs":…,"nanos":…,"size":…}],"errors":[…]}
//!   pack/       the converted library (Library::save_dir)
//! ```
//! The cache is used only when the format, both versions, the path and the stamp (every input
//! file with its modification time and size) are the same as now; anything else, including a
//! cache that cannot be read, simply converts again. `meta.json` is written last, so a folder
//! without it is never used.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use protasena_library::Library;
use serde::{Deserialize, Serialize};

use super::scan::Inputs;

/// Format tag of `meta.json`.
pub const CACHE_FORMAT: &str = "protasena-kicad-cache";
/// Version of the cache; raised whenever the conversion gives different output.
pub const CACHE_VERSION: u32 = 1;
/// Largest `meta.json` read.
const MAX_META_BYTES: u64 = 64 << 20;

/// One input file as the cache remembers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    /// The file.
    pub file: String,
    /// Modification time: seconds since 1970 (0 when unknown).
    pub secs: u64,
    /// Nanoseconds of the modification time.
    pub nanos: u32,
    /// Size in bytes (`u64::MAX` when unreadable).
    pub size: u64,
}

#[derive(Serialize, Deserialize)]
struct Meta {
    format: String,
    version: u32,
    pack_version: u32,
    path: String,
    stamp: Vec<Stamp>,
    #[serde(default)]
    errors: Vec<String>,
}

/// Every input file with its modification time and size.
pub fn stamp(inputs: &Inputs) -> Vec<Stamp> {
    inputs
        .files()
        .map(|p| {
            let meta = fs::metadata(p).ok();
            let time = meta.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(UNIX_EPOCH).ok()).unwrap_or_default();
            Stamp { file: p.display().to_string(), secs: time.as_secs(), nanos: time.subsec_nanos(), size: meta.map_or(u64::MAX, |m| m.len()) }
        })
        .collect()
}

/// The cache folder of `path` in `cache`.
fn folder(cache: &Path, path: &Path) -> PathBuf {
    cache.join("kicad").join(format!("{:016x}", fnv1a(path.display().to_string().as_bytes())))
}

/// FNV-1a: a stable hash (the folder name must not change between builds).
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

/// The cached library of `path` and its problems, when the cache matches `stamp`.
pub fn load(cache: &Path, path: &Path, stamp: &[Stamp]) -> Option<(Library, Vec<String>)> {
    let dir = folder(cache, path);
    let file = fs::File::open(dir.join("meta.json")).ok()?;
    let mut bytes = Vec::new();
    file.take(MAX_META_BYTES).read_to_end(&mut bytes).ok()?;
    let meta: Meta = serde_json::from_slice(&bytes).ok()?;
    let same = meta.format == CACHE_FORMAT
        && meta.version == CACHE_VERSION
        && meta.pack_version == protasena_library::VERSION
        && meta.path == path.display().to_string()
        && meta.stamp == stamp;
    if !same {
        return None;
    }
    let lib = Library::load_dir(&dir.join("pack")).ok()?;
    Some((lib, meta.errors))
}

/// Replaces the cache of `path` with `lib`.
///
/// # Errors
/// What could not be written.
pub fn store(cache: &Path, path: &Path, stamp: &[Stamp], lib: &Library, errors: &[String]) -> Result<(), String> {
    let dir = folder(cache, path);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    lib.save_dir(&dir.join("pack")).map_err(|e| e.to_string())?;
    let meta = Meta {
        format: CACHE_FORMAT.into(),
        version: CACHE_VERSION,
        pack_version: protasena_library::VERSION,
        path: path.display().to_string(),
        stamp: stamp.to_vec(),
        errors: errors.to_vec(),
    };
    let text = serde_json::to_vec(&meta).map_err(|e| e.to_string())?;
    let meta_path = dir.join("meta.json");
    fs::write(&meta_path, text).map_err(|e| format!("{}: {e}", meta_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_name_is_stable() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        let a = folder(Path::new("/c"), Path::new("/x/lib.kicad_sym"));
        assert_eq!(a, folder(Path::new("/c"), Path::new("/x/lib.kicad_sym")));
        assert_ne!(a, folder(Path::new("/c"), Path::new("/x/other.kicad_sym")));
    }
}
