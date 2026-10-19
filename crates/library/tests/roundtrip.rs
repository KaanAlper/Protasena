//! Serialise → deserialise gives back the same data, for single values and whole packs.
// Test code: helpers outside `#[test]` functions may panic too (clippy.toml covers only those).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::{Scratch, two_pad, two_pin};
use protasena_library::{Device, Library, LibraryError, Package, PropDef, PropKind, SimModelRef, Source};

#[test]
fn every_builtin_device_and_package_round_trips_through_json() {
    let lib = Library::builtin();
    for d in lib.devices() {
        let text = serde_json::to_string(d).unwrap();
        assert_eq!(&serde_json::from_str::<Device>(&text).unwrap(), d, "{}", d.name);
    }
    for p in lib.packages() {
        let text = serde_json::to_string(p).unwrap();
        assert_eq!(&serde_json::from_str::<Package>(&text).unwrap(), p, "{}", p.name);
    }
}

#[test]
fn optional_fields_round_trip() {
    let d = Device {
        manufacturer: "Acme".into(),
        stock_code: "AC-123".into(),
        sub_category: "Odd".into(),
        keywords: vec!["thing".into()],
        prop_defs: vec![
            PropDef::new("MODE", "Mode", PropKind::Keywords { choices: vec!["FAST".into(), "SLOW".into()] }, Some("FAST")),
            PropDef::new("NOTE", "Note", PropKind::String, None).hidden(),
            PropDef::new("GAIN", "Gain", PropKind::Float { unit: None }, Some("0.1")),
        ],
        sim_model: Some(SimModelRef { primitive: "SPICE".into(), model: "X".into(), params: vec![("A".into(), "1.5".into())] }),
        source: Some(Source { upstream: "kicad".into(), lib: "L".into(), symbol: "S".into(), licence: "CC-BY-SA-4.0".into() }),
        ..two_pin("ODD", "LIB", "P2")
    };
    let text = serde_json::to_string_pretty(&d).unwrap();
    assert_eq!(serde_json::from_str::<Device>(&text).unwrap(), d);
}

#[test]
fn awkward_floats_survive_exactly() {
    let mut d = two_pin("F", "LIB", "P2");
    let pin = &mut d.symbol.units[0].pins[0];
    pin.at = protasena_geom::Pt::new(0.1 + 0.2, -1.0e-300);
    pin.length = 39.370_078_740_157_48;
    let back: Device = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
    assert_eq!(back, d);
}

#[test]
fn builtin_pack_saves_and_loads_unchanged() {
    let dir = Scratch::new("builtin-roundtrip");
    let lib = Library::builtin();
    lib.save_dir(dir.path()).unwrap();
    let back = Library::load_dir(dir.path()).unwrap();
    assert_eq!(back.devices(), lib.devices());
    assert_eq!(back.packages(), lib.packages());
    assert_eq!(back.libraries(), lib.libraries());
}

#[test]
fn multi_library_pack_round_trips_and_keeps_order() {
    let dir = Scratch::new("multi");
    let devices = vec![two_pin("A1", "ZETA", "P2"), two_pin("B1", "ALPHA", "P2"), two_pin("A2", "ZETA", "Q2")];
    let packages = vec![two_pad("P2", "PKGS"), two_pad("Q2", "ZETA")];
    let lib = Library::from_parts(devices, packages).unwrap();
    let names: Vec<&str> = lib.libraries().iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["ZETA", "ALPHA", "PKGS"]);
    lib.save_dir(dir.path()).unwrap();
    let back = Library::load_dir(dir.path()).unwrap();
    assert_eq!(back.devices(), lib.devices());
    assert_eq!(back.packages(), lib.packages());
    assert_eq!(back.libraries(), lib.libraries());
    assert_eq!(back.package("Q2").map(|p| p.library.as_str()), Some("ZETA"));
}

#[test]
fn library_names_that_are_not_file_names_still_save() {
    let dir = Scratch::new("odd-names");
    let devices = vec![two_pin("A", "a/b", "P2"), two_pin("B", "a:b", "P2"), two_pin("C", "index", "P2"), two_pin("D", "A_B", "P2")];
    let lib = Library::from_parts(devices, vec![two_pad("P2", "a/b")]).unwrap();
    lib.save_dir(dir.path()).unwrap();
    let back = Library::load_dir(dir.path()).unwrap();
    assert_eq!(back.devices(), lib.devices());
}

#[test]
fn saving_into_a_file_path_is_an_io_error() {
    let dir = Scratch::new("save-into-file");
    dir.write("blocker", "x");
    let err = Library::builtin().save_dir(&dir.path().join("blocker")).unwrap_err();
    assert!(matches!(err, LibraryError::Io { .. }), "{err:?}");
}
