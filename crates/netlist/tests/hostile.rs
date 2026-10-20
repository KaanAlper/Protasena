//! Hostile projects (empty, NaN, huge coordinates, duplicate ids, malformed items, large
//! designs) never panic.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use protasena_doc::*;
use protasena_geom::{Orient, Transform, Vec2};
use protasena_library::Library;
use protasena_netlist::{ErcKind, build, export};

fn exports_never_panic(n: &protasena_netlist::Netlist) {
    let json = export::to_json(n, "x\n\"y\"").unwrap();
    assert_eq!(&export::from_json(&json).unwrap().0, n);
    assert!(export::to_spice(n, "x\ny").ends_with(".end\n"));
}

#[test]
fn hostile_empty_projects_never_panic() {
    let lib = Library::builtin();
    let mut p = Project::new("empty");
    let n = build(&p, &lib);
    assert!(n.nets.is_empty() && n.parts.is_empty() && n.erc.is_empty());
    exports_never_panic(&n);
    p.sheets.clear();
    assert_eq!(build(&p, &lib), build(&Project::default(), &lib));
}

#[test]
fn hostile_coordinates_never_panic() {
    let mut d = Design::new();
    let s = &mut d.p.sheets[0];
    let huge = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -1e300, f64::MAX, 9.5e12];
    for (i, &v) in huge.iter().enumerate() {
        let raw = 1000 + i as u64 * 10;
        let c = Component::new(ComponentId(raw), DeviceRef::new("", "RES"), &format!("R{i}"), "1k")
            .placed(Transform::new(Orient::rotation(1), Vec2::new(v, 0.0)));
        s.components.push(c);
        s.wires.push(Wire { id: WireId(raw + 1), points: vec![pt(v, v), pt(0.0, v), pt(-v, 0.0)] });
        s.junctions.push(Junction { id: JunctionId(raw + 2), at: pt(0.0, v) });
        s.labels.push(Label { id: LabelId(raw + 3), at: pt(v, 0.0), text: "L".into(), orient: Orient::IDENTITY });
        s.terminals.push(Terminal {
            id: TerminalId(raw + 4),
            at: pt(v, v),
            kind: TerminalKind::Ground,
            name: String::new(),
            orient: Orient::IDENTITY,
        });
    }
    // A sane wire from far away to the origin still works next to the junk.
    s.wires.push(Wire { id: WireId(1), points: vec![pt(-1e12, 0.0), pt(1e12, 0.0)] });
    let n = d.netlist();
    assert!(n.erc_of(ErcKind::InvalidGeometry).count() >= huge.len());
    exports_never_panic(&n);
}

#[test]
fn hostile_items_and_duplicate_ids_never_panic() {
    let mut d = Design::new();
    d.part("RES", "R1", (0.0, 0.0));
    let s = &mut d.p.sheets[0];
    let dup = s.components[0].clone();
    s.components.push(dup.clone());
    s.components.push(Component { unit: u32::MAX, ..dup.clone() });
    s.components.push(Component { reference: "\u{0}\n( )=".into(), value: "a b\nc".into(), ..dup });
    s.wires.push(Wire { id: WireId(7), points: Vec::new() });
    s.wires.push(Wire { id: WireId(7), points: vec![pt(200.0, 0.0)] });
    s.labels.push(Label { id: LabelId(7), at: pt(200.0, 0.0), text: "  ".into(), orient: Orient::IDENTITY });
    let copy = s.clone();
    d.p.sheets.push(copy);
    let n = d.netlist();
    assert!(n.erc_of(ErcKind::UnknownDevice).count() >= 1, "gate u32::MAX does not exist");
    assert!(n.erc_of(ErcKind::DuplicateReference).count() >= 1);
    exports_never_panic(&n);
}

#[test]
fn hostile_large_designs_finish() {
    let mut d = Design::new();
    let s = &mut d.p.sheets[0];
    // A chain of 3000 resistors joined pin to pin, and 3000 long wires above it that the
    // segment index has to keep apart.
    for i in 0..3000u64 {
        let x = i as f64 * 400.0;
        s.components.push(
            Component::new(ComponentId(10 + i), DeviceRef::new("", "RES"), &format!("R{}", i + 1), "1k")
                .placed(Transform::translation(Vec2::new(x, 0.0))),
        );
    }
    for i in 0..3000u64 {
        let y = 1000.0 + i as f64 * 100.0;
        s.wires.push(Wire { id: WireId(10_000 + i), points: vec![pt(-1e6, y), pt(2e6, y)] });
    }
    let n = d.netlist();
    assert_eq!(n.nets.len(), 2999, "wires without pins make no nets");
    assert_eq!(n.unconnected_pins.len(), 2);
    assert_eq!(n.parts.len(), 3000);
}
