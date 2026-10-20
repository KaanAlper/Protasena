//! Dragging and turning parts keeps their wires attached: the netlist before equals the
//! netlist after, and the wires stay orthogonal.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_engine::Session;
use serde_json::json;

/// A resistor, a capacitor and an LED in a chain with a ground terminal, wired by routing.
fn chain() -> Session {
    let mut s = divider();
    exec(&mut s, "place-component", json!({ "at": [2500, 500], "device": "CAP" }));
    exec(&mut s, "wire", json!({ "from": { "component": "R2", "pin": "2" }, "to": { "component": "C1", "pin": "1" } }));
    exec(&mut s, "place-component", json!({ "at": [2500, -1000], "device": "LED", "orient": 90 }));
    exec(&mut s, "wire", json!({ "from": { "component": "C1", "pin": "2" }, "to": { "component": "D1", "pin": "1" } }));
    s
}

#[test]
fn dragging_any_part_anywhere_keeps_the_nets() {
    let base = chain();
    let before = nets(&base);
    assert!(before.len() >= 3, "{before:?}");
    assert!(all_orthogonal(&base));
    for part in ["R1", "R2", "C1", "D1"] {
        for by in [[0, 300], [300, 0], [-500, 200], [700, -900], [100, 100], [0, -1500]] {
            let mut s = base.clone();
            let id = s.sheet().unwrap().components.iter().find(|c| c.reference == part).unwrap().id;
            exec(&mut s, "drag", json!({ "ids": [id.0], "by": by }));
            assert_eq!(nets(&s), before, "drag {part} by {by:?}");
            assert!(all_orthogonal(&s), "drag {part} by {by:?}: {:?}", sheet(&s).wires);
            exec(&mut s, "undo", json!({}));
            assert_eq!(design(s.project()), design(base.project()));
        }
    }
}

#[test]
fn turning_parts_keeps_the_nets() {
    let base = chain();
    let before = nets(&base);
    for part in ["R1", "R2", "C1"] {
        for cmd in ["rotate-cw", "rotate-ccw", "mirror-x", "mirror-y"] {
            let mut s = base.clone();
            let id = s.sheet().unwrap().components.iter().find(|c| c.reference == part).unwrap().id;
            exec(&mut s, cmd, json!({ "ids": [id.0] }));
            assert_eq!(nets(&s), before, "{cmd} {part}");
        }
    }
}

#[test]
fn dragging_several_parts_together() {
    let mut s = chain();
    let before = nets(&s);
    let ids: Vec<u64> = s.sheet().unwrap().components.iter().filter(|c| c.reference != "R1").map(|c| c.id.0).collect();
    exec(&mut s, "drag", json!({ "ids": ids, "by": [400, 800] }));
    assert_eq!(nets(&s), before);
    assert!(all_orthogonal(&s));
    exec(&mut s, "select-all", json!({}));
    exec(&mut s, "block-rotate", json!({ "angle": 180 }));
    assert_eq!(nets(&s), before, "everything turned together");
}
