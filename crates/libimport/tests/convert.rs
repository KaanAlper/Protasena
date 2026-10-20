//! End-to-end conversion of the hand-written fixtures in `tests/data/` (CC0, see
//! ATTRIBUTION.md): device fields, units and pins, packages, both y axes, naming, the report,
//! loading the written pack, and determinism.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::fs;
use std::path::{Path, PathBuf};

use protasena_geom::Pt;
use protasena_libimport::{Map, Options, Report, run};
use protasena_library::{Fill, Library, PadShape, PinDir, PinKind, PropKind, Shape};

fn data(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data").join(p)
}

fn out_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("libconv").join(name);
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn convert(name: &str) -> (Report, Library, PathBuf) {
    convert_with(name, &fs::read_to_string(data("map.toml")).unwrap())
}

fn convert_with(name: &str, map_text: &str) -> (Report, Library, PathBuf) {
    let map = Map::parse(map_text).unwrap();
    let out = out_dir(name);
    let opts = Options { symbols: data("symbols"), footprints: data("footprints"), map, out: out.clone(), jobs: 3, upstream: "fixtures".into() };
    let report = run(&opts).unwrap();
    let lib = Library::load_dir(&out).unwrap();
    (report, lib, out)
}

#[test]
fn converts_fixtures_with_zero_failures() {
    let (report, lib, _) = convert("basic");
    assert_eq!(report.failures, vec![], "{report:#?}");
    let c = &report.counts;
    assert_eq!((c.symbol_libraries, c.symbols, c.derived_symbols, c.devices), (2, 5, 1, 5));
    assert_eq!((c.footprint_libraries, c.footprints, c.packages), (2, 4, 4));
    assert_eq!(c.alternates_dropped, 1);
    assert_eq!(report.unmapped_libraries, ["Extra"]);
    assert_eq!(report.unused_map_devices, ["Passives:Missing"]);
    assert_eq!(report.footprint_links_not_found.len(), 1);
    assert_eq!(report.footprint_links_not_found[0].footprint, "Missing:Nowhere");
    assert!(lib.issues().is_empty());
}

#[test]
fn device_fields_come_from_kicad_properties_and_the_map() {
    let (_, lib, _) = convert("fields");
    let res = lib.get("RES").unwrap();
    assert_eq!((res.library.as_str(), res.prefix.as_str(), res.description.as_str()), ("Passives", "R", "Fixture resistor"));
    assert_eq!((res.category.as_str(), res.sub_category.as_str()), ("Resistors", "Generic"));
    assert_eq!(res.keywords, ["R", "res", "resistor"], "repeats dropped without case");
    let src = res.source.as_ref().unwrap();
    assert_eq!((src.upstream.as_str(), src.lib.as_str(), src.symbol.as_str(), src.licence.as_str()), ("kicad", "Passives", "R", "CC-BY-SA-4.0"));
    let value = res.prop_def("VALUE").unwrap();
    assert_eq!((value.kind.clone(), value.default.as_deref()), (PropKind::String, Some("R")));
    assert_eq!(res.prop_def("PACKAGE").unwrap().default.as_deref(), Some("TestFP:R_Test"));
    assert_eq!(res.prop_def("PRIMITIVE").unwrap().default.as_deref(), Some("ANALOG"));
    let names: Vec<&str> = res.packages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["TestFP:R_Test", "Other_2pad"], "field first, then filters with equal pad sets");
    assert_eq!(res.default_package.as_deref(), Some("TestFP:R_Test"));

    let gnd = lib.get("GROUND").unwrap();
    assert!(gnd.symbol.power && gnd.prefix.is_empty() && gnd.packages.is_empty());
    assert_eq!(gnd.prop_def("VALUE").unwrap().prompt, "Net");

    let extra = lib.get("R_Small").unwrap();
    assert_eq!((extra.library.as_str(), extra.category.as_str(), extra.sub_category.as_str()), ("Extra", "Extra", ""));
    let derived = lib.get("R_Small_Passives").unwrap();
    assert_eq!(derived.description, "Fixture small resistor");
    assert_eq!(derived.symbol.units.len(), 1, "units inherited from R");
    assert_eq!(derived.prop_def("VALUE").unwrap().default.as_deref(), Some("R_Small"));
}

