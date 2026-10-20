//! Importing synthetic `.pdsprj` files (written by the `protasena-pdsprj` test writer: the
//! observed structure, our own content).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

#[path = "../../pdsprj/tests/common/mod.rs"]
mod pds;

use pds::dsn::{Dsn, TESTRES, sample_design, th};
use protasena_doc::{DeviceRef, SheetSize, TerminalKind};
use protasena_geom::{Pt, Vec2};
use protasena_io::{EmbeddedDevice, ImportReport, PDSPRJ_LIBRARY, UnknownRecords, decode, encode, import_pdsprj, import_pdsprj_bytes};

fn import(design: &[u8]) -> (protasena_doc::Project, ImportReport) {
    import_pdsprj_bytes(pds::archive(design), "demo").unwrap()
}

#[test]
fn maps_wires_components_junctions_and_terminals() {
    let (project, report) = import(&sample_design());
    assert_eq!(project.name, "demo");
    assert_eq!(project.sheets.len(), 1, "the circuit without objects is not a sheet");
    let sheet = &project.sheets[0];
    assert_eq!(sheet.size, SheetSize::Custom { width: 10_000.0, height: 7_000.0 });
    assert_eq!(sheet.wires.len(), 3);
    assert_eq!(sheet.wires[0].points, [Pt::new(-100.0, 0.0), Pt::new(-100.0, 100.0), Pt::new(0.0, 100.0)]);
    assert_eq!(sheet.buses.len(), 1);
    assert_eq!(sheet.junctions.len(), 1);
    assert_eq!(sheet.junctions[0].at, Pt::new(0.0, 100.0));

    let x1 = &sheet.components[0];
    assert_eq!(x1.device, DeviceRef::new(PDSPRJ_LIBRARY, "TESTRES"));
    assert_eq!((x1.reference.as_str(), x1.value.as_str()), ("X1", "4k7"));
    assert_eq!(x1.origin(), Pt::new(-100.0, -100.0));
    assert_eq!(x1.properties.iter().collect::<Vec<_>>(), [("RATING", "0.5W"), ("CODE", "AB")]);
    assert_eq!(x1.reference_label.offset, Vec2::new(0.0, 200.0));
    assert_eq!(x1.value_label.offset, Vec2::new(0.0, -200.0));
    assert_eq!(sheet.components[1].reference, "X2");

    let t = &sheet.terminals[0];
    assert_eq!((t.kind, t.at, t.name.as_str()), (TerminalKind::Ground, Pt::new(0.0, 300.0), ""));
    assert!(project.issues().is_empty());

    assert_eq!(report.members.len(), 4);
    assert_eq!(report.release, Some(901));
    assert_eq!((report.sheets, report.components, report.wires, report.buses, report.junctions, report.terminals), (1, 2, 3, 1, 1, 1));
    assert_eq!((report.skipped_generators, report.skipped_nodes), (2, 0));
    assert_eq!(report.unknown_records, [UnknownRecords { tag: 0x28, count: 1, bytes: 301 }]);
    assert_eq!(report.embedded_devices, [EmbeddedDevice { name: "TESTRES".into(), category: Some("Test parts".into()), property_defs: 1 }]);
    assert_eq!(report.embedded_symbols, 1);
    assert!(report.warnings.iter().any(|w| w.contains("orientation is not decoded")));
    assert!(report.warnings.iter().any(|w| w.contains("0x28")));
    assert!(TESTRES.contains("Test parts"));
}

#[test]
fn imported_projects_save_and_load() {
    let (project, _) = import(&sample_design());
    let text = encode(&project).unwrap();
    let (back, report) = decode(text.as_bytes()).unwrap();
    assert_eq!(back, project);
    assert!(report.warnings.is_empty());
}

#[test]
fn package_property_becomes_the_package() {
    let mut d = Dsn::new(&[]);
    d.circuit();
    let w = d.wire("WIRE", &[th(0, 0), th(100, 0)]);
    d.node(th(0, 0), [w, 0, 0, 0]);
    d.component("X9", "", "TESTCAP", th(0, 0), "{PACKAGE=OWNPKG}\n{TOL=5%}\n", &[w]);
    d.terminal(th(100, 0), "$TERPOWER", "VTEST", w);
    d.end(&[]);
    let (project, report) = import(&d.buf);
    let c = &project.sheets[0].components[0];
    assert_eq!(c.package.as_deref(), Some("OWNPKG"));
    assert_eq!(c.properties.iter().collect::<Vec<_>>(), [("TOL", "5%")]);
    let t = &project.sheets[0].terminals[0];
    assert_eq!((t.kind, t.name.as_str()), (TerminalKind::Power, "VTEST"));
    assert_eq!((report.junctions, report.skipped_nodes), (0, 1));
}

#[test]
fn a_design_without_circuits_gives_one_empty_sheet() {
    let (project, report) = import(&Dsn::new(&[]).buf);
    assert_eq!(project.sheets.len(), 1);
    assert!(project.issues().is_empty());
    assert!(report.warnings.iter().any(|w| w.contains("no circuit section")));
}

#[test]
fn reads_files_and_reports_errors() {
    let dir = std::env::temp_dir().join(format!("protasena-io-pdsprj-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("board one.pdsprj");
    std::fs::write(&path, pds::archive(&sample_design())).unwrap();
    let (project, _) = import_pdsprj(&path).unwrap();
    assert_eq!(project.name, "board one");
    std::fs::write(&path, b"not an archive").unwrap();
    let err = import_pdsprj(&path).unwrap_err();
    assert!(err.to_string().starts_with("cannot import the Proteus project"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn hostile_pdsprj_never_panics() {
    let base = pds::archive(&sample_design());
    let stored = pds::zip::zip(&[pds::zip::Entry::new("ROOT.DSN", &sample_design(), false)], b"");
    let mut rng = pds::Rng(0x5DEE_CE66_D1CE_4E5B);
    for _ in 0..1500 {
        for src in [&base, &stored] {
            let mut b = src.clone();
            for _ in 0..=rng.below(6) {
                let at = rng.below(b.len());
                b[at] = rng.next() as u8;
            }
            if let Ok((project, _)) = import_pdsprj_bytes(b, "x") {
                assert!(!project.sheets.is_empty());
            }
        }
    }
}
