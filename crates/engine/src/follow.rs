//! Moving and turning items with the wires attached to them following (Proteus's drag keeps
//! wires connected): the planner works out every changed item first and returns
//! [`Op::Modify`]s, so a refused edit changes nothing.

use std::collections::HashSet;

use protasena_doc::{Bus, BusEntry, Component, DocError, Graphic, Item, ItemId, Junction, Label, Op, Sheet, Terminal, Text, Wire, WireId};
use protasena_geom::{Orient, Pt, Transform, Vec2};
use protasena_library::Library;

use crate::bodies::visible_pins;
use crate::scene::Scene;
use crate::wiring::{SAME, stretch};

/// `item` under the rigid transform `t` (rotation or mirror about a pivot, plus a shift).
/// Components compose their placement; labels, terminals and texts turn their orientation.
///
/// # Errors
/// [`DocError::NonFinite`] when a result is not finite.
pub fn transformed(item: &Item, t: Transform) -> Result<Item, DocError> {
    let o = t.orient;
    let pts = |v: &[Pt]| v.iter().map(|&p| t.apply_pt(p)).collect::<Vec<_>>();
    let turned = |x: Orient| x.then(o);
    let out = match item {
        Item::Component(c) => {
            let mut c: Component = c.clone();
            c.transform = c.transform.then(t);
            c.reference_label.offset = o.apply_vec(c.reference_label.offset);
            c.value_label.offset = o.apply_vec(c.value_label.offset);
            Item::Component(c)
        }
        Item::Wire(w) => Item::Wire(Wire { id: w.id, points: pts(&w.points) }),
        Item::Bus(b) => Item::Bus(Bus { id: b.id, points: pts(&b.points) }),
        Item::Junction(j) => Item::Junction(Junction { id: j.id, at: t.apply_pt(j.at) }),
        Item::Label(l) => Item::Label(Label { at: t.apply_pt(l.at), orient: turned(l.orient), ..l.clone() }),
        Item::Terminal(x) => Item::Terminal(Terminal { at: t.apply_pt(x.at), orient: turned(x.orient), ..x.clone() }),
        Item::Text(x) => Item::Text(Text { at: t.apply_pt(x.at), orient: turned(x.orient), ..x.clone() }),
        Item::BusEntry(e) => Item::BusEntry(BusEntry { id: e.id, bus_end: t.apply_pt(e.bus_end), wire_end: t.apply_pt(e.wire_end) }),
        Item::Graphic(g) => Item::Graphic(Graphic { id: g.id, shape: g.shape.map(|p| t.apply_pt(p)) }),
    };
    out.check()?;
    Ok(out)
}

/// An item turned by `o` about its own origin (a component's origin, a text's anchor, a
/// polyline's or graphic's first point).
pub fn turned_in_place(item: &Item, o: Orient) -> Result<Item, DocError> {
    let pivot = match item {
        Item::Component(c) => c.origin(),
        Item::Label(l) => l.at,
        Item::Terminal(t) => t.at,
        Item::Text(t) => t.at,
        Item::Junction(j) => j.at,
        Item::Wire(w) => w.points.first().copied().unwrap_or(Pt::ORIGIN),
        Item::Bus(b) => b.points.first().copied().unwrap_or(Pt::ORIGIN),
        Item::BusEntry(e) => e.bus_end,
        // A circle turns about its centre, the other shapes about their first point (it stays
        // on the grid).
        Item::Graphic(g) => g.shape.points().first().copied().unwrap_or(Pt::ORIGIN),
    };
    transformed(item, Transform::about(pivot, o))
}

/// The points other wires attach to, before and after an item changes.
fn anchor_moves(library: &Library, old: &Item, new: &Item) -> Vec<(Pt, Pt)> {
    match (old, new) {
        (Item::Component(a), Item::Component(b)) => {
            let after = visible_pins(library, b);
            visible_pins(library, a).into_iter().filter_map(|p| after.iter().find(|q| q.number == p.number).map(|q| (p.at, q.at))).collect()
        }
        (Item::Wire(a), Item::Wire(b)) => {
            let ends = |w: &Wire| [w.points.first().copied(), w.points.last().copied()];
            ends(a).into_iter().zip(ends(b)).filter_map(|(x, y)| Some((x?, y?))).collect()
        }
        (Item::Junction(a), Item::Junction(b)) => vec![(a.at, b.at)],
        (Item::Terminal(a), Item::Terminal(b)) => vec![(a.at, b.at)],
        (Item::BusEntry(a), Item::BusEntry(b)) => vec![(a.wire_end, b.wire_end)],
        _ => Vec::new(),
    }
}

