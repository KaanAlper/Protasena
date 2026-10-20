//! Memory savings of a loaded pack: package files read on first use, equal symbols and pin maps
//! shared, and laying a pack over the built-in parts without copying it.
// Test code: helpers outside `#[test]` functions may panic too (clippy.toml covers only those).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use std::sync::Arc;

use common::{Scratch, device_file, index, package_file, two_pad, two_pin};
use protasena_library::Library;

/// Two device libraries sharing a package library: `A` (R1, R2) and `B` (R3), packages P1/P2
/// in `FP` and P3 in `FP2`.
fn pack(name: &str) -> Scratch {
    let dir = Scratch::new(name);
    dir.write(
        "index.json",
        &index(&[
            ("A", "a.json", 2, None),
            ("B", "b.json", 1, None),
            ("FP", "fp.json", 0, Some("fp.p.json")),
            ("FP2", "fp2.json", 0, Some("fp2.p.json")),
        ]),
    );
    dir.write("a.json", &device_file(&[two_pin("R1", "", "P1"), two_pin("R2", "", "P2")]));
    dir.write("b.json", &device_file(&[two_pin("R3", "", "P3")]));
    dir.write("fp.json", &device_file(&[]));
    dir.write("fp2.json", &device_file(&[]));
    dir.write("fp.p.json", &package_file(&[two_pad("P1", ""), two_pad("P2", "")]));
    dir.write("fp2.p.json", &package_file(&[two_pad("P3", "")]));
    dir
}

#[test]
fn packages_are_read_on_first_use_and_listed_without_reading() {
    let dir = pack("lazy_ok");
    let lib = Library::load_dir(dir.path()).unwrap();
    assert_eq!(lib.package_count(), 3);
    let names: Vec<(&str, &str)> = lib.package_names().collect();
    assert_eq!(names, [("P1", "FP"), ("P2", "FP"), ("P3", "FP2")]);
    assert_eq!(lib.package("P2").unwrap().library, "FP");
    assert_eq!(lib.packages().count(), 3);
    assert!(lib.package("nope").is_none());
    assert!(lib.issues().is_empty());
    let counts: Vec<(&str, usize, usize)> = lib.libraries().iter().map(|l| (l.name.as_str(), l.device_count, l.package_count)).collect();
    assert_eq!(counts, [("A", 2, 0), ("B", 1, 0), ("FP", 0, 2), ("FP2", 0, 1)]);
}

#[test]
fn a_package_file_broken_after_loading_gives_no_packages_and_says_why() {
    let dir = pack("lazy_broken");
    let lib = Library::load_dir(dir.path()).unwrap();
    assert!(lib.package("P1").is_some(), "FP read now");
    dir.write("fp2.p.json", "{ not json");
    assert!(lib.package("P3").is_none());
    let failures = lib.package_failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].0, "FP2");
    assert!(lib.package("P1").is_some(), "already read packages stay");
    assert!(lib.get("R3").is_some(), "devices are unaffected");
}

#[test]
fn a_package_file_changed_after_loading_keeps_only_valid_packages() {
    let dir = pack("lazy_changed");
    let lib = Library::load_dir(dir.path()).unwrap();
    let mut dup = two_pad("P1", "");
    dup.pads.push(dup.pads[0].clone());
    dir.write("fp.p.json", &package_file(&[dup, two_pad("P2", "")]));
    assert!(lib.package("P1").is_none(), "a pad repeated: left out");
    assert!(lib.package("P2").is_some());
}

#[test]
fn equal_symbols_and_pin_maps_are_shared_after_loading() {
    let dir = pack("lazy_shared");
    let lib = Library::load_dir(dir.path()).unwrap();
    let (r1, r2, r3) = (lib.get("R1").unwrap(), lib.get("R2").unwrap(), lib.get("R3").unwrap());
    assert!(Arc::ptr_eq(&r1.symbol, &r2.symbol) && Arc::ptr_eq(&r1.symbol, &r3.symbol));
    assert!(Arc::ptr_eq(&r1.packages[0].pin_map, &r3.packages[0].pin_map));
}

#[test]
fn a_pack_laid_over_the_builtin_parts_replaces_clashing_names() {
    let dir = Scratch::new("overlay");
    dir.write("index.json", &index(&[("PACK", "p.json", 1, Some("p.p.json"))]));
    dir.write("p.json", &device_file(&[two_pin("RES", "", "RES40")]));
    dir.write("p.p.json", &package_file(&[two_pad("RES40", "")]));
    let builtin = Library::builtin();
    let builtin_count = builtin.len();
    let lib = builtin.overlay(Library::load_dir(dir.path()).unwrap());
    assert_eq!(lib.get("res").unwrap().library, "PACK");
    assert_eq!(lib.package("RES40").unwrap().library, "PACK");
    assert!(lib.issues().is_empty(), "{:?}", lib.issues());
    assert_eq!(lib.libraries()[0].name, protasena_library::BUILTIN_LIBRARY, "built-in library first");
    assert!(lib.len() < builtin_count + 1, "RES replaced, devices on RES40 dropped");
    assert_eq!(lib.package_names().filter(|(n, _)| *n == "RES40").count(), 1);
}
