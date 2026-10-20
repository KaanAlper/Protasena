//! The run-time import of the user's own KiCad libraries: what paths may be, the cache, the
//! problems reported, and fitting the result in beside a loaded library. Fixtures are our own
//! (`tests/data`, CC0).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use protasena_libimport::user::{self, LIBRARY_SUFFIX, fit};
use protasena_library::Library;

fn data(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(p)
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("libimport-user").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A folder holding `Passives.kicad_sym` and `TestFP.pretty`, copied from the fixtures.
fn user_folder(root: &Path) -> PathBuf {
    let lib = root.join("mylibs");
    fs::create_dir_all(lib.join("TestFP.pretty")).unwrap();
    fs::copy(data("symbols/Passives.kicad_sym"), lib.join("Passives.kicad_sym")).unwrap();
    for f in ["R_Test.kicad_mod", "DIP14_Test.kicad_mod"] {
        fs::copy(data("footprints/TestFP.pretty").join(f), lib.join("TestFP.pretty").join(f)).unwrap();
    }
    lib
}

#[test]
fn a_folder_gives_its_symbol_libraries_linked_to_its_footprints() {
    let root = scratch("folder");
    let lib = user_folder(&root);
    let got = user::import(std::slice::from_ref(&lib), None, 2);
    assert_eq!(got.len(), 1);
    let src = &got[0];
    assert!(src.report.errors.is_empty(), "{:?}", src.report.errors);
    assert_eq!(src.report.libraries, ["Passives", "TestFP.pretty"]);
    assert_eq!((src.report.devices, src.report.packages, src.report.cached), (4, 2, false));
    let r = src.devices.iter().find(|d| d.name == "R").unwrap();
    assert_eq!(r.library, "Passives");
    assert_eq!(r.default_package.as_deref(), Some("R_Test"), "the Footprint field links the imported footprint");
    assert!(src.packages.iter().any(|p| p.name == "R_Test" && p.library == "TestFP.pretty"));
}

#[test]
fn a_symbol_file_and_a_footprint_folder_import_on_their_own() {
    let root = scratch("single");
    let lib = user_folder(&root);
    let got = user::import(&[lib.join("Passives.kicad_sym"), lib.join("TestFP.pretty")], None, 1);
    assert_eq!(got[0].report.libraries, ["Passives"]);
    assert_eq!(got[0].report.packages, 0);
    assert!(got[0].devices.iter().all(|d| d.packages.is_empty()), "no footprints in the same path");
    assert_eq!((got[1].report.devices, got[1].report.packages), (0, 2));
}

#[test]
fn the_cache_is_used_until_a_file_changes() {
    let root = scratch("cache");
    let lib = user_folder(&root);
    let cache = root.join("cache");
    let first = user::import(std::slice::from_ref(&lib), Some(&cache), 2);
    assert!(!first[0].report.cached);
    let again = user::import(std::slice::from_ref(&lib), Some(&cache), 2);
    assert!(again[0].report.cached, "nothing changed");
    assert_eq!(again[0].devices, first[0].devices);
    assert_eq!(again[0].packages, first[0].packages);
    // A changed modification time converts again.
    let sym = lib.join("Passives.kicad_sym");
    let later = SystemTime::now() + Duration::from_secs(120);
    fs::File::options().write(true).open(&sym).unwrap().set_modified(later).unwrap();
    let changed = user::import(std::slice::from_ref(&lib), Some(&cache), 2);
    assert!(!changed[0].report.cached);
    assert_eq!(changed[0].devices, first[0].devices);
    // A new file too.
    fs::copy(data("symbols/Extra.kicad_sym"), lib.join("Extra.kicad_sym")).unwrap();
    let grown = user::import(std::slice::from_ref(&lib), Some(&cache), 2);
    assert!(!grown[0].report.cached);
    assert!(grown[0].report.libraries.contains(&"Extra".to_owned()));
}

#[test]
fn a_damaged_cache_is_converted_again() {
    let root = scratch("damaged");
    let lib = user_folder(&root);
    let cache = root.join("cache");
    let first = user::import(std::slice::from_ref(&lib), Some(&cache), 1);
    for entry in fs::read_dir(cache.join("kicad")).unwrap() {
        let dir = entry.unwrap().path();
        fs::write(dir.join("pack/index.json"), "{ not json").unwrap();
    }
    let again = user::import(std::slice::from_ref(&lib), Some(&cache), 1);
    assert!(!again[0].report.cached);
    assert!(again[0].report.errors.is_empty(), "{:?}", again[0].report.errors);
    assert_eq!(again[0].devices, first[0].devices);
    for entry in fs::read_dir(cache.join("kicad")).unwrap() {
        fs::write(entry.unwrap().path().join("meta.json"), [0xff, 0x00, 0x7b]).unwrap();
    }
    assert!(!user::import(std::slice::from_ref(&lib), Some(&cache), 1)[0].report.cached);
}

#[test]
fn problems_are_reported_and_the_rest_imported() {
    let root = scratch("problems");
    let lib = user_folder(&root);
    fs::write(lib.join("Broken.kicad_sym"), "(kicad_symbol_lib (symbol \"X\" (pin").unwrap();
    fs::write(lib.join("TestFP.pretty/Bad.kicad_mod"), "\u{0}\u{1} not a footprint").unwrap();
    fs::write(root.join("notes.txt"), "hello").unwrap();
    fs::create_dir_all(root.join("empty")).unwrap();
    let paths = [lib.clone(), root.join("missing"), root.join("notes.txt"), root.join("empty")];
    let got = user::import(&paths, None, 2);
    let errors: Vec<&Vec<String>> = got.iter().map(|s| &s.report.errors).collect();
    assert!(errors[0].iter().any(|e| e.contains("Broken.kicad_sym")), "{:?}", errors[0]);
    assert!(errors[0].iter().any(|e| e.contains("Bad.kicad_mod")), "{:?}", errors[0]);
    assert_eq!(got[0].report.devices, 4, "the good library is still imported");
    assert!(errors[1][0].contains("missing"), "{:?}", errors[1]);
    assert!(errors[2][0].contains("not a KiCad symbol library"), "{:?}", errors[2]);
    assert!(errors[3][0].contains("holds no KiCad"), "{:?}", errors[3]);
    assert!(got[1..].iter().all(|s| s.devices.is_empty() && s.packages.is_empty()));
}

#[test]
fn fit_renames_taken_library_and_package_names() {
    let root = scratch("fit");
    let lib = user_folder(&root);
    let sources = user::import(std::slice::from_ref(&lib), None, 1);
    // A loaded library that already has a `Passives` library and an `R_Test` package.
    let base_sources = user::import(std::slice::from_ref(&lib), None, 1);
    let (bd, bp, _) = fit(&base_sources, &Library::default());
    let base = Library::from_parts(bd, bp).unwrap();
    let (devices, packages, notes) = fit(&sources, &base);
    let passives = format!("Passives{LIBRARY_SUFFIX}");
    assert!(devices.iter().all(|d| d.library == passives), "{:?}", devices.iter().map(|d| &d.library).collect::<Vec<_>>());
    assert!(packages.iter().any(|p| p.name == "TestFP:R_Test" && p.library == format!("TestFP.pretty{LIBRARY_SUFFIX}")));
    let r = devices.iter().find(|d| d.name == "R").unwrap();
    assert_eq!(r.default_package.as_deref(), Some("TestFP:R_Test"));
    assert!(r.packages.iter().all(|p| p.name.starts_with("TestFP:")));
    assert_eq!(r.prop_def("PACKAGE").and_then(|p| p.default.as_deref()), Some("TestFP:R_Test"));
    assert!(notes.iter().any(|n| n.contains("imported as")), "{notes:?}");
    let (merged, issues) = base.with_extra(devices, packages);
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(merged.len(), 8);
}
