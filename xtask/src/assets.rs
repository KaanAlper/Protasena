//! `cargo xtask assets`: every non-code file (image, font, data) must be attributed in
//! `ATTRIBUTION.md`.
//!
//! Scans the git-tracked and untracked-but-not-ignored files under [`ASSET_ROOTS`]. A file there is
//! an asset unless it is code or a manifest ([`CODE_EXT`], [`CODE_NAMES`]); unknown extensions
//! count as assets, so a new data format can't slip through. An asset is attributed when
//! `ATTRIBUTION.md` names it as `` `path` `` or names a directory containing it as `` `dir/` ``.

use std::path::Path;
use std::process::Command;

/// Where assets are looked for.
pub const ASSET_ROOTS: &[&str] = &["crates/", "apps/", "assets/", "docs/images/"];

/// Extensions of code and documentation files (never assets).
pub const CODE_EXT: &[&str] = &["rs", "toml", "md", "wgsl", "sh", "ps1"];

/// File names that are never assets.
pub const CODE_NAMES: &[&str] = &[".gitignore", ".gitattributes"];

pub fn is_asset(path: &str) -> bool {
    if !ASSET_ROOTS.iter().any(|r| path.starts_with(r)) {
        return false;
    }
    let p = Path::new(path);
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let ext = p.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    !CODE_NAMES.contains(&name) && !CODE_EXT.contains(&ext.as_str())
}

/// Every `` `…` `` span in the table rows of `md` (lines starting with `|`; prose that names
/// a directory attributes nothing).
fn backticked(md: &str) -> Vec<&str> {
    md.lines().filter(|l| l.trim_start().starts_with('|')).flat_map(|l| l.split('`').skip(1).step_by(2)).collect()
}

/// Paths that need an ATTRIBUTION.md entry but lack one.
pub fn missing(files: &[String], attribution_md: &str) -> Vec<String> {
    let named = backticked(attribution_md);
    files
        .iter()
        .filter(|f| is_asset(f))
        .filter(|f| !named.iter().any(|n| *n == f.as_str() || (n.ends_with('/') && f.starts_with(n))))
        .cloned()
        .collect()
}

pub fn run(root: &Path) -> Result<(), String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .output()
        .map_err(|e| format!("git ls-files: {e}"))?;
    if !out.status.success() {
        return Err(format!("git ls-files failed:\n{}", String::from_utf8_lossy(&out.stderr)));
    }
    let files: Vec<String> = String::from_utf8_lossy(&out.stdout).lines().filter(|l| root.join(l).exists()).map(str::to_owned).collect();
    let md = std::fs::read_to_string(root.join("ATTRIBUTION.md")).map_err(|e| format!("ATTRIBUTION.md: {e}"))?;
    let miss = missing(&files, &md);
    if miss.is_empty() {
        println!("assets: all {} asset file(s) attributed in ATTRIBUTION.md", files.iter().filter(|f| is_asset(f)).count());
        Ok(())
    } else {
        Err(format!("{} asset file(s) lack an ATTRIBUTION.md entry (author, source, licence):\n  {}", miss.len(), miss.join("\n  ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn code_and_manifests_are_not_assets() {
        for f in [
            "crates/geom/src/lib.rs",
            "crates/geom/Cargo.toml",
            "apps/protasena/README.md",
            "crates/x/.gitignore",
            "README.md",
            "tools/proteus/uia-dump.ps1",
        ] {
            assert!(!is_asset(f), "{f}");
        }
    }

    #[test]
    fn images_fonts_and_data_are_assets() {
        for f in [
            "crates/ui-egui/icons/x.svg",
            "apps/protasena/icon.png",
            "crates/kicad/tests/data/r.kicad_sym",
            "crates/library/data/index.json",
            "assets/fonts/A.ttf",
            "docs/images/shot.png",
            "crates/x/blob",
        ] {
            assert!(is_asset(f), "{f}");
        }
    }

    #[test]
    fn detects_unattributed_assets() {
        let files = strings(&[
            "assets/icons/a.svg",
            "crates/x/src/lib.rs",
            "docs/images/b.png",
            "crates/kicad/tests/data/r.kicad_sym",
            "crates/kicad/tests/data/c.kicad_mod",
            "apps/p/logo.png",
        ]);
        let md = "| `assets/icons/a.svg` | us | original | MIT OR Apache-2.0 |\n| `crates/kicad/tests/data/` | us | original | CC0-1.0 |";
        assert_eq!(missing(&files, md), strings(&["docs/images/b.png", "apps/p/logo.png"]));
    }

    #[test]
    fn paths_named_outside_the_table_attribute_nothing() {
        let files = strings(&["crates/ui-egui/i18n/en.tsv"]);
        let md = "Every non-code file under `crates/` and `apps/` is listed.\n\n| Asset | Author |\n|---|---|\n| `apps/x/` | us |";
        assert_eq!(missing(&files, md), files, "the prose mention of `crates/` is not a row");
    }

    #[test]
    fn a_directory_row_needs_the_trailing_slash() {
        let files = strings(&["crates/kicad/tests/data/r.kicad_sym"]);
        assert_eq!(missing(&files, "`crates/kicad/tests/data`"), files);
        assert_eq!(missing(&files, "`crates/kicad/tests/dat/`"), files);
    }
}