/// The ops that change `moves` (pairs of an item and its new state) and make the wires
/// attached to them follow: a wire with both ends on moved points moves with them; a wire
/// with one end on a moved point is stretched with an elbow so it stays orthogonal, or routed
/// again when the stretched shape would cross a body or another connection point.
pub fn follow_ops(sheet: &Sheet, library: &Library, moves: &[(Item, Item)], grid: f64) -> Vec<Op> {
    let moved: HashSet<ItemId> = moves.iter().map(|(a, _)| a.id()).collect();
    let anchors: Vec<(Pt, Pt)> = moves.iter().flat_map(|(a, b)| anchor_moves(library, a, b)).filter(|(a, b)| a.distance(*b) > SAME).collect();
    let lookup = |p: Option<&Pt>| p.and_then(|p| anchors.iter().find(|(a, _)| a.distance(*p) <= SAME).map(|(a, b)| (*a, *b)));
    let mut ops: Vec<Op> = moves.iter().map(|(_, b)| Op::Modify { item: b.clone() }).collect();
    let mut stretched: Vec<(WireId, Vec<Pt>)> = Vec::new();
    for w in sheet.wires.iter().filter(|w| !moved.contains(&ItemId::Wire(w.id))) {
        let start = lookup(w.points.first());
        let end = lookup(w.points.last());
        let points = match (start, end) {
            (None, None) => continue,
            (Some((a0, b0)), Some((a1, b1))) if same_vec(b0 - a0, b1 - a1) => w.points.iter().map(|&p| p + (b0 - a0)).collect(),
            _ => {
                let mut pts = w.points.clone();
                if let Some((_, to)) = start {
                    pts = stretch(&pts, true, to, grid);
                }
                if let Some((_, to)) = end {
                    pts = stretch(&pts, false, to, grid);
                }
                stretched.push((w.id, pts));
                continue;
            }
        };
        ops.push(Op::Modify { item: Item::Wire(Wire { id: w.id, points }) });
    }
    // A stretched wire that would run through a body or over another connection point is
    // routed again.
    let reshaped: Vec<ItemId> = stretched.iter().map(|(id, _)| ItemId::Wire(*id)).collect();
    let scene = Scene::after(sheet, library, moves, &reshaped);
    for (id, pts) in stretched {
        let points = if scene.is_bad(&pts) { scene.reroute(&pts, grid).unwrap_or(pts) } else { pts };
        ops.push(Op::Modify { item: Item::Wire(Wire { id, points }) });
    }
    ops
}

fn same_vec(a: Vec2, b: Vec2) -> bool {
    (a - b).length() <= SAME
}

/// The items of `ids` on `sheet` (ids on other sheets are skipped).
pub fn items_on(sheet: &Sheet, ids: &[ItemId]) -> Vec<Item> {
    ids.iter().filter_map(|&id| sheet.item(id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use protasena_doc::{ComponentId, DeviceRef};

    #[test]
    fn dragging_a_part_stretches_its_wire() {
        let lib = Library::builtin();
        let r = Component::new(ComponentId(1), DeviceRef::new("", "RES"), "R1", "");
        let mut sheet = Sheet::default();
        sheet.components.push(r.clone());
        sheet.wires.push(Wire { id: WireId(2), points: vec![Pt::new(200.0, 0.0), Pt::new(800.0, 0.0)] });
        let old = Item::Component(r);
        let new = old.translated(Vec2::new(0.0, 300.0)).unwrap();
        let ops = follow_ops(&sheet, &lib, &[(old, new)], 100.0);
        assert_eq!(ops.len(), 2);
        let Op::Modify { item: Item::Wire(w) } = &ops[1] else { panic!("{ops:?}") };
        assert_eq!(w.points, [Pt::new(200.0, 300.0), Pt::new(500.0, 300.0), Pt::new(500.0, 0.0), Pt::new(800.0, 0.0)]);
    }

    #[test]
    fn turning_in_place_keeps_the_origin() {
        let c = Component::new(ComponentId(1), DeviceRef::new("", "RES"), "R1", "").placed(Transform::translation(Vec2::new(500.0, 100.0)));
        let Item::Component(t) = turned_in_place(&Item::Component(c), Orient::rotation(1)).unwrap() else { panic!() };
        assert_eq!(t.origin(), Pt::new(500.0, 100.0));
        assert_eq!(t.transform.orient, Orient::rotation(1));
    }
}
