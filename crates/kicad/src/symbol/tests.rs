use super::*;
use crate::common::{FillType, HAlign, Pt, StrokeType, VAlign};

const SAMPLE: &str = include_str!("../../tests/data/sample.kicad_sym");
const LEGACY: &str = include_str!("../../tests/data/legacy_v6.kicad_sym");

fn sample() -> SymbolLib {
    parse_symbol_lib(SAMPLE.as_bytes()).unwrap()
}

#[test]
fn reads_header_and_symbols() {
    let lib = sample();
    assert_eq!(lib.version, Some(20241209));
    assert_eq!(lib.generator.as_deref(), Some("protasena_fixture"));
    assert_eq!(lib.generator_version.as_deref(), Some("1.0"));
    let names: Vec<&str> = lib.symbols.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["TestGate", "TestGateB", "TestGateC", "TestSupply"]);
}

#[test]
fn reads_symbol_flags_and_properties() {
    let lib = sample();
    let g = lib.get("TestGate").unwrap();
    assert_eq!(g.pin_names.as_ref().unwrap().offset, Some(1.016));
    assert!(!g.pin_names.as_ref().unwrap().hide);
    assert_eq!((g.in_bom, g.on_board, g.exclude_from_sim, g.embedded_fonts), (Some(true), Some(true), Some(false), Some(false)));
    let r = g.property("Reference").unwrap();
    assert_eq!(r.value, "U");
    assert_eq!(r.at, Some(At { x: 0.0, y: 6.35, angle: 0.0 }));
    let e = r.effects.as_ref().unwrap();
    assert!(e.font.bold && !e.font.italic);
    assert_eq!((e.font.height, e.font.width), (Some(1.27), Some(1.27)));
    assert_eq!((e.justify.h, e.justify.v), (HAlign::Left, VAlign::Bottom));
    assert!(!r.is_hidden());
    // KiCad 9 `(hide yes)` on the property, KiCad 8 `(hide yes)` in effects.
    assert!(g.property("Footprint").unwrap().hide);
    assert!(g.property("Footprint").unwrap().is_hidden());
    assert!(g.property("Datasheet").unwrap().is_hidden());
    assert_eq!(g.value_of("Datasheet"), Some("https://example.org/testgate \"v2\""));
    let private = g.property("Note").unwrap();
    assert!(private.private && private.is_hidden());
    assert_eq!(private.value, "private field");
    assert!(!r.private);
    let k = g.property("ki_keywords").unwrap();
    assert_eq!((k.show_name, k.do_not_autoplace), (Some(false), Some(false)));
}

#[test]
fn reads_units_and_body_styles() {
    let lib = sample();
    let g = lib.get("TestGate").unwrap();
    let ids: Vec<(u32, u32)> = g.units.iter().map(|u| (u.unit, u.style)).collect();
    assert_eq!(ids, [(0, 1), (1, 1), (1, 2), (2, 1)]);
    assert_eq!(g.unit_count(), 2);
    assert!(g.has_alternate_style());
    assert_eq!(g.units[3].unit_name.as_deref(), Some("Power"));
    let GraphicKind::Text { text, at, effects } = &g.units[0].graphics[0].kind else { panic!() };
    assert_eq!(text, "multi\nline");
    assert_eq!(at.angle, 900.0);
    assert!(effects.as_ref().unwrap().font.italic);
}

#[test]
fn reads_graphics() {
    let lib = sample();
    let g = lib.get("TestGate").unwrap();
    let u1 = &g.units[1];
    let arc = &u1.graphics[0];
    assert!(matches!(arc.kind, GraphicKind::Arc { mid: Some(Pt { x, .. }), .. } if x == 3.81));
    assert_eq!(arc.fill.as_ref().unwrap().kind, FillType::Background);
    let poly = &u1.graphics[1];
    assert!(matches!(&poly.kind, GraphicKind::Polyline { pts } if pts.len() == 4));
    let stroke = poly.stroke.as_ref().unwrap();
    assert_eq!(stroke.kind, Some(StrokeType::DashDot));
    assert_eq!(stroke.color.unwrap().a, 0.5);
    let u2 = &g.units[2];
    assert!(matches!(u2.graphics[0].kind, GraphicKind::Circle { radius, .. } if radius == 0.635));
    assert_eq!(u2.graphics[0].fill.as_ref().unwrap().kind, FillType::Outline);
    assert!(matches!(&u2.graphics[1].kind, GraphicKind::Bezier { pts } if pts.len() == 4));
    let GraphicKind::TextBox { text, geometry, margins, .. } = &u2.graphics[2].kind else { panic!() };
    assert_eq!(text, "note");
    assert_eq!(geometry.size, Some(Pt { x: 4.0, y: 2.0 }));
    assert_eq!(*margins, Some([0.5; 4]));
    assert!(matches!(g.units[3].graphics[0].kind, GraphicKind::Rectangle { .. }));
}

