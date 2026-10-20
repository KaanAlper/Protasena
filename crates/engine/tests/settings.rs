//! Template settings (per design, undoable, saved in the project) and system settings (per
//! user, saved to the settings file): the Template and System menu commands.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use common::*;
use protasena_doc::{Colour, JunctionShape};
use protasena_engine::Session;
use protasena_io::{SystemSettings, load_settings};
use serde_json::json;

#[test]
fn template_commands_patch_their_group_in_one_undo_step() {
    let mut s = Session::new();
    let v = exec(&mut s, "set-design-colours", json!({ "paper": "#fffff0", "highlight": "#ff00ff" }));
    assert_eq!(v["changed"], json!(true));
    assert_eq!(s.project().template.colours.paper, Colour::parse("#fffff0"));
    exec(&mut s, "set-design-colours", json!({ "paper": null }));
    assert_eq!((s.project().template.colours.paper, s.project().template.colours.highlight), (None, Colour::parse("#ff00ff")));
    exec(&mut s, "set-graphic-styles", json!({ "wire": { "width": 16, "colour": "#008000" }, "component": { "fill": "#ffffe0" } }));
    exec(&mut s, "set-text-styles", json!({ "label": { "height": 120 } }));
    exec(&mut s, "set-2d-graphics-defaults", json!({ "fill": "#e0e0ff" }));
    exec(&mut s, "set-junction-style", json!({ "size": 60, "shape": "diamond" }));
    let t = s.project().template;
    assert_eq!((t.graphic_styles.wire.width, t.text_styles.label.height, t.junction.shape), (Some(16.0), Some(120.0), JunctionShape::Diamond));
    let steps = s.undo_steps().len();
    assert_eq!(exec(&mut s, "set-junction-style", json!({}))["changed"], json!(false));
    assert_eq!(s.undo_steps().len(), steps, "no change, no undo step");
    for (id, bad) in [
        ("set-design-colours", json!({ "paper": "white" })),
        ("set-design-colours", json!({ "carpet": "#000000" })),
        ("set-graphic-styles", json!({ "wire": { "width": -1 } })),
        ("set-graphic-styles", json!({ "wire": { "width": 1e9 } })),
        ("set-text-styles", json!({ "label": 5 })),
        ("set-junction-style", json!({ "shape": "star" })),
    ] {
        assert!(s.execute(id, &bad).is_err(), "{id} {bad}");
    }
    assert_eq!(s.project().template, t, "refused patches change nothing");
    exec(&mut s, "set-junction-style", json!({ "reset": true }));
    assert_eq!(s.project().template.junction, Default::default());
    exec(&mut s, "undo", json!({}));
    assert_eq!(s.project().template.junction.shape, JunctionShape::Diamond);
    assert!(s.is_dirty());
}

#[test]
fn the_template_is_saved_with_the_design() {
    let dir = scratch("template-save");
    let path = dir.join("t.psproj");
    let mut s = divider();
    exec(&mut s, "set-junction-style", json!({ "shape": "square" }));
    exec(&mut s, "save-as", json!({ "path": path.display().to_string() }));
    let mut t = Session::new();
    exec(&mut t, "open", json!({ "path": path.display().to_string() }));
    assert_eq!(t.project().template.junction.shape, JunctionShape::Square);
}

#[test]
fn keys_can_be_rebound_and_a_key_never_runs_two_commands() {
    let mut s = Session::new();
    exec(&mut s, "set-keyboard-mapping", json!({ "bindings": { "zoom-in": "ctrl+shift+i", "zoom-out": "" } }));
    assert_eq!(s.shortcut("zoom-in").as_deref(), Some("Ctrl+Shift+I"));
    assert_eq!(s.shortcut("zoom-out"), None);
    let zoom = s.view().zoom;
    s.key("Ctrl+Shift+I").unwrap();
    assert!(s.view().zoom > zoom, "the new key runs the command");
    let fb = s.key("F6").unwrap();
    assert!(fb.result.is_none() && !fb.handled, "the old key does nothing now");
    let e = s.execute("set-keyboard-mapping", &json!({ "bindings": { "redraw": "G" } })).unwrap_err().to_string();
    assert!(e.contains("G") && e.contains("redraw") && e.contains("toggle-grid"), "{e}");
    // Swapping two keys in one call is fine.
    exec(&mut s, "set-keyboard-mapping", json!({ "bindings": { "redraw": "G", "toggle-grid": "R" } }));
    assert_eq!((s.shortcut("redraw").as_deref(), s.shortcut("toggle-grid").as_deref()), (Some("G"), Some("R")));
    for bad in [
        json!({ "bindings": { "nope": "F9" } }),
        json!({ "bindings": { "undo": "Hyper+Z" } }),
        json!({ "bindings": { "undo": "Escape" } }),
        json!({ "bindings": { "undo": 5 } }),
        json!({ "bindings": [] }),
    ] {
        assert!(s.execute("set-keyboard-mapping", &bad).is_err(), "{bad}");
    }
    exec(&mut s, "set-keyboard-mapping", json!({ "bindings": { "zoom-in": null } }));
    assert_eq!(s.shortcut("zoom-in").as_deref(), Some("F6"));
    exec(&mut s, "set-keyboard-mapping", json!({ "reset": true }));
    assert!(s.settings().keys.is_empty());
    assert!(!s.is_dirty(), "system settings are not part of the design");
}

