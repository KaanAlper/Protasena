//! Subcircuit mode and hierarchical design: boxes, ports, the PORTS list, Goto Child Sheet,
//! Exit to Parent Sheet, the sheet order and netlists across the hierarchy.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_doc::{ItemId, PortKind};
use protasena_engine::{Feedback, PointerEvent, Request, Session};
use serde_json::json;

fn click(s: &mut Session, x: f64, y: f64) -> Feedback {
    s.pointer(PointerEvent::moved(p(x, y))).unwrap();
    let fb = s.pointer(PointerEvent::press(p(x, y))).unwrap();
    s.pointer(PointerEvent::release(p(x, y))).unwrap();
    fb
}

/// A session with SUB1 from (1000, -500) to (2000, 500) and ports IN (left) and OUT (right).
fn with_sub() -> (Session, u64) {
    let mut s = Session::new();
    let v = exec(&mut s, "subcircuit", json!({ "from": [1000, -500], "to": [2000, 500] }));
    let id = v["id"].as_u64().unwrap();
    exec(&mut s, "subcircuit-port", json!({ "subcircuit": id, "at": [1000, 0], "name": "IN", "kind": "input" }));
    exec(&mut s, "subcircuit-port", json!({ "subcircuit": id, "at": [2000, 0], "name": "OUT", "kind": "output" }));
    (s, id)
}

#[test]
fn boxes_are_drawn_in_subcircuit_mode_and_named_in_turn() {
    let mut s = Session::new();
    exec(&mut s, "set-mode", json!({ "mode": "subcircuit" }));
    click(&mut s, 0.0, 0.0);
    click(&mut s, 1000.0, 600.0);
    s.pointer(PointerEvent::moved(p(2000.0, 0.0))).unwrap();
    s.pointer(PointerEvent::press(p(2000.0, 0.0))).unwrap();
    s.pointer(PointerEvent::moved(p(3000.0, 600.0))).unwrap();
    s.pointer(PointerEvent::release(p(3000.0, 600.0))).unwrap();
    let names: Vec<&str> = sheet(&s).subcircuits.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["SUB1", "SUB2"], "two clicks or a drag");
    assert_eq!(s.undo_steps().len(), 2);
    assert!(s.execute("subcircuit", &json!({ "from": [0, 0], "to": [0, 500] })).is_err(), "no width");
}

#[test]
fn a_click_on_an_edge_asks_for_a_port_of_the_chosen_kind() {
    let (mut s, id) = with_sub();
    exec(&mut s, "select-port", json!({ "kind": "bidir" }));
    assert_eq!(s.port_kind(), PortKind::Bidir);
    let fb = click(&mut s, 1002.0, 300.0);
    let Some(Request::SubcircuitPort { subcircuit, at }) = fb.request else { panic!("{fb:?}") };
    assert_eq!((subcircuit.0, at), (id, p(1000.0, 300.0)), "on the edge, at the snapped height");
    exec(&mut s, "subcircuit-port", json!({ "subcircuit": id, "at": [at.x, at.y], "name": "EN" }));
    let ports = &sheet(&s).subcircuits[0].ports;
    assert_eq!((ports.len(), ports[2].kind), (3, PortKind::Bidir));
    for bad in [
        json!({ "subcircuit": id, "at": [1500, 0], "name": "X" }),
        json!({ "subcircuit": id, "at": [1000, 100], "name": "IN" }),
        json!({ "subcircuit": id, "at": [1000, 0], "name": "Y" }),
        json!({ "subcircuit": id, "at": [1000, 500], "name": "Z" }),
        json!({ "subcircuit": id, "at": [1000, 200], "name": "  " }),
        json!({ "subcircuit": 99_999, "at": [1000, 200], "name": "W" }),
    ] {
        assert!(s.execute("subcircuit-port", &bad).is_err(), "{bad}");
    }
    let fb = click(&mut s, 1500.0, 0.0);
    assert!(fb.request.is_none());
    assert_eq!(s.selection(), [ItemId::Subcircuit(protasena_doc::SubcircuitId(id))], "a click inside selects it");
    let fb = s.pointer(PointerEvent::double_click(p(1500.0, 0.0))).unwrap();
    assert!(matches!(fb.request, Some(Request::EditSubcircuit { .. })));
}

#[test]
fn edit_subcircuit_renames_and_rewrites_the_ports() {
    let (mut s, id) = with_sub();
    exec(&mut s, "edit-subcircuit", json!({ "id": id, "name": "FILTER", "ports": [{ "name": "VIN", "kind": "bidir" }] }));
    let x = &sheet(&s).subcircuits[0];
    assert_eq!((x.name.as_str(), x.ports.len(), x.ports[0].name.as_str(), x.ports[0].at), ("FILTER", 1, "VIN", p(1000.0, 0.0)));
    assert!(s.execute("edit-subcircuit", &json!({ "id": id, "ports": [{ "name": "A" }, { "name": "A", "at": [2000, 100] }] })).is_err());
    assert!(s.execute("edit-subcircuit", &json!({ "id": id, "ports": [{ "name": "A" }, { "name": "B" }] })).is_err(), "a new port needs `at`");
    exec(&mut s, "undo", json!({}));
    assert_eq!(sheet(&s).subcircuits[0].name, "SUB1");
}

