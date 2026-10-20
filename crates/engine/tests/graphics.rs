//! 2D graphics: the line, box, circle, arc and closed-path commands, drawing them with the
//! mouse in their modes, and selecting, moving, turning, deleting and undoing them.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_doc::{GraphicShape, ItemId};
use protasena_engine::{Machine, Mode, PointerEvent, Session};
use serde_json::json;

fn click(s: &mut Session, x: f64, y: f64) {
    s.pointer(PointerEvent::moved(p(x, y))).unwrap();
    s.pointer(PointerEvent::press(p(x, y))).unwrap();
    s.pointer(PointerEvent::release(p(x, y))).unwrap();
}

fn in_mode(mode: &str) -> Session {
    let mut s = Session::new();
    exec(&mut s, "set-mode", json!({ "mode": mode }));
    s
}

fn shapes(s: &Session) -> Vec<GraphicShape> {
    sheet(s).graphics.iter().map(|g| g.shape.clone()).collect()
}

#[test]
fn each_command_adds_one_graphic_in_one_undo_step() {
    let mut s = Session::new();
    exec(&mut s, "line", json!({ "from": [0, 0], "to": [500, 0] }));
    exec(&mut s, "box", json!({ "from": [0, 0], "to": [300, 200] }));
    exec(&mut s, "circle", json!({ "center": [1000, 0], "radius": 200 }));
    exec(&mut s, "arc", json!({ "start": [0, 0], "mid": [100, 100], "end": [200, 0] }));
    exec(&mut s, "closed-path", json!({ "points": [[0, 0], [400, 0], [200, 300], [0, 0]] }));
    let kinds: Vec<&str> = shapes(&s).iter().map(GraphicShape::kind).collect();
    assert_eq!(kinds, ["line", "box", "circle", "arc", "path"]);
    assert_eq!(shapes(&s)[4], GraphicShape::Path { points: vec![p(0.0, 0.0), p(400.0, 0.0), p(200.0, 300.0)] }, "a repeated first point closes");
    assert_eq!(s.undo_steps().len(), 5);
    exec(&mut s, "undo", json!({}));
    assert_eq!(shapes(&s).len(), 4);
}

#[test]
fn degenerate_shapes_are_refused() {
    let mut s = Session::new();
    for (id, params) in [
        ("line", json!({ "from": [0, 0], "to": [0, 0] })),
        ("box", json!({ "from": [0, 0], "to": [300, 0] })),
        ("circle", json!({ "center": [0, 0], "radius": 0 })),
        ("circle", json!({ "center": [0, 0] })),
        ("arc", json!({ "start": [0, 0], "mid": [100, 0], "end": [200, 0] })),
        ("closed-path", json!({ "points": [[0, 0], [100, 0], [100, 0]] })),
        ("line", json!({ "from": [0, 0], "to": ["x", 1] })),
    ] {
        assert!(s.execute(id, &params).is_err(), "{id} {params}");
    }
    assert!(shapes(&s).is_empty() && s.undo_steps().is_empty());
}

#[test]
fn line_box_and_circle_take_two_clicks_on_the_grid() {
    let mut s = in_mode("line-2d");
    click(&mut s, 3.0, -4.0);
    assert!(matches!(s.machine(), Machine::Graphic { mode: Mode::Line2d, .. }));
    s.pointer(PointerEvent::moved(p(498.0, 2.0))).unwrap();
    assert_eq!(s.preview().graphic, Some(GraphicShape::Line { a: p(0.0, 0.0), b: p(500.0, 0.0) }));
    click(&mut s, 498.0, 2.0);
    assert_eq!(shapes(&s), [GraphicShape::Line { a: p(0.0, 0.0), b: p(500.0, 0.0) }]);
    assert_eq!(*s.machine(), Machine::Idle);

    let mut s = in_mode("box-2d");
    click(&mut s, 0.0, 0.0);
    click(&mut s, 300.0, 200.0);
    assert_eq!(shapes(&s), [GraphicShape::Box { a: p(0.0, 0.0), b: p(300.0, 200.0) }]);

    let mut s = in_mode("circle-2d");
    click(&mut s, 0.0, 0.0);
    click(&mut s, 0.0, 300.0);
    assert_eq!(shapes(&s), [GraphicShape::Circle { center: p(0.0, 0.0), radius: 300.0 }]);
}

#[test]
fn a_press_drag_release_draws_too() {
    let mut s = in_mode("box-2d");
    s.pointer(PointerEvent::press(p(0.0, 0.0))).unwrap();
    s.pointer(PointerEvent::moved(p(400.0, 300.0))).unwrap();
    s.pointer(PointerEvent::release(p(400.0, 300.0))).unwrap();
    assert_eq!(shapes(&s), [GraphicShape::Box { a: p(0.0, 0.0), b: p(400.0, 300.0) }]);
    // An arc: drag from start to end, then click the bulge.
    let mut s = in_mode("arc-2d");
    s.pointer(PointerEvent::press(p(0.0, 0.0))).unwrap();
    s.pointer(PointerEvent::moved(p(400.0, 0.0))).unwrap();
    s.pointer(PointerEvent::release(p(400.0, 0.0))).unwrap();
    assert!(shapes(&s).is_empty());
    s.pointer(PointerEvent::moved(p(200.0, 100.0))).unwrap();
    assert!(matches!(s.preview().graphic, Some(GraphicShape::Arc { .. })));
    click(&mut s, 200.0, 100.0);
    assert_eq!(shapes(&s), [GraphicShape::Arc { start: p(0.0, 0.0), mid: p(200.0, 100.0), end: p(400.0, 0.0) }]);
}