#[test]
fn system_settings_are_saved_to_the_settings_file_and_read_back() {
    let dir = scratch("system-settings");
    let path = dir.join("cfg").join("settings.json");
    let mut s = Session::new();
    assert!(s.set_settings(SystemSettings::default(), Some(path.clone())).is_empty());
    let v = exec(&mut s, "set-display-options", json!({ "grid_lines": true, "pick_pixels": 12 }));
    assert_eq!(v["saved"], json!(true));
    let tol = s.view().pick_tolerance();
    exec(&mut s, "set-keyboard-mapping", json!({ "bindings": { "zoom-in": "Ctrl+I" } }));
    exec(&mut s, "set-sheet-sizes", json!({ "sizes": [{ "name": "Wide", "width": 20000, "height": 5000 }] }));
    exec(&mut s, "set-property-definitions", json!({ "definitions": [{ "name": "STOCK", "default": "-" }] }));
    exec(&mut s, "set-text-editor", json!({ "monospace": true }));
    exec(&mut s, "system-settings", json!({ "library_dir": "/opt/pack" }));
    let (back, warnings) = load_settings(&path);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(&back, s.settings());
    let mut t = Session::new();
    assert!(t.set_settings(back, None).is_empty());
    assert_eq!(t.shortcut("zoom-in").as_deref(), Some("Ctrl+I"));
    assert!((t.view().pick_tolerance() - tol).abs() < 1e-9, "the pick tolerance follows the setting");
    for (id, bad) in [
        ("set-display-options", json!({ "wheel_zoom": 100 })),
        ("set-sheet-sizes", json!({ "sizes": [] })),
        ("set-sheet-sizes", json!({ "sizes": [{ "name": "Tiny", "width": 1, "height": 1 }] })),
        ("set-property-definitions", json!({ "definitions": [{ "name": "A" }, { "name": "a" }] })),
        ("set-property-definitions", json!({ "definitions": [{ "name": "" }] })),
        ("set-text-editor", json!({ "font_size": 2 })),
    ] {
        assert!(s.execute(id, &bad).is_err(), "{id} {bad}");
    }
    exec(&mut s, "restore-default-settings", json!({}));
    assert_eq!(s.settings(), &SystemSettings::default());
    assert_eq!(load_settings(&path).0, SystemSettings::default());
}

#[test]
fn a_settings_file_that_cannot_be_written_or_read_never_stops_the_session() {
    let dir = scratch("settings-blocked");
    let mut s = Session::new();
    s.set_settings(SystemSettings::default(), Some(dir.clone()));
    let v = exec(&mut s, "set-text-editor", json!({ "monospace": true }));
    assert_eq!(v["saved"], json!(false), "a directory is in the way");
    assert!(s.settings().text_editor.monospace, "the setting is still in use");
    assert!(s.log().last().is_some_and(|l| l.contains("could not be saved")));
    let mut clash = SystemSettings::default();
    clash.keys.insert("redraw".into(), "G".into());
    clash.keys.insert("zoom-in".into(), "Ctrl+Q".into());
    clash.keys.insert("ghost".into(), "F9".into());
    clash.display.pick_pixels = 1e9;
    let warnings = s.set_settings(clash, None);
    assert_eq!(warnings.len(), 3, "{warnings:?}");
    assert_eq!(s.shortcut("redraw").as_deref(), Some("R"));
    assert_eq!(s.shortcut("zoom-in").as_deref(), Some("Ctrl+Q"));
    assert_eq!(s.settings().display.pick_pixels, 6.0);
}
