//! What a configured path holds: `.kicad_sym` files and `.pretty` footprint folders.

use std::path::{Path, PathBuf};

use crate::inputs::{FpFile, list};

/// Most input files one path may hold (symbol files plus footprint files).
pub const MAX_FILES: usize = 100_000;

/// The KiCad files of one path, sorted by name.
#[derive(Debug, Clone, Default)]
pub struct Inputs {
    /// Symbol libraries: `(library name, file)`.
    pub symbols: Vec<(String, PathBuf)>,
    /// Footprint files of every `.pretty` folder.
    pub footprints: Vec<FpFile>,
}

impl Inputs {
    /// Every input file, symbol files first.
    pub fn files(&self) -> impl Iterator<Item = &Path> {
        self.symbols.iter().map(|(_, p)| p.as_path()).chain(self.footprints.iter().map(|f| f.path.as_path()))
    }
}

/// The inputs of `path`: a `.kicad_sym` file, a `.pretty` folder, or a folder holding them.
///
/// # Errors
/// A message for the user when the path does not exist, is not one of those, holds nothing
/// to import or more than [`MAX_FILES`] files.
pub fn scan(path: &Path) -> Result<Inputs, String> {
    let shown = path.display();
    let meta = std::fs::metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    let stem = || path.file_stem().and_then(|s| s.to_str()).map(str::to_owned).ok_or_else(|| format!("{shown}: the name is not valid text"));
    let mut inputs = Inputs::default();
    if meta.is_file() {
        if ext.as_deref() != Some("kicad_sym") {
            return Err(format!("{shown}: not a KiCad symbol library (.kicad_sym)"));
        }
        inputs.symbols.push((stem()?, path.to_owned()));
    } else if ext.as_deref() == Some("pretty") {
        add_pretty(&mut inputs, &stem()?, path)?;
    } else if meta.is_dir() {
        inputs.symbols = list(path, "kicad_sym", false)?;
        for (lib, dir) in list(path, "pretty", true)? {
            add_pretty(&mut inputs, &lib, &dir)?;
        }
        if inputs.symbols.is_empty() && inputs.footprints.is_empty() {
            return Err(format!("{shown}: holds no KiCad symbol libraries (.kicad_sym) or footprint folders (.pretty)"));
        }
    } else {
        return Err(format!("{shown}: not a file or folder"));
    }
    if inputs.symbols.len() + inputs.footprints.len() > MAX_FILES {
        return Err(format!("{shown}: more than {MAX_FILES} KiCad files"));
    }
    Ok(inputs)
}

fn add_pretty(inputs: &mut Inputs, lib: &str, dir: &Path) -> Result<(), String> {
    for (name, path) in list(dir, "kicad_mod", false)? {
        if inputs.footprints.len() > MAX_FILES {
            break;
        }
        inputs.footprints.push(FpFile { lib: lib.to_owned(), name, path });
    }
    Ok(())
}