#[test]
fn reads_pins_with_alternates_and_hide() {
    let lib = sample();
    let pins = &lib.get("TestGate").unwrap().units[1].pins;
    assert_eq!(pins.len(), 2);
    let a = &pins[0];
    assert_eq!((a.electrical.clone(), a.style.clone()), (ElectricalType::Input, PinStyle::Line));
    assert_eq!(a.at, At { x: -7.62, y: 2.54, angle: 0.0 });
    assert_eq!(a.length, 3.81);
    assert_eq!((a.name.text.as_str(), a.number.text.as_str()), ("A", "1"));
    assert_eq!(a.alternates.len(), 2);
    assert_eq!(a.alternates[1].name, "TX");
    assert_eq!(a.alternates[1].electrical, ElectricalType::Output);
    assert_eq!(a.alternates[1].style, PinStyle::Clock);
    assert!(!a.hide);
    assert!(pins[1].hide);
    assert_eq!(pins[1].style, PinStyle::Inverted);
}

#[test]
fn power_symbol_and_shared_unit() {
    let lib = sample();
    let s = lib.get("TestSupply").unwrap();
    assert!(s.power);
    assert_eq!(s.power_scope, None);
    assert_eq!(s.pin_numbers_hide, Some(true));
    assert!(s.pin_names.as_ref().unwrap().hide);
    // Graphics and pins written directly in the symbol land in the shared unit 0.
    assert_eq!((s.units[0].unit, s.units[0].style), (0, 0));
    assert_eq!(s.units[0].pins.len(), 1);
    assert_eq!(s.units[0].graphics.len(), 1);
}

#[test]
fn unknown_tokens_are_kept_and_reported() {
    let lib = sample();
    let s = lib.get("TestSupply").unwrap();
    assert_eq!(s.unknown.len(), 1);
    assert_eq!(s.unknown[0].tree.to_string(), "(frobnicate 1 2)");
    let report = lib.report();
    assert_eq!(report.unknown_tokens.get("symbol/frobnicate"), Some(&1));
    assert_eq!(report.unknown_count(), 1);
    assert!(report.other_values.is_empty());
}

#[test]
fn unknown_values_are_kept_as_other() {
    let src = "(kicad_symbol_lib (symbol \"X\" (symbol \"X_1_1\" (pin weird zigzag (at 0 0 0) (length 1) (name \"a\") (number \"1\")))))";
    let lib = parse_symbol_lib(src.as_bytes()).unwrap();
    let pin = &lib.symbols[0].units[0].pins[0];
    assert_eq!(pin.electrical, ElectricalType::Other("weird".into()));
    assert_eq!(pin.style.keyword(), "zigzag");
    let r = lib.report();
    assert_eq!(r.other_values.get("pin.type=weird"), Some(&1));
    assert_eq!(r.other_values.get("pin.style=zigzag"), Some(&1));
}

#[test]
fn reads_kicad6_bare_hide_flags() {
    let lib = parse_symbol_lib(LEGACY.as_bytes()).unwrap();
    assert_eq!(lib.version, Some(20211014));
    let r = &lib.symbols[0];
    assert_eq!(r.pin_numbers_hide, Some(true));
    let names = r.pin_names.as_ref().unwrap();
    assert_eq!((names.offset, names.hide), (Some(0.0), true));
    assert_eq!(r.property("Reference").unwrap().id, Some(0));
    assert!(r.property("Footprint").unwrap().is_hidden());
    assert!(!r.property("Value").unwrap().is_hidden());
    let pins = &r.units[1].pins;
    assert!(pins[0].hide && !pins[1].hide);
    let font = &pins[1].name.effects.as_ref().unwrap().font;
    assert!(font.bold && font.italic);
    assert!(lib.report().is_clean());
}

