//! The on-disk pack: a directory with `index.json` and one JSON file per library (devices)
//! plus an optional package file per library.
//!
//! ```text
//! pack/
//!   index.json            {"format":"protasena-library","version":1,"libraries":[
//!                           {"name":"GENERIC","file":"GENERIC.json","device_count":11,
//!                            "package_file":"GENERIC.packages.json"}]}
//!   GENERIC.json          {"format":"protasena-library-devices","version":1,"devices":[…]}
//!   GENERIC.packages.json {"format":"protasena-library-packages","version":1,"packages":[…]}
//! ```
//! Unknown fields are ignored (newer writers may add some); the format tag and version are
//! checked. Files referenced by the index must be relative paths inside the pack directory.

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::device::Device;
use crate::error::{Issue, IssueKind, LibraryError};
use crate::intern::Interner;
use crate::library::Library;
use crate::limits::{MAX_FILE_BYTES, MAX_PACK_BYTES};
use crate::package::Package;
use crate::store::Packages;
use crate::validate::{Validator, name_problem};

/// Name of the index file.
pub const INDEX_FILE: &str = "index.json";
/// Format tag of `index.json`.
pub const INDEX_FORMAT: &str = "protasena-library";
/// Format tag of a device file.
pub const DEVICES_FORMAT: &str = "protasena-library-devices";
/// Format tag of a package file.
pub const PACKAGES_FORMAT: &str = "protasena-library-packages";
/// The pack version this crate reads and writes.
pub const VERSION: u32 = 1;

/// Contents of `index.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackIndex {
    /// Always [`INDEX_FORMAT`].
    pub format: String,
    /// Always [`VERSION`].
    pub version: u32,
    /// The libraries, in display order.
    #[serde(default, deserialize_with = "crate::bounded::index_entries")]
    pub libraries: Vec<IndexEntry>,
}

/// One library of the pack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Library name (the Library column of Pick Devices).
    pub name: String,
    /// Device file, relative to the pack directory.
    pub file: String,
    /// Number of devices in the file.
    #[serde(default)]
    pub device_count: u64,
    /// Package file, relative to the pack directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_file: Option<String>,
}

#[derive(Serialize)]
struct DeviceFileOut<'a> {
    format: &'a str,
    version: u32,
    devices: Vec<&'a Device>,
}

#[derive(Serialize)]
struct PackageFileOut<'a> {
    format: &'a str,
    version: u32,
    packages: Vec<&'a Package>,
}

/// A device file's list (its header is checked first, see [`Head`]).
#[derive(Deserialize)]
struct DeviceFile {
    #[serde(default, deserialize_with = "crate::bounded::devices")]
    devices: Vec<Device>,
}

/// A package file's list.
#[derive(Deserialize)]
struct PackageFile {
    #[serde(default, deserialize_with = "crate::bounded::packages")]
    packages: Vec<Package>,
}

/// Just the header, read first so that another version is reported as such rather than as a
/// shape error.
#[derive(Deserialize)]
struct Head {
    format: Option<serde_json::Value>,
    version: Option<serde_json::Value>,
}

/// Parses `index.json` text.
///
/// # Errors
/// [`LibraryError::Json`] or [`LibraryError::Format`] (`path` is used in the message only).
pub fn parse_index(path: &Path, bytes: &[u8]) -> Result<PackIndex, LibraryError> {
    parse(path, bytes, INDEX_FORMAT)
}

/// Parses a device file.
///
/// # Errors
/// [`LibraryError::Json`] or [`LibraryError::Format`].
pub fn parse_devices(path: &Path, bytes: &[u8]) -> Result<Vec<Device>, LibraryError> {
    parse::<DeviceFile>(path, bytes, DEVICES_FORMAT).map(|f| f.devices)
}

/// Parses a package file.
///
/// # Errors
/// [`LibraryError::Json`] or [`LibraryError::Format`].
pub fn parse_packages(path: &Path, bytes: &[u8]) -> Result<Vec<Package>, LibraryError> {
    parse::<PackageFile>(path, bytes, PACKAGES_FORMAT).map(|f| f.packages)
}

