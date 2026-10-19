//! The built-in mini set is valid and drawn on Protasena's conventions.
// Test code: helpers outside `#[test]` functions may panic too (clippy.toml covers only those).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use protasena_library::{BUILTIN_LIBRARY, Library, PinKind, units};

#[test]
fn builtin_validates_and_lives_in_one_library() {
    let lib = Library::builtin();
    assert!(lib.issues().is_empty(), "{:?}", lib.issues());
    assert_eq!(lib.libraries().len(), 1);
    assert_eq!(lib.libraries()[0].name, BUILTIN_LIBRARY);
    assert_eq!(lib.libraries()[0].device_count, 11);
    assert_eq!(lib.libraries()[0].package_count, 7);
    assert!(Library::from_parts(lib.devices().to_vec(), lib.packages().to_vec()).is_ok());
}

#[test]
fn pin_ends_sit_on_the_schematic_grid() {
    for d in Library::builtin().devices() {
        for pin in d.symbol.pins() {
            for c in [pin.at.x, pin.at.y] {
                assert_eq!(c % units::GRID, 0.0, "{} pin {} at {:?}", d.name, pin.number, pin.at);
            }
        }
    }
}

#[test]
fn packaged_devices_map_every_pin_and_default_to_their_package_property() {
    let lib = Library::builtin();
    for d in lib.devices().iter().filter(|d| !d.packages.is_empty()) {
        let default = d.default_package.as_deref().unwrap();
        assert_eq!(d.prop_def("PACKAGE").and_then(|p| p.default.as_deref()), Some(default), "{}", d.name);
        for pref in &d.packages {
            let pkg = lib.package(&pref.name).unwrap();
            for pin in d.symbol.pins() {
                let pad = pref.pad_for(&pin.number).unwrap_or_else(|| panic!("{} pin {} unmapped", d.name, pin.number));
                assert!(pkg.pad(pad).is_some());
            }
        }
    }
}

#[test]
fn conventions_of_individual_parts() {
    let lib = Library::builtin();
    let npn = lib.get("NPN").unwrap();
    let names: Vec<&str> = npn.symbol.pins().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["B", "C", "E"]);
    assert_eq!(npn.package("TO92").unwrap().pad_for("3"), Some("1"), "E-B-C pinout");
    for name in ["GROUND", "POWER"] {
        let d = lib.get(name).unwrap();
        assert!(d.symbol.power && d.prefix.is_empty() && d.packages.is_empty(), "{name}");
        assert!(d.symbol.pins().all(|p| p.kind == PinKind::PowerIn));
    }
    assert_eq!(lib.get("RES").unwrap().default_properties(), [("VALUE".to_owned(), "10k".to_owned()), ("PACKAGE".to_owned(), "RES40".to_owned())]);
    assert_eq!(lib.get("OPAMP").unwrap().symbol.pins().count(), 5);
    let led = lib.get("LED").unwrap();
    assert_eq!(led.sim_model.as_ref().map(|m| m.primitive.as_str()), Some("ANALOG"));
    assert!(lib.devices().iter().all(|d| !d.description.is_empty() && !d.category.is_empty()));
}
