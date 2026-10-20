//! Libraries from the window: the user's KiCad libraries in System Settings, pin alternates in
//! Pick Devices and Edit Component.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use std::path::{Path, PathBuf};

use common::*;
use egui::Modifiers;
use egui_kittest::kittest::Queryable;
use protasena_ui_egui::dialogs::Dialog;

/// Our own one-symbol KiCad library in a fresh folder.
fn kicad_folder(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ui-libraries").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let text = r#"(kicad_symbol_lib (version 20241209)
  (symbol "SENSOR9" (property "Reference" "U" (at 0 0 0)) (property "Value" "SENSOR9" (at 0 0 0))
    (property "Description" "my humidity sensor" (at 0 0 0))
    (symbol "SENSOR9_1_1" (rectangle (start -2.54 2.54) (end 2.54 -2.54) (stroke (width 0.254)) (fill (type background)))
      (pin power_in line (at 0 5.08 270) (length 2.54) (name "VDD") (number "1"))
      (pin output line (at 5.08 0 180) (length 2.54) (name "OUT") (number "2")
        (alternate "SDA" bidirectional line) (alternate "TX" output line)))))"#;
    std::fs::write(dir.join("MyParts.kicad_sym"), text).unwrap();
    dir
}

pub fn open_menu_item(h: &mut egui_kittest::Harness<'_, protasena_ui_egui::App>, menu: &str, item: &str) {
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
fn system_settings_lists_the_kicad_libraries_and_imports_them() {
    let dir = kicad_folder("settings");
    let mut h = harness(1600.0, 1000.0, 1.0);
    open_menu_item(&mut h, "System", "System Settings");
    find(&h, "KiCad libraries kicad_libraries 1", "field");
    let Some(Dialog::Settings(d)) = &mut h.state_mut().dialog else { panic!("System Settings opens") };
    d.typed.insert("kicad_libraries".into(), format!("{}\n\n", dir.display()));
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert!(h.state().dialog.is_none(), "{:?}", h.state().dialog.as_ref().map(|d| d.title()));
    let s = &h.state().session;
    assert_eq!(s.settings().kicad_libraries, [dir.display().to_string()]);
    assert_eq!(s.library().get("SENSOR9").unwrap().library, "MyParts");
}

#[test]
fn pick_devices_and_edit_component_list_pin_alternates() {
    let dir = kicad_folder("alternates");
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.state_mut().run("system-settings", serde_json::json!({ "kicad_libraries": [dir.display().to_string()] })).unwrap();
    h.state_mut().invoke("pick-device");
    h.run_steps(2);
    let Some(Dialog::PickDevices(d)) = &mut h.state_mut().dialog else { panic!("Pick Devices opens") };
    d.model.keywords = "humidity".into();
    h.run_steps(3);
    h.get_by_label("SENSOR9").click();
    h.run_steps(3);
    find(&h, "Pin alternates", "button");
    h.state_mut().dialog = None;
    let id = h.state_mut().run("place-component", serde_json::json!({ "device": "SENSOR9", "library": "MyParts", "at": [0, 0] })).unwrap()["id"]
        .as_u64()
        .unwrap();
    h.state_mut().request(protasena_engine::Request::EditComponent { id: protasena_doc::ComponentId(id) });
    h.run_steps(3);
    let Some(Dialog::EditComponent(d)) = &h.state().dialog else { panic!("Edit Component opens") };
    assert_eq!(d.alternates.len(), 1);
    assert_eq!((d.alternates[0].number.as_str(), d.alternates[0].name.as_str()), ("2", "OUT"));
    assert_eq!(d.alternates[0].alternates, ["SDA (bidirectional)", "TX (output)"]);
    find(&h, "Pin alternates", "button");
}