impl Library {
    /// Loads and validates the pack in `dir`.
    ///
    /// # Errors
    /// I/O, size, JSON and format errors stop at the first bad file; consistency problems are
    /// all collected into [`LibraryError::Invalid`].
    ///
    /// Package files are checked here and read again on first use (see [`Library::package`]);
    /// equal symbols and pin maps are shared between devices.
    pub fn load_dir(dir: &Path) -> Result<Library, LibraryError> {
        let mut budget = 0u64;
        let index_path = dir.join(INDEX_FILE);
        let index = parse_index(&index_path, &read_limited(&index_path, dir, &mut budget)?)?;
        let mut issues = Vec::new();
        let files = library_files(dir, &index, &mut issues);
        let mut v = Validator::default();
        let mut packages = Packages::default();
        for (entry, _, file) in &files {
            let Some(path) = file else { continue };
            let pkgs: Vec<Package> =
                parse_packages(path, &read_limited(path, dir, &mut budget)?)?.into_iter().map(|p| adopt(p, &entry.name, &mut issues)).collect();
            v.packages(&pkgs);
            packages.add_file(&entry.name, path.clone(), pkgs.into_iter().map(|p| p.name).collect());
        }
        let mut interner = Interner::default();
        let mut devices = Vec::new();
        for (entry, path, _) in &files {
            let devs = parse_devices(path, &read_limited(path, dir, &mut budget)?)?;
            if usize::try_from(entry.device_count).ok() != Some(devs.len()) {
                issues.push(Issue {
                    library: entry.name.clone(),
                    item: None,
                    kind: IssueKind::CountMismatch { index: entry.device_count, file: devs.len() },
                });
            }
            let devs: Vec<Device> = devs.into_iter().map(|d| interner.device(adopt(d, &entry.name, &mut issues))).collect();
            v.devices(&devs);
            devices.extend(devs);
        }
        issues.extend(v.finish());
        if issues.is_empty() { Ok(Library::assemble(devices, packages)) } else { Err(LibraryError::Invalid(issues)) }
    }

    /// Writes the library as a pack into `dir` (created if missing; existing files with the
    /// same names are replaced).
    ///
    /// # Errors
    /// [`LibraryError::Io`] when a file cannot be written.
    pub fn save_dir(&self, dir: &Path) -> Result<(), LibraryError> {
        fs::create_dir_all(dir).map_err(|e| io_error(dir, &e))?;
        // `index` is taken by the index file itself.
        let mut used = HashSet::from(["index".to_owned()]);
        let mut entries = Vec::new();
        for info in self.libraries() {
            let stem = unique_stem(&info.name, &mut used);
            let devices: Vec<&Device> = self.devices().iter().filter(|d| d.library == info.name).collect();
            let file = format!("{stem}.json");
            write_json(&dir.join(&file), &DeviceFileOut { format: DEVICES_FORMAT, version: VERSION, devices })?;
            let package_file = if info.package_count > 0 {
                let packages: Vec<&Package> = self.packages().filter(|p| p.library == info.name).collect();
                let file = format!("{stem}.packages.json");
                write_json(&dir.join(&file), &PackageFileOut { format: PACKAGES_FORMAT, version: VERSION, packages })?;
                Some(file)
            } else {
                None
            };
            entries.push(IndexEntry { name: info.name.clone(), file, device_count: info.device_count as u64, package_file });
        }
        let index = PackIndex { format: INDEX_FORMAT.into(), version: VERSION, libraries: entries };
        write_json(&dir.join(INDEX_FILE), &index)
    }
}

/// The index entries that can be read, with their device file and package file paths; bad
/// names, repeated libraries and bad paths become issues.
fn library_files<'a>(dir: &Path, index: &'a PackIndex, issues: &mut Vec<Issue>) -> Vec<(&'a IndexEntry, PathBuf, Option<PathBuf>)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for entry in &index.libraries {
        let issue = |kind| Issue { library: entry.name.clone(), item: None, kind };
        if let Some(kind) = name_problem(&entry.name, "library") {
            issues.push(issue(kind));
            continue;
        }
        if !seen.insert(entry.name.to_uppercase()) {
            issues.push(issue(IssueKind::DuplicateLibrary));
            continue;
        }
        let Some(path) = pack_path(dir, &entry.file) else {
            issues.push(issue(IssueKind::BadPath(entry.file.clone())));
            continue;
        };
        let package_file = entry.package_file.as_ref().and_then(|file| {
            let p = pack_path(dir, file);
            if p.is_none() {
                issues.push(issue(IssueKind::BadPath(file.clone())));
            }
            p
        });
        out.push((entry, path, package_file));
    }
    out
}

