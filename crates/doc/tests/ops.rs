//! Every operation is undone exactly by the op it returns, and redone by the undo's inverse.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use protasena_doc::*;
use protasena_geom::{Orient, Transform, Vec2};

/// Applies `op`, undoes it, redoes it; checks the states line up and returns the final project.
fn round_trip(mut p: Project, op: Op) -> Project {
    let before = p.clone();
    let undo = p.apply(op).unwrap();
    let after = p.clone();
    let redo = p.apply(undo).unwrap();
    assert_eq!(p, before, "undo restores the project");
    p.apply(redo).unwrap();
    assert_eq!(p, after, "redo restores the edit");
    p
}

fn sample() -> Project {
    let mut p = Project::new("t");
    part(&mut p, "RES", "R1", (0.0, 0.0));
    part(&mut p, "CAP", "C1", (1000.0, 0.0));
    wire(&mut p, &[(200.0, 0.0), (800.0, 0.0)]);
    p
}

#[test]
fn add_remove_and_modify_every_kind_round_trip() {
    let mut p = sample();
    let s = first_sheet(&p);
    let items = vec![
        Item::Junction(junction(&mut p, (500.0, 0.0))),
        Item::Label(label(&mut p, (400.0, 0.0), "N1")),
        Item::Terminal(Terminal {
            id: p.fresh_id().unwrap(),
            at: pt(0.0, 500.0),
            kind: TerminalKind::Power,
            name: String::new(),
            orient: Orient::IDENTITY,
        }),
        Item::Bus(Bus { id: p.fresh_id().unwrap(), points: vec![pt(0.0, 900.0), pt(900.0, 900.0)] }),
        Item::BusEntry(BusEntry { id: p.fresh_id().unwrap(), bus_end: pt(100.0, 900.0), wire_end: pt(200.0, 800.0) }),
        Item::Text(Text { id: p.fresh_id().unwrap(), at: pt(0.0, -500.0), text: "note".into(), size: 0.0, orient: Orient::rotation(1) }),
    ];
    for item in items {
        let id = item.id();
        p = round_trip(p, Op::Add { sheet: s, item, index: Some(0) });
        assert!(p.item(id).is_some());
        p = round_trip(p, Op::Remove { id });
        assert!(p.item(id).is_none());
    }
    let r1 = p.sheets[0].components[0].clone();
    let renamed = Component { value: "4k7".into(), properties: Properties::new().with("TOLERANCE", "1%"), ..r1.clone() };
    let p = round_trip(p, Op::Modify { item: Item::Component(renamed) });
    assert_eq!(p.component(r1.id).unwrap().value, "4k7");
}

#[test]
fn remove_restores_the_list_position() {
    let mut p = sample();
    let c1 = p.sheets[0].components[0].id;
    let undo = p.apply(Op::Remove { id: ItemId::Component(c1) }).unwrap();
    assert!(matches!(undo, Op::Add { index: Some(0), .. }));
    p.apply(undo).unwrap();
    assert_eq!(p.sheets[0].components[0].id, c1);
}

#[test]
fn moves_are_exact_and_cheap_on_the_grid() {
    let p = sample();
    let ids: Vec<ItemId> = p.sheets[0].item_ids().collect();
    let mut q = p.clone();
    let undo = q.apply(Op::Move { ids: ids.clone(), by: Vec2::new(100.0, -200.0) }).unwrap();
    assert_eq!(undo, Op::Move { ids: ids.clone(), by: Vec2::new(-100.0, 200.0) });
    assert_eq!(q.sheets[0].wires[0].points[0], pt(300.0, -200.0));
    assert_eq!(q.sheets[0].components[1].origin(), pt(1100.0, -200.0));
    // Off-grid offsets that do not move back exactly restore the old items verbatim.
    let mut q = p.clone();
    q.apply(Op::Modify { item: Item::Wire(Wire { id: p.sheets[0].wires[0].id, points: vec![pt(0.1, 0.0), pt(1e17, 0.0)] }) }).unwrap();
    let before = q.clone();
    let undo = q.apply(Op::Move { ids: vec![ItemId::Wire(p.sheets[0].wires[0].id)], by: Vec2::new(0.2, 0.0) }).unwrap();
    assert!(matches!(undo, Op::Batch { .. }));
    q.apply(undo).unwrap();
    assert_eq!(q, before);
    round_trip(p.clone(), Op::Move { ids: [ids.clone(), ids].concat(), by: Vec2::new(0.5, 0.25) });
}

#[test]
fn set_transform_rotates_and_restores() {
    let p = sample();
    let id = p.sheets[0].components[0].id;
    let t = Transform::new(Orient::rotation(1).mirror_x(), Vec2::new(300.0, 400.0));
    let p = round_trip(p, Op::SetTransform { id, transform: t });
    assert_eq!(p.component(id).unwrap().transform, t);
}

#[test]
fn sheets_design_and_batches_round_trip() {
    let mut p = sample();
    let new_sheet = Sheet::new(p.fresh_id().unwrap(), "Sheet 2", SheetSize::A3);
    let id = new_sheet.id;
    let p = round_trip(p, Op::AddSheet { sheet: new_sheet, index: None });
    let p = round_trip(p, Op::SetSheet { id, name: "Power".into(), size: SheetSize::Custom { width: 5000.0, height: 4000.0 } });
    assert_eq!(p.sheet(id).unwrap().name, "Power");
    let design = DesignProperties { title: "Amp".into(), revision: "B".into(), ..DesignProperties::default() };
    let p = round_trip(p, Op::SetDesign { name: "amp".into(), design });
    assert_eq!(p.design.title, "Amp");
    let first = p.sheets[0].id;
    let mut p = round_trip(p, Op::RemoveSheet { id: first });
    let a: ComponentId = p.fresh_id().unwrap();
    let batch = Op::batch(vec![
        Op::Add { sheet: id, item: Item::Component(Component::new(a, DeviceRef::new("GENERIC", "RES"), "R?", "1k")), index: None },
        Op::SetTransform { id: a, transform: Transform::translation(Vec2::new(100.0, 100.0)) },
        Op::batch(vec![Op::Remove { id: ItemId::Component(a) }]),
    ]);
    let p = round_trip(p, batch);
    assert!(p.component(a).is_none());
}

#[test]
fn added_ids_are_never_handed_out_again() {
    let mut p = Project::new("t");
    let s = first_sheet(&p);
    let j = Junction { id: JunctionId(100), at: pt(0.0, 0.0) };
    p.apply(Op::Add { sheet: s, item: Item::Junction(j), index: None }).unwrap();
    let next: WireId = p.fresh_id().unwrap();
    assert_eq!(next, WireId(101));
    let dup = Item::Label(Label { id: LabelId(100), at: pt(0.0, 0.0), text: "A".into(), orient: Orient::IDENTITY });
    assert_eq!(p.apply(Op::Add { sheet: s, item: dup, index: None }), Err(DocError::DuplicateItem(ItemId::Label(LabelId(100)))));
    assert!(Op::batch(vec![Op::batch(vec![])]).is_empty());
}
