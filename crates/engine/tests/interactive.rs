//! Interactive sequences: clicks and keys in the editing window, per mode.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_doc::{ItemId, TerminalKind};
use protasena_engine::{Machine, Mode, PointerEvent, Request, Session};
use protasena_geom::Orient;
use serde_json::json;

fn click(s: &mut Session, x: f64, y: f64) -> protasena_engine::Feedback {
    s.pointer(PointerEvent::moved(p(x, y))).unwrap();
    let fb = s.pointer(PointerEvent::press(p(x, y))).unwrap();
    s.pointer(PointerEvent::release(p(x, y))).unwrap();
    fb
}

/// R1 at (0, 0) and R2 at (1000, 500) placed by clicks in Component mode.
fn two_parts() -> Session {
    let mut s = with_res();
    click(&mut s, 12.0, -7.0);
    click(&mut s, 1004.0, 493.0);
    s
}

#[test]
fn component_mode_clicks_place_parts_on_the_grid() {
    let mut s = with_res();
    s.pointer(PointerEvent::moved(p(37.0, -12.0))).unwrap();
    let ghost = s.preview().ghost.unwrap();
    assert_eq!((ghost.origin(), ghost.reference.as_str()), (p(0.0, 0.0), "R1"));
    let s2 = two_parts();
    let refs: Vec<_> = sheet(&s2).components.iter().map(|c| (c.reference.clone(), c.origin())).collect();
    assert_eq!(refs, [("R1".to_owned(), p(0.0, 0.0)), ("R2".to_owned(), p(1000.0, 500.0))]);
    assert_eq!(s2.undo_steps().len(), 2, "one undo step per click");
    // The Orientation toolbar applies to the next placement.
    exec(&mut s, "rotate-ccw", json!({}));
    click(&mut s, 3000.0, 0.0);
    assert_eq!(sheet(&s).components[0].transform.orient, Orient::rotation(1));
    // Clicking a placed part selects it instead of placing another on top.
    click(&mut s, 3000.0, 30.0);
    assert_eq!(sheet(&s).components.len(), 1);
    assert_eq!(s.selection(), [ItemId::Component(sheet(&s).components[0].id)]);
    let mut none = Session::new();
    assert!(click(&mut none, 0.0, 0.0).message.unwrap().contains("pick"));
}

#[test]
fn pin_to_pin_wire_is_routed_orthogonally() {
    let mut s = two_parts();
    s.pointer(PointerEvent::moved(p(203.0, 4.0))).unwrap();
    assert!(matches!(s.hover(), Some(protasena_engine::Hit::PinEnd { .. })));
    s.pointer(PointerEvent::press(p(203.0, 4.0))).unwrap();
    assert!(matches!(s.machine(), Machine::Wire(_)));
    s.pointer(PointerEvent::moved(p(590.0, 310.0))).unwrap();
    assert_eq!(s.preview().wire, [p(200.0, 0.0), p(600.0, 0.0), p(600.0, 300.0)], "follows the mouse, leaving the pin along its axis");
    s.pointer(PointerEvent::moved(p(797.0, 502.0))).unwrap();
    let expected = [p(200.0, 0.0), p(500.0, 0.0), p(500.0, 500.0), p(800.0, 500.0)];
    assert_eq!(s.preview().wire, expected);
    s.pointer(PointerEvent::press(p(797.0, 502.0))).unwrap();
    assert!(matches!(s.machine(), Machine::Idle));
    assert_eq!(sheet(&s).wires[0].points, expected);
    assert!(nets(&s).contains(&["R1.2", "R2.1"].iter().map(|x| x.to_string()).collect()));
    exec(&mut s, "undo", json!({}));
    assert!(sheet(&s).wires.is_empty());
}

#[test]
fn manual_corners_with_the_autorouter_off() {
    let mut s = two_parts();
    s.key("W").unwrap();
    assert!(!s.autorouter());
    s.pointer(PointerEvent::press(p(200.0, 0.0))).unwrap();
    s.pointer(PointerEvent::press(p(300.0, 0.0))).unwrap();
    s.pointer(PointerEvent::moved(p(300.0, 700.0))).unwrap();
    assert_eq!(s.preview().wire, [p(200.0, 0.0), p(300.0, 0.0), p(300.0, 700.0)]);
    s.pointer(PointerEvent::press(p(300.0, 700.0))).unwrap();
    s.pointer(PointerEvent::press(p(800.0, 500.0))).unwrap();
    assert_eq!(sheet(&s).wires[0].points, [p(200.0, 0.0), p(300.0, 0.0), p(300.0, 700.0), p(800.0, 500.0)]);
}

