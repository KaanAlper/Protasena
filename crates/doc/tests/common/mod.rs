//! Small builders shared by the integration tests.
#![allow(dead_code)]

use protasena_doc::{Component, ComponentId, DeviceRef, Item, Junction, Label, Op, Project, SheetId, Wire};
use protasena_geom::{Orient, Pt, Transform, Vec2};

pub fn pt(x: f64, y: f64) -> Pt {
    Pt::new(x, y)
}

pub fn first_sheet(p: &Project) -> SheetId {
    p.sheets[0].id
}

/// Adds an item through an op and returns the undo op.
pub fn add(p: &mut Project, item: Item) -> Op {
    let sheet = first_sheet(p);
    p.apply(Op::Add { sheet, item, index: None }).unwrap()
}

pub fn part(p: &mut Project, device: &str, reference: &str, at: (f64, f64)) -> ComponentId {
    let id: ComponentId = p.fresh_id().unwrap();
    let c = Component::new(id, DeviceRef::new("GENERIC", device), reference, "").placed(Transform::new(Orient::IDENTITY, Vec2::new(at.0, at.1)));
    add(p, Item::Component(c));
    id
}

pub fn wire(p: &mut Project, points: &[(f64, f64)]) -> Wire {
    let w = Wire { id: p.fresh_id().unwrap(), points: points.iter().map(|&(x, y)| pt(x, y)).collect() };
    add(p, Item::Wire(w.clone()));
    w
}

pub fn junction(p: &mut Project, at: (f64, f64)) -> Junction {
    Junction { id: p.fresh_id().unwrap(), at: pt(at.0, at.1) }
}

pub fn label(p: &mut Project, at: (f64, f64), text: &str) -> Label {
    Label { id: p.fresh_id().unwrap(), at: pt(at.0, at.1), text: text.to_owned(), orient: Orient::IDENTITY }
}
