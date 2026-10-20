//! `.psproj` save/load: round trip, determinism, versions and load reports.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::{sample_project, scratch};
use protasena_doc::{DocError, Project};
use protasena_io::{FORMAT, LoadError, SaveError, VERSION, backup_path, decode, encode, load, save};

#[test]
fn save_then_load_gives_the_same_project() {
    let dir = scratch("roundtrip");
    let path = dir.join("divider.psproj");
    let p = sample_project();
    save(&p, &path).unwrap();
    let (back, report) = load(&path).unwrap();
    assert_eq!(back, p);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(report.version, VERSION);
}

#[test]
fn output_is_deterministic_and_stable_across_round_trips() {
    let dir = scratch("determinism");
    let (a, b) = (dir.join("a.psproj"), dir.join("b.psproj"));
    let p = sample_project();
    save(&p, &a).unwrap();
    save(&p, &b).unwrap();
    let bytes = std::fs::read(&a).unwrap();
    assert_eq!(bytes, std::fs::read(&b).unwrap());
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(encode(&back).unwrap().into_bytes(), bytes);
}

#[test]
fn file_is_tagged_pretty_and_ends_with_a_newline() {
    let text = encode(&sample_project()).unwrap();
    let head = format!("{{\n  \"format\": \"{FORMAT}\",\n  \"version\": {VERSION},\n  \"project\": {{\n    \"name\": \"divider\",");
    assert!(text.starts_with(&head), "{}", &text[..200]);
    assert!(text.ends_with("}\n"));
    // Floats are written so they read back bit for bit.
    assert!(text.contains("0.30000000000000004"), "{text}");
    assert!(text.contains("1e-7") || text.contains("1e-07"), "{text}");
}

#[test]
fn the_list_order_of_the_model_is_kept() {
    let p = sample_project();
    let (back, _) = decode(encode(&p).unwrap().as_bytes()).unwrap();
    // The wire inserted at index 0 has the larger id and stays first.
    assert!(back.sheets[0].wires[0].id.0 > back.sheets[0].wires[1].id.0);
}

#[test]
fn unknown_fields_are_ignored() {
    let text = encode(&sample_project()).unwrap();
    let extended = text.replacen("\"version\": 1,", "\"version\": 1,\n  \"written_by\": {\"app\": \"future\"},", 1).replacen(
        "\"name\": \"divider\",",
        "\"name\": \"divider\",\n    \"colour\": [1, 2, 3],",
        1,
    );
    let (back, report) = decode(extended.as_bytes()).unwrap();
    assert_eq!(back, sample_project());
    assert!(report.warnings.is_empty());
}

#[test]
fn newer_versions_are_refused_with_a_clear_error() {
    let text = encode(&sample_project()).unwrap().replacen("\"version\": 1", "\"version\": 2", 1);
    let err = decode(text.as_bytes()).unwrap_err();
    assert_eq!(err, LoadError::UnsupportedVersion { found: 2, supported: VERSION });
    assert!(err.to_string().contains("newer"), "{err}");
    let zero = decode(br#"{"format": "protasena-project", "version": 0, "project": {}}"#).unwrap_err();
    assert!(matches!(zero, LoadError::UnsupportedVersion { found: 0, .. }));
}

#[test]
fn other_files_are_not_projects() {
    for text in [
        r#"{"format": "protasena-netlist", "version": 1}"#,
        r#"{"version": 1, "project": {}}"#,
        r#"{"format": 7, "version": 1}"#,
        r#"{"format": "protasena-project", "project": {}}"#,
        r#"{"format": "protasena-project", "version": 1.5, "project": {}}"#,
        r#"{"format": "protasena-project", "version": -1, "project": {}}"#,
        r#"[1, 2, 3]"#,
        r#""protasena-project""#,
    ] {
        assert!(matches!(decode(text.as_bytes()), Err(LoadError::NotAProject(_))), "{text}");
    }
    let missing = decode(br#"{"format": "protasena-project", "version": 1}"#).unwrap_err();
    assert!(matches!(missing, LoadError::NotAProject(_)), "{missing:?}");
}

#[test]
fn inconsistencies_are_warnings_and_duplicate_ids_are_errors() {
    let mut p = sample_project();
    p.sheets[1].wires.push(protasena_doc::Wire { id: protasena_doc::WireId(10_000), points: vec![] });
    p.next_id = 1;
    let (back, report) = decode(encode(&p).unwrap().as_bytes()).unwrap();
    assert_eq!(back, p);
    assert_eq!(report.warnings.len(), 2, "{:?}", report.warnings);

    let mut dup = sample_project();
    let w = dup.sheets[0].wires[0].clone();
    dup.sheets[1].wires.push(w);
    match decode(encode(&dup).unwrap().as_bytes()) {
        Err(LoadError::Invalid(errors)) => assert!(matches!(errors[0], DocError::DuplicateItem(_))),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_project_without_sheets_loads_with_a_warning() {
    let (p, report) = decode(br#"{"format": "protasena-project", "version": 1, "project": {}}"#).unwrap();
    assert_eq!(p, Project::default());
    assert_eq!(report.warnings.len(), 1);
}

#[test]
fn non_finite_numbers_are_not_saved() {
    let dir = scratch("non-finite");
    let path = dir.join("x.psproj");
    let mut p = sample_project();
    p.sheets[0].junctions[0].at.x = f64::NAN;
    assert_eq!(encode(&p).unwrap_err(), SaveError::NonFinite("junction position"));
    assert!(save(&p, &path).is_err());
    assert!(!path.exists());
}

#[test]
fn saving_over_a_project_keeps_a_backup() {
    let dir = scratch("backup");
    let path = dir.join("design.psproj");
    let bak = backup_path(&path);
    assert_eq!(bak, dir.join("design.psproj.bak"));
    let first = sample_project();
    save(&first, &path).unwrap();
    assert!(!bak.exists(), "no backup of a file that did not exist");
    let mut second = first.clone();
    second.name = "second".into();
    save(&second, &path).unwrap();
    assert_eq!(load(&bak).unwrap().0, first);
    assert_eq!(load(&path).unwrap().0, second);
    let mut third = second.clone();
    third.name = "third".into();
    save(&third, &path).unwrap();
    assert_eq!(load(&bak).unwrap().0, second);
    assert_eq!(load(&path).unwrap().0, third);
    // Only the project and its backup are left: no temp files.
    let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name()).collect();
    names.sort();
    assert_eq!(names, ["design.psproj", "design.psproj.bak"]);
}

#[test]
fn load_reports_missing_files_with_the_path() {
    let dir = scratch("missing");
    let path = dir.join("nope.psproj");
    match load(&path) {
        Err(LoadError::Io { path: p, .. }) => assert_eq!(p, path),
        other => panic!("{other:?}"),
    }
    assert!(load(&dir).is_err(), "a directory is not a project");
}
