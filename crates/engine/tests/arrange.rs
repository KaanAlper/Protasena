//! Edit ▸ Align Objects, Send To Back, Bring To Front and Tidy Design.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_doc::ItemId;
use protasena_engine::{EngineError, Outcome, Request, Session, command_for_key};
use serde_json::{Value, json};

fn ids(v: &Value) -> Vec<u64> {
    v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect()
}

fn graphics() -> (Session, Vec<u64>) {
    let mut s = Session::new();
    let mut out = Vec::new();
    for (x, y) in [(0, 0), (300, 1000), (-500, 200)] {
        let r = exec(&mut s, "box", json!({ "from": [x, y], "to": [x + 200, y + 100] }));
        out.push(r["id"].as_u64().unwrap());
    }
    (s, out)
}

#[test]
fn align_lines_objects_up_on_the_grid_and_undoes_exactly() {
    let (mut s, g) = graphics();
    exec(&mut s, "select", json!({ "ids": g }));
    let before = design(s.project());
    exec(&mut s, "align", json!({ "edge": "left" }));
    let lefts: Vec<f64> = sheet(&s).graphics.iter().map(|x| x.shape.bounds().unwrap().min.x).collect();
    assert_eq!(lefts, [-500.0, -500.0, -500.0]);
    let after = design(s.project());
    exec(&mut s, "undo", json!({}));
    assert_eq!(design(s.project()), before);
    exec(&mut s, "redo", json!({}));
    assert_eq!(design(s.project()), after);
    exec(&mut s, "align", json!({ "edge": "top" }));
    assert!(sheet(&s).graphics.iter().all(|x| x.shape.bounds().unwrap().max.y == 1100.0));
    exec(&mut s, "align", json!({ "edge": "center-y" }));
    let r = exec(&mut s, "align", json!({ "edge": "center-y" }));
    assert_eq!(r["moved"], json!([]), "already aligned");
    exec(&mut s, "align", json!({ "edge": "right" }));
    assert!(sheet(&s).graphics.iter().all(|x| x.shape.bounds().unwrap().max.x == -300.0));
}

#[test]
fn aligned_parts_stay_on_the_grid_and_keep_their_wires() {
    let mut s = divider();
    let before = nets(&s);
    let parts: Vec<u64> = sheet(&s).components.iter().map(|c| c.id.0).collect();
    exec(&mut s, "select", json!({ "ids": parts }));
    exec(&mut s, "align", json!({ "edge": "bottom" }));
    let ys: Vec<f64> = sheet(&s).components.iter().map(|c| c.origin().y).collect();
    assert_eq!(ys[0], ys[1]);
    assert!(sheet(&s).components.iter().all(|c| c.origin().x % 100.0 == 0.0 && c.origin().y % 100.0 == 0.0));
    assert_eq!(nets(&s), before, "wires follow");
}

#[test]
fn send_to_back_and_bring_to_front_reorder_within_the_kind() {
    let (mut s, g) = graphics();
    let order = |s: &Session| sheet(s).graphics.iter().map(|x| x.id.0).collect::<Vec<_>>();
    let before = design(s.project());
    exec(&mut s, "select", json!({ "ids": [g[2], g[1]] }));
    exec(&mut s, "send-to-back", json!({}));
    assert_eq!(order(&s), [g[1], g[2], g[0]], "their own order kept");
    assert_eq!(s.selection().len(), 2, "still selected");
    let steps = s.undo_steps().len();
    let r = exec(&mut s, "send-to-back", json!({}));
    assert_eq!((r["unchanged"].as_bool(), s.undo_steps().len()), (Some(true), steps));
    exec(&mut s, "bring-to-front", json!({ "ids": [g[1]] }));
    assert_eq!(order(&s), [g[2], g[0], g[1]]);
    exec(&mut s, "undo", json!({}));
    exec(&mut s, "undo", json!({}));
    assert_eq!(design(s.project()), before);
    exec(&mut s, "redo", json!({}));
    exec(&mut s, "redo", json!({}));
    assert_eq!(order(&s), [g[2], g[0], g[1]]);
}

#[test]
fn tidy_removes_what_nothing_uses_in_one_step() {
    let mut s = divider();
    exec(&mut s, "pick-device", json!({ "pick": "CAP" }));
    exec(&mut s, "junction", json!({ "at": [5000, 5000] }));
    exec(&mut s, "junction", json!({ "at": [-300, 0] }));
    exec(&mut s, "text", json!({ "at": [0, 3000], "text": "keep" }));
    let before = design(s.project());
    let r = exec(&mut s, "tidy", json!({}));
    assert_eq!(r["devices_removed"], json!(["CAP"]));
    assert_eq!(ids(&r["removed"]).len(), 1, "only the stray dot");
    assert_eq!(sheet(&s).junctions.len(), 1, "the dot on the wire stays");
    assert_eq!(s.picked().iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["RES"]);
    assert!(s.device().is_none(), "the chosen device was unused");
    exec(&mut s, "undo", json!({}));
    assert_eq!(design(s.project()), before);
    let mut clean = divider();
    let steps = clean.undo_steps().len();
    exec(&mut clean, "tidy", json!({}));
    assert_eq!(clean.undo_steps().len(), steps, "nothing to tidy: no undo step");
}

#[test]
fn keys_dialogs_and_refusals() {
    let (mut s, g) = graphics();
    assert_eq!(command_for_key("Ctrl+A").map(|c| c.id), Some("align"));
    assert_eq!(command_for_key("Ctrl+B").map(|c| c.id), Some("send-to-back"));
    assert_eq!(command_for_key("Ctrl+F").map(|c| c.id), Some("bring-to-front"));
    for id in ["align", "send-to-back", "bring-to-front"] {
        assert!(matches!(s.execute(id, &json!({})), Err(EngineError::Disabled { .. })), "{id} needs a selection");
    }
    exec(&mut s, "select", json!({ "ids": [g[0]] }));
    assert_eq!(s.invoke("align").unwrap(), Outcome::Dialog(Request::Align));
    assert!(matches!(s.execute("align", &json!({})), Err(EngineError::Other(_))), "one object cannot be aligned");
    exec(&mut s, "select", json!({ "ids": g }));
    assert!(matches!(s.execute("align", &json!({ "edge": "diagonal" })), Err(EngineError::BadParams { .. })));
    assert!(matches!(s.selection()[0], ItemId::Graphic(_)));
}
