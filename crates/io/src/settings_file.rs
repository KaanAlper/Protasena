//! Where the system settings file lives, and reading and writing it.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::SaveError;
use crate::settings::{MAX_SETTINGS_BYTES, SystemSettings, decode_settings, encode_settings};

/// Environment variable naming the configuration directory (overrides the platform's).
pub const CONFIG_DIR_VAR: &str = "PROTASENA_CONFIG_DIR";
/// File name of the settings file in the configuration directory.
pub const SETTINGS_FILE: &str = "settings.json";

/// The configuration directory: [`CONFIG_DIR_VAR`] when set, else the platform's per-user
/// configuration directory with a `Protasena` folder (`%APPDATA%\Protasena` on Windows,
/// `~/Library/Application Support/Protasena` on macOS, `$XDG_CONFIG_HOME/protasena` or
/// `~/.config/protasena` elsewhere). `None` when no home directory is known.
pub fn config_dir() -> Option<PathBuf> {
    config_dir_from(|k| std::env::var_os(k).map(|v| v.to_string_lossy().into_owned()), std::env::consts::OS)
}

/// [`config_dir`] for a given environment (`var` looks a variable up) and operating system.
pub fn config_dir_from(var: impl Fn(&str) -> Option<String>, os: &str) -> Option<PathBuf> {
    let get = |k: &str| var(k).filter(|v| !v.trim().is_empty());
    if let Some(dir) = get(CONFIG_DIR_VAR) {
        return Some(PathBuf::from(dir));
    }
    match os {
        "windows" => get("APPDATA").map(|d| PathBuf::from(d).join("Protasena")),
        "macos" => get("HOME").map(|h| PathBuf::from(h).join("Library").join("Application Support").join("Protasena")),
        _ => match get("XDG_CONFIG_HOME").filter(|d| Path::new(d).is_absolute()) {
            Some(d) => Some(PathBuf::from(d).join("protasena")),
            None => get("HOME").map(|h| PathBuf::from(h).join(".config").join("protasena")),
        },
    }
}

/// Environment variable naming the cache directory (overrides the platform's).
pub const CACHE_DIR_VAR: &str = "PROTASENA_CACHE_DIR";

/// The cache directory (converted KiCad libraries): [`CACHE_DIR_VAR`] when set, else the
/// platform's per-user cache directory with a `Protasena` folder (`%LOCALAPPDATA%\Protasena`
/// on Windows, `~/Library/Caches/Protasena` on macOS, `$XDG_CACHE_HOME/protasena` or
/// `~/.cache/protasena` elsewhere). `None` when no home directory is known.
pub fn cache_dir() -> Option<PathBuf> {
    cache_dir_from(|k| std::env::var_os(k).map(|v| v.to_string_lossy().into_owned()), std::env::consts::OS)
}

/// [`cache_dir`] for a given environment and operating system.
pub fn cache_dir_from(var: impl Fn(&str) -> Option<String>, os: &str) -> Option<PathBuf> {
    let get = |k: &str| var(k).filter(|v| !v.trim().is_empty());
    if let Some(dir) = get(CACHE_DIR_VAR) {
        return Some(PathBuf::from(dir));
    }
    match os {
        "windows" => get("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Protasena")),
        "macos" => get("HOME").map(|h| PathBuf::from(h).join("Library").join("Caches").join("Protasena")),
        _ => match get("XDG_CACHE_HOME").filter(|d| Path::new(d).is_absolute()) {
            Some(d) => Some(PathBuf::from(d).join("protasena")),
            None => get("HOME").map(|h| PathBuf::from(h).join(".cache").join("protasena")),
        },
    }
}

/// Folder name of the user's writable libraries in [`config_dir`].
pub const USER_LIBRARY_DIR: &str = "libraries";

/// The user's writable libraries (Make Device, the Library Manager): a pack in
/// [`USER_LIBRARY_DIR`] of [`config_dir`].
pub fn user_library_dir() -> Option<PathBuf> {
    config_dir().map(|d| d.join(USER_LIBRARY_DIR))
}

/// The settings file: [`SETTINGS_FILE`] in [`config_dir`].
pub fn settings_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join(SETTINGS_FILE))
}

/// Reads the settings file. Never fails: a missing file gives the defaults silently; an
/// unreadable, oversized or malformed one gives the defaults (or what could be read) and
/// warnings.
pub fn load_settings(path: &Path) -> (SystemSettings, Vec<String>) {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (SystemSettings::default(), Vec::new()),
        Err(e) => return (SystemSettings::default(), vec![format!("{}: {e}; the default settings are used", path.display())]),
    };
    let too_big = file.metadata().map(|m| !m.is_file() || m.len() > MAX_SETTINGS_BYTES).unwrap_or(true);
    if too_big {
        return (SystemSettings::default(), vec![format!("{} is not a settings file of at most {MAX_SETTINGS_BYTES} bytes", path.display())]);
    }
    let mut bytes = Vec::new();
    if let Err(e) = file.take(MAX_SETTINGS_BYTES.saturating_add(1)).read_to_end(&mut bytes) {
        return (SystemSettings::default(), vec![format!("{}: {e}; the default settings are used", path.display())]);
    }
    let (s, warnings) = decode_settings(&bytes);
    (s, warnings.into_iter().map(|w| format!("{}: {w}", path.display())).collect())
}

/// Writes the settings file atomically, making its directory when needed.
///
/// # Errors
/// [`SaveError::Io`] when the directory or the file cannot be written.
pub fn save_settings(settings: &SystemSettings, path: &Path) -> Result<(), SaveError> {
    let io = |e: std::io::Error| SaveError::Io { path: path.to_owned(), message: e.to_string() };
    let text = encode_settings(settings)?;
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    crate::atomic::atomic_write(path, false, |out| out.write_all(text.as_bytes())).map_err(io)
}