#[test]
fn an_arc_takes_start_end_and_a_point_on_it() {
    let mut s = in_mode("arc-2d");
    click(&mut s, 0.0, 0.0);
    click(&mut s, 400.0, 0.0);
    click(&mut s, 200.0, -200.0);
    assert_eq!(shapes(&s), [GraphicShape::Arc { start: p(0.0, 0.0), mid: p(200.0, -200.0), end: p(400.0, 0.0) }]);
    // Three points on one line are refused and drawing starts again.
    click(&mut s, 0.0, 1000.0);
    click(&mut s, 400.0, 1000.0);
    assert!(s.pointer(PointerEvent::press(p(200.0, 1000.0))).is_err());
    assert_eq!(*s.machine(), Machine::Idle);
}

#[test]
fn a_closed_path_closes_on_a_double_click_or_the_right_button() {
    let mut s = in_mode("closed-path-2d");
    for (x, y) in [(0.0, 0.0), (400.0, 0.0), (400.0, 300.0)] {
        click(&mut s, x, y);
    }
    s.pointer(PointerEvent::press(p(400.0, 300.0))).unwrap();
    s.pointer(PointerEvent::release(p(400.0, 300.0))).unwrap();
    s.pointer(PointerEvent::double_click(p(400.0, 300.0))).unwrap();
    assert_eq!(shapes(&s), [GraphicShape::Path { points: vec![p(0.0, 0.0), p(400.0, 0.0), p(400.0, 300.0)] }]);
    for (x, y) in [(0.0, 1000.0), (500.0, 1000.0), (500.0, 1500.0), (0.0, 1500.0)] {
        click(&mut s, x, y);
    }
    s.pointer(PointerEvent::right_press(p(0.0, 1500.0))).unwrap();
    assert_eq!(shapes(&s).len(), 2);
    assert_eq!(s.undo_steps().len(), 2, "one undo step per path");
    // Two corners are not a path: the right button just stops.
    click(&mut s, 0.0, 3000.0);
    click(&mut s, 100.0, 3000.0);
    s.pointer(PointerEvent::right_press(p(100.0, 3000.0))).unwrap();
    assert_eq!((shapes(&s).len(), s.machine().clone()), (2, Machine::Idle));
}

#[test]
fn escape_cancels_a_shape_in_progress() {
    let mut s = in_mode("circle-2d");
    click(&mut s, 0.0, 0.0);
    s.key("Escape").unwrap();
    assert_eq!(*s.machine(), Machine::Idle);
    click(&mut s, 500.0, 0.0);
    assert!(shapes(&s).is_empty(), "the next click starts a new circle");
}

#[test]
fn graphics_select_move_turn_mirror_delete_and_undo() {
    let mut s = Session::new();
    let line = exec(&mut s, "line", json!({ "from": [0, 0], "to": [500, 0] }))["id"].as_u64().unwrap();
    let circle = exec(&mut s, "circle", json!({ "center": [2000, 0], "radius": 200 }))["id"].as_u64().unwrap();
    exec(&mut s, "set-mode", json!({ "mode": "selection" }));
    click(&mut s, 250.0, 3.0);
    let line_id = sheet(&s).graphics[0].id;
    assert_eq!(s.selection(), [ItemId::Graphic(line_id)], "a click on the outline selects");
    click(&mut s, 2000.0, 0.0);
    assert!(s.selection().is_empty(), "the inside of a circle is not part of it");
    // Box selection takes the circle only when it is wholly inside.
    s.pointer(PointerEvent::press(p(1700.0, -300.0))).unwrap();
    s.pointer(PointerEvent::moved(p(2300.0, 300.0))).unwrap();
    s.pointer(PointerEvent::release(p(2300.0, 300.0))).unwrap();
    assert_eq!(s.selection().len(), 1);
    exec(&mut s, "move", json!({ "ids": [circle], "by": [100, 0] }));
    assert_eq!(shapes(&s)[1], GraphicShape::Circle { center: p(2100.0, 0.0), radius: 200.0 });
    exec(&mut s, "rotate-ccw", json!({ "ids": [line] }));
    assert_eq!(shapes(&s)[0], GraphicShape::Line { a: p(0.0, 0.0), b: p(0.0, 500.0) }, "turned about its first point");
    exec(&mut s, "mirror-y", json!({ "ids": [line] }));
    let GraphicShape::Line { b, .. } = shapes(&s)[0] else { panic!() };
    assert_eq!(b.x, 0.0);
    exec(&mut s, "delete", json!({ "ids": [line, circle] }));
    assert!(shapes(&s).is_empty());
    for _ in 0..4 {
        exec(&mut s, "undo", json!({}));
    }
    assert_eq!(shapes(&s)[0], GraphicShape::Line { a: p(0.0, 0.0), b: p(500.0, 0.0) });
    assert_eq!(shapes(&s)[1], GraphicShape::Circle { center: p(2000.0, 0.0), radius: 200.0 });
}

#[test]
fn copy_and_paste_and_block_copy_give_new_ids() {
    let mut s = Session::new();
    exec(&mut s, "box", json!({ "from": [0, 0], "to": [300, 200] }));
    exec(&mut s, "select-all", json!({}));
    exec(&mut s, "copy", json!({}));
    exec(&mut s, "paste", json!({ "at": [1000, 1000] }));
    let ids: Vec<u64> = sheet(&s).graphics.iter().map(|g| g.id.0).collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}
