//! JSON forms of the selection, clipboard, block, orientation, view and file commands.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_doc::ItemId;
use protasena_engine::{Mode, Session, Snap};
use protasena_geom::Orient;
use serde_json::json;

#[test]
fn select_by_point_box_ids() {
    let mut s = divider();
    let r1 = ItemId::Component(sheet(&s).components[0].id);
    let r2 = ItemId::Component(sheet(&s).components[1].id);
    exec(&mut s, "select", json!({ "at": [0, 10] }));
    assert_eq!(s.selection(), [r1]);
    exec(&mut s, "select", json!({ "at": [1000, 500], "add": true }));
    assert_eq!(s.selection(), [r1, r2]);
    exec(&mut s, "select", json!({ "at": [1000, 500], "toggle": true }));
    assert_eq!(s.selection(), [r1]);
    exec(&mut s, "select", json!({ "box": [[-250, -100], [250, 100]] }));
    assert!(s.selection().contains(&r1) && !s.selection().contains(&r2));
    exec(&mut s, "select", json!({ "at": [5000, 5000] }));
    assert!(s.selection().is_empty());
    exec(&mut s, "select-all", json!({}));
    assert_eq!(s.selection().len(), sheet(&s).item_ids().count());
    exec(&mut s, "clear-selection", json!({}));
    assert!(s.selection().is_empty());
    assert!(s.execute("select", &json!({})).is_err());
}

#[test]
fn copy_paste_cut() {
    let mut s = with_res();
    exec(&mut s, "place-component", json!({ "at": [0, 0] }));
    exec(&mut s, "select-all", json!({}));
    exec(&mut s, "copy", json!({}));
    assert_eq!(s.clipboard().items.len(), 1);
    let r = exec(&mut s, "paste", json!({ "at": [0, 1000] }));
    assert_eq!(r["ids"].as_array().unwrap().len(), 1);
    let pasted = &sheet(&s).components[1];
    assert_eq!((pasted.reference.as_str(), pasted.origin()), ("R2", p(0.0, 1000.0)));
    assert_eq!(s.selection(), [ItemId::Component(pasted.id)]);
    exec(&mut s, "cut", json!({}));
    assert_eq!(sheet(&s).components.len(), 1);
    exec(&mut s, "paste", json!({ "at": [500, 0] }));
    assert_eq!(sheet(&s).components[1].reference, "R2");
}

#[test]
fn block_tools() {
    let mut s = divider();
    let before = nets(&s);
    let r2 = sheet(&s).components[1].id;
    exec(&mut s, "select", json!({ "ids": [r2.0] }));
    exec(&mut s, "block-move", json!({ "by": [0, 300] }));
    assert_eq!(sheet(&s).components[1].origin(), p(1000.0, 800.0));
    assert_eq!(nets(&s), before, "wires follow a block move");
    assert!(all_orthogonal(&s));
    exec(&mut s, "block-rotate", json!({}));
    assert_eq!(sheet(&s).components[1].transform.orient, Orient::rotation(1));
    assert_eq!(nets(&s), before);
    let r = exec(&mut s, "block-copy", json!({ "by": [2000, 0] }));
    assert_eq!(r["ids"].as_array().unwrap().len(), 1);
    assert_eq!(sheet(&s).components[2].reference, "R3");
    exec(&mut s, "block-delete", json!({}));
    assert_eq!(sheet(&s).components.len(), 2);
    assert!(s.execute("block-rotate", &json!({})).is_err(), "nothing selected after block delete");
}

#[test]
fn move_is_rigid_and_drag_follows() {
    let mut s = divider();
    let r2 = sheet(&s).components[1].id;
    let wire_before = sheet(&s).wires[0].points.clone();
    exec(&mut s, "move", json!({ "ids": [r2.0], "by": [100, 0] }));
    assert_eq!(sheet(&s).wires[0].points, wire_before, "move leaves wires alone");
    exec(&mut s, "undo", json!({}));
    let before = nets(&s);
    exec(&mut s, "drag", json!({ "ids": [r2.0], "to": [1000, -500] }));
    assert_eq!(nets(&s), before);
    assert!(all_orthogonal(&s));
    assert!(s.execute("drag", &json!({ "ids": [r2.0] })).is_err());
}

#[test]
fn orientation_toolbar() {
    let mut s = with_res();
    exec(&mut s, "rotate-cw", json!({}));
    assert_eq!(s.placement(), Orient::rotation(-1));
    exec(&mut s, "rotate-ccw", json!({}));
    exec(&mut s, "mirror-x", json!({}));
    assert_eq!(s.placement(), Orient::MIRROR_X);
    exec(&mut s, "set-orientation", json!({ "angle": 180 }));
    assert_eq!(s.placement(), Orient::rotation(2));
    assert!(s.execute("set-orientation", &json!({ "angle": 45 })).is_err());
    exec(&mut s, "place-component", json!({ "at": [0, 0] }));
    assert_eq!(sheet(&s).components[0].transform.orient, Orient::rotation(2));
    let id = sheet(&s).components[0].id;
    exec(&mut s, "rotate-ccw", json!({ "ids": [id.0] }));
    assert_eq!(sheet(&s).components[0].transform.orient, Orient::rotation(3));
    exec(&mut s, "mirror-y", json!({ "ids": [id.0] }));
    assert!(sheet(&s).components[0].transform.orient.is_mirrored());
    assert_eq!(sheet(&s).components[0].origin(), p(0.0, 0.0));
}