/// Reads one package file again (for [`crate::store::Packages`]): packages that no longer pass
/// the checks are left out.
///
/// # Errors
/// The reason when the file cannot be read or parsed, or a problem concerns the whole file.
pub(crate) fn load_package_file(path: &Path, library: &str) -> Result<Vec<Package>, String> {
    let mut budget = 0;
    let bytes = read_limited(path, path, &mut budget).map_err(|e| e.to_string())?;
    let mut issues = Vec::new();
    let pkgs: Vec<Package> = parse_packages(path, &bytes).map_err(|e| e.to_string())?.into_iter().map(|p| adopt(p, library, &mut issues)).collect();
    let mut v = Validator::default();
    v.packages(&pkgs);
    issues.extend(v.finish());
    if let Some(whole) = issues.iter().find(|i| i.item.is_none()) {
        return Err(whole.to_string());
    }
    let bad: HashSet<&str> = issues.iter().filter_map(|i| i.item.as_deref()).collect();
    Ok(pkgs.into_iter().filter(|p| !bad.contains(p.name.as_str())).collect())
}

/// Things that record the library they belong to.
trait InLibrary {
    fn library_mut(&mut self) -> (&mut String, &str);
}

impl InLibrary for Device {
    fn library_mut(&mut self) -> (&mut String, &str) {
        (&mut self.library, &self.name)
    }
}

impl InLibrary for Package {
    fn library_mut(&mut self) -> (&mut String, &str) {
        (&mut self.library, &self.name)
    }
}

/// Puts an entry into `library`, reporting an entry that claims another one.
fn adopt<T: InLibrary>(mut item: T, library: &str, issues: &mut Vec<Issue>) -> T {
    let (lib, name) = item.library_mut();
    if lib.is_empty() {
        library.clone_into(lib);
    } else if lib != library {
        issues.push(Issue { library: library.to_owned(), item: Some(name.to_owned()), kind: IssueKind::LibraryMismatch(lib.clone()) });
    }
    item
}

/// `dir/file` when `file` is a plain relative path (no root, drive, `..` or backslash).
fn pack_path(dir: &Path, file: &str) -> Option<PathBuf> {
    let rel = Path::new(file);
    let plain = !file.is_empty() && !file.contains(['\\', ':']) && rel.components().all(|c| matches!(c, Component::Normal(_)));
    plain.then(|| dir.join(rel))
}

fn read_limited(path: &Path, dir: &Path, budget: &mut u64) -> Result<Vec<u8>, LibraryError> {
    let file = fs::File::open(path).map_err(|e| io_error(path, &e))?;
    let too_large = |size| LibraryError::TooLarge { path: path.to_owned(), size, limit: MAX_FILE_BYTES };
    if let Ok(meta) = file.metadata()
        && meta.len() > MAX_FILE_BYTES
    {
        return Err(too_large(meta.len()));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes).map_err(|e| io_error(path, &e))?;
    let size = bytes.len() as u64;
    if size > MAX_FILE_BYTES {
        return Err(too_large(size));
    }
    *budget = budget.saturating_add(size);
    if *budget > MAX_PACK_BYTES {
        return Err(LibraryError::TooLarge { path: dir.to_owned(), size: *budget, limit: MAX_PACK_BYTES });
    }
    Ok(bytes)
}

fn parse<T: DeserializeOwned>(path: &Path, bytes: &[u8], format: &str) -> Result<T, LibraryError> {
    let json_error = |e: serde_json::Error| LibraryError::Json { path: path.to_owned(), message: e.to_string() };
    let head: Head = serde_json::from_slice(bytes).map_err(json_error)?;
    let tag = head.format.as_ref().and_then(serde_json::Value::as_str);
    if tag != Some(format) {
        return Err(format_error(path, format!("not a {format} file (format tag {})", head.format.unwrap_or_default())));
    }
    match head.version.as_ref().and_then(serde_json::Value::as_u64) {
        Some(v) if v == u64::from(VERSION) => {}
        _ => {
            let found = head.version.map_or_else(|| "missing".to_owned(), |v| v.to_string());
            return Err(format_error(path, format!("version {found} is not supported (this build reads version {VERSION})")));
        }
    }
    serde_json::from_slice(bytes).map_err(json_error)
}

fn format_error(path: &Path, message: String) -> LibraryError {
    LibraryError::Format { path: path.to_owned(), message }
}

fn io_error(path: &Path, e: &std::io::Error) -> LibraryError {
    LibraryError::Io { path: path.to_owned(), message: e.to_string() }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), LibraryError> {
    let text = serde_json::to_vec_pretty(value).map_err(|e| LibraryError::Json { path: path.to_owned(), message: e.to_string() })?;
    fs::write(path, text).map_err(|e| io_error(path, &e))
}

/// A file stem for a library name: safe characters only, unique within the pack.
fn unique_stem(name: &str, used: &mut HashSet<String>) -> String {
    let base: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let mut stem = base.clone();
    let mut n = 2;
    while !used.insert(stem.to_lowercase()) {
        stem = format!("{base}-{n}");
        n += 1;
    }
    stem
}
