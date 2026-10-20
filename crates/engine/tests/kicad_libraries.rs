//! The user's own KiCad libraries: System Settings keeps their paths, they are imported at once
//! (cached), their devices are picked under their library's name, and problems are reported,
//! never a failure of the command.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use std::path::{Path, PathBuf};

use common::*;
use protasena_engine::{Session, load_libraries};
use protasena_library::Library;
use serde_json::json;

/// Our own two-symbol KiCad library (one of them shares its name with a built-in part).
const MY_PARTS: &str = r#"(kicad_symbol_lib (version 20241209)
  (symbol "RES" (property "Reference" "R" (at 0 0 0)) (property "Value" "RES" (at 0 0 0))
    (property "Description" "my own resistor" (at 0 0 0))
    (symbol "RES_1_1" (pin passive line (at -5.08 0 0) (length 2.54) (name "~") (number "1"))
                      (pin passive line (at 5.08 0 180) (length 2.54) (name "~") (number "2"))))
  (symbol "SENSOR9" (property "Reference" "U" (at 0 0 0)) (property "Value" "SENSOR9" (at 0 0 0))
    (property "Description" "my humidity sensor" (at 0 0 0))
    (symbol "SENSOR9_1_1" (pin power_in line (at 0 5.08 270) (length 2.54) (name "VDD") (number "1"))
                          (pin output line (at 5.08 0 180) (length 2.54) (name "OUT") (number "2")))))"#;

fn folder(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("engine-kicad").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("libs")).unwrap();
    std::fs::write(dir.join("libs/MyParts.kicad_sym"), MY_PARTS).unwrap();
    dir
}

#[test]
fn system_settings_imports_the_libraries_and_pick_devices_lists_them() {
    let dir = folder("import");
    let mut s = Session::new();
    s.set_cache_dir(Some(dir.join("cache")));
    let libs = dir.join("libs").display().to_string();
    let v = exec(&mut s, "system-settings", json!({ "kicad_libraries": [libs, "  ", libs] }));
    assert_eq!(s.settings().kicad_libraries, std::slice::from_ref(&libs), "blanks and repeats are dropped");
    assert_eq!(v["kicad"][0]["devices"], json!(2), "{v}");
    assert_eq!(v["kicad"][0]["libraries"], json!(["MyParts"]));
    assert_eq!(v["kicad"][0]["cached"], json!(false));
    let hits = exec(&mut s, "pick-device", json!({ "keywords": "humidity" }));
    assert_eq!(hits["results"][0]["name"], json!("SENSOR9"), "{hits}");
    assert_eq!(hits["results"][0]["library"], json!("MyParts"));
    // `RES` stays the built-in part; the imported one is picked with its library.
    assert_eq!(s.library().get("RES").unwrap().library, "GENERIC");
    exec(&mut s, "pick-device", json!({ "pick": "RES", "library": "MyParts" }));
    exec(&mut s, "place-component", json!({ "at": [0, 0] }));
    let c = &sheet(&s).components[0];
    assert_eq!((c.device.library.as_str(), c.reference.as_str()), ("MyParts", "R1"));
    // Importing again reads the cache.
    let again = exec(&mut s, "import-kicad-libraries", json!({}));
    assert_eq!(again["kicad"][0]["cached"], json!(true), "{again}");
    // Removing the path removes the devices.
    exec(&mut s, "system-settings", json!({ "kicad_libraries": [] }));
    assert!(s.library().get("SENSOR9").is_none());
}

#[test]
fn a_pack_loaded_later_keeps_the_imported_libraries() {
    let dir = folder("later");
    let mut s = Session::new();
    exec(&mut s, "system-settings", json!({ "kicad_libraries": [dir.join("libs").display().to_string()] }));
    assert!(s.library().get("SENSOR9").is_some());
    s.set_library(Library::builtin());
    assert!(s.library().get("SENSOR9").is_some(), "the base changed, the import stays");
    assert_eq!(s.library().len(), Library::builtin().len() + 2);
}

#[test]
fn problems_are_reported_not_failures() {
    let dir = folder("problems");
    std::fs::write(dir.join("libs/Broken.kicad_sym"), "(kicad_symbol_lib (symbol").unwrap();
    let mut s = Session::new();
    let missing = dir.join("nowhere").display().to_string();
    let v = exec(&mut s, "system-settings", json!({ "kicad_libraries": [dir.join("libs").display().to_string(), missing] }));
    assert_eq!(v["kicad"][0]["devices"], json!(2));
    assert!(v["kicad"][0]["errors"][0].as_str().unwrap().contains("Broken.kicad_sym"), "{v}");
    assert!(v["kicad"][1]["errors"][0].as_str().unwrap().contains("nowhere"), "{v}");
    assert!(s.log().iter().any(|l| l.contains("nowhere")), "{:?}", s.log());
    let bad = s.execute("system-settings", &json!({ "kicad_libraries": "not a list" }));
    assert!(bad.is_err());
}

#[test]
fn a_background_load_gives_the_pack_and_the_imports() {
    let dir = folder("background");
    let loaded = load_libraries(None, &[dir.join("libs")], Some(&dir.join("cache")));
    assert!(loaded.base.is_none());
    let mut s = Session::new();
    assert_eq!(s.take_libraries(loaded), None);
    assert!(s.library().get("SENSOR9").is_some());
    assert_eq!(s.kicad_sources().count(), 1);
    let failed = load_libraries(Some(&dir.join("no-pack")), &[], None);
    assert!(s.take_libraries(failed).is_some(), "the pack's error is returned");
    assert!(s.library().get("RES").is_some(), "the built-in parts stay");
}
