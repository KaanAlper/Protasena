//! Reading synthetic projects written in the observed structure.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::dsn::{TESTRES, UNIT, sample_design, th};
use protasena_pdsprj::{Component, Coord, Generator, Project, Record, Terminal, UNITS_PER_THOU, Wire, parse_design, record_offset};

fn c((x, y): (i32, i32)) -> Coord {
    Coord { x, y }
}

fn project() -> Project {
    Project::from_bytes(common::archive(&sample_design())).unwrap()
}

#[test]
fn reads_the_manifest_without_unknown_attributes() {
    let p = project();
    let names: Vec<_> = p.members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["PROJECT.XML", "ROOT.CDB", "ROOT.DSN", "SCRIPTS/RAILS.DAT"]);
    let m = p.manifest.as_ref().unwrap();
    assert_eq!((m.release, m.file_version, m.created, m.modified), (Some(901), Some(902), Some(1000), Some(2000)));
    assert_eq!(m.scripts, ["RAILS.DAT"]);
    assert!(!format!("{p:?}").contains("must-not-appear"));
}

#[test]
fn reads_the_header_styles_and_embedded_library() {
    let d = project().design;
    assert_eq!((d.header.release, d.header.file_version, d.header.created, d.header.modified), (901, 902, 1000, 2000));
    assert_eq!((d.header.created_date.as_str(), d.header.modified_date.as_str()), ("01/02/03", "04/05/06"));
    let config = d.config.as_ref().unwrap();
    assert_eq!(config.graphic_styles, common::dsn::GRAPHIC_STYLES);
    assert_eq!(config.text_styles, common::dsn::TEXT_STYLES);
    assert_eq!(config.symbols.len(), 1);
    assert_eq!((config.symbols[0].name.as_str(), config.symbols[0].bytes_len), ("$TESTSYM", 40));
    let dev = &config.devices[0];
    assert_eq!(dev.name, "TESTRES");
    let names: Vec<_> = dev.sections.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["DEVICE", "PROPDEFS", "INDEX", "COMPONENT"]);
    assert_eq!(dev.section("device").unwrap().get("PREFIX"), Some("X"));
    assert_eq!(dev.section("PROPDEFS").unwrap().records[0].value, "\"Test rating\",STRING");
    assert_eq!(dev.section("INDEX").unwrap().get("CAT"), Some("Test parts"));
    assert!(TESTRES.starts_with("{*DEVICE}"));
}

#[test]
fn reads_the_circuit_records() {
    let d = project().design;
    assert_eq!(d.circuits.len(), 2);
    let sheet = &d.circuits[0];
    assert_eq!((sheet.sheet_width, sheet.sheet_height), ((10_000 * UNIT) as u32, (7_000 * UNIT) as u32));
    assert!(sheet.ended);
    assert_eq!(sheet.trailer_len, 0);
    let wires: Vec<&Wire> = sheet.records.iter().filter_map(|r| if let Record::Wire(w) = r { Some(w) } else { None }).collect();
    assert_eq!(wires.len(), 4);
    assert_eq!(wires[0].points, [c(th(-100, 0)), c(th(-100, 100)), c(th(0, 100))]);
    assert_eq!(wires[3].style, "BUS WIRE");
    assert_eq!(wires[0].points[1].to_thou(), (-100.0, 100.0));
    assert_eq!(UNITS_PER_THOU, UNIT);

    let Record::Node(node) = &sheet.records[3] else { panic!("node expected: {:?}", sheet.records[3]) };
    assert_eq!((node.at, node.degree()), (c(th(0, 100)), 3));
    assert!(node.links[..3].iter().all(|&l| matches!(sheet.record_at(l), Some(Record::Wire(_)))));

    let comps: Vec<&Component> = sheet.records.iter().filter_map(|r| if let Record::Component(c) = r { Some(c) } else { None }).collect();
    assert_eq!(comps.len(), 2);
    let x1 = comps[0];
    assert_eq!((x1.reference(), x1.value(), x1.device.as_str(), x1.at, x1.ordinal), ("X1", "4k7", "TESTRES", c(th(-100, -100)), 3));
    assert_eq!(x1.properties(), [("RATING".to_owned(), "0.5W".to_owned()), ("CODE".to_owned(), "AB".to_owned())]);
    assert_eq!(x1.pins, [wires[0].offset, 0]);
    assert_eq!(x1.field("COMPONENT ID").unwrap().at, c(th(-100, 100)));
    assert!(!x1.field("SUBCKT NAME").unwrap().visible);
    assert!(comps[1].properties().is_empty());

    // The unknown record between the components is kept as tag and size only.
    let unknown: Vec<_> = sheet.records.iter().filter(|r| matches!(r, Record::Unknown { .. })).collect();
    assert_eq!(unknown, [&Record::Unknown { offset: record_offset(&sheet.records[5]), tag: 0x28, bytes_len: 301 }]);
    assert_eq!(d.unknown_count(), 1);

    let Some(Record::Terminal(Terminal { symbol, link, .. })) = sheet.records.iter().find(|r| matches!(r, Record::Terminal(_))) else { panic!() };
    assert_eq!((symbol.as_str(), *link), ("$TERGROUND", wires[2].offset));
    let gens: Vec<&Generator> = sheet.records.iter().filter_map(|r| if let Record::Generator(g) = r { Some(g) } else { None }).collect();
    assert_eq!(
        gens.iter().map(|g| (g.tag, g.symbol.as_str(), g.fields[0].text.as_str())).collect::<Vec<_>>(),
        [(0x38, "$TESTGEN", "Vtest"), (0x30, "$TESTPROBE", "P1")]
    );
    assert_eq!(gens[0].links, [wires[1].offset, 0]);

    let second = &d.circuits[1];
    assert!(second.records.is_empty() && second.ended);
    assert_eq!(second.trailer_len, 50);
}