#[test]
fn goto_child_sheet_makes_it_once_and_exit_to_parent_comes_back() {
    let (mut s, id) = with_sub();
    let root = s.sheet_id();
    assert!(s.execute("exit-to-parent", &json!({})).is_err(), "a root sheet has no parent");
    assert!(s.execute("enter-child-sheet", &json!({})).is_err(), "nothing selected");
    exec(&mut s, "select", json!({ "ids": [id] }));
    let fb = s.key("Ctrl+C").unwrap();
    assert_eq!(fb.result.as_ref().unwrap()["created"], json!(true));
    let child = s.sheet_id();
    assert_ne!(child, root);
    assert_eq!(sheet(&s).name, "SUB1");
    assert_eq!(s.project().parent_of(child).map(|(sh, _)| sh), Some(root));
    let fb = s.key("Ctrl+X").unwrap();
    assert!(fb.result.is_some(), "{fb:?}");
    assert_eq!((s.sheet_id(), s.selection().len()), (root, 1), "back on the parent with the subcircuit selected");
    let v = exec(&mut s, "enter-child-sheet", json!({ "id": id }));
    assert_eq!((v["created"].clone(), s.sheet_id()), (json!(false), child));
    exec(&mut s, "undo", json!({}));
    assert_eq!(s.project().sheets.len(), 1, "one undo step removes the child sheet and the link");
    assert!(s.project().sheets[0].subcircuits[0].child.is_none());
    assert_eq!(s.sheet_id(), root);
}

#[test]
fn previous_and_next_sheet_follow_the_hierarchy() {
    let (mut s, id) = with_sub();
    exec(&mut s, "enter-child-sheet", json!({ "id": id }));
    let child = s.sheet_id();
    exec(&mut s, "exit-to-parent", json!({}));
    // New (Root) Sheet goes right after the current sheet, before the child in the list.
    exec(&mut s, "new-sheet", json!({ "name": "Second root" }));
    let listed: Vec<&str> = s.project().sheets.iter().map(|sh| sh.name.as_str()).collect();
    assert_eq!(listed, ["Root sheet 1", "Second root", "SUB1"]);
    let names = |s: &Session| s.project().sheet_tree().iter().map(|n| s.project().sheet(n.sheet).unwrap().name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&s), ["Root sheet 1", "SUB1", "Second root"]);
    exec(&mut s, "goto-sheet", json!({ "sheet": { "index": 0 } }));
    exec(&mut s, "next-sheet", json!({}));
    assert_eq!(s.sheet_id(), child);
    exec(&mut s, "next-sheet", json!({}));
    assert_eq!(sheet(&s).name, "Second root");
    exec(&mut s, "previous-sheet", json!({}));
    assert_eq!(s.sheet_id(), child);
}

#[test]
fn wires_follow_a_dragged_subcircuit_and_copies_have_no_child() {
    let (mut s, id) = with_sub();
    exec(&mut s, "wire", json!({ "points": [[0, 0], [1000, 0]] }));
    exec(&mut s, "drag", json!({ "ids": [id], "by": [0, 200] }));
    let w = &sheet(&s).wires[0];
    assert_eq!(w.points.last(), Some(&p(1000.0, 200.0)), "{:?}", w.points);
    exec(&mut s, "enter-child-sheet", json!({ "id": id }));
    exec(&mut s, "exit-to-parent", json!({}));
    exec(&mut s, "copy", json!({}));
    exec(&mut s, "paste", json!({ "at": [5000, 0] }));
    let subs = &sheet(&s).subcircuits;
    assert_eq!(subs.len(), 2);
    assert!(subs[0].child.is_some() && subs[1].child.is_none());
}

#[test]
fn netlists_cross_the_hierarchy() {
    let (mut s, id) = with_sub();
    exec(&mut s, "pick-device", json!({ "pick": "RES" }));
    exec(&mut s, "place-component", json!({ "at": [0, 0] }));
    exec(&mut s, "wire", json!({ "points": [[200, 0], [1000, 0]] }));
    exec(&mut s, "enter-child-sheet", json!({ "id": id }));
    exec(&mut s, "terminal", json!({ "at": [-600, 0], "kind": "input", "name": "IN" }));
    exec(&mut s, "place-component", json!({ "at": [0, 0] }));
    exec(&mut s, "wire", json!({ "points": [[-600, 0], [-200, 0]] }));
    exec(&mut s, "wire", json!({ "points": [[200, 0], [600, 0]] }));
    exec(&mut s, "wire-label", json!({ "at": [400, 0], "text": "MID" }));
    let named = named_nets(&s);
    assert!(named.iter().any(|(_, nodes)| nodes.contains("R1.2") && nodes.contains("R2.1")), "{named:?}");
    assert!(named.iter().any(|(name, _)| name == "SUB1.MID"), "{named:?}");
    let v = exec(&mut s, "erc", json!({}));
    let text = v.to_string();
    assert!(text.contains("port OUT of SUB1"), "the unused port is reported: {text}");
}