#[test]
fn escape_cancels_and_double_click_ends_in_free_space() {
    let mut s = two_parts();
    s.pointer(PointerEvent::press(p(200.0, 0.0))).unwrap();
    s.pointer(PointerEvent::moved(p(400.0, 400.0))).unwrap();
    assert!(s.key("Escape").unwrap().handled);
    assert!(matches!(s.machine(), Machine::Idle) && sheet(&s).wires.is_empty());
    s.pointer(PointerEvent::press(p(200.0, 0.0))).unwrap();
    s.pointer(PointerEvent::right_press(p(400.0, 0.0))).unwrap();
    assert!(sheet(&s).wires.is_empty(), "the right button cancels too");
    s.pointer(PointerEvent::press(p(200.0, 0.0))).unwrap();
    s.pointer(PointerEvent::press(p(400.0, 0.0))).unwrap();
    s.pointer(PointerEvent::press(p(400.0, 0.0))).unwrap();
    assert_eq!(sheet(&s).wires[0].points, [p(200.0, 0.0), p(400.0, 0.0)], "a second click on the same point ends the wire");
    s.pointer(PointerEvent::double_click(p(400.0, 0.0))).unwrap();
    assert_eq!(sheet(&s).wires.len(), 1);
}

#[test]
fn selection_mode_drag_keeps_wires_attached() {
    let mut s = divider();
    exec(&mut s, "set-mode", json!({ "mode": "selection" }));
    let before = nets(&s);
    let steps = s.undo_steps().len();
    s.pointer(PointerEvent::press(p(1000.0, 510.0))).unwrap();
    let r2 = ItemId::Component(sheet(&s).components[1].id);
    assert_eq!(s.selection(), [r2]);
    s.pointer(PointerEvent::moved(p(1000.0, 700.0))).unwrap();
    s.pointer(PointerEvent::moved(p(1003.0, 905.0))).unwrap();
    let preview = s.preview();
    assert!(preview.moved.iter().any(|i| i.id() == r2) && preview.moved.len() >= 2, "{preview:?}");
    s.pointer(PointerEvent::release(p(1003.0, 905.0))).unwrap();
    assert_eq!(sheet(&s).components[1].origin(), p(1000.0, 900.0));
    assert_eq!(nets(&s), before);
    assert!(all_orthogonal(&s));
    assert_eq!(s.undo_steps().len(), steps + 1);
    // A click without moving only selects.
    click(&mut s, 0.0, 10.0);
    assert_eq!(sheet(&s).components[0].origin(), p(0.0, 0.0));
}

#[test]
fn selection_box_click_and_double_click() {
    let mut s = divider();
    exec(&mut s, "set-mode", json!({ "mode": "selection" }));
    s.pointer(PointerEvent::press(p(-600.0, -300.0))).unwrap();
    s.pointer(PointerEvent::moved(p(300.0, 300.0))).unwrap();
    assert_eq!(s.preview().rect, Some([p(-600.0, -300.0), p(300.0, 300.0)]));
    s.pointer(PointerEvent::release(p(300.0, 300.0))).unwrap();
    let r1 = ItemId::Component(sheet(&s).components[0].id);
    assert!(s.selection().contains(&r1) && s.selection().len() == 3, "R1, the terminal and its wire: {:?}", s.selection());
    click(&mut s, 5000.0, 5000.0);
    assert!(s.selection().is_empty());
    let fb = s.pointer(PointerEvent::double_click(p(0.0, 20.0))).unwrap();
    assert_eq!(fb.request, Some(Request::EditComponent { id: sheet(&s).components[0].id }));
}

#[test]
fn junction_label_terminal_and_text_modes() {
    let mut s = Session::new();
    exec(&mut s, "wire", json!({ "points": [[0, 0], [1000, 0]] }));
    exec(&mut s, "set-mode", json!({ "mode": "junction-dot" }));
    click(&mut s, 503.0, 8.0);
    assert_eq!(sheet(&s).junctions[0].at, p(500.0, 0.0));
    exec(&mut s, "set-mode", json!({ "mode": "wire-label" }));
    let fb = click(&mut s, 296.0, 5.0);
    assert_eq!(fb.request, Some(Request::WireLabel { at: p(300.0, 0.0) }));
    exec(&mut s, "wire-label", json!({ "at": [300, 0], "text": "A" }));
    let fb = click(&mut s, 310.0, 40.0);
    assert!(matches!(fb.request, Some(Request::EditLabel { .. })), "{fb:?}");
    assert!(click(&mut s, 300.0, 900.0).message.is_some(), "not on a wire");
    exec(&mut s, "select-terminal", json!({ "kind": "ground" }));
    click(&mut s, 2000.0, 0.0);
    assert_eq!(sheet(&s).terminals[0].kind, TerminalKind::Ground);
    exec(&mut s, "set-mode", json!({ "mode": "text-2d" }));
    assert_eq!(click(&mut s, 0.0, 1000.0).request, Some(Request::Text { at: p(0.0, 1000.0) }));
    exec(&mut s, "set-mode", json!({ "mode": "probe" }));
    assert!(click(&mut s, 0.0, 1000.0).message.unwrap().contains("not available"));
}

