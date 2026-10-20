//! Atomic file replacement with an optional backup of the previous file.

use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// How many temp file names are tried before giving up (they are only taken by crashed saves).
const TEMP_ATTEMPTS: u32 = 64;

/// The backup kept next to `path` by [`atomic_write`]: the file name with `.bak` appended
/// (`design.psproj` → `design.psproj.bak`).
pub fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(".bak");
    path.with_file_name(name)
}

/// Writes a file so that it is either fully replaced or left as it was.
///
/// `write` fills a temp file in the same directory; the temp file is flushed to disk and then
/// renamed over `path` (which replaces an existing file on every platform). With
/// `keep_backup`, the previous `path` is first copied to [`backup_path`]. If anything fails —
/// `write` included — the temp file is removed and `path` and its backup are untouched.
///
/// # Errors
/// The first I/O error (or the error `write` returned). A `path` that names a directory or no
/// file at all is an error too.
pub fn atomic_write<F>(path: &Path, keep_backup: bool, write: F) -> io::Result<()>
where
    F: FnOnce(&mut dyn Write) -> io::Result<()>,
{
    let name = path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the path does not name a file"))?;
    let dir = parent_dir(path);
    if fs::metadata(path).is_ok_and(|m| m.is_dir()) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "a directory is in the way"));
    }
    let (temp, file) = create_temp(dir, name)?;
    let result = fill_and_replace(file, &temp, path, keep_backup, write);
    if result.is_err() {
        // The write already failed; that error is the one to report, so a failure to clean up
        // the temp file (which only leaves a stray `.tmp` file behind) is not reported.
        let _ = fs::remove_file(&temp);
    }
    result?;
    sync_dir(dir);
    Ok(())
}

fn fill_and_replace<F>(file: File, temp: &Path, path: &Path, keep_backup: bool, write: F) -> io::Result<()>
where
    F: FnOnce(&mut dyn Write) -> io::Result<()>,
{
    let mut out = BufWriter::new(file);
    write(&mut out)?;
    let file = out.into_inner().map_err(io::IntoInnerError::into_error)?;
    file.sync_all()?;
    drop(file);
    if keep_backup && fs::metadata(path).is_ok_and(|m| m.is_file()) {
        make_backup(path)?;
    }
    fs::rename(temp, path)
}

/// Copies `path` to its backup through a temp file, so a failed copy leaves the old backup.
fn make_backup(path: &Path) -> io::Result<()> {
    let backup = backup_path(path);
    let name = backup.file_name().map(OsString::from).unwrap_or_default();
    let (temp, file) = create_temp(parent_dir(path), &name)?;
    drop(file);
    let result = fs::copy(path, &temp).and_then(|_| fs::rename(&temp, &backup));
    if result.is_err() {
        // As in `atomic_write`: the copy error is the one that matters.
        let _ = fs::remove_file(&temp);
    }
    result
}

/// The directory of `path` (`.` for a bare file name).
fn parent_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    }
}

/// Creates a new, unused temp file `.<name>.<pid>.<n>.tmp` in `dir`.
fn create_temp(dir: &Path, name: &std::ffi::OsStr) -> io::Result<(PathBuf, File)> {
    let pid = std::process::id();
    for n in 0..TEMP_ATTEMPTS {
        let mut temp_name = OsString::from(".");
        temp_name.push(name);
        temp_name.push(format!(".{pid}.{n}.tmp"));
        let temp = dir.join(temp_name);
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => return Ok((temp, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, "no free temp file name"))
}

/// Flushes the directory entry of the rename to disk where the platform allows it.
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    {
        // Best effort: the file itself is already on disk and in place, and some file systems
        // refuse to sync a directory; failing the save at this point would report a saved file
        // as not saved.
        let _ = File::open(dir).and_then(|d| d.sync_all());
    }
    #[cfg(not(unix))]
    let _ = dir;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_appends_bak_to_the_file_name() {
        assert_eq!(backup_path(Path::new("a/b/x.psproj")), Path::new("a/b/x.psproj.bak"));
        assert_eq!(backup_path(Path::new("x")), Path::new("x.bak"));
        assert_eq!(parent_dir(Path::new("x")), Path::new("."));
        assert_eq!(parent_dir(Path::new("a/x")), Path::new("a"));
    }

    #[test]
    fn a_path_without_a_file_name_is_refused() {
        assert!(atomic_write(Path::new("/"), false, |_| Ok(())).is_err());
        assert!(atomic_write(Path::new(".."), false, |_| Ok(())).is_err());
    }
}
