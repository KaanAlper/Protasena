//! `Library::with_extra`: libraries added on top of the pack (the user's imported KiCad
//! libraries and the writable user libraries) keep their own names, may repeat device names of
//! other libraries, and never replace what is already there.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::{two_pad, two_pin};
use protasena_library::{Device, IssueKind, Library};

fn base() -> Library {
    Library::from_parts(vec![two_pin("R", "PACK", "P2")], vec![two_pad("P2", "PACK")]).unwrap()
}

#[test]
fn extra_devices_join_under_their_own_library_and_may_repeat_names() {
    let extra =
        Library::from_parts(vec![two_pin("R", "MINE", "P2X"), two_pin("NEW", "MINE", "P2")], vec![two_pad("P2X", "MINE.pretty")]).unwrap_err();
    // `NEW` uses the base's package: on its own the extra set does not validate…
    assert!(matches!(extra, protasena_library::LibraryError::Invalid(_)));
    // …but added to the base it does.
    let extra_devices = vec![two_pin("R", "MINE", "P2X"), two_pin("NEW", "MINE", "P2")];
    let (lib, issues) = base().with_extra(extra_devices, vec![two_pad("P2X", "MINE.pretty")]);
    assert!(issues.is_empty(), "{issues:?}");
    assert_eq!(lib.len(), 3);
    // A plain lookup finds the base's device first; both are listed by search.
    assert_eq!(lib.get("r").unwrap().library, "PACK");
    let rs: Vec<&str> = lib.search(&protasena_library::Query::keywords("")).iter().filter(|d| d.name == "R").map(|d| d.library.as_str()).collect();
    assert_eq!(rs, ["MINE", "PACK"]);
    assert_eq!(lib.get("new").unwrap().library, "MINE");
    assert!(lib.package("P2X").is_some());
    let names: Vec<&str> = lib.libraries().iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["PACK", "MINE", "MINE.pretty"]);
}

#[test]
fn clashes_and_broken_items_are_left_out_and_reported() {
    let devices: Vec<Device> = vec![
        two_pin("R", "PACK", "P2"),    // a library already loaded
        two_pin("A", "MINE", "NOPE"),  // no such package
        two_pin("B", "MINE", "P2"),    // fine
        two_pin("b", "MINE", "P2"),    // B again in the same library
        two_pin("C", "MINE", "P2"),    // the base's package
        two_pin("D", "MINE", "P2DUP"), // its package clashes with nothing, but is dropped as broken
    ];
    let mut broken = two_pad("P2DUP", "MINE.pretty");
    broken.pads.push(broken.pads[0].clone());
    let packages = vec![two_pad("P2", "MINE.pretty"), broken];
    let (lib, issues) = base().with_extra(devices, packages);
    let kinds: Vec<(Option<&str>, &IssueKind)> = issues.iter().map(|i| (i.item.as_deref(), &i.kind)).collect();
    assert!(kinds.iter().any(|(i, k)| *i == Some("P2") && matches!(k, IssueKind::DuplicatePackage { .. })), "{issues:?}");
    assert!(kinds.iter().any(|(i, k)| *i == Some("P2DUP") && matches!(k, IssueKind::DuplicatePad(_))), "{issues:?}");
    assert!(kinds.iter().any(|(i, k)| *i == Some("R") && matches!(k, IssueKind::LibraryTaken)), "{issues:?}");
    assert!(kinds.iter().any(|(i, k)| *i == Some("A") && matches!(k, IssueKind::DanglingPackage(_))), "{issues:?}");
    assert!(kinds.iter().any(|(i, k)| *i == Some("b") && matches!(k, IssueKind::DuplicateDevice { .. })), "{issues:?}");
    assert!(kinds.iter().any(|(i, k)| *i == Some("D") && matches!(k, IssueKind::DanglingPackage(_))), "{issues:?}");
    let mine: Vec<&str> = lib.devices().iter().filter(|d| d.library == "MINE").map(|d| d.name.as_str()).collect();
    assert_eq!(mine, ["B", "C"]);
    assert_eq!(lib.package("P2").unwrap().library, "PACK", "the base's package is kept");
    assert!(lib.package("P2DUP").is_none());
    assert!(lib.issues().iter().all(|i| matches!(i.kind, IssueKind::DuplicateDevice { .. })), "only the allowed repeats: {:?}", lib.issues());
}

#[test]
fn without_extra_gives_the_library_back() {
    let (lib, issues) = base().with_extra(vec![two_pin("R", "MINE", "P2X")], vec![two_pad("P2X", "MINE.pretty")]);
    assert!(issues.is_empty());
    assert_eq!(lib.extra_libraries(), ["MINE", "MINE.pretty"]);
    let (lib, _) = lib.with_extra(vec![two_pin("S", "USER", "P2")], Vec::new());
    assert_eq!(lib.extra_libraries(), ["MINE", "MINE.pretty", "USER"]);
    let back = lib.without_extra();
    assert_eq!(back.len(), 1);
    assert!(back.package("P2X").is_none());
    assert_eq!(back.libraries().len(), 1);
    assert!(back.extra_libraries().is_empty());
    assert_eq!(back.get("r").unwrap().library, "PACK");
}

#[test]
fn nothing_extra_changes_nothing() {
    let (lib, issues) = base().with_extra(Vec::new(), Vec::new());
    assert!(issues.is_empty());
    assert_eq!(lib.len(), 1);
    assert_eq!(lib.libraries().len(), 1);
}
