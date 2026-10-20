//! Tool ▸ Search & Tag (T) and Property Assignment Tool (A) from the window.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable;
use protasena_ui_egui::dialogs::Dialog;
use serde_json::json;

fn three_resistors(h: &mut egui_kittest::Harness<'_, protasena_ui_egui::App>) {
    for (x, v) in [(0, "10k"), (1000, "4k7"), (2000, "1k")] {
        h.state_mut().run("place-component", json!({ "device": "RES", "at": [x, 0], "value": v })).unwrap();
    }
    h.run_steps(2);
}

#[test]
fn t_tags_by_property_and_a_assigns_to_the_tagged_parts() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    three_resistors(&mut h);
    press(&mut h, Modifiers::NONE, Key::T);
    let Some(Dialog::SearchTag(d)) = &mut h.state_mut().dialog else { panic!("T opens Search & Tag") };
    assert_eq!(d.property, "REF");
    d.property = "VALUE".into();
    d.value = "k".into();
    d.op = 5; // Contains
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(h.state().session.selection().len(), 3);
    press(&mut h, Modifiers::NONE, Key::A);
    let Some(Dialog::PropertyAssign(d)) = &mut h.state_mut().dialog else { panic!("A opens the Property Assignment Tool") };
    assert_eq!(d.apply_to, 0, "the tagged parts, as there are some");
    d.string = "TOLERANCE=5%".into();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    assert!(h.state().session.project().components().all(|(_, c)| c.properties.get("TOLERANCE") == Some("5%")));
}

#[test]
fn the_tool_menu_and_the_design_toolbar_reach_both_tools() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    assert!(find(&h, "Search & Tag", "tool").enabled);
    assert!(find(&h, "Property Assignment Tool", "tool").enabled);
    h.get_by_label("Tool").click();
    h.run_steps(3);
    assert!(find(&h, "Search & Tag", "menu-item").enabled);
    let at = find(&h, "Property Assignment Tool", "menu-item").rect.center();
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(1);
    }
    h.run_steps(2);
    let Some(Dialog::PropertyAssign(d)) = &mut h.state_mut().dialog else { panic!("the menu item opens the tool") };
    assert_eq!(d.apply_to, 1, "nothing tagged: the sheet");
    d.string = "X=1".into();
    d.count = "one".into();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let Some(Dialog::PropertyAssign(d)) = &h.state().dialog else { panic!("a bad count keeps it open") };
    assert!(d.error.is_some());
}