#[test]
fn bus_mode_clicks_and_double_click() {
    let mut s = Session::new();
    exec(&mut s, "set-mode", json!({ "mode": "buses" }));
    click(&mut s, 0.0, 0.0);
    click(&mut s, 1000.0, 0.0);
    s.pointer(PointerEvent::moved(p(1000.0, 600.0))).unwrap();
    assert_eq!(s.preview().bus, [p(0.0, 0.0), p(1000.0, 0.0), p(1000.0, 600.0)]);
    click(&mut s, 1000.0, 600.0);
    click(&mut s, 1000.0, 600.0);
    s.pointer(PointerEvent::double_click(p(1000.0, 600.0))).unwrap();
    assert_eq!(sheet(&s).buses[0].points, [p(0.0, 0.0), p(1000.0, 0.0), p(1000.0, 600.0)]);
    assert!(matches!(s.machine(), Machine::Idle));
}

#[test]
fn undo_and_disabled_keys_leave_a_usable_machine() {
    let mut s = two_parts();
    s.pointer(PointerEvent::press(p(200.0, 0.0))).unwrap();
    s.key("Ctrl+Z").unwrap();
    assert!(matches!(s.machine(), Machine::Idle), "undo drops the wire in progress");
    assert_eq!(sheet(&s).components.len(), 1);
    let mut fresh = Session::new();
    let fb = fresh.key("Ctrl+Z").unwrap();
    assert!(!fb.handled && fb.message.as_deref() == Some("nothing to undo"));
}

#[test]
fn ctrl_click_deselect_does_not_drag() {
    let mut s = divider();
    exec(&mut s, "set-mode", json!({ "mode": "selection" }));
    exec(&mut s, "select-all", json!({}));
    let ctrl = protasena_engine::Modifiers { ctrl: true, ..Default::default() };
    s.pointer(PointerEvent { modifiers: ctrl, ..PointerEvent::press(p(1000.0, 510.0)) }).unwrap();
    s.pointer(PointerEvent::moved(p(1000.0, 900.0))).unwrap();
    s.pointer(PointerEvent::release(p(1000.0, 900.0))).unwrap();
    assert_eq!(sheet(&s).components[1].origin(), p(1000.0, 500.0));
}

#[test]
fn drafts_are_bounded() {
    let mut s = two_parts();
    s.pointer(PointerEvent::press(p(200.0, 0.0))).unwrap();
    for i in 0..100 {
        s.pointer(PointerEvent::press(p(300.0 + 100.0 * f64::from(i), 2000.0 + 100.0 * f64::from(i % 2)))).unwrap();
    }
    let Machine::Wire(draft) = s.machine() else { panic!() };
    assert_eq!(draft.corners.len(), 64);
    let via: Vec<[i32; 2]> = (0..65).map(|i| [i * 100, 3000]).collect();
    assert!(s.execute("wire", &json!({ "from": [0, 0], "to": [0, 100], "via": via })).is_err());
    exec(&mut s, "set-orientation", json!({ "angle": 90.0 }));
}

#[test]
fn keys_reach_commands_and_dialogs() {
    let mut s = divider();
    assert!(s.view().grid);
    s.key("g").unwrap();
    assert!(!s.view().grid);
    assert_eq!(s.key("P").unwrap().request, Some(Request::PickDevices));
    assert_eq!(s.key("E").unwrap().request, Some(Request::FindComponent));
    let r2 = sheet(&s).components[1].id;
    exec(&mut s, "select", json!({ "ids": [r2.0] }));
    assert_eq!(s.key("E").unwrap().request, Some(Request::EditComponent { id: r2 }));
    s.key("Delete").unwrap();
    assert_eq!(sheet(&s).components.len(), 1);
    s.key("Ctrl+Z").unwrap();
    assert_eq!(sheet(&s).components.len(), 2);
    s.key("ctrl+y").unwrap();
    assert_eq!(sheet(&s).components.len(), 1);
    s.key("F8").unwrap();
    s.key("F2").unwrap();
    assert_eq!(s.view().snap.step(), 50.0);
    assert!(!s.key("Q").unwrap().handled, "unbound keys are not used");
    assert_eq!(s.key("Ctrl+S").unwrap().request, Some(Request::SaveProjectAs));
    assert_eq!(s.key("Ctrl+N").unwrap().request, Some(Request::ConfirmDiscard { then: "new" }));
    exec(&mut s, "set-mode", json!({ "mode": "component" }));
    assert_eq!(s.mode(), Mode::Component);
}
