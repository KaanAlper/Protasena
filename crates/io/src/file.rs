//! Saving and loading `.psproj` files.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use protasena_doc::Project;

use crate::atomic::atomic_write;
use crate::codec::{LoadReport, decode, encode};
use crate::error::{LoadError, SaveError};
use crate::limits::MAX_FILE_BYTES;

/// Saves a project atomically: the new file replaces `path` in one step, and the previous file
/// (if any) is kept as [`backup_path`](crate::backup_path) (`design.psproj.bak`). A failed save
/// leaves both as they were.
///
/// # Errors
/// [`SaveError::NonFinite`] for a project with NaN/∞ numbers, [`SaveError::Io`] when the file
/// cannot be written.
pub fn save(project: &Project, path: &Path) -> Result<(), SaveError> {
    let text = encode(project)?;
    atomic_write(path, true, |out| out.write_all(text.as_bytes())).map_err(|e| SaveError::Io { path: path.to_owned(), message: e.to_string() })
}

/// Loads a project file (see [`decode`] for the checks).
///
/// # Errors
/// [`LoadError::Io`] when the file cannot be read or is not a file,
/// [`LoadError::TooLarge`] for a file over [`MAX_FILE_BYTES`] (refused without reading it),
/// and every error of [`decode`].
pub fn load(path: &Path) -> Result<(Project, LoadReport), LoadError> {
    decode(&read_limited(path)?)
}

fn read_limited(path: &Path) -> Result<Vec<u8>, LoadError> {
    let io_error = |message: String| LoadError::Io { path: path.to_owned(), message };
    let file = File::open(path).map_err(|e| io_error(e.to_string()))?;
    let meta = file.metadata().map_err(|e| io_error(e.to_string()))?;
    if !meta.is_file() {
        return Err(io_error("not a file".to_owned()));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(LoadError::TooLarge { size: meta.len(), limit: MAX_FILE_BYTES });
    }
    // The file may grow while it is read: read at most one byte more than the limit.
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES.saturating_add(1)).read_to_end(&mut bytes).map_err(|e| io_error(e.to_string()))?;
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size > MAX_FILE_BYTES {
        return Err(LoadError::TooLarge { size, limit: MAX_FILE_BYTES });
    }
    Ok(bytes)
}
