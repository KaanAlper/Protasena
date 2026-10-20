//! The window's layout follows Proteus 8 Schematic Capture: menus, toolbars, modes and status
//! fields in Proteus's order and places.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod common;

use common::*;
use protasena_engine::Mode;
use protasena_ui_egui::menus::model::MENUS;
use protasena_ui_egui::toolbars::TOOLBARS;

#[test]
fn menus_sit_left_to_right_in_proteus_s_order() {
    let h = harness(1600.0, 1000.0, 1.0);
    let xs: Vec<f32> = MENUS.iter().map(|m| find(&h, m.label, "menu").rect.left()).collect();
    assert!(xs.windows(2).all(|w| w[0] < w[1]), "{xs:?}");
    let names: Vec<&str> = MENUS.iter().map(|m| m.label).collect();
    assert_eq!(names, ["File", "Edit", "View", "Tool", "Design", "Graph", "Debug", "Library", "Template", "System", "Help"]);
}

#[test]
fn the_four_toolbars_follow_each_other() {
    let h = harness(1900.0, 1000.0, 1.0);
    let rects: Vec<egui::Rect> = TOOLBARS.iter().map(|(name, _)| region(&h, name)).collect();
    assert!(rects.windows(2).all(|w| w[0].right() <= w[1].left() && (w[0].top() - w[1].top()).abs() < 1.0), "one row, in order: {rects:?}");
    let tools = with_role(&h, "tool");
    let labels: Vec<&str> = tools.iter().map(|w| w.label.as_str()).collect();
    let at = |l: &str| labels.iter().position(|x| *x == l).unwrap_or_else(|| panic!("no tool {l}"));
    assert!(at("New Project") < at("Help") && at("Help") < at("Redraw Display") && at("Zoom To Area") < at("Undo Changes"));
    assert!(at("Decompose") < at("Wire Autorouter") && at("Wire Autorouter") < at("Electrical Rules Check"));
}

#[test]
fn a_narrow_window_wraps_the_toolbars_onto_a_second_row() {
    let h = harness(1100.0, 760.0, 1.0);
    let first = region(&h, "Global Toolbar");
    let last = region(&h, "Design Toolbar");
    assert!(last.top() > first.bottom() - 1.0, "{first:?} {last:?}");
    assert!(region(&h, "Toolbars").bottom() <= region(&h, "Tabs").top() + 0.5);
}

#[test]
fn the_mode_toolbar_lists_22_modes_top_to_bottom_with_unbuilt_ones_disabled() {
    let h = harness(1600.0, 1100.0, 1.0);
    let modes = with_role(&h, "mode");
    assert_eq!(modes.len(), 22);
    for (w, m) in modes.iter().zip(Mode::ALL) {
        assert_eq!(w.label, m.label());
        assert_eq!(w.enabled, m.is_built(), "{}", m.label());
    }
    assert!(modes.windows(2).all(|w| w[0].rect.bottom() <= w[1].rect.top() + 0.5));
    assert_eq!(modes.iter().filter(|w| w.selected == Some(true)).map(|w| w.label.as_str()).collect::<Vec<_>>(), ["Component Mode"]);
    let orient = region(&h, "Orientation toolbar");
    assert!(region(&h, "Mode toolbar").bottom() < orient.top());
    let labels: Vec<String> = with_role(&h, "tool").into_iter().filter(|w| orient.contains_rect(w.rect)).map(|w| w.label).collect();
    assert_eq!(labels, ["Rotate Clockwise", "Rotate Anti-clockwise", "X-Mirror", "Y-Mirror"]);
}

#[test]
fn areas_stack_like_proteus_s_window() {
    let h = harness(1600.0, 1000.0, 1.0);
    let (menu, bars, tabs, canvas, status) =
        (region(&h, "Menu bar"), region(&h, "Toolbars"), region(&h, "Tabs"), region(&h, "Editing window"), region(&h, "Status bar"));
    assert!(menu.bottom() <= bars.top() + 0.5 && bars.bottom() <= tabs.top() + 0.5 && tabs.bottom() <= canvas.top() + 0.5);
    assert!(canvas.bottom() <= status.top() + 0.5);
    let (modes, overview, selector) = (region(&h, "Mode toolbar"), region(&h, "Overview"), region(&h, "Object selector"));
    assert!(modes.right() <= overview.left() && overview.right() <= canvas.left() && overview.bottom() <= selector.top());
    assert_eq!(find(&h, "Schematic Capture", "tab").selected, Some(true));
}

#[test]
fn the_status_bar_reads_left_to_right_like_proteus_s() {
    let h = harness(1600.0, 1000.0, 1.0);
    let status = with_role(&h, "status");
    let x = |pred: &dyn Fn(&str) -> bool| status.iter().find(|w| pred(&w.label)).map(|w| w.rect.left()).unwrap_or_else(|| panic!("missing"));
    let play = x(&|l| l == "Play");
    let stop = x(&|l| l == "Stop");
    let messages = x(&|l| l == "No Messages");
    let variant = x(&|l| l == "Base Design");
    let sheet = x(&|l| l.starts_with("Sheet: Root sheet 1"));
    let coords = x(&|l| l.starts_with("x: "));
    let unit = x(&|l| l == "Unit: th");
    assert!(play < stop && stop < messages && messages < variant && variant < sheet && sheet < coords && coords < unit);
    assert!(status.iter().filter(|w| ["Play", "Step", "Pause", "Stop", "Base Design"].contains(&w.label.as_str())).all(|w| !w.enabled));
}

#[test]
fn the_object_selector_follows_the_mode() {
    let mut h = harness(1600.0, 1000.0, 1.0);
    h.state_mut().run("pick-device", serde_json::json!({ "pick": "RES" })).unwrap();
    h.run_steps(3);
    assert_eq!(find(&h, "RES", "list-item").selected, Some(true));
    assert!(find(&h, "Pick Devices", "button").enabled);
    h.state_mut().run("set-mode", serde_json::json!({ "mode": "terminals" })).unwrap();
    h.run_steps(3);
    let kinds: Vec<String> = with_role(&h, "list-item").into_iter().map(|w| w.label).collect();
    assert_eq!(kinds, ["DEFAULT", "INPUT", "OUTPUT", "BIDIR", "POWER", "GROUND", "BUS"]);
    assert!(!find(&h, "Pick Devices", "button").enabled, "Pick Terminals is not built");
}
