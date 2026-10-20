//! Hostile packs never panic: garbage, deep nesting, huge counts and oversized files all end in
//! an error value.
// Test code: helpers outside `#[test]` functions may panic too (clippy.toml covers only those).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use std::path::Path;

use common::{Scratch, index};
use proptest::prelude::*;
use protasena_library::{IssueKind, Library, LibraryError, limits, parse_devices, parse_index, parse_packages};

fn p() -> &'static Path {
    Path::new("x.json")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn hostile_random_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = parse_index(p(), &bytes);
        let _ = parse_devices(p(), &bytes);
        let _ = parse_packages(p(), &bytes);
    }

    #[test]
    fn hostile_mangled_builtin_never_panics(cut in 0usize..4000, flip in 0usize..4000, byte in any::<u8>()) {
        let text = format!(
            "{{\"format\":\"protasena-library-devices\",\"version\":1,\"devices\":{}}}",
            serde_json::to_string(Library::builtin().devices()).unwrap()
        );
        let mut bytes = text.into_bytes();
        if let Some(b) = bytes.get_mut(flip) { *b = byte; }
        bytes.truncate(cut.max(1) * 4);
        let _ = parse_devices(p(), &bytes);
    }
}

#[test]
fn hostile_deep_nesting_is_an_error() {
    for depth in [200usize, 100_000] {
        let text = format!("{{\"format\":\"protasena-library-devices\",\"version\":1,\"devices\":{}{}}}", "[".repeat(depth), "]".repeat(depth));
        assert!(matches!(parse_devices(p(), text.as_bytes()), Err(LibraryError::Json { .. })));
        let head = format!("{{\"format\":{}{}}}", "[".repeat(depth), "]".repeat(depth));
        assert!(parse_index(p(), head.as_bytes()).is_err());
    }
}

#[test]
fn hostile_huge_counts_are_errors() {
    let dir = Scratch::new("huge-count");
    dir.write("index.json", &index(&[("L", "l.json", u64::MAX, None)]));
    dir.write("l.json", r#"{"format":"protasena-library-devices","version":1,"devices":[]}"#);
    let Err(LibraryError::Invalid(issues)) = Library::load_dir(dir.path()) else { panic!("expected issues") };
    assert_eq!(issues[0].kind, IssueKind::CountMismatch { index: u64::MAX, file: 0 });

    let pins = vec![r#"{"number":"1","at":[0,0],"orient":"up"}"#; limits::MAX_PINS + 1].join(",");
    let text =
        format!(r#"{{"format":"protasena-library-devices","version":1,"devices":[{{"name":"X","symbol":{{"units":[{{"pins":[{pins}]}}]}}}}]}}"#);
    let err = parse_devices(p(), text.as_bytes()).unwrap_err();
    assert!(err.to_string().contains("more than 10000 items"), "{err}");

    let libs = vec![r#"{"name":"L","file":"l.json"}"#; limits::MAX_LIBRARIES + 1].join(",");
    let text = format!(r#"{{"format":"protasena-library","version":1,"libraries":[{libs}]}}"#);
    assert!(matches!(parse_index(p(), text.as_bytes()), Err(LibraryError::Json { .. })));
}

#[test]
fn hostile_numbers_and_types_are_errors() {
    let cases = [
        r#"{"format":"protasena-library-devices","version":1,"devices":[{"name":"X","symbol":{"units":[{"pins":[{"number":"1","at":[1e999,0],"orient":"up"}]}]}}]}"#,
        r#"{"format":"protasena-library-devices","version":1,"devices":[{"name":7}]}"#,
        r#"{"format":"protasena-library-devices","version":1,"devices":{"name":"X"}}"#,
        r#"{"format":"protasena-library-devices","version":1,"devices":[{"name":"X","symbol":{"units":[{"body_style":300}]}}]}"#,
        r#"{"format":"protasena-library-devices","version":18446744073709551616,"devices":[]}"#,
        "\u{feff}{}",
        "",
    ];
    for text in cases {
        assert!(parse_devices(p(), text.as_bytes()).is_err(), "{text}");
    }
}

#[test]
fn hostile_oversized_file_is_rejected_before_reading() {
    let dir = Scratch::new("oversized");
    let file = std::fs::File::create(dir.path().join("index.json")).unwrap();
    // Sparse: takes no disk space.
    file.set_len(limits::MAX_FILE_BYTES + 1).unwrap();
    let err = Library::load_dir(dir.path()).unwrap_err();
    assert!(matches!(err, LibraryError::TooLarge { size, .. } if size == limits::MAX_FILE_BYTES + 1), "{err:?}");
}

#[test]
fn hostile_directory_as_file_is_an_error() {
    let dir = Scratch::new("dir-as-file");
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    dir.write("index.json", &index(&[("L", "sub", 0, None)]));
    assert!(Library::load_dir(dir.path()).is_err());
}
