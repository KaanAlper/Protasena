use super::*;
use crate::common::Pt;

const SAMPLE: &str = include_str!("../../tests/data/sample.kicad_mod");
const LEGACY: &str = include_str!("../../tests/data/legacy_v5.kicad_mod");

fn sample() -> Footprint {
    parse_footprint(SAMPLE.as_bytes()).unwrap()
}

fn shape(f: &Footprint, i: usize) -> &FpShape {
    match &f.graphics[i] {
        FpGraphic::Shape(s) => s,
        other => panic!("graphic {i} is {other:?}"),
    }
}

#[test]
fn reads_header_properties_and_settings() {
    let f = sample();
    assert_eq!(f.name, "Test_Package_2x3");
    assert_eq!(f.version, Some(20241229));
    assert_eq!(f.layer.as_deref(), Some("F.Cu"));
    assert_eq!(f.descr.as_deref(), Some("Synthetic test footprint"));
    assert_eq!(f.tags.as_deref(), Some("test synthetic"));
    let r = f.property("Reference").unwrap();
    assert_eq!(r.value, "REF**");
    assert_eq!(r.attrs.layer.as_deref(), Some("F.SilkS"));
    assert_eq!(r.attrs.at.unwrap().y, -3.0);
    assert!(!r.attrs.is_hidden());
    let v = f.property("Value").unwrap();
    assert!(v.attrs.unlocked && v.attrs.hide);
    assert!(v.attrs.effects.as_ref().unwrap().justify.mirror);
    assert_eq!(f.overrides.solder_mask_margin, Some(0.05));
    assert_eq!(f.overrides.clearance, Some(0.2));
    assert_eq!(f.overrides.zone_connect, Some(2));
    let attr = f.attr.as_ref().unwrap();
    assert_eq!(attr.kind, Some(AttrType::Smd));
    assert!(attr.exclude_from_bom && attr.allow_missing_courtyard && !attr.board_only);
    assert_eq!(f.embedded_fonts, Some(false));
}

#[test]
fn reads_graphics() {
    let f = sample();
    let line = shape(&f, 0);
    assert_eq!(line.kind, FpShapeKind::Line { start: Pt { x: -2.0, y: -1.5 }, end: Pt { x: 2.0, y: -1.5 } });
    assert_eq!(line.stroke.as_ref().unwrap().width, Some(0.12));
    assert_eq!(line.layer.as_deref(), Some("F.SilkS"));
    assert!(line.uuid.is_some());
    assert_eq!(shape(&f, 1).fill, Some(false));
    assert!(matches!(shape(&f, 1).kind, FpShapeKind::Rect { .. }));
    assert_eq!(shape(&f, 2).fill, Some(true));
    assert!(matches!(shape(&f, 3).kind, FpShapeKind::Arc { mid: Some(_), angle: None, .. }));
    assert_eq!(shape(&f, 4).fill, Some(true));
    assert!(matches!(&shape(&f, 4).kind, FpShapeKind::Poly { pts } if pts.len() == 3));
    assert!(matches!(&shape(&f, 5).kind, FpShapeKind::Curve { pts } if pts.len() == 4));
    let FpGraphic::Text(t) = &f.graphics[6] else { panic!() };
    assert_eq!((t.kind.clone(), t.text.as_str()), (FpTextType::User, "${REFERENCE}"));
    assert!(t.attrs.knockout);
    assert_eq!(t.attrs.at.unwrap().angle, 90.0);
    let FpGraphic::TextBox(b) = &f.graphics[7] else { panic!() };
    assert_eq!(b.text, "Box");
    assert_eq!((b.start, b.end), (Some(Pt { x: -1.0, y: -1.0 }), Some(Pt { x: 1.0, y: 1.0 })));
    assert_eq!(b.border, Some(true));
    assert_eq!(b.margins, Some([0.1; 4]));
}

#[test]
fn reads_pads() {
    let f = sample();
    let p = &f.pads;
    assert_eq!(p.len(), 7);
    assert_eq!((p[0].kind.clone(), p[0].shape.clone()), (PadType::ThruHole, PadShape::Circle));
    assert_eq!(p[0].drill.as_ref().unwrap().diameter, Some(0.7));
    assert_eq!(p[0].layers, ["*.Cu", "*.Mask"]);
    assert_eq!(p[0].remove_unused_layers, Some(false));
    let d = p[1].drill.as_ref().unwrap();
    assert!(d.oval);
    assert_eq!((d.diameter, d.width, d.offset), (Some(0.6), Some(1.2), Some(Pt { x: 0.0, y: 0.1 })));
    assert_eq!(p[1].at.angle, 90.0);
    assert_eq!(p[2].shape, PadShape::RoundRect);
    assert_eq!(p[2].roundrect_rratio, Some(0.25));
    assert_eq!(p[2].overrides.solder_paste_ratio, Some(-0.1));
    assert_eq!(p[3].chamfer, [ChamferCorner::TopLeft, ChamferCorner::BottomRight]);
    assert_eq!(p[3].chamfer_ratio, Some(0.2));
    assert_eq!(p[3].property.as_deref(), Some("pad_prop_heatsink"));
    assert_eq!(p[4].shape, PadShape::Custom);
    assert_eq!(p[4].overrides.thermal_bridge_angle, Some(45.0));
    let o = p[4].options.as_ref().unwrap();
    assert_eq!((o.clearance.as_deref(), o.anchor.as_deref()), (Some("outline"), Some("rect")));
    let prims = p[4].primitives.as_ref().unwrap();
    assert_eq!(prims.items.len(), 3);
    assert_eq!(prims.items[0].fill, Some(true));
    assert!(matches!(prims.items[1].kind, FpShapeKind::Circle { .. }));
    assert!(matches!(prims.items[2].kind, FpShapeKind::BBox { .. }));
    assert_eq!(p[5].rect_delta, Some(Pt { x: 0.0, y: 0.2 }));
    assert_eq!((p[6].number.as_str(), p[6].kind.clone()), ("", PadType::NpThruHole));
}

