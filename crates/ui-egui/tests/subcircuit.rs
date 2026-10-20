//! Subcircuit mode from the window: the PORTS list, a port added by clicking a subcircuit's
//! edge, Edit Subcircuit and its child sheet, Exit to Parent Sheet and the sheet tree.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable;
use protasena_doc::PortKind;
use protasena_engine::Mode;
use protasena_geom::Pt;
use protasena_ui_egui::dialogs::Dialog;
use serde_json::json;

fn click_sheet(h: &mut egui_kittest::Harness<'_, protasena_ui_egui::App>, at: Pt) {
    let x = h.state().canvas.xform.unwrap();
    let pos = x.to_screen(at);
    h.hover_at(pos);
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(1);
    }
    h.run_steps(2);
}

#[test]
fn subcircuit_mode_places_ports_of_the_kind_chosen_in_the_ports_list() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.get_by_label("Subcircuit Mode").click();
    h.run_steps(3);
    assert_eq!(h.state().session.mode(), Mode::Subcircuit);
    for name in ["DEFAULT", "INPUT", "OUTPUT", "BIDIR"] {
        find(&h, name, "list-item");
    }
    h.get_by_label("INPUT").click();
    h.run_steps(3);
    assert_eq!(h.state().session.port_kind(), PortKind::Input);
    h.state_mut().run("subcircuit", json!({ "from": [-1000, -1000], "to": [1000, 1000] })).unwrap();
    h.run_steps(2);
    click_sheet(&mut h, Pt::new(-1000.0, 300.0));
    let Some(Dialog::Port(d)) = &mut h.state_mut().dialog else { panic!("a click on the edge asks for the port") };
    assert_eq!(d.kind, PortKind::Input);
    d.name = "DATA".into();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let ports = &h.state().session.sheet().unwrap().subcircuits[0].ports;
    assert_eq!((ports.len(), ports[0].name.as_str(), ports[0].kind), (1, "DATA", PortKind::Input));
}

#[test]
fn edit_subcircuit_makes_the_child_sheet_and_exit_to_parent_comes_back() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    let v = h.state_mut().run("subcircuit", json!({ "from": [-1000, -1000], "to": [1000, 1000] })).unwrap();
    h.state_mut().run("set-mode", json!({ "mode": "selection" })).unwrap();
    h.run_steps(2);
    assert!(!find(&h, "Exit to Parent Sheet", "tool").enabled, "a root sheet has no parent");
    let pos = h.state().canvas.xform.unwrap().to_screen(Pt::new(0.0, 0.0));
    h.hover_at(pos);
    h.run_steps(2);
    h.event(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.event(egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.state_mut().request(protasena_engine::Request::EditSubcircuit { id: protasena_doc::SubcircuitId(v["id"].as_u64().unwrap()) });
    h.run_steps(3);
    assert!(matches!(h.state().dialog, Some(Dialog::Subcircuit(_))));
    h.get_by_label("Create Child Sheet").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    assert_eq!(h.state().session.sheet().unwrap().name, "SUB1");
    assert!(find(&h, "Exit to Parent Sheet", "tool").enabled);
    h.get_by_label("Design").click();
    h.run_steps(3);
    assert!(find(&h, "Exit to Parent Sheet", "menu-item").enabled);
    assert!(
        widgets(&h).iter().any(|w| w.role == "menu-item" && w.label.contains("SUB1 (SUB1)") && w.label.starts_with("2.")),
        "the child is listed under its parent"
    );
    press(&mut h, Modifiers::NONE, Key::Escape);
    press(&mut h, Modifiers::CTRL, Key::X);
    assert_eq!(h.state().session.sheet().unwrap().name, "Root sheet 1");
    assert_eq!(h.state().session.selection().len(), 1);
}
