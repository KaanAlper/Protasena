//! Pack loading: format and version checks, and validation problems reported as a list.
// Test code: helpers outside `#[test]` functions may panic too (clippy.toml covers only those).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::{Scratch, device_file, index, package_file, two_pad, two_pin};
use protasena_library::{Device, IssueKind, Library, LibraryError, PropDef, PropKind};

/// Loads a one-library pack ("LIB") and returns the issue kinds.
fn issues_of(devices: &[Device], packages: &[protasena_library::Package]) -> Vec<IssueKind> {
    let dir = Scratch::new("issues");
    dir.write("index.json", &index(&[("LIB", "lib.json", devices.len() as u64, Some("pk.json"))]));
    dir.write("lib.json", &device_file(devices));
    dir.write("pk.json", &package_file(packages));
    match Library::load_dir(dir.path()) {
        Err(LibraryError::Invalid(issues)) => issues.into_iter().map(|i| i.kind).collect(),
        other => panic!("expected validation issues, got {other:?}"),
    }
}

#[test]
fn a_consistent_pack_loads_and_fills_in_library_names() {
    let dir = Scratch::new("ok");
    let dev = Device { library: String::new(), ..two_pin("R", "", "P2") };
    let pkg = protasena_library::Package { library: String::new(), ..two_pad("P2", "") };
    dir.write("index.json", &index(&[("LIB", "lib.json", 1, Some("pk.json"))]));
    dir.write("lib.json", &device_file(&[dev]));
    dir.write("pk.json", &package_file(&[pkg]));
    let lib = Library::load_dir(dir.path()).unwrap();
    assert_eq!(lib.get("r").unwrap().library, "LIB");
    assert_eq!(lib.package("P2").unwrap().library, "LIB");
}

#[test]
fn every_problem_is_listed_not_just_the_first() {
    let mut bad_map = two_pin("MAP", "LIB", "P2");
    bad_map.packages[0].pin_map = vec![("1".into(), "9".into()), ("7".into(), "2".into()), ("1".into(), "1".into())].into();
    let kinds =
        issues_of(&[two_pin("DUP", "LIB", "P2"), two_pin("dup", "LIB", "P2"), two_pin("DANGLE", "LIB", "NOPE"), bad_map], &[two_pad("P2", "LIB")]);
    assert!(kinds.contains(&IssueKind::DuplicateDevice { first_library: "LIB".into() }), "{kinds:?}");
    assert!(kinds.contains(&IssueKind::DanglingPackage("NOPE".into())));
    assert!(kinds.contains(&IssueKind::PinMapMissingPad { package: "P2".into(), pad: "9".into() }));
    assert!(kinds.contains(&IssueKind::PinMapMissingPin { package: "P2".into(), pin: "7".into() }));
    assert!(kinds.contains(&IssueKind::PinMappedTwice { package: "P2".into(), pin: "1".into() }));
    assert_eq!(kinds.len(), 5, "{kinds:?}");
}

#[test]
fn device_level_problems() {
    let mut d = two_pin("D", "LIB", "P2");
    d.default_package = Some("OTHER".into());
    d.packages.push(d.packages[0].clone());
    d.prop_defs = vec![
        PropDef::new("MODE", "Mode", PropKind::Keywords { choices: vec!["A".into()] }, Some("B")),
        PropDef::new("mode", "Mode again", PropKind::String, None),
        PropDef::new("", "No name", PropKind::String, None),
    ];
    std::sync::Arc::make_mut(&mut d.symbol).units[0].pins[1].number = "1".into();
    let mut no_units = two_pin("EMPTY", "LIB", "P2");
    std::sync::Arc::make_mut(&mut no_units.symbol).units.clear();
    no_units.packages.clear();
    no_units.default_package = None;
    let kinds = issues_of(&[d, no_units], &[two_pad("P2", "LIB"), two_pad("P2", "LIB")]);
    for want in [
        IssueKind::BadDefaultPackage("OTHER".into()),
        IssueKind::PackageListedTwice("P2".into()),
        IssueKind::BadKeywordDefault { prop: "MODE".into(), value: "B".into() },
        IssueKind::DuplicatePropDef("mode".into()),
        IssueKind::EmptyName("property"),
        IssueKind::DuplicatePin("1".into()),
        IssueKind::NoUnits,
        IssueKind::DuplicatePackage { first_library: "LIB".into() },
    ] {
        assert!(kinds.contains(&want), "missing {want:?} in {kinds:?}");
    }
}

