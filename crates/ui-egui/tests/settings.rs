//! The Template and System menus from the window: their dialogs change the template and the
//! system settings, and the window honours them (keys, sheet sizes, property definitions, the
//! text editor).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use egui::{Key, Modifiers};
use egui_kittest::kittest::Queryable;
use protasena_doc::{Colour, JunctionShape};
use protasena_ui_egui::dialogs::Dialog;
use serde_json::json;

fn open_menu_item(h: &mut egui_kittest::Harness<'_, protasena_ui_egui::App>, menu: &str, item: &str) {
    h.get_by_label(menu).click();
    h.run_steps(3);
    let w = find(h, item, "menu-item");
    assert!(w.enabled, "{menu} ▸ {item} is enabled");
    let at = w.rect.center();
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run_steps(1);
    }
    h.run_steps(2);
}

#[test]
fn template_dialogs_change_the_design_s_template() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    for item in ["Set Design Colours", "Set Graphic Styles", "Set Text Styles", "Set 2D Graphics Defaults", "Set Junction Dot Style"] {
        open_menu_item(&mut h, "Template", item);
        let Some(Dialog::Settings(d)) = &h.state().dialog else { panic!("{item} opens its dialog") };
        assert_eq!(d.title, item);
        h.state_mut().dialog = None;
        h.run_steps(1);
    }
    open_menu_item(&mut h, "Template", "Set Junction Dot Style");
    let Some(Dialog::Settings(d)) = &mut h.state_mut().dialog else { panic!() };
    d.typed.insert("size".into(), "60".into());
    d.value["shape"] = json!("square");
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    let j = h.state().session.project().template.junction;
    assert_eq!((j.size, j.shape), (Some(60.0), JunctionShape::Square));
    open_menu_item(&mut h, "Template", "Set Design Colours");
    let Some(Dialog::Settings(d)) = &mut h.state_mut().dialog else { panic!() };
    d.value["paper"] = json!("#fdf6e3");
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(h.state().session.project().template.colours.paper, Colour::parse("#fdf6e3"));
    open_menu_item(&mut h, "Template", "Set Graphic Styles");
    let Some(Dialog::Settings(d)) = &mut h.state_mut().dialog else { panic!() };
    d.typed.insert("wire.width".into(), "thick".into());
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let Some(Dialog::Settings(d)) = &h.state().dialog else { panic!("a bad number keeps the dialog open") };
    assert!(d.error.is_some());
}

#[test]
fn the_keyboard_mapping_rebinds_keys_and_refuses_conflicts() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    open_menu_item(&mut h, "System", "Set Keyboard Mapping");
    let Some(Dialog::Keymap(d)) = &mut h.state_mut().dialog else { panic!("the keyboard mapping opens") };
    let zoom = d.rows.iter().position(|r| r.id == "zoom-in").unwrap();
    let redraw = d.rows.iter().position(|r| r.id == "redraw").unwrap();
    assert_eq!(d.rows[zoom].key, "F6");
    d.rows[zoom].key = "ctrl+shift+i".into();
    d.rows[redraw].key = "G".into();
    h.run_steps(2);
    assert!(!find(&h, "OK", "button").enabled, "G would run two commands");
    let Some(Dialog::Keymap(d)) = &mut h.state_mut().dialog else { panic!() };
    d.rows[redraw].key = "R".into();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none());
    assert_eq!(h.state().session.shortcut("zoom-in").as_deref(), Some("Ctrl+Shift+I"));
    assert!(h.state().session.settings().keys.len() == 1, "only the changed key is stored: {:?}", h.state().session.settings().keys);
    let before = h.state().session.view().zoom;
    press(&mut h, Modifiers::CTRL | Modifiers::SHIFT, Key::I);
    assert!(h.state().session.view().zoom > before, "the new key zooms in");
    let before = h.state().session.view().zoom;
    press(&mut h, Modifiers::NONE, Key::F6);
    assert_eq!(h.state().session.view().zoom, before, "F6 no longer does");
}

#[test]
fn sheet_sizes_property_definitions_and_the_text_editor_are_used_by_their_dialogs() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    open_menu_item(&mut h, "System", "Set Sheet Sizes");
    let Some(Dialog::List(d)) = &mut h.state_mut().dialog else { panic!() };
    d.rows.push(["Banner".into(), "30000".into(), "6000".into()]);
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    h.state_mut().invoke("sheet-properties");
    h.run_steps(2);
    let Some(Dialog::Sheet(d)) = &mut h.state_mut().dialog else { panic!() };
    let banner = d.sizes.iter().position(|z| z.0 == "Banner").unwrap();
    d.size = Some(banner);
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(h.state().session.sheet().unwrap().size.dimensions(), (30_000.0, 6_000.0));

    h.state_mut()
        .run("set-property-definitions", json!({ "definitions": [{ "name": "STOCK", "default": "N/A", "description": "Stock code" }] }))
        .unwrap();
    h.state_mut().run("set-text-editor", json!({ "font_size": 18, "monospace": true })).unwrap();
    let id = h.state_mut().run("place-component", json!({ "device": "RES", "at": [0, 0] })).unwrap()["id"].as_u64().unwrap();
    h.state_mut().request(protasena_engine::Request::EditComponent { id: protasena_doc::ComponentId(id) });
    h.run_steps(2);
    let Some(Dialog::EditComponent(d)) = &h.state().dialog else { panic!() };
    let row = d.props.iter().find(|p| p.name == "STOCK").expect("the defined property is offered");
    assert_eq!((row.value.as_str(), row.prompt.as_str()), ("N/A", "STOCK (Stock code)"));
    h.state_mut().dialog = None;
    h.state_mut().request(protasena_engine::Request::Text { at: protasena_geom::Pt::ORIGIN });
    h.run_steps(2);
    let Some(Dialog::Text(d)) = &h.state().dialog else { panic!() };
    assert_eq!(d.font, (18.0, true));
    h.state_mut().dialog = None;
    open_menu_item(&mut h, "System", "Restore Default Settings");
    assert!(matches!(h.state().dialog, Some(Dialog::RestoreDefaults)));
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(h.state().session.settings(), &protasena_engine::SystemSettings::default());
}

#[test]
fn every_system_and_template_item_asked_for_is_enabled() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.get_by_label("System").click();
    h.run_steps(3);
    for item in [
        "System Settings",
        "Set Display Options",
        "Set Keyboard Mapping",
        "Set Property Definitions",
        "Set Sheet Sizes",
        "Set Text Editor",
        "Restore Default Settings",
    ] {
        assert!(find(&h, item, "menu-item").enabled, "{item}");
    }
    for item in ["Text Viewer", "Set Animation Options", "Set Simulation Options"] {
        assert!(!find(&h, item, "menu-item").enabled, "{item} is not built");
    }
}
