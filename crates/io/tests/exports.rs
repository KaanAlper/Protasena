//! Netlist export files and the library pack location.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use std::ffi::OsString;

use common::{sample_project, scratch};
use protasena_io::{ExportError, NetlistFormat, export_netlist, library_dir_from, load_library_from, merge_libraries};
use protasena_library::{BUILTIN_LIBRARY, Library, LibraryError};

#[test]
fn json_netlist_file_reads_back() {
    let dir = scratch("netlist-json");
    let path = dir.join("divider.json");
    let p = sample_project();
    let lib = Library::builtin();
    let netlist = export_netlist(&p, &lib, &path, NetlistFormat::Json).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let (back, design) = protasena_netlist::export::from_json(&text).unwrap();
    assert_eq!(design, "divider");
    assert_eq!(back, netlist);
    assert_eq!(netlist, protasena_netlist::build(&p, &lib));
    assert!(netlist.net_of("R1", "2").is_some());
}

#[test]
fn spice_netlist_file_matches_the_exporter() {
    let dir = scratch("netlist-spice");
    let path = dir.join("divider.cir");
    let p = sample_project();
    let lib = Library::builtin();
    let netlist = export_netlist(&p, &lib, &path, NetlistFormat::Spice).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), protasena_netlist::export::to_spice(&netlist, "divider"));
    assert_eq!(NetlistFormat::Spice.extension(), "cir");
    assert_eq!(NetlistFormat::Json.extension(), "json");
}

#[test]
fn netlist_export_into_a_missing_directory_is_an_error() {
    let dir = scratch("netlist-missing");
    let path = dir.join("none").join("x.cir");
    let err = export_netlist(&sample_project(), &Library::builtin(), &path, NetlistFormat::Spice).unwrap_err();
    assert!(matches!(&err, ExportError::Io { path: p, .. } if *p == path), "{err:?}");
}

#[test]
fn library_dir_comes_from_the_variable_when_it_is_set() {
    assert_eq!(library_dir_from(None), None);
    assert_eq!(library_dir_from(Some(OsString::new())), None);
    assert_eq!(library_dir_from(Some(OsString::from("/opt/packs"))), Some("/opt/packs".into()));
}

#[test]
fn without_a_pack_the_library_is_the_builtin_one() {
    let lib = load_library_from(None).unwrap();
    assert_eq!(lib.len(), Library::builtin().len());
}

/// A pack with one library holding a renamed copy of RES, and its own `CAP` (clashing).
fn pack() -> Library {
    let builtin = Library::builtin();
    let mut res = builtin.get("RES").unwrap().clone();
    res.name = "MYRES".into();
    res.library = "PACK".into();
    let mut cap = builtin.get("CAP").unwrap().clone();
    cap.library = "PACK".into();
    cap.description = "pack capacitor".into();
    let packages: Vec<_> = builtin
        .packages()
        .iter()
        .filter(|p| res.packages.iter().chain(&cap.packages).any(|r| r.name == p.name))
        .cloned()
        .map(|mut p| {
            p.library = "PACK".into();
            p
        })
        .collect();
    Library::from_parts(vec![res, cap], packages).unwrap()
}

#[test]
fn pack_devices_win_name_clashes_with_builtin_ones() {
    let merged = merge_libraries(&Library::builtin(), &pack()).unwrap();
    assert!(merged.issues().is_empty(), "{:?}", merged.issues());
    assert_eq!(merged.get("CAP").unwrap().library, "PACK");
    assert_eq!(merged.get("CAP").unwrap().description, "pack capacitor");
    assert_eq!(merged.get("MYRES").unwrap().library, "PACK");
    // RES used a package the pack replaced, so it went with it; unrelated builtins stay.
    assert_eq!(merged.get("NPN").unwrap().library, BUILTIN_LIBRARY);
    assert_eq!(merged.libraries()[0].name, BUILTIN_LIBRARY);
    assert!(merged.get("RES").is_none());
}

#[test]
fn a_saved_pack_loads_merged_and_a_broken_one_is_an_error() {
    let dir = scratch("pack");
    pack().save_dir(&dir).unwrap();
    let lib = load_library_from(Some(&dir)).unwrap();
    assert_eq!(lib.get("MYRES").unwrap().library, "PACK");
    assert_eq!(lib.get("GROUND").unwrap().library, BUILTIN_LIBRARY);
    let broken = scratch("pack-broken");
    std::fs::write(broken.join(protasena_library::INDEX_FILE), b"{ not json").unwrap();
    assert!(matches!(load_library_from(Some(&broken)), Err(LibraryError::Json { .. })));
    assert!(load_library_from(Some(&broken.join("missing"))).is_err());
}