#[test]
fn extends_inherits_units_and_overrides_properties() {
    let lib = sample();
    let b = lib.resolve("TestGateB").unwrap();
    assert_eq!(b.value_of("Value"), Some("TestGateB"));
    assert_eq!(b.value_of("Reference"), Some("U"));
    assert_eq!(b.value_of("ki_description"), Some("a derived test gate"));
    assert_eq!(b.units.len(), 4);
    assert_eq!(b.units[1].name, "TestGateB_1_1");
    assert_eq!(b.in_bom, Some(true));
    // Two levels: C extends B extends TestGate.
    let c = lib.resolve("TestGateC").unwrap();
    assert_eq!(c.value_of("Value"), Some("TestGateC"));
    assert_eq!(c.value_of("ki_description"), Some("a derived test gate"));
    assert_eq!(c.in_bom, Some(false));
    assert_eq!(c.units[3].pins[0].number.text, "14");
    let all = lib.resolve_all().unwrap();
    assert_eq!(all.len(), 4);
    assert_eq!(lib.resolve("TestGate").unwrap(), *lib.get("TestGate").unwrap());
}

#[test]
fn extends_errors() {
    let missing = parse_symbol_lib(b"(kicad_symbol_lib (symbol \"A\" (extends \"Nope\")))").unwrap();
    let e = missing.resolve("A").unwrap_err();
    assert_eq!(e.kind, ErrorKind::MissingParent { symbol: "A".into(), parent: "Nope".into() });
    assert!(missing.resolve_all().is_err());
    let cycle =
        parse_symbol_lib(b"(kicad_symbol_lib (symbol \"A\" (extends \"B\")) (symbol \"B\" (extends \"C\")) (symbol \"C\" (extends \"A\")))").unwrap();
    assert!(matches!(cycle.resolve("A").unwrap_err().kind, ErrorKind::ExtendsCycle { .. }));
    let selfish = parse_symbol_lib(b"(kicad_symbol_lib (symbol \"A\" (extends \"A\")))").unwrap();
    assert!(matches!(selfish.resolve("A").unwrap_err().kind, ErrorKind::ExtendsCycle { .. }));
    assert!(matches!(selfish.resolve("Z").unwrap_err().kind, ErrorKind::UnknownSymbol(_)));
}

#[test]
fn model_errors_have_positions() {
    let e =
        parse_symbol_lib(b"(kicad_symbol_lib\n (symbol \"A\" (symbol \"A_1_1\" (pin input line (at x 0) (length 1) (name \"\") (number \"\")))))")
            .unwrap_err();
    assert_eq!(e.pos, Pos { line: 2, col: 50 });
    assert!(e.to_string().contains("expected a number"), "{e}");
    let e = parse_symbol_lib(b"(footprint \"x\")").unwrap_err();
    assert!(matches!(e.kind, ErrorKind::WrongRoot { .. }));
    let e = parse_symbol_lib(b"(kicad_symbol_lib (symbol \"A\" (symbol \"bad\")))").unwrap_err();
    assert!(e.to_string().contains("unit identifier"), "{e}");
    let e = parse_symbol_lib(b"(kicad_symbol_lib (symbol \"A\" (arc (start 0 0))))").unwrap_err();
    assert!(e.to_string().contains("missing (end)"), "{e}");
}

#[test]
fn duplicate_tokens_are_not_dropped() {
    let lib = parse_symbol_lib(b"(kicad_symbol_lib (symbol \"A\" (rectangle (start 0 0) (end 1 1) (end 2 2))))").unwrap();
    let r = lib.report();
    assert_eq!(r.unknown_tokens.get("duplicate/end"), Some(&1));
    assert!(matches!(lib.symbols[0].units[0].graphics[0].kind, GraphicKind::Rectangle { end: Pt { x, .. }, .. } if x == 1.0));
}