#[test]
fn a_design_without_configuration_still_yields_its_circuits() {
    let mut dsn = common::dsn::Dsn::new(&[]);
    dsn.buf.truncate(0x104);
    dsn.buf.extend(b"UNEXPECTED SECTION\x1a");
    dsn.circuit();
    dsn.wire("WIRE", &[th(0, 0), th(100, 0)]);
    dsn.end(&[]);
    let d = parse_design(&dsn.buf).unwrap();
    assert!(d.config.is_none());
    assert_eq!(d.circuits[0].records.len(), 1);
}

#[test]
fn a_circuit_without_its_object_mark_is_one_unknown_record() {
    let mut dsn = common::dsn::Dsn::new(&[]);
    dsn.buf.extend(b"ISIS CIRCUIT FILE\x1a");
    dsn.buf.extend([1u8; 40]);
    let d = parse_design(&dsn.buf).unwrap();
    assert!(matches!(d.circuits[0].records[..], [Record::Unknown { tag: 1, bytes_len: 40, .. }]));
}

#[test]
fn unknown_records_at_the_end_run_to_the_end_of_the_objects() {
    let mut dsn = common::dsn::Dsn::new(&[]);
    dsn.circuit();
    dsn.wire("WIRE", &[th(0, 0), th(100, 0)]);
    dsn.opaque(0x77, 64);
    let d = parse_design(&dsn.buf).unwrap();
    let circuit = &d.circuits[0];
    assert!(!circuit.ended);
    assert!(matches!(circuit.records[..], [Record::Wire(_), Record::Unknown { tag: 0x77, bytes_len: 65, .. }]));
}

#[test]
fn rejects_data_that_is_not_a_schematic() {
    assert!(parse_design(b"").is_err());
    assert!(parse_design(&[0u8; 400]).is_err());
    let no_design = common::zip::zip(&[common::zip::Entry::new("PROJECT.XML", b"<PROJECT/>", false)], b"");
    assert!(matches!(Project::from_bytes(no_design), Err(protasena_pdsprj::Error::MissingMember(_))));
}

#[test]
fn opens_files_from_disk() {
    let dir = std::env::temp_dir().join(format!("protasena-pdsprj-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.pdsprj");
    std::fs::write(&path, common::archive(&sample_design())).unwrap();
    let p = Project::open(&path).unwrap();
    assert_eq!(p.design.circuits.len(), 2);
    assert!(matches!(Project::open(&dir.join("missing.pdsprj")), Err(protasena_pdsprj::Error::Io { .. })));
    std::fs::remove_dir_all(&dir).unwrap();
}