#[test]
fn index_level_problems() {
    let dir = Scratch::new("index-problems");
    dir.write(
        "index.json",
        &index(&[
            ("A", "a.json", 5, None),
            ("a", "a.json", 0, None),
            ("B", "../b.json", 0, None),
            ("C", "/etc/c.json", 0, None),
            ("", "e.json", 0, None),
        ]),
    );
    dir.write("a.json", &device_file(&[two_pin("X", "OTHER", "P2")]));
    let Err(LibraryError::Invalid(issues)) = Library::load_dir(dir.path()) else { panic!("expected issues") };
    let kinds: Vec<IssueKind> = issues.into_iter().map(|i| i.kind).collect();
    for want in [
        IssueKind::CountMismatch { index: 5, file: 1 },
        IssueKind::LibraryMismatch("OTHER".into()),
        IssueKind::DuplicateLibrary,
        IssueKind::BadPath("../b.json".into()),
        IssueKind::BadPath("/etc/c.json".into()),
        IssueKind::EmptyName("library"),
        IssueKind::DanglingPackage("P2".into()),
    ] {
        assert!(kinds.contains(&want), "missing {want:?} in {kinds:?}");
    }
}

#[test]
fn format_tag_and_version_are_checked() {
    let dir = Scratch::new("version");
    let cases = [
        r#"{"format":"protasena-library","version":2,"libraries":[{"weird":"new shape"}]}"#,
        r#"{"format":"something-else","version":1,"libraries":[]}"#,
        r#"{"format":"protasena-library","libraries":[]}"#,
        r#"{"version":1,"libraries":[]}"#,
        r#"{"format":"protasena-library","version":"1"}"#,
    ];
    for text in cases {
        dir.write("index.json", text);
        let err = Library::load_dir(dir.path()).unwrap_err();
        assert!(matches!(err, LibraryError::Format { .. }), "{text}: {err:?}");
    }
    dir.write("index.json", &index(&[("L", "l.json", 0, None)]));
    dir.write("l.json", r#"{"format":"protasena-library-devices","version":7,"devices":[]}"#);
    let err = Library::load_dir(dir.path()).unwrap_err();
    assert!(err.to_string().contains("version 7 is not supported"), "{err}");
}

#[test]
fn missing_files_are_io_errors_naming_the_file() {
    let dir = Scratch::new("missing");
    assert!(matches!(Library::load_dir(dir.path()), Err(LibraryError::Io { .. })));
    dir.write("index.json", &index(&[("L", "gone.json", 0, None)]));
    let err = Library::load_dir(dir.path()).unwrap_err();
    assert!(matches!(&err, LibraryError::Io { path, .. } if path.ends_with("gone.json")), "{err:?}");
}

#[test]
fn unknown_fields_are_ignored() {
    let dir = Scratch::new("unknown-fields");
    dir.write(
        "index.json",
        r#"{"format":"protasena-library","version":1,"generator":"x","libraries":[{"name":"L","file":"l.json","device_count":0,"extra":1}]}"#,
    );
    dir.write("l.json", r#"{"format":"protasena-library-devices","version":1,"devices":[],"notes":{"a":[1,2]}}"#);
    assert!(Library::load_dir(dir.path()).unwrap().is_empty());
}

#[test]
fn errors_display_their_location() {
    let kinds = issues_of(&[two_pin("R", "LIB", "NOPE")], &[]);
    assert_eq!(kinds, [IssueKind::DanglingPackage("NOPE".into())]);
    let err = Library::from_parts(vec![two_pin("R", "LIB", "NOPE")], vec![]).unwrap_err();
    assert_eq!(err.to_string(), "1 problem(s) in the library\n  LIB/R: package NOPE does not exist");
}
