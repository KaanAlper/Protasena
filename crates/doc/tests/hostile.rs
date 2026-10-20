//! Hostile operations are refused with an error and leave the project unchanged.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use protasena_doc::*;
use protasena_geom::{Orient, Transform, Vec2};

fn refused(p: &mut Project, op: Op) -> DocError {
    let before = p.clone();
    let err = p.apply(op).unwrap_err();
    assert_eq!(*p, before, "a refused op changes nothing");
    err
}

fn sample() -> Project {
    let mut p = Project::new("t");
    part(&mut p, "RES", "R1", (0.0, 0.0));
    wire(&mut p, &[(200.0, 0.0), (800.0, 0.0)]);
    p
}

#[test]
fn hostile_coordinates_never_panic() {
    let mut p = sample();
    let s = first_sheet(&p);
    let r1 = p.sheets[0].components[0].id;
    let w = p.sheets[0].wires[0].id;
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let item = Item::Junction(Junction { id: p.fresh_id().unwrap(), at: pt(bad, 0.0) });
        assert!(matches!(refused(&mut p, Op::Add { sheet: s, item, index: None }), DocError::NonFinite(_)));
        refused(&mut p, Op::Move { ids: vec![ItemId::Wire(w)], by: Vec2::new(0.0, bad) });
        refused(&mut p, Op::SetTransform { id: r1, transform: Transform::translation(Vec2::new(bad, 0.0)) });
        refused(&mut p, Op::SetSheet { id: s, name: "x".into(), size: SheetSize::Custom { width: bad, height: 1.0 } });
    }
    // Overflowing a huge coordinate is refused too.
    p.apply(Op::Modify { item: Item::Wire(Wire { id: w, points: vec![pt(f64::MAX, 0.0), pt(0.0, 0.0)] }) }).unwrap();
    refused(&mut p, Op::Move { ids: vec![ItemId::Component(r1), ItemId::Wire(w)], by: Vec2::new(f64::MAX, 0.0) });
    refused(&mut p, Op::SetSheet { id: s, name: "x".into(), size: SheetSize::Custom { width: 0.0, height: 1.0 } });
}

#[test]
fn hostile_ids_never_panic() {
    let mut p = sample();
    let s = first_sheet(&p);
    let w = p.sheets[0].wires[0].clone();
    assert_eq!(refused(&mut p, Op::Remove { id: ItemId::Text(TextId(999)) }), DocError::NoSuchItem(ItemId::Text(TextId(999))));
    refused(&mut p, Op::Modify { item: Item::Wire(Wire { id: WireId(999), ..w.clone() }) });
    refused(&mut p, Op::Add { sheet: SheetId(999), item: Item::Wire(Wire { id: WireId(500), ..w.clone() }), index: None });
    // The same raw id under another kind is a duplicate too: ids are project-wide.
    let clash = Item::Label(Label { id: LabelId(w.id.0), at: pt(0.0, 0.0), text: "A".into(), orient: Orient::IDENTITY, global: false });
    refused(&mut p, Op::Add { sheet: s, item: clash, index: None });
    refused(&mut p, Op::Move { ids: vec![ItemId::Wire(w.id), ItemId::Bus(BusId(77))], by: Vec2::new(100.0, 0.0) });
    refused(&mut p, Op::SetTransform { id: ComponentId(w.id.0), transform: Transform::IDENTITY });
    refused(&mut p, Op::RemoveSheet { id: SheetId(42) });
    refused(&mut p, Op::SetSheet { id: SheetId(42), name: String::new(), size: SheetSize::A4 });
    let dup_sheet = Sheet::new(s, "again", SheetSize::A4);
    assert_eq!(refused(&mut p, Op::AddSheet { sheet: dup_sheet, index: None }), DocError::DuplicateSheet(s));
    let mut twice = Sheet::new(SheetId(300), "twice", SheetSize::A4);
    twice.wires = vec![Wire { id: WireId(301), ..w.clone() }, Wire { id: WireId(301), ..w.clone() }];
    refused(&mut p, Op::AddSheet { sheet: twice, index: None });
    // A clamped index instead of a panic.
    let far = Item::Wire(Wire { id: WireId(400), ..w });
    p.apply(Op::Add { sheet: s, item: far, index: Some(usize::MAX) }).unwrap();
    assert_eq!(p.sheets[0].wires.last().map(|w| w.id), Some(WireId(400)));
}

#[test]
fn hostile_items_never_panic() {
    let mut p = sample();
    let s = first_sheet(&p);
    let item = Item::Wire(Wire { id: WireId(50), points: vec![pt(0.0, 0.0)] });
    assert!(matches!(refused(&mut p, Op::Add { sheet: s, item, index: None }), DocError::Invalid(_)));
    let bus = Item::Bus(Bus { id: BusId(51), points: Vec::new() });
    refused(&mut p, Op::Add { sheet: s, item: bus, index: None });
    let text = Item::Text(Text { id: TextId(52), at: pt(0.0, 0.0), text: String::new(), size: -1.0, orient: Orient::IDENTITY });
    refused(&mut p, Op::Add { sheet: s, item: text, index: None });
}

#[test]
fn failed_batches_roll_back_and_deep_batches_are_refused() {
    let mut p = sample();
    let s = first_sheet(&p);
    let w = p.sheets[0].wires[0].id;
    let ok = Op::Move { ids: vec![ItemId::Wire(w)], by: Vec2::new(100.0, 0.0) };
    let bad = Op::Remove { id: ItemId::Wire(WireId(12345)) };
    let new_sheet = Op::AddSheet { sheet: Sheet::new(SheetId(60), "x", SheetSize::A4), index: None };
    let add_j = Op::Add { sheet: s, item: Item::Junction(Junction { id: JunctionId(61), at: pt(1.0, 1.0) }), index: None };
    refused(&mut p, Op::batch(vec![ok.clone(), new_sheet, add_j, Op::batch(vec![ok.clone(), bad])]));
    let mut deep = ok;
    for _ in 0..1000 {
        deep = Op::batch(vec![deep]);
    }
    assert_eq!(refused(&mut p, deep), DocError::BatchTooDeep);
    let mut fine = Op::batch(vec![]);
    for _ in 0..(MAX_BATCH_DEPTH - 1) {
        fine = Op::batch(vec![fine]);
    }
    assert!(p.apply(fine).is_ok());
}