#[test]
fn view_state() {
    let mut s = Session::new();
    assert!(s.view().grid);
    exec(&mut s, "toggle-grid", json!({}));
    assert!(!s.view().grid);
    exec(&mut s, "snap-10th", json!({}));
    assert_eq!(s.view().snap, Snap::Th10);
    exec(&mut s, "snap-500th", json!({}));
    assert_eq!(s.view().snap.step(), 500.0);
    let z = s.view().zoom;
    exec(&mut s, "zoom-in", json!({ "at": [100, 100] }));
    assert_eq!((s.view().zoom, s.view().center), (z * 2.0, p(100.0, 100.0)));
    exec(&mut s, "zoom-out", json!({}));
    assert_eq!(s.view().zoom, z);
    exec(&mut s, "set-viewport", json!({ "width": 1169.3, "height": 826.8 }));
    exec(&mut s, "zoom-all", json!({}));
    assert!((s.view().zoom - 0.095).abs() < 1e-6);
    assert_eq!(s.view().center, p(0.0, 0.0));
    exec(&mut s, "zoom-area", json!({ "from": [0, 0], "to": [100, 100] }));
    assert_eq!(s.view().center, p(50.0, 50.0));
    exec(&mut s, "pan", json!({ "by": [10, 0] }));
    exec(&mut s, "center-at-cursor", json!({ "at": [7, 7] }));
    assert_eq!(s.view().center, p(7.0, 7.0));
    exec(&mut s, "toggle-false-origin", json!({ "at": [500, 500] }));
    assert_eq!(s.view().false_origin, Some(p(500.0, 500.0)));
    exec(&mut s, "toggle-false-origin", json!({}));
    assert_eq!(s.view().false_origin, None);
    exec(&mut s, "toggle-x-cursor", json!({}));
    assert!(s.view().x_cursor);
    let r = s.view().redraws;
    exec(&mut s, "redraw", json!({}));
    assert_eq!(s.view().redraws, r + 1);
    assert!(s.autorouter());
    exec(&mut s, "toggle-autorouter", json!({}));
    assert!(!s.autorouter());
    exec(&mut s, "set-mode", json!({ "mode": "selection" }));
    assert_eq!(s.mode(), Mode::Selection);
    assert!(s.execute("set-mode", &json!({ "mode": "nope" })).is_err());
    assert!(s.undo_steps().is_empty(), "view changes are not undo steps");
    assert!(!s.is_dirty());
}

#[test]
fn files_round_trip_and_netlists() {
    let dir = scratch("files");
    let mut s = divider();
    assert!(s.is_dirty());
    assert!(s.execute("save", &json!({})).is_err(), "no file yet");
    let path = dir.join("divider.psproj");
    exec(&mut s, "save-as", json!({ "path": path.to_str().unwrap() }));
    assert!(!s.is_dirty() && s.path() == Some(path.as_path()));
    let saved = design(s.project());
    exec(&mut s, "junction", json!({ "at": [0, 3000] }));
    assert!(s.execute("new", &json!({})).is_err(), "unsaved changes");
    exec(&mut s, "save", json!({}));
    exec(&mut s, "new", json!({ "name": "fresh" }));
    assert_eq!(s.project().name, "fresh");
    assert!(sheet(&s).components.is_empty() && s.undo_steps().is_empty());
    exec(&mut s, "open", json!({ "path": path.to_str().unwrap() }));
    assert_eq!(sheet(&s).components.len(), 2);
    assert_ne!(design(s.project()), saved, "the junction was saved");
    let r = exec(&mut s, "export-netlist", json!({}));
    assert_eq!(r["path"], dir.join("divider.json").to_str().unwrap());
    let r = exec(&mut s, "export-netlist", json!({ "format": "spice", "path": dir.join("d.cir").to_str().unwrap() }));
    assert_eq!(r["parts"], 2);
    assert!(std::fs::read_to_string(dir.join("d.cir")).unwrap().contains("R1"));
    assert!(s.execute("open", &json!({ "path": dir.join("missing.psproj").to_str().unwrap() })).is_err());
    assert!(s.execute("export-netlist", &json!({ "format": "xml" })).is_err());
    exec(&mut s, "junction", json!({ "at": [0, 4000] }));
    exec(&mut s, "open", json!({ "path": path.to_str().unwrap(), "discard": true }));
}