#[test]
fn the_map_default_package_comes_first_with_its_pin_map() {
    let map = fs::read_to_string(data("map.toml")).unwrap().replace(
        r#""Passives:R" = { name = "RES", primitive = "ANALOG" }"#,
        r#""Passives:R" = { name = "RES", primitive = "ANALOG", default_package = "OtherFP:Other_2pad", pin_map = { "1" = "2", "2" = "1" } }"#,
    );
    let (report, lib, _) = convert_with("map_default", &map);
    assert_eq!(report.failures, vec![], "{report:#?}");
    assert_eq!(report.counts.map_default_links, 1);
    let res = lib.get("RES").unwrap();
    let names: Vec<&str> = res.packages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Other_2pad", "TestFP:R_Test"], "map default first, field next, no repeats");
    assert_eq!(res.default_package.as_deref(), Some("Other_2pad"));
    assert_eq!(res.prop_def("PACKAGE").unwrap().default.as_deref(), Some("Other_2pad"));
    assert_eq!(*res.packages[0].pin_map, [("1".into(), "2".into()), ("2".into(), "1".into())]);
}

#[test]
fn a_map_default_package_that_does_not_exist_is_a_failure() {
    let map = fs::read_to_string(data("map.toml"))
        .unwrap()
        .replace(r#""Passives:GND" = { name = "GROUND" }"#, r#""Passives:GND" = { name = "GROUND", default_package = "Nowhere" }"#);
    let (report, lib, _) = convert_with("map_default_missing", &map);
    assert_eq!(report.failures.len(), 1, "{report:#?}");
    assert!(report.failures[0].what.contains("Passives:GND") && report.failures[0].error.contains("Nowhere"));
    assert!(lib.get("GROUND").unwrap().packages.is_empty(), "the device is still converted");
}

#[test]
fn units_body_styles_shared_pins_and_pin_details() {
    let (report, lib, _) = convert("units");
    let gate = lib.get("Gate").unwrap();
    let units: Vec<(&str, u8)> = gate.symbol.units.iter().map(|u| (u.name.as_str(), u.body_style)).collect();
    assert_eq!(units, [("A", 1), ("A", 2), ("B", 1), ("B", 2)]);
    for u in &gate.symbol.units {
        assert!(u.pins.iter().any(|p| p.number == "14" && p.hidden && p.kind == PinKind::PowerIn), "shared power pin in {}", u.name);
    }
    let a = &gate.symbol.units[0];
    let pin1 = a.pins.iter().find(|p| p.number == "1").unwrap();
    assert_eq!((pin1.at, pin1.orient, pin1.length), (Pt::new(-300.0, 0.0), PinDir::Right, 150.0));
    assert!(a.graphics.iter().any(|g| matches!(g.shape, Shape::Circle { radius, .. } if radius == 25.0) && g.fill == Fill::Body), "inverted bubble");
    assert!(a.graphics.iter().any(|g| matches!(&g.shape, Shape::Text { text, angle, .. } if text == "&" && *angle == 90.0)), "text angle in degrees");
    assert!(a.graphics.iter().any(|g| matches!(g.shape, Shape::Arc { .. }) && g.fill == Fill::Body));
    let b = &gate.symbol.units[2];
    assert_eq!(b.pins.iter().find(|p| p.number == "3").unwrap().kind, PinKind::Unspecified, "free → unspecified");
    assert!(b.pins.iter().any(|p| p.number == "4#2"), "repeated pin number renamed");
    assert!(
        b.graphics.iter().any(|g| matches!(g.shape, Shape::Rect { a, b } if a == Pt::new(-100.0, 100.0) && b == Pt::new(100.0, -100.0))),
        "text box frame"
    );
    let dip = gate.package("DIP14_Test").unwrap();
    assert!(dip.pin_map.contains(&("4#2".into(), "4".into())));
    assert_eq!(dip.pin_map.len(), 7, "1 2 3 4 4#2 14 7");
    assert_eq!(report.notes.get("pin type free → unspecified"), 1);
}

#[test]
fn symbol_y_up_is_kept() {
    let (_, lib, _) = convert("sym_y");
    let res = lib.get("RES").unwrap();
    let pins = &res.symbol.units[0].pins;
    let p1 = pins.iter().find(|p| p.number == "1").unwrap();
    let p2 = pins.iter().find(|p| p.number == "2").unwrap();
    // KiCad: pin 1 at (0, 3.81) pointing down (270°), pin 2 at (0, -3.81) pointing up (90°).
    assert_eq!((p1.at, p1.orient), (Pt::new(0.0, 150.0), PinDir::Down));
    assert_eq!((p2.at, p2.orient), (Pt::new(0.0, -150.0), PinDir::Up));
    assert_eq!(p1.body_end(), Pt::new(0.0, 100.0));
    assert!(!p1.number_visible && p1.name.is_empty(), "pin_numbers hide, name ~");
}

#[test]
fn footprint_y_down_is_mirrored() {
    let (_, lib, _) = convert("fp_y");
    let r = lib.package("TestFP:R_Test").unwrap();
    assert_eq!(r.library, "TestFP.pretty");
    let pad1 = r.pad("1").unwrap();
    // KiCad: pad 1 at (-1, 0.5) mm, y down → (-39.3701, -19.685) th, y up.
    assert_eq!(pad1.at, Pt::new(-39.3701, -19.685));
    assert_eq!(pad1.shape, PadShape::RoundRect { ratio: 0.2 });
    assert_eq!((pad1.drill, pad1.layers.clone()), (None, vec!["copper.top".to_owned(), "mask.top".into(), "paste.top".into()]));
    assert_eq!(r.pad("2").unwrap().angle, 90.0);
    let silk_line = r.graphics.iter().find(|g| g.layer == "silk.top" && matches!(g.graphic.shape, Shape::Polyline { .. })).unwrap();
    // KiCad: line at y = -1 mm (above the origin on screen) → y = +39.3701 th.
    assert_eq!(silk_line.graphic.shape, Shape::line(Pt::new(-39.3701, 39.3701), Pt::new(39.3701, 39.3701)));
    let arc = r.graphics.iter().find(|g| matches!(g.graphic.shape, Shape::Arc { .. })).unwrap();
    assert_eq!(arc.graphic.shape, Shape::Arc { start: Pt::new(-39.3701, -39.3701), mid: Pt::new(0.0, -78.7402), end: Pt::new(39.3701, -39.3701) });
    let reference = r.graphics.iter().find(|g| matches!(&g.graphic.shape, Shape::Text { text, .. } if text == "REF**")).unwrap();
    assert!(matches!(reference.graphic.shape, Shape::Text { at, .. } if at == Pt::new(0.0, 78.7402)));
    assert!(!r.graphics.iter().any(|g| matches!(&g.graphic.shape, Shape::Text { text, .. } if text == "R_Test")), "hidden value skipped");
}

#[test]
fn pads_drills_duplicates_and_custom_shapes() {
    let (_, lib, _) = convert("pads");
    let dip = lib.package("DIP14_Test").unwrap();
    assert_eq!(dip.pads.len(), 17);
    assert_eq!(dip.pad("2").unwrap().drill, Some(31.4961), "oval drill keeps its smaller size");
    assert!(dip.pad("7#2").is_some() && dip.pad("").is_some() && dip.pad("#2").is_some());
    let pad1 = dip.pad("1").unwrap();
    assert_eq!((pad1.shape, pad1.layers[0].as_str()), (PadShape::Rect, "copper.*"));
    let outline: Vec<_> = dip.graphics.iter().filter(|g| g.layer == "copper.*").collect();
    assert_eq!(outline.len(), 1);
    assert_eq!(outline[0].graphic.fill, Fill::Solid);
    let Shape::Polyline { points } = &outline[0].graphic.shape else { panic!("polygon expected") };
    assert!(points.contains(&Pt::new(47.2441, 31.4961)), "custom outline mirrored in y: {points:?}");
    let fab = dip.graphics.iter().find(|g| g.layer == "fab.top").unwrap();
    assert!(matches!(fab.graphic.shape, Shape::Text { angle, at, .. } if angle == 90.0 && at == Pt::new(0.0, -39.3701)));
}

#[test]
fn conversion_is_deterministic() {
    let (_, _, a) = convert("det_a");
    let (_, _, b) = convert("det_b");
    let files = |root: &Path| {
        let mut out = Vec::new();
        for sub in ["", "symbols", "footprints"] {
            for e in fs::read_dir(root.join(sub)).unwrap() {
                let p = e.unwrap().path();
                if p.is_file() {
                    out.push(p.strip_prefix(root).unwrap().to_path_buf());
                }
            }
        }
        out.sort();
        out
    };
    let (fa, fb) = (files(&a), files(&b));
    assert_eq!(fa, fb);
    assert!(fa.len() >= 6);
    for f in &fa {
        assert_eq!(fs::read(a.join(f)).unwrap(), fs::read(b.join(f)).unwrap(), "{}", f.display());
    }
}
