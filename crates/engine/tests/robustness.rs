//! Hostile parameters and events never panic; enabled-state rules; the registry's shape and
//! Proteus's shortcuts.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use std::collections::HashSet;

use common::*;
use protasena_engine::{Button, EngineError, Mode, Modifiers, Outcome, PointerEvent, PointerKind, Session, command_specs, find_command};
use serde_json::{Value, json};

fn garbage() -> Vec<Value> {
    let long = "x".repeat(70_000);
    let many: Vec<Value> = (0..20_000).map(|i| json!([i, i])).collect();
    let mut deep = json!(1);
    for _ in 0..100 {
        deep = json!([deep]);
    }
    vec![
        json!(null),
        json!([]),
        json!("x"),
        json!(5),
        json!(true),
        json!({}),
        json!({ "at": [1e308, -1e308] }),
        json!({ "at": ["a"], "points": "x", "ids": 5, "box": [[0, 0]] }),
        json!({ "at": [0, 0, 0], "points": [[0, 0], [1e9, 0]], "by": [1e300, 0], "to": [0] }),
        json!({ "points": many, "via": [[0, 0]] }),
        json!({ "ids": [99999, -1, "x", { "component": 1 }, { "wire": "a" }], "id": [] }),
        json!({ "from": { "component": "R1" }, "to": { "pin": 5 }, "route": "yes" }),
        json!({ "from": { "component": "R2", "pin": "1" }, "to": { "component": 1e30, "pin": "2" } }),
        json!({ "angle": 45, "orient": { "angle": 91 }, "mirror": 3 }),
        json!({ "angle": 1e300, "orient": 9_007_199_254_740_993_i64 }),
        json!({ "mode": 5, "kind": [], "format": 7, "size": -1, "limit": 1e20 }),
        json!({ "text": long, "name": "y".repeat(300), "device": long }),
        json!({ "properties": { "": "x" }, "component": "R1" }),
        json!({ "properties": [1], "component": {} }),
        json!({ "sheet": { "index": 99 }, "size": { "custom": { "width": 1e300, "height": 1 } } }),
        json!({ "sheet": "nope", "size": "huge" }),
        json!({ "path": 5 }),
        json!({ "path": "" }),
        json!({ "width": 0, "height": -5 }),
        json!({ "unit": 4_294_967_296_u64, "at": [0, 0], "device": "RES" }),
        json!({ "device": "GROUND", "at": [0, 0], "unit": 99 }),
        json!({ "by": [0, 0], "ids": [] }),
        json!({ "pick": "", "keywords": deep, "category": [] }),
        json!({ "at": { "x": "1", "y": 2 }, "text": "" }),
    ]
}

fn sessions() -> Vec<Session> {
    let mut busy = divider();
    exec(&mut busy, "select-all", json!({}));
    exec(&mut busy, "copy", json!({}));
    exec(&mut busy, "new-sheet", json!({}));
    exec(&mut busy, "previous-sheet", json!({}));
    exec(&mut busy, "select-all", json!({}));
    vec![Session::new(), busy]
}

fn consistent(s: &Session, what: &str) {
    assert!(s.project().issues().is_empty(), "{what}: {:?}", s.project().issues());
    assert!(s.sheet().is_some(), "{what}: no current sheet");
}

#[test]
fn hostile_params_never_panic() {
    let params = garbage();
    for base in sessions() {
        for spec in command_specs() {
            let mut s = base.clone();
            for p in &params {
                let r = s.execute(spec.id, p);
                assert!(!matches!(r, Err(EngineError::Internal { .. })), "{} {p}: {r:?}", spec.id);
                consistent(&s, spec.id);
            }
        }
    }
}

#[test]
fn hostile_command_ids_and_keys_never_panic() {
    let mut s = divider();
    for id in ["", " ", "UNDO", "wire\0", &"z".repeat(10_000), "pläce", "set-mode "] {
        let _ = s.execute(id, &json!({}));
        let _ = s.invoke(id);
    }
    assert!(matches!(s.execute("nope", &json!({})), Err(EngineError::UnknownCommand(_))));
    for key in ["", "Escape", "ctrl+", "+", "F99", "Ctrl+Alt+Shift+Z", "Delete", "W", "Page-Down", "Page-Up", &"k".repeat(5000)] {
        let _ = s.key(key);
        consistent(&s, key);
    }
}

#[test]
fn hostile_pointer_events_never_panic() {
    let kinds = [PointerKind::Move, PointerKind::Press, PointerKind::Release, PointerKind::DoubleClick];
    let buttons = [Button::Left, Button::Right, Button::Middle];
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = move |n: u64| {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng % n
    };
    for mode in Mode::ALL {
        let mut s = divider();
        exec(&mut s, "set-mode", json!({ "mode": mode.id() }));
        for bad in [f64::NAN, f64::INFINITY, -1e300] {
            assert!(s.pointer(PointerEvent::press(p(bad, 0.0))).is_err());
        }
        for step in 0..300 {
            let at = p((next(30) as f64 - 10.0) * 100.0 + next(3) as f64, (next(20) as f64 - 10.0) * 100.0);
            let kind = kinds[next(4) as usize];
            let ev = PointerEvent {
                kind,
                at,
                button: buttons[next(3) as usize],
                modifiers: Modifiers { shift: next(5) == 0, ctrl: next(5) == 0, alt: false },
            };
            let r = s.pointer(ev);
            assert!(!matches!(r, Err(EngineError::Internal { .. })), "{mode:?} step {step}: {r:?}");
            if next(25) == 0 {
                let _ = s.key(["Escape", "Delete", "Ctrl+Z", "Ctrl+Y", "W", "F7", "F6"][next(7) as usize]);
            }
            let _ = s.preview();
            consistent(&s, mode.id());
        }
    }
}

