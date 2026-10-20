//! Shared helpers: a scratch directory per test and a small design built through operations.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::PathBuf;

use protasena_doc::*;
use protasena_geom::{Orient, Pt, Transform, Vec2};

/// An empty directory for one test under Cargo's per-target temp dir.
pub fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("protasena-io").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn add(p: &mut Project, sheet: SheetId, item: Item) {
    p.apply(Op::Add { sheet, item, index: None }).unwrap();
}

/// Two sheets with a resistor divider, a label, a terminal, a junction, a bus, a text, one 2D
/// graphic of each kind and properties, built only through operations (including an insertion in the middle of a list).
pub fn sample_project() -> Project {
    let mut p = Project::new("divider");
    let design = DesignProperties { title: "Divider \"test\" ünïcode".into(), author: "Tester".into(), ..DesignProperties::default() };
    p.apply(Op::SetDesign { name: "divider".into(), design }).unwrap();
    let s1 = p.sheets[0].id;
    let place = |x: f64, y: f64, orient| Transform::new(orient, Vec2::new(x, y));
    let r1: ComponentId = p.fresh_id().unwrap();
    let mut c = Component::new(r1, DeviceRef::new("GENERIC", "RES"), "R?", "10k").placed(place(0.0, 0.0, Orient::IDENTITY));
    c.properties = Properties::new().with("TOLERANCE", "1%").with("POWER", "0.25W");
    add(&mut p, s1, Item::Component(c));
    let r2: ComponentId = p.fresh_id().unwrap();
    let c2 = Component::new(r2, DeviceRef::new("GENERIC", "RES"), "R?", "4k7").placed(place(1000.0, -300.0, Orient::rotation(1).mirror_x()));
    add(&mut p, s1, Item::Component(c2));
    let w: WireId = p.fresh_id().unwrap();
    add(&mut p, s1, Item::Wire(Wire { id: w, points: vec![Pt::new(200.0, 0.0), Pt::new(0.1 + 0.2, 1e-7), Pt::new(800.0, 0.0)] }));
    let w0: WireId = p.fresh_id().unwrap();
    p.apply(Op::Add { sheet: s1, item: Item::Wire(Wire { id: w0, points: vec![Pt::new(-5.5, 3.25), Pt::new(-5.5, 1e15)] }), index: Some(0) })
        .unwrap();
    let j: JunctionId = p.fresh_id().unwrap();
    add(&mut p, s1, Item::Junction(Junction { id: j, at: Pt::new(500.0, 0.0) }));
    let l: LabelId = p.fresh_id().unwrap();
    add(&mut p, s1, Item::Label(Label { id: l, at: Pt::new(500.0, 0.0), text: "MID".into(), orient: Orient::rotation(2) }));
    p.apply(p.annotate()).unwrap();

    let s2: SheetId = p.fresh_id().unwrap();
    p.apply(Op::AddSheet { sheet: Sheet::new(s2, "Sheet 2", SheetSize::Custom { width: 12_345.5, height: 6_000.0 }), index: None }).unwrap();
    let t: TerminalId = p.fresh_id().unwrap();
    add(
        &mut p,
        s2,
        Item::Terminal(Terminal { id: t, at: Pt::new(100.0, 100.0), kind: TerminalKind::Power, name: "VCC".into(), orient: Orient::IDENTITY }),
    );
    let b: BusId = p.fresh_id().unwrap();
    add(&mut p, s2, Item::Bus(Bus { id: b, points: vec![Pt::new(0.0, 0.0), Pt::new(0.0, 2000.0)] }));
    let e: BusEntryId = p.fresh_id().unwrap();
    add(&mut p, s2, Item::BusEntry(BusEntry { id: e, bus_end: Pt::new(0.0, 500.0), wire_end: Pt::new(100.0, 600.0) }));
    let x: TextId = p.fresh_id().unwrap();
    add(&mut p, s2, Item::Text(Text { id: x, at: Pt::new(-1.0, 2.0), text: "line 1\nline 2\t✓".into(), size: 12.5, orient: Orient::IDENTITY }));
    let pt = Pt::new;
    for shape in [
        GraphicShape::Line { a: pt(0.0, 0.0), b: pt(500.0, 0.0) },
        GraphicShape::Box { a: pt(-100.0, -100.0), b: pt(300.0, 200.5) },
        GraphicShape::Circle { center: pt(1000.0, 0.0), radius: 0.1 + 0.2 },
        GraphicShape::Arc { start: pt(0.0, 0.0), mid: pt(100.0, 100.0), end: pt(200.0, 0.0) },
        GraphicShape::Path { points: vec![pt(0.0, 0.0), pt(400.0, 0.0), pt(200.0, 300.0)] },
    ] {
        let id: GraphicId = p.fresh_id().unwrap();
        add(&mut p, s2, Item::Graphic(Graphic { id, shape }));
    }
    assert!(p.issues().is_empty(), "{:?}", p.issues());
    p
}
