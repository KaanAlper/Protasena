//! Broken input is reported as failures, never a crash, and the rest is still converted.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::fs;
use std::path::{Path, PathBuf};

use protasena_libimport::{Map, Options, run};
use protasena_library::Library;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("libconv-hostile").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("sym")).unwrap();
    fs::create_dir_all(dir.join("fp/Bad.pretty")).unwrap();
    dir
}

const GOOD: &str = r#"(kicad_symbol_lib (version 20241209)
  (symbol "OK" (property "Reference" "U" (at 0 0 0)) (property "Value" "OK" (at 0 0 0))
    (symbol "OK_1_1" (pin input line (at 0 0 0) (length 2.54) (name "A") (number "1"))))
  (symbol "Orphan" (extends "NoSuchParent") (property "Value" "Orphan" (at 0 0 0))))"#;

fn options(dir: &Path) -> Options {
    Options { symbols: dir.join("sym"), footprints: dir.join("fp"), map: Map::default(), out: dir.join("out"), jobs: 2, upstream: String::new() }
}

#[test]
fn broken_files_and_symbols_become_failures() {
    let dir = scratch("broken");
    fs::write(dir.join("sym/Good.kicad_sym"), GOOD).unwrap();
    fs::write(dir.join("sym/Garbage.kicad_sym"), [0xff, 0x00, b'(', b'(', b'"']).unwrap();
    fs::write(dir.join("sym/Wrong.kicad_sym"), "(footprint \"x\")").unwrap();
    fs::write(dir.join("fp/Bad.pretty/Broken.kicad_mod"), "(footprint \"Broken\" (pad").unwrap();
    fs::write(dir.join("fp/Bad.pretty/Huge.kicad_mod"), "(footprint \"Huge\" (pad \"1\" smd rect (at 1e999 0) (size 1 1) (layers \"F.Cu\")))")
        .unwrap();
    let report = run(&options(&dir)).unwrap();
    let what: Vec<&str> = report.failures.iter().map(|f| f.what.as_str()).collect();
    assert!(what.contains(&"Garbage.kicad_sym") && what.contains(&"Wrong.kicad_sym"), "{what:?}");
    assert!(what.contains(&"Good/Orphan"), "{what:?}");
    assert!(what.contains(&"Bad.pretty/Broken.kicad_mod"), "{what:?}");
    assert!(what.iter().any(|w| w.contains("Huge")), "non-finite pad rejected by validation: {what:?}");
    assert_eq!(report.counts.devices, 1);
    let lib = Library::load_dir(&dir.join("out")).unwrap();
    assert!(lib.get("OK").is_some());
}

#[test]
fn refuses_to_overwrite_a_directory_that_is_not_a_pack() {
    let dir = scratch("guard");
    fs::create_dir_all(dir.join("out")).unwrap();
    fs::write(dir.join("out/precious.txt"), "keep me").unwrap();
    let err = run(&options(&dir)).unwrap_err();
    assert!(err.contains("not a pack"), "{err}");
    assert!(dir.join("out/precious.txt").exists());
}

#[test]
fn rewrites_a_previous_pack_in_place() {
    let dir = scratch("again");
    fs::write(dir.join("sym/Good.kicad_sym"), GOOD).unwrap();
    run(&options(&dir)).unwrap();
    fs::write(dir.join("out/symbols/Stale.json"), "{}").unwrap();
    run(&options(&dir)).unwrap();
    assert!(!dir.join("out/symbols/Stale.json").exists());
    assert!(Library::load_dir(&dir.join("out")).is_ok());
}

#[test]
fn missing_input_directories_stop_the_run() {
    let dir = scratch("missing");
    let opts = Options { symbols: dir.join("nope"), ..options(&dir) };
    assert!(run(&opts).is_err());
}