#[test]
fn enabled_state_rules() {
    let mut s = Session::new();
    let off = |s: &Session, id: &str| s.commands().into_iter().find(|c| c.id == id).map(|c| !c.enabled).unwrap();
    for id in ["undo", "redo", "clear-selection", "cut", "copy", "paste", "block-copy", "block-move", "block-rotate", "block-delete", "remove-sheet"]
    {
        assert!(off(&s, id), "{id} should be disabled in a new project");
        assert!(matches!(s.execute(id, &json!({})), Err(EngineError::Disabled { .. })), "{id}");
        assert!(matches!(s.invoke(id), Err(EngineError::Disabled { .. })), "{id}");
    }
    for id in ["new", "pick-device", "select-all", "toggle-grid", "zoom-all", "erc", "new-sheet", "rotate-cw", "set-mode"] {
        assert!(!off(&s, id), "{id} should be enabled");
    }
    exec(&mut s, "junction", json!({ "at": [0, 0] }));
    assert!(!off(&s, "undo") && off(&s, "redo"));
    exec(&mut s, "select-all", json!({}));
    for id in ["clear-selection", "cut", "copy", "block-copy", "block-move", "block-rotate", "block-delete"] {
        assert!(!off(&s, id), "{id} with a selection");
    }
    exec(&mut s, "copy", json!({}));
    assert!(!off(&s, "paste"));
    exec(&mut s, "undo", json!({}));
    assert!(off(&s, "undo") && !off(&s, "redo"));
    assert!(s.selection().is_empty(), "undo dropped the selected junction");
    exec(&mut s, "new-sheet", json!({}));
    assert!(!off(&s, "remove-sheet"));
    let info = s.commands().into_iter().find(|c| c.id == "undo").unwrap();
    assert!(info.enabled && info.disabled_reason.is_none());
    assert_eq!(Session::new().commands().into_iter().find(|c| c.id == "undo").unwrap().disabled_reason.as_deref(), Some("nothing to undo"));
}

#[test]
fn registry_shape() {
    let mut ids = HashSet::new();
    let mut keys = HashSet::new();
    for c in command_specs() {
        assert!(ids.insert(c.id), "duplicate id {}", c.id);
        assert!(!c.id.is_empty() && c.id.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'), "{}", c.id);
        assert!(!c.label.is_empty() && !c.params.is_empty(), "{}", c.id);
        if let Some(k) = c.shortcut {
            assert!(keys.insert(k.to_ascii_lowercase()), "shortcut {k} used twice");
        }
        assert_eq!(find_command(&c.id.to_ascii_uppercase()).map(|f| f.id), Some(c.id));
    }
    assert!(ids.len() >= 60, "{}", ids.len());
}

#[test]
fn proteus_shortcuts_and_menus() {
    let expected = [
        ("new", "Ctrl+N"),
        ("open", "Ctrl+O"),
        ("save", "Ctrl+S"),
        ("undo", "Ctrl+Z"),
        ("redo", "Ctrl+Y"),
        ("edit-component", "E"),
        ("redraw", "R"),
        ("toggle-grid", "G"),
        ("toggle-false-origin", "O"),
        ("toggle-x-cursor", "X"),
        ("snap-10th", "Ctrl+F1"),
        ("snap-50th", "F2"),
        ("snap-100th", "F3"),
        ("snap-500th", "F4"),
        ("center-at-cursor", "F5"),
        ("zoom-in", "F6"),
        ("zoom-out", "F7"),
        ("zoom-all", "F8"),
        ("toggle-autorouter", "W"),
        ("pick-device", "P"),
        ("previous-sheet", "Page-Up"),
        ("next-sheet", "Page-Down"),
    ];
    for (id, key) in expected {
        assert_eq!(find_command(id).and_then(|c| c.shortcut), Some(key), "{id}");
    }
    // Proteus binds these to other things (Exit to Parent Sheet, View Simulation Log, Align
    // Objects, Bring To Front): no clipboard or select-all keys.
    for key in ["Ctrl+X", "Ctrl+V", "Ctrl+A", "Ctrl+F", "Ctrl+C"] {
        assert!(protasena_engine::command_for_key(key).is_none(), "{key}");
    }
    assert_eq!(find_command("undo").unwrap().menu, ["Edit", "Undo Changes"]);
    assert_eq!(find_command("annotate").unwrap().menu, ["Tool", "Global Annotator"]);
    let mut s = Session::new();
    assert!(matches!(s.invoke("toggle-grid").unwrap(), Outcome::Done(_)));
    assert!(matches!(s.invoke("design-properties").unwrap(), Outcome::Dialog(_)));
}

#[test]
fn large_selections_stay_fast() {
    let mut s = Session::new();
    let points: Vec<Value> = (0..3000).map(|i| json!([i * 10, 0])).collect();
    exec(&mut s, "wire", json!({ "points": points }));
    for i in 0..2000 {
        exec(&mut s, "junction", json!({ "at": [i * 10, 500] }));
    }
    let start = std::time::Instant::now();
    exec(&mut s, "select", json!({ "box": [[-10, 400], [30_000, 600]] }));
    assert_eq!(s.selection().len(), 2000);
    exec(&mut s, "copy", json!({}));
    exec(&mut s, "paste", json!({ "at": [0, 5000] }));
    exec(&mut s, "select-all", json!({}));
    exec(&mut s, "drag", json!({ "by": [100, 100] }));
    exec(&mut s, "undo", json!({}));
    assert!(start.elapsed().as_secs() < 20, "{:?}", start.elapsed());
    consistent(&s, "large");
}
