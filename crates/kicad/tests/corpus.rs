//! Opt-in corpus test: parses every file of a KiCad library checkout.
//!
//! ```sh
//! KICAD_SYMBOL_DIR=…/kicad-symbols KICAD_FOOTPRINT_DIR=…/kicad-footprints \
//!   cargo test -p protasena-kicad --release --test corpus -- --nocapture
//! ```
//!
//! Skipped (passes) when neither variable is set. Fails on any parse or `extends` error and
//! prints totals and the unknown-token counts. `KICAD_CORPUS_SAMPLES=1` also prints one
//! example of each unknown token (to the terminal only).

use std::path::{Path, PathBuf};

use protasena_kicad::{Report, parse_footprint, parse_symbol_lib};

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            files(&p, ext, out);
        } else if p.extension().is_some_and(|e| e == ext) {
            out.push(p);
        }
    }
}

#[derive(Default)]
struct Totals {
    files: usize,
    errors: Vec<String>,
    report: Report,
}

impl Totals {
    fn print(&self, what: &str) {
        println!("{what}: {} files, {} errors", self.files, self.errors.len());
        for e in self.errors.iter().take(30) {
            println!("  ERROR {e}");
        }
        println!("  unknown tokens ({} total):", self.report.unknown_count());
        let show = std::env::var_os("KICAD_CORPUS_SAMPLES").is_some();
        for (k, n) in &self.report.unknown_tokens {
            println!("    {n:>8}  {k}");
            if show && let Some(t) = self.report.examples.get(k) {
                let s: String = t.to_string().chars().take(200).collect();
                println!("              e.g. {s}");
            }
        }
        println!("  values outside the documented sets:");
        for (k, n) in &self.report.other_values {
            println!("    {n:>8}  {k}");
        }
    }
}

#[test]
fn corpus_parses_without_errors() {
    let sym_dir = std::env::var_os("KICAD_SYMBOL_DIR").map(PathBuf::from);
    let fp_dir = std::env::var_os("KICAD_FOOTPRINT_DIR").map(PathBuf::from);
    if sym_dir.is_none() && fp_dir.is_none() {
        println!("corpus test skipped: set KICAD_SYMBOL_DIR and/or KICAD_FOOTPRINT_DIR");
        return;
    }
    let mut failed = false;
    if let Some(dir) = sym_dir {
        let mut paths = Vec::new();
        files(&dir, "kicad_sym", &mut paths);
        assert!(!paths.is_empty(), "no .kicad_sym files under {}", dir.display());
        let mut t = Totals { files: paths.len(), ..Totals::default() };
        let (mut symbols, mut derived, mut units, mut pins) = (0usize, 0usize, 0usize, 0usize);
        for p in &paths {
            let bytes = std::fs::read(p).unwrap();
            match parse_symbol_lib(&bytes) {
                Ok(lib) => {
                    symbols += lib.symbols.len();
                    derived += lib.symbols.iter().filter(|s| s.extends.is_some()).count();
                    for s in &lib.symbols {
                        units += s.units.len();
                        pins += s.units.iter().map(|u| u.pins.len()).sum::<usize>();
                    }
                    if let Err(e) = lib.resolve_all() {
                        t.errors.push(format!("{}: {e}", p.display()));
                    }
                    t.report.merge(&lib.report());
                }
                Err(e) => t.errors.push(format!("{}: {e}", p.display())),
            }
        }
        println!("symbols: {symbols} ({derived} derived), units: {units}, pins: {pins}");
        t.print("symbol libraries");
        failed |= !t.errors.is_empty();
    }
    if let Some(dir) = fp_dir {
        let mut paths = Vec::new();
        files(&dir, "kicad_mod", &mut paths);
        assert!(!paths.is_empty(), "no .kicad_mod files under {}", dir.display());
        let mut t = Totals { files: paths.len(), ..Totals::default() };
        let (mut footprints, mut pads, mut graphics, mut models) = (0usize, 0usize, 0usize, 0usize);
        for p in &paths {
            let bytes = std::fs::read(p).unwrap();
            match parse_footprint(&bytes) {
                Ok(f) => {
                    footprints += 1;
                    pads += f.pads.len();
                    graphics += f.graphics.len();
                    models += f.models.len();
                    t.report.merge(&f.report());
                }
                Err(e) => t.errors.push(format!("{}: {e}", p.display())),
            }
        }
        println!("footprints: {footprints}, pads: {pads}, graphics: {graphics}, 3D model refs: {models}");
        t.print("footprints");
        failed |= !t.errors.is_empty();
    }
    assert!(!failed, "corpus had parse errors (see output)");
}
