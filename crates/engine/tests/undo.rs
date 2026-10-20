//! Undo and redo are exact after every command that changes the design, and each such command
//! is exactly one undo step; the other commands record none.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_engine::{Session, command_specs};
use serde_json::{Value, json};

/// Commands that change the design, with their parameters on the prepared session.
fn cases(s: &Session) -> Vec<(&'static str, Value)> {
    let r1 = s.sheet().unwrap().components[0].id.0;
    let label_wire = s.sheet().unwrap().wires[0].points[0];
    vec![
        ("place-component", json!({ "at": [3000, 0] })),
        ("wire", json!({ "from": { "component": "R1", "pin": "1" }, "to": [-1000, 1500] })),
        ("wire", json!({ "points": [[0, 2000], [500, 2000]] })),
        ("junction", json!({ "at": [0, 3000] })),
        ("wire-label", json!({ "at": [label_wire.x, label_wire.y], "text": "N1" })),
        ("terminal", json!({ "at": [0, -2000], "kind": "output", "name": "OUT" })),
        ("bus", json!({ "points": [[0, -3000], [2000, -3000]] })),
        ("text", json!({ "at": [0, -3500], "text": "note" })),
        ("edit-component", json!({ "component": "R1", "value": "1M", "properties": { "X": "1" } })),
        ("annotate", json!({ "mode": "total" })),
        ("design-properties", json!({ "title": "T" })),
        ("sheet-properties", json!({ "name": "S", "size": "a2" })),
        ("new-sheet", json!({})),
        ("delete", json!({ "ids": [r1] })),
        ("cut", json!({})),
        ("paste", json!({ "at": [4000, 4000] })),
        ("block-copy", json!({ "by": [0, 2000] })),
        ("block-move", json!({ "by": [0, -300] })),
        ("block-rotate", json!({ "angle": 270, "mirror": true })),
        ("block-delete", json!({})),
        ("move", json!({ "by": [100, 100] })),
        ("drag", json!({ "by": [300, 300] })),
        ("rotate-cw", json!({})),
        ("rotate-ccw", json!({})),
        ("mirror-x", json!({})),
        ("mirror-y", json!({})),
    ]
}

/// The divider with R2 selected and a copy of it on the clipboard.
fn prepared() -> Session {
    let mut s = divider();
    exec(&mut s, "annotate", json!({}));
    let r2 = s.sheet().unwrap().components[1].id.0;
    exec(&mut s, "select", json!({ "ids": [r2] }));
    exec(&mut s, "copy", json!({}));
    exec(&mut s, "edit-component", json!({ "component": "R2", "reference": "R?" }));
    exec(&mut s, "select", json!({ "ids": [r2] }));
    s
}

#[test]
fn undo_and_redo_are_exact_after_every_command() {
    let base = prepared();
    for (id, params) in cases(&base) {
        let mut s = base.clone();
        let before = design(s.project());
        let steps = s.undo_steps().len();
        exec(&mut s, id, params.clone());
        let after = design(s.project());
        assert_ne!(before, after, "{id} {params} changed nothing");
        assert_eq!(s.undo_steps().len(), steps + 1, "{id} is one undo step");
        exec(&mut s, "undo", json!({}));
        assert_eq!(design(s.project()), before, "undo of {id}");
        exec(&mut s, "redo", json!({}));
        assert_eq!(design(s.project()), after, "redo of {id}");
        exec(&mut s, "undo", json!({}));
        assert_eq!(design(s.project()), before, "second undo of {id}");
        assert!(s.project().issues().is_empty(), "{id}: {:?}", s.project().issues());
    }
}

#[test]
fn a_long_session_undoes_to_the_start() {
    let mut s = prepared();
    let start = design(s.project());
    let mut states = vec![start.clone()];
    for (id, params) in cases(&s.clone()).into_iter().filter(|(id, _)| !matches!(*id, "delete" | "cut" | "block-delete")) {
        if s.execute(id, &params).is_ok() {
            states.push(design(s.project()));
        }
    }
    assert!(states.len() > 15, "{}", states.len());
    for expected in states.iter().rev().skip(1) {
        exec(&mut s, "undo", json!({}));
        assert_eq!(&design(s.project()), expected);
    }
    assert_eq!(design(s.project()), start);
    while s.execute("redo", &json!({})).is_ok() {}
    assert_eq!(&design(s.project()), states.last().unwrap());
}

#[test]
fn other_commands_record_no_undo_step() {
    let changing: Vec<&str> = cases(&prepared()).iter().map(|(id, _)| *id).chain(["undo", "redo", "remove-sheet", "new", "open"]).collect();
    for spec in command_specs().iter().filter(|c| !changing.contains(&c.id)) {
        let mut s = prepared();
        let before = design(s.project());
        let steps = s.undo_steps().len();
        let _ = s.execute(spec.id, &json!({}));
        assert_eq!(s.undo_steps().len(), steps, "{}", spec.id);
        assert_eq!(design(s.project()), before, "{} changed the design", spec.id);
    }
}

#[test]
fn a_new_change_clears_redo() {
    let mut s = prepared();
    exec(&mut s, "junction", json!({ "at": [0, 0] }));
    exec(&mut s, "undo", json!({}));
    assert_eq!(s.redo_steps().len(), 1);
    exec(&mut s, "junction", json!({ "at": [100, 0] }));
    assert!(s.redo_steps().is_empty());
    assert!(s.execute("redo", &json!({})).is_err());
    assert_eq!(s.undo_steps().last().unwrap().label, "Junction Dot");
}