#[test]
fn reads_groups_models_and_keeps_unknown_tokens() {
    let f = sample();
    assert_eq!(f.groups.len(), 1);
    assert_eq!(f.groups[0].name, "Outline");
    assert_eq!(f.groups[0].members.len(), 2);
    let m = &f.models[0];
    assert_eq!(m.path, "${KICAD9_3DMODEL_DIR}/Test.3dshapes/Test_Package_2x3.wrl");
    assert_eq!(m.rotate.unwrap().z, -90.0);
    assert_eq!(m.scale.unwrap().x, 1.0);
    assert_eq!(f.unknown.len(), 1);
    assert_eq!(f.unknown[0].tree.to_string(), "(sparkle \"unknown token kept\")");
    let r = f.report();
    assert_eq!(r.unknown_tokens.get("footprint/sparkle"), Some(&1));
    assert_eq!(r.unknown_count(), 1);
}

#[test]
fn reads_keepout_zone() {
    let f = sample();
    let z = &f.zones[0];
    assert_eq!(z.layers, ["F.Cu", "B.Cu"]);
    assert_eq!(z.name.as_deref(), Some("Antenna keepout"));
    assert_eq!(z.hatch, Some(("edge".to_owned(), 0.5)));
    assert_eq!(z.connect_pads.as_ref().unwrap().clearance, Some(0.0));
    let k = z.keepout.as_ref().unwrap();
    assert_eq!((k.tracks.clone(), k.pads.clone()), (Some(KeepoutRule::NotAllowed), Some(KeepoutRule::Allowed)));
    let fill = z.fill.as_ref().unwrap();
    assert!(!fill.filled);
    assert_eq!(fill.thermal_gap, Some(0.5));
    assert_eq!(z.polygons[0].len(), 4);
    assert!(z.unknown.is_empty());
}

#[test]
fn reads_pre_kicad6_module() {
    let f = parse_footprint(LEGACY.as_bytes()).unwrap();
    assert_eq!(f.name, "Test_Legacy");
    assert_eq!(f.tedit.as_deref(), Some("5A02FF57"));
    let FpGraphic::Text(r) = &f.graphics[0] else { panic!() };
    assert_eq!((r.kind.clone(), r.text.as_str()), (FpTextType::Reference, "REF**"));
    let FpGraphic::Text(v) = &f.graphics[1] else { panic!() };
    assert!(v.attrs.hide);
    assert_eq!(shape(&f, 2).width, Some(0.12));
    assert!(matches!(shape(&f, 4).kind, FpShapeKind::Arc { mid: None, angle: Some(a), .. } if a == 90.0));
    assert_eq!(f.pads[0].number, "1");
    assert_eq!(f.models[0].offset, Some(Xyz::default()));
    // `virtual` was dropped from the format with KiCad 6; it stays visible as unknown.
    assert_eq!(f.report().unknown_tokens.get("attr/virtual"), Some(&1));
}

#[test]
fn rejects_malformed_known_tokens() {
    let e = parse_footprint(b"(footprint \"x\"\n (pad \"1\" smd rect (size 1 1) (layers F.Cu)))").unwrap_err();
    assert!(e.to_string().contains("missing (at)"), "{e}");
    assert_eq!(e.pos.line, 2);
    let e = parse_footprint(b"(footprint \"x\" (fp_line (start 0 0) (layer F.Cu)))").unwrap_err();
    assert!(e.to_string().contains("missing (end)"), "{e}");
    for bad in ["nan", "inf", "-infinity", "1e400", "0x10"] {
        let src = format!("(footprint \"x\" (pad \"1\" smd rect (at {bad} 0) (size 1 1)))");
        assert!(parse_footprint(src.as_bytes()).unwrap_err().to_string().contains("expected a number"), "{bad}");
    }
    assert!(parse_footprint(b"(kicad_symbol_lib)").is_err());
    assert!(parse_footprint(b"(footprint)").is_err());
}
