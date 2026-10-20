//! Driving the window like a user: modes, Pick Devices, Edit Component, shortcuts and the
//! editing window.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable;
use protasena_doc::{ComponentId, ItemId};
use protasena_engine::Mode;
use protasena_ui_egui::dialogs::Dialog;
use serde_json::json;

#[test]
fn clicking_a_mode_button_switches_mode_and_disabled_ones_do_nothing() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.get_by_label("Selection Mode").click();
    h.run_steps(3);
    assert_eq!(h.state().session.mode(), Mode::Selection);
    h.get_by_label("Terminals Mode").click();
    h.run_steps(3);
    assert_eq!(h.state().session.mode(), Mode::Terminals);
    h.get_by_label("Graph Mode").click();
    h.run_steps(3);
    assert_eq!(h.state().session.mode(), Mode::Terminals, "Graph Mode is not built: its button is disabled");
}

#[test]
fn pick_devices_searches_and_adds_the_part_to_the_selector() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    press(&mut h, Modifiers::NONE, Key::P);
    assert!(matches!(h.state().dialog, Some(Dialog::PickDevices(_))), "P opens Pick Devices");
    type_text(&mut h, "cap");
    let Some(Dialog::PickDevices(d)) = &h.state().dialog else { panic!("closed") };
    let lib = h.state().session.library();
    let first = d.model.results.first().map(|&i| lib.devices()[i].name.clone());
    assert_eq!(first.as_deref(), Some("CAP"));
    assert!(d.model.results.len() >= 2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    assert_eq!(h.state().session.device().map(|d| d.name.as_str()), Some("CAP"));
    assert_eq!(find(&h, "CAP", "list-item").selected, Some(true));
}

#[test]
fn a_click_in_the_editing_window_places_the_chosen_part() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.state_mut().run("pick-device", json!({ "pick": "RES" })).unwrap();
    h.run_steps(2);
    let canvas = region(&h, "Editing window");
    let at = canvas.center();
    h.hover_at(at);
    h.run_steps(2);
    h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    let sheet = h.state().session.sheet().unwrap();
    assert_eq!(sheet.components.len(), 1);
    assert_eq!(sheet.components[0].reference, "R1");
}

#[test]
fn edit_component_changes_the_value() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    let r = h.state_mut().run("place-component", json!({ "device": "RES", "at": [0, 0] })).unwrap();
    let id = ComponentId(r["id"].as_u64().unwrap());
    h.state_mut().run("select", json!({ "ids": [id.0] })).unwrap();
    h.run_steps(2);
    press(&mut h, Modifiers::NONE, Key::E);
    let Some(Dialog::EditComponent(d)) = &mut h.state_mut().dialog else { panic!("E opens Edit Component") };
    assert_eq!((d.reference.as_str(), d.value.as_str()), ("R1", "10k"));
    d.value = "22k".into();
    d.hide_value = true;
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    let c = h.state().session.project().component(id).unwrap();
    assert_eq!(c.value, "22k");
    assert!(!c.value_label.visible);
    assert_eq!(h.state().session.selection(), [ItemId::Component(id)]);
}

#[test]
fn shortcuts_reach_their_commands() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    assert!(h.state().session.view().grid);
    press(&mut h, Modifiers::NONE, Key::G);
    assert!(!h.state().session.view().grid, "G toggles the grid");
    let zoom = h.state().session.view().zoom;
    press(&mut h, Modifiers::NONE, Key::F6);
    assert!((h.state().session.view().zoom - zoom * 2.0).abs() < 1e-9, "F6 zooms in");
    press(&mut h, Modifiers::NONE, Key::F8);
    assert!(h.state().session.view().zoom < zoom * 2.0, "F8 shows the whole sheet");
    press(&mut h, Modifiers::NONE, Key::W);
    assert!(!h.state().session.autorouter(), "W toggles the autorouter");
    press(&mut h, Modifiers::NONE, Key::F2);
    assert_eq!(h.state().session.view().snap, protasena_engine::Snap::Th50);
    h.state_mut().run("junction", json!({ "at": [0, 0] })).unwrap();
    press(&mut h, Modifiers::CTRL, Key::Z);
    assert!(h.state().session.sheet().unwrap().junctions.is_empty(), "Ctrl+Z undoes");
    press(&mut h, Modifiers::CTRL, Key::Y);
    assert_eq!(h.state().session.sheet().unwrap().junctions.len(), 1, "Ctrl+Y redoes");
}

#[test]
fn menu_items_run_commands_and_show_check_marks() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.get_by_label("View").click();
    h.run_steps(3);
    assert_eq!(find(&h, "Toggle Grid", "menu-item").selected, Some(true));
    assert_eq!(find(&h, "Snap 0.1in", "menu-item").selected, Some(true));
    assert!(!find(&h, "Toolbar Configuration", "menu-item").enabled, "items without a command are disabled");
    h.get_by_label("Toggle X-Cursor").click();
    h.run_steps(3);
    assert!(h.state().session.view().x_cursor);
    h.get_by_label("Design").click();
    h.run_steps(3);
    assert_eq!(find(&h, "1. Root sheet 1", "menu-item").selected, Some(true), "the sheet list checks the current sheet");
}

#[test]
fn the_window_title_names_the_project() {
    let h = harness(1200.0, 800.0, 1.0);
    assert_eq!(h.state().window_title(), "UNTITLED - Protasena - Schematic Capture");
}
