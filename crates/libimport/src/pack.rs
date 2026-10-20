//! Writes the pack in the format [`protasena_library::Library::load_dir`] reads.
//!
//! ```text
//! out/
//!   index.json                    symbol libraries first, then footprint libraries, by name
//!   symbols/<library>.json        devices of one KiCad symbol library
//!   footprints/<library>.json     packages of one KiCad footprint library
//!   footprints/no-devices.json    the (empty) device file the footprint libraries point to
//! ```
//! Device and package files hold one item per line (compact JSON inside a JSON array), so
//! they stay small and a regenerated pack diffs line by line. Output is deterministic: the
//! caller sorts the items, and nothing here depends on time or hash order.

use std::collections::HashSet;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use protasena_library::{DEVICES_FORMAT, Device, INDEX_FILE, INDEX_FORMAT, IndexEntry, PACKAGES_FORMAT, PackIndex, Package, VERSION};
use serde::Serialize;

/// Sub-directory of device files.
pub const SYMBOLS_DIR: &str = "symbols";
/// Sub-directory of package files.
pub const FOOTPRINTS_DIR: &str = "footprints";
/// The shared empty device file of footprint libraries.
pub const NO_DEVICES: &str = "footprints/no-devices.json";

/// One library to write.
pub enum Lib<'a> {
    /// A symbol library: devices only.
    Devices(&'a str, Vec<&'a Device>),
    /// A footprint library: packages only.
    Packages(&'a str, Vec<&'a Package>),
}

/// Prepares `dir`: creates it, or clears a previous pack in it. Refuses a non-empty
/// directory that does not hold a pack, so a wrong `--out` cannot delete unrelated files.
///
/// # Errors
/// A message naming the directory.
pub fn prepare(dir: &Path) -> Result<(), String> {
    let io = |what: &str, e: std::io::Error| format!("{}: {what}: {e}", dir.display());
    if dir.exists() {
        let mut entries = fs::read_dir(dir).map_err(|e| io("cannot read", e))?;
        if dir.join(INDEX_FILE).is_file() {
            for sub in [SYMBOLS_DIR, FOOTPRINTS_DIR] {
                let p = dir.join(sub);
                if p.is_dir() {
                    fs::remove_dir_all(&p).map_err(|e| io("cannot clear old pack", e))?;
                }
            }
            fs::remove_file(dir.join(INDEX_FILE)).map_err(|e| io("cannot clear old pack", e))?;
        } else if entries.next().is_some() {
            return Err(format!("{}: not empty and not a pack (no {INDEX_FILE}); choose an empty or new directory", dir.display()));
        }
    }
    for sub in [SYMBOLS_DIR, FOOTPRINTS_DIR] {
        fs::create_dir_all(dir.join(sub)).map_err(|e| io("cannot create", e))?;
    }
    Ok(())
}

/// Writes every library and the index into a [`prepare`]d directory.
///
/// # Errors
/// A message naming the file that could not be written.
pub fn write(dir: &Path, libs: &[Lib<'_>]) -> Result<(), String> {
    let mut used: HashSet<String> = HashSet::from(["no-devices".to_owned()]);
    let mut entries = Vec::new();
    let mut need_empty = false;
    for lib in libs {
        match lib {
            Lib::Devices(name, devices) => {
                let file = format!("{SYMBOLS_DIR}/{}.json", stem(name, &mut used));
                write_items(&dir.join(&file), DEVICES_FORMAT, "devices", devices)?;
                entries.push(IndexEntry { name: (*name).to_owned(), file, device_count: devices.len() as u64, package_file: None });
            }
            Lib::Packages(name, packages) => {
                let file = format!("{FOOTPRINTS_DIR}/{}.json", stem(name, &mut used));
                write_items(&dir.join(&file), PACKAGES_FORMAT, "packages", packages)?;
                need_empty = true;
                entries.push(IndexEntry { name: (*name).to_owned(), file: NO_DEVICES.to_owned(), device_count: 0, package_file: Some(file) });
            }
        }
    }
    if need_empty {
        write_items::<Device>(&dir.join(NO_DEVICES), DEVICES_FORMAT, "devices", &[])?;
    }
    let index = PackIndex { format: INDEX_FORMAT.into(), version: VERSION, libraries: entries };
    let path = dir.join(INDEX_FILE);
    let mut text = serde_json::to_string_pretty(&index).map_err(|e| format!("{}: {e}", path.display()))?;
    text.push('\n');
    fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// `{"format":…,"version":1,"<key>":[` + one item per line + `]}`.
fn write_items<T: Serialize>(path: &Path, format: &str, key: &str, items: &[&T]) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
    let file = fs::File::create(path).map_err(|e| err(&e))?;
    let mut w = BufWriter::new(file);
    let fmt = serde_json::to_string(format).map_err(|e| err(&e))?;
    write!(w, "{{\"format\":{fmt},\"version\":{VERSION},\"{key}\":[").map_err(|e| err(&e))?;
    for (i, item) in items.iter().enumerate() {
        w.write_all(if i == 0 { b"\n" } else { b",\n" }).map_err(|e| err(&e))?;
        serde_json::to_writer(&mut w, item).map_err(|e| err(&e))?;
    }
    w.write_all(b"\n]}\n").map_err(|e| err(&e))?;
    w.flush().map_err(|e| err(&e))
}

/// A file stem for a library name: letters, digits, `-`, `_`, `.` (not leading), unique
/// without case.
fn stem(name: &str, used: &mut HashSet<String>) -> String {
    let mut base: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' }).collect();
    if base.is_empty() || base.starts_with('.') {
        base.insert(0, '_');
    }
    let mut s = base.clone();
    let mut n = 2;
    while !used.insert(s.to_lowercase()) {
        s = format!("{base}-{n}");
        n += 1;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems_are_safe_and_unique() {
        let mut used = HashSet::new();
        assert_eq!(stem("Connector_PinHeader_2.54mm", &mut used), "Connector_PinHeader_2.54mm");
        assert_eq!(stem("a/b", &mut used), "a_b");
        assert_eq!(stem("A_B", &mut used), "A_B-2");
        assert_eq!(stem("..", &mut used), "_..");
        assert_eq!(stem("", &mut used), "_");
    }
}
